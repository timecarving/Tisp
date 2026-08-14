use logos::Logos;
use tisp_core::span::Span;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r";[^\n]*")]
// §3.2 块注释 #| ... |#(可跨行;内容为 非| 或 |后非# 的序列)
#[logos(skip r"#\|([^|]|\|[^#])*\|#")]
pub enum Token {
    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("[")] LBracket,
    #[token("]")] RBracket,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("'")] Quote,
    #[token("`")] SyntaxQuote,
    #[token("~@")] UnquoteSplice,
    #[token("~")] Unquote,
    #[token("#")] Hash,
    #[token("@")] At,
    #[token(".")] Dot,
    #[token(",")] Comma,
    #[token(":")] Colon,
    #[token("->")] Arrow,
    #[token("|")] Pipe,
    // ⃝ (U+20DD) 时态算子:⃝ A = 下一时刻可用的值(§18.1)
    #[token("⃝")] Next,
    // □ (U+25A1) 分级必然模态算子(§11.2):(□_level a) 中 □ 后随等级下标
    #[token("□")] Necessity,

    #[regex(r"-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", priority = 3, callback = |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r"-?[0-9]+", priority = 3, callback = |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        // §3.4 字符串转义解码:\n \t \r \\ \" \0
        let raw = &s[1..s.len()-1];
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('0') => out.push('\0'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(other) => { out.push('\\'); out.push(other); }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        Some(out)
    })]
    Str(String),

    #[regex(r"\\(newline|space|tab|.)", parse_char_literal)]
    Char(char),

    #[regex(r":[a-zA-Z_!\-?+*/<>=&%#][a-zA-Z0-9_!\-?+*/<>=&%#.:]*", priority = 2, callback = |lex| {
        let s = lex.slice();
        Some(s[1..].to_string())
    })]
    Keyword(String),

    // 第一字符类含 ':' 以支持 :: / ::: 等构造器名(§18.2 Stream 的 (::: ...));
    // Keyword 的 priority 更高,`:foo` 仍归 Keyword
    #[regex(r"[a-zA-Z_!\-?+*/<>=&%:][a-zA-Z0-9_!\-?+*/<>=&%.:]*", priority = 1, callback = |lex| lex.slice().to_string())]
    Ident(String),

    #[token("true", |_| true)] #[token("false", |_| false)]
    Bool(bool),

    #[token("nil")] Nil,
}

fn parse_char_literal(lex: &mut logos::Lexer<Token>) -> Option<char> {
    let s = lex.slice();
    match &s[1..] {
        "newline" => Some('\n'), "space" => Some(' '), "tab" => Some('\t'),
        other => other.chars().next(),
    }
}

#[derive(Debug, Clone)]
pub struct SpannedToken { pub token: Token, pub span: Span }

pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let span_range = lexer.span();
        let span = Span::new(span_range.start, span_range.end);
        match result {
            Ok(token) => tokens.push(SpannedToken { token, span }),
            Err(()) => return Err(LexError { span, input: input[span_range].to_string() }),
        }
    }
    Ok(tokens)
}

#[derive(Debug, Clone)]
pub struct LexError { pub span: Span, pub input: String }
impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unexpected character '{}' at {}", self.input, self.span)
    }
}
impl std::error::Error for LexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_parens() {
        let tokens = tokenize("()").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::LParen));
        assert!(matches!(tokens[1].token, Token::RParen));
    }

    #[test] fn test_colon() {
        let tokens = tokenize(":").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token, Token::Colon));
    }

    #[test] fn test_arrow() {
        let tokens = tokenize("->").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token, Token::Arrow));
    }

    #[test] fn test_pipe() {
        let tokens = tokenize("|").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token, Token::Pipe));
    }

    #[test] fn test_keyword_vs_colon() {
        let tokens = tokenize(": :i64").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Colon));
        assert!(matches!(tokens[1].token, Token::Keyword(ref s) if s == "i64"));
    }

    #[test] fn test_integers() {
        let tokens = tokenize("42 -7 0").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].token, Token::Int(42)));
        assert!(matches!(tokens[1].token, Token::Int(-7)));
        assert!(matches!(tokens[2].token, Token::Int(0)));
    }

    #[test] fn test_booleans() {
        let tokens = tokenize("true false").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Bool(true)));
        assert!(matches!(tokens[1].token, Token::Bool(false)));
    }

    #[test] fn test_nil() {
        let tokens = tokenize("nil").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token, Token::Nil));
    }

    #[test] fn test_comment() {
        let tokens = tokenize("42 ; comment\n 43").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Int(42)));
        assert!(matches!(tokens[1].token, Token::Int(43)));
    }

    #[test] fn test_type_annotation() {
        let tokens = tokenize("x : i64").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].token, Token::Ident(_)));
        assert!(matches!(tokens[1].token, Token::Colon));
        assert!(matches!(tokens[2].token, Token::Ident(_)));
    }

    #[test] fn test_return_type() {
        let tokens = tokenize("-> i64").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Arrow));
        assert!(matches!(tokens[1].token, Token::Ident(_)));
    }

    #[test] fn test_necessity() {
        let tokens = tokenize("□_level").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Necessity));
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == "_level"));
    }

    #[test] fn test_at() {
        let tokens = tokenize("@1").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::At));
        assert!(matches!(tokens[1].token, Token::Int(1)));
    }
}
