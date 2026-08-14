//! Everything-as-ADT:规则/程序/演化指令的一等数据模型(EVOLP/DLP 基底)
//!
//! 把逻辑程序的「规则」「约束项」「演化指令」建模为不可变 ADT 值,使
//! `Program` 可绑定、传递、匹配、增删查,演化操作成为纯函数(`Program -> Program`)。
use crate::symbol::Symbol;

/// 逻辑项:谓词原子 / 常量 / 复合项 / 逻辑变量
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LTerm {
    /// 整数常量(有限域/数值项)
    Int(i64),
    /// 字符串常量
    Str(String),
    /// 谓词原子或复合项:名 + 参数(0 元即命题原子)
    Fun(Symbol, Vec<LTerm>),
    /// 逻辑变量(grounding 时被替换)
    Var(Symbol),
}

impl LTerm {
    /// 命题原子(0 元谓词)
    pub fn atom(name: &str) -> Self {
        LTerm::Fun(Symbol::new(name), Vec::new())
    }
    /// 一元谓词原子
    pub fn atom1(name: &str, arg: LTerm) -> Self {
        LTerm::Fun(Symbol::new(name), vec![arg])
    }
    /// 整数项
    pub fn int(n: i64) -> Self {
        LTerm::Int(n)
    }
}

/// 字面量:正原子或(缺省)否定原子
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Pos(LTerm),
    Neg(LTerm),
}

/// 演化指令:`assert`(添加规则)/ `retract`(按规则 id 删除)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EvolInstr {
    Assert(Rule),
    Retract(Symbol),
}

/// 规则:`head :- body`,可携带演化指令
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub id: Symbol,
    pub head: LTerm,
    pub body: Vec<Literal>,
    pub evol: Vec<EvolInstr>,
}

impl Rule {
    /// 事实规则:`head` 无体
    pub fn fact(id: &str, head: LTerm) -> Self {
        Rule { id: Symbol::new(id), head, body: Vec::new(), evol: Vec::new() }
    }
    /// 普通规则:`head :- body`
    pub fn rule(id: &str, head: LTerm, body: Vec<Literal>) -> Self {
        Rule { id: Symbol::new(id), head, body, evol: Vec::new() }
    }
    /// 携带演化指令的规则
    pub fn evolving(id: &str, head: LTerm, body: Vec<Literal>, evol: Vec<EvolInstr>) -> Self {
        Rule { id: Symbol::new(id), head, body, evol }
    }
}

/// 不可变程序:当前生效的规则集合(按规则 id 索引)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub rules: im::HashMap<Symbol, Rule>,
}

impl Program {
    pub fn new() -> Self {
        Program { rules: im::HashMap::new() }
    }

    /// 从规则迭代器构造
    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Self {
        let mut p = Program::new();
        for r in rules {
            p.add(r);
        }
        p
    }

    /// 添加/覆盖规则
    pub fn add(&mut self, rule: Rule) {
        self.rules.insert(rule.id.clone(), rule);
    }

    /// 按 id 删除规则(返回旧规则)
    pub fn remove(&mut self, id: &Symbol) -> Option<Rule> {
        self.rules.remove(id)
    }

    /// 查询规则
    pub fn get(&self, id: &Symbol) -> Option<&Rule> {
        self.rules.get(id)
    }

    pub fn contains(&self, id: &Symbol) -> bool {
        self.rules.contains_key(id)
    }

    /// 规则总数
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// 迭代所有规则
    pub fn iter(&self) -> impl Iterator<Item = &Rule> {
        self.rules.values()
    }

    /// 折叠:对规则集合做纯函数归约(配合演化 foldl)
    pub fn fold<T>(&self, init: T, f: impl Fn(T, &Rule) -> T) -> T {
        self.rules.values().fold(init, f)
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_add_remove() {
        let mut p = Program::new();
        p.add(Rule::fact("r1", LTerm::atom("a")));
        assert_eq!(p.len(), 1);
        assert!(p.contains(&Symbol::new("r1")));
        assert!(p.remove(&Symbol::new("r1")).is_some());
        assert!(p.is_empty());
    }

    #[test]
    fn test_program_immutable_fold() {
        let p = Program::from_rules([
            Rule::fact("r1", LTerm::atom("a")),
            Rule::fact("r2", LTerm::atom("b")),
        ]);
        // fold 不改变原程序,原程序保持不可变
        let n = p.fold(0usize, |acc, _| acc + 1);
        assert_eq!(n, 2);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn test_rule_literal_structure() {
        let r = Rule::rule(
            "r",
            LTerm::atom("p"),
            vec![Literal::Pos(LTerm::atom("q")), Literal::Neg(LTerm::atom("r"))],
        );
        assert_eq!(r.head, LTerm::atom("p"));
        assert_eq!(r.body.len(), 2);
    }

    #[test]
    fn test_evol_instr() {
        let target = Rule::fact("s", LTerm::atom("t"));
        let r = Rule::evolving(
            "r",
            LTerm::atom("p"),
            vec![],
            vec![EvolInstr::Assert(target.clone()), EvolInstr::Retract(Symbol::new("old"))],
        );
        assert_eq!(r.evol.len(), 2);
        assert_eq!(r.evol[0], EvolInstr::Assert(target));
    }
}
