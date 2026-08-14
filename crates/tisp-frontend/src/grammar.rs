//! §语法 DSL(drafts/draft):`::>` 声明式语法定义 + meta-tag 扫描器框架。
//! 解析 `左 ::> 右` 形式为语法表,支持 meta-tag 特殊形式与内置字符类标签,
//! 生成可逐字符匹配的扫描器。

use std::collections::HashMap;

/// 语法具体结构(右侧)——产生状态序列的元素
#[derive(Debug, Clone, PartialEq)]
pub enum Spec {
    /// `<tag>` 元标签:展开同名语法结构
    MetaTag(String),
    /// `\x` 字面转义
    Literal(char),
    /// `[x]` 可选
    Optional(Box<Spec>),
    /// `{x}` 重复 1 次及以上
    Repeat(Box<Spec>),
    /// `[{...}]` 重复 0 次及以上
    RepeatZeroOrMore(Box<Spec>),
    /// `| a b` / `|| a b c` 逻辑或(多个分支)
    Or(Vec<Spec>),
    /// 具体结构序列(空格分隔的多个元素)
    Seq(Vec<Spec>),
    /// `<nonterm>` 空格/换行分隔符(1+)
    NonTerm,
    /// `<error>` 永不满足
    Error,
}

/// 语法表:结构名 → 具体结构(同名多条视为「或」)
#[derive(Debug, Default)]
pub struct GrammarTable {
    rules: HashMap<String, Vec<Spec>>,
}

impl GrammarTable {
    pub fn rule_names(&self) -> impl Iterator<Item = &String> {
        self.rules.keys()
    }
}

/// 内置字符类标签名(带 < > 的完整名)
fn builtin_char_class(tag: &str) -> Option<fn(char) -> bool> {
    Some(match tag {
        "<ztonLetter>" => |c: char| c.is_ascii_digit(),
        "<atozLetter>" => |c: char| c.is_ascii_lowercase(),
        "<AtoZLetter>" => |c: char| c.is_ascii_uppercase(),
        "<ASCIILetter>" => |c: char| c.is_ascii_alphanumeric(),
        "<ASCIILetterAll>" => |c: char| c.is_ascii(),
        "<ASCIISpecLetter>" => |c: char| c.is_ascii() && !c.is_ascii_alphanumeric(),
        _ => return None,
    })
}

/// 解析 `::>` 语法定义文本为语法表
pub fn parse_grammar(source: &str) -> Result<GrammarTable, String> {
    let mut table = GrammarTable::default();
    for (idx, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (left, right) = line
            .split_once("::>")
            .ok_or_else(|| format!("第 {} 行缺少 '::>' 分隔符:{}", idx + 1, line))?;
        let name = left.trim().to_string();
        if name.is_empty() {
            return Err(format!("第 {} 行语法结构名为空", idx + 1));
        }
        let spec = parse_spec(right.trim()).map_err(|e| format!("第 {} 行:{}", idx + 1, e))?;
        table.rules.entry(name).or_default().push(spec);
    }
    Ok(table)
}

/// 字符级递归下降解析具体结构
fn parse_spec(text: &str) -> Result<Spec, String> {
    let mut p = CharParser { chars: text.chars().collect(), pos: 0 };
    let spec = p.parse_body()?;
    p.skip_ws();
    if p.pos != p.chars.len() {
        return Err(format!("有无法解析的剩余内容 '{}'", text[p.pos..].to_string()));
    }
    Ok(spec)
}

struct CharParser {
    chars: Vec<char>,
    pos: usize,
}

impl CharParser {
    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// 解析一个「整体」:前缀 `|`(Or,count(|)+1 个备选)或元素序列(Seq)
    fn parse_body(&mut self) -> Result<Spec, String> {
        self.skip_ws();
        if self.peek() == Some('|') {
            // 前缀 `|` / `||` …:count(|)+1 个备选,每个是单个元素
            let mut count = 0;
            while self.peek() == Some('|') {
                self.pos += 1;
                count += 1;
            }
            let mut alts = Vec::new();
            for _ in 0..=count {
                self.skip_ws();
                alts.push(self.parse_element()?);
            }
            Ok(Spec::Or(alts))
        } else {
            let mut elems = Vec::new();
            loop {
                self.skip_ws();
                match self.peek() {
                    None | Some(']') | Some('}') | Some('|') => break,
                    Some(_) => elems.push(self.parse_element()?),
                }
            }
            Ok(match elems.len() {
                0 => Spec::Seq(vec![]),
                1 => elems.into_iter().next().unwrap(),
                _ => Spec::Seq(elems),
            })
        }
    }

    fn parse_element(&mut self) -> Result<Spec, String> {
        self.skip_ws();
        match self.peek() {
            Some('[') => {
                self.pos += 1;
                self.skip_ws();
                // `[{...}]` 特化:0+ 重复
                if self.peek() == Some('{') {
                    self.pos += 1;
                    let inner = self.parse_body()?;
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.pos += 1;
                        self.skip_ws();
                        if self.peek() == Some(']') {
                            self.pos += 1;
                            return Ok(Spec::RepeatZeroOrMore(Box::new(inner)));
                        }
                    }
                    return Err("'[{...}]' 缺少结尾 '}]'".to_string());
                }
                // 普通 `[x]` 可选
                let inner = self.parse_body()?;
                self.skip_ws();
                if self.peek() == Some(']') {
                    self.pos += 1;
                    Ok(Spec::Optional(Box::new(inner)))
                } else {
                    Err("可选组缺少 ']'".to_string())
                }
            }
            Some('{') => {
                self.pos += 1;
                let inner = self.parse_body()?;
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.pos += 1;
                    Ok(Spec::Repeat(Box::new(inner)))
                } else {
                    Err("重复组缺少 '}'".to_string())
                }
            }
            Some('\\') => {
                self.pos += 1;
                match self.peek() {
                    Some(c) => {
                        self.pos += 1;
                        Ok(Spec::Literal(c))
                    }
                    None => Err("转义缺字符".to_string()),
                }
            }
            Some('<') => {
                self.pos += 1;
                let mut name = String::new();
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == '>' {
                        break;
                    }
                    name.push(c);
                }
                let tag = format!("<{}>", name);
                match tag.as_str() {
                    "<nonterm>" => Ok(Spec::NonTerm),
                    "<error>" => Ok(Spec::Error),
                    _ => Ok(Spec::MetaTag(tag)),
                }
            }
            // 裸字符视为字面量(如 `+` `-` `.` `(` `)` `'`;与 `\x` 转义等价)
            Some(c) => {
                self.pos += 1;
                Ok(Spec::Literal(c))
            }
            None => Err("语法结构意外结束".to_string()),
        }
    }
}

/// 扫描器:对输入逐字符按语法表匹配
pub struct Scanner<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Scanner { input, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    /// 匹配一个 Spec;成功则不回退(消费),失败回退到调用前位置
    pub fn match_spec(&mut self, table: &GrammarTable, spec: &Spec) -> bool {
        let start = self.pos;
        if self.match_spec_inner(table, spec) {
            true
        } else {
            self.pos = start;
            false
        }
    }

    fn match_spec_inner(&mut self, table: &GrammarTable, spec: &Spec) -> bool {
        match spec {
            Spec::MetaTag(tag) => {
                if let Some(pred) = builtin_char_class(tag) {
                    match self.input[self.pos..].chars().next() {
                        Some(c) if pred(c) => {
                            self.pos += c.len_utf8();
                            true
                        }
                        _ => false,
                    }
                } else {
                    match table.rules.get(tag.as_str()) {
                        Some(rules) => rules.iter().any(|r| self.match_spec_inner(table, r)),
                        None => false,
                    }
                }
            }
            Spec::Literal(c) => {
                if self.input[self.pos..].starts_with(*c) {
                    self.pos += c.len_utf8();
                    true
                } else {
                    false
                }
            }
            Spec::Optional(inner) => {
                let _ = self.match_spec_inner(table, inner);
                true
            }
            Spec::Repeat(inner) => {
                let mut count = 0;
                while self.match_spec_inner(table, inner) {
                    count += 1;
                }
                count >= 1
            }
            Spec::RepeatZeroOrMore(inner) => {
                while self.match_spec_inner(table, inner) {}
                true
            }
            Spec::Or(alts) => alts.iter().any(|a| self.match_spec_inner(table, a)),
            Spec::Seq(elems) => {
                let start = self.pos;
                for e in elems {
                    if !self.match_spec_inner(table, e) {
                        self.pos = start;
                        return false;
                    }
                }
                true
            }
            Spec::NonTerm => {
                let mut consumed = false;
                while let Some(c) = self.input[self.pos..].chars().next() {
                    if c.is_whitespace() {
                        self.pos += c.len_utf8();
                        consumed = true;
                    } else {
                        break;
                    }
                }
                consumed
            }
            Spec::Error => false,
        }
    }

    /// 按规则名匹配整条规则,成功返回匹配到的文本切片
    pub fn match_rule<'b>(&mut self, table: &'b GrammarTable, name: &str) -> Option<&'a str> {
        let start = self.pos;
        let rules = table.rules.get(name)?;
        if rules.iter().any(|r| self.match_spec_inner(table, r)) {
            Some(&self.input[start..self.pos])
        } else {
            self.pos = start;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_grammar() {
        let src = "identifier ::> <atozLetter> [{<ASCIILetter>}]\n\
                   integer ::> [| + -]{<ztonLetter>}";
        let table = parse_grammar(src).unwrap();
        assert_eq!(table.rules.len(), 2);
        let mut s = Scanner::new("abc1");
        assert_eq!(s.match_rule(&table, "identifier"), Some("abc1"));
    }

    #[test]
    fn test_integer_scan() {
        let table = parse_grammar("integer ::> [| + -]{<ztonLetter>}").unwrap();
        let mut s = Scanner::new("-42");
        assert_eq!(s.match_rule(&table, "integer"), Some("-42"));
        let mut s2 = Scanner::new("abc");
        assert_eq!(s2.match_rule(&table, "integer"), None);
    }

    #[test]
    fn test_float_or() {
        let table = parse_grammar("float ::> [| + -]{<ztonLetter>}.{<ztonLetter>}").unwrap();
        let mut s = Scanner::new("3.14");
        assert_eq!(s.match_rule(&table, "float"), Some("3.14"));
    }

    #[test]
    fn test_error_tag_never_matches() {
        let table = parse_grammar("bad ::> <error>").unwrap();
        let mut s = Scanner::new("x");
        assert_eq!(s.match_rule(&table, "bad"), None);
    }

    #[test]
    fn test_repeat_zero_or_more() {
        // [{...}] 0+ 重复:空序列也匹配
        let table = parse_grammar("vec ::> \\[ [{<ASCIILetter>}] \\]").unwrap();
        let mut s = Scanner::new("[]");
        assert_eq!(s.match_rule(&table, "vec"), Some("[]"));
        let mut s2 = Scanner::new("[ab1]");
        assert_eq!(s2.match_rule(&table, "vec"), Some("[ab1]"));
    }
}
