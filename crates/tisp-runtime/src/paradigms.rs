//! 12 类逻辑编程范式(组合优先)
//!
//! 每个范式优先复用既有特性(一等值/类型/效应/时序/模块),仅少量新增原语;
//! 全部为纯函数与 ADT 数据,不引入命令式逃逸。
use std::collections::HashMap;

use tisp_core::evolp::LTerm;
use tisp_core::symbol::Symbol;

// ── 5.1 高阶逻辑编程 ──

pub type TypedPred<T> = fn(&T) -> bool;

/// 谓词作为值被 `call` 调用
pub fn call<T>(p: TypedPred<T>, arg: &T) -> bool {
    p(arg)
}

/// 高阶组合子:对列表逐项应用谓词
pub fn filter_by<T>(p: TypedPred<T>, items: &[T]) -> Vec<&T> {
    items.iter().filter(|x| p(x)).collect()
}

// ── 5.2 归纳逻辑编程(ILP) ──

/// 从正/负例归纳:返回覆盖正例、排除负例的假设(自底向上)
pub fn induce(pos: &[LTerm], neg: &[LTerm]) -> Vec<LTerm> {
    let neg_set: im::HashSet<LTerm> = neg.iter().cloned().collect();
    pos.iter().filter(|a| !neg_set.contains(a)).cloned().collect()
}

// ── 5.3 概率逻辑编程(PLP) ──

/// 概率事实:(原子, 概率)
#[derive(Debug, Clone)]
pub struct ProbFact {
    pub atom: LTerm,
    pub prob: f64,
}

/// 目标的边际概率(独立事实,精确枚举)
pub fn marginal(query: &LTerm, facts: &[ProbFact]) -> f64 {
    let n = facts.len();
    let mut total = 0.0;
    for mask in 0..(1usize << n) {
        let mut p = 1.0;
        let mut query_true = false;
        for (i, f) in facts.iter().enumerate() {
            let is_true = mask & (1 << i) != 0;
            p *= if is_true { f.prob } else { 1.0 - f.prob };
            if is_true && &f.atom == query {
                query_true = true;
            }
        }
        if query_true {
            total += p;
        }
    }
    total
}

// ── 5.4 时序逻辑编程 ──

/// 时间索引事实序列
#[derive(Debug, Clone)]
pub struct TemporalKb {
    pub facts: Vec<(usize, LTerm)>,
}

impl TemporalKb {
    pub fn eventually(&self, atom: &LTerm) -> bool {
        self.facts.iter().any(|(_, a)| a == atom)
    }
    pub fn always(&self, atom: &LTerm) -> bool {
        !self.facts.is_empty() && self.facts.iter().all(|(_, a)| a == atom)
    }
    pub fn next(&self, t: usize, atom: &LTerm) -> bool {
        self.facts.iter().any(|(tt, a)| *tt == t + 1 && a == atom)
    }
}

// ── 5.5 描述逻辑编程 ──

/// 概念层次:(子概念, 父概念)
#[derive(Debug, Clone)]
pub struct Ontology {
    pub subsumes: Vec<(Symbol, Symbol)>,
}

impl Ontology {
    /// 个体概念是否(自反 + 传递)满足查询概念
    pub fn is_instance(&self, concept: &Symbol, query: &Symbol) -> bool {
        if concept == query {
            return true;
        }
        self.subsumes
            .iter()
            .any(|(sub, sup)| sub == concept && self.is_instance(sup, query))
    }
}

// ── 5.6 可废止逻辑编程 ──

/// 可废止规则:结论 + 优先级 + 是否否定(击败者)
#[derive(Debug, Clone)]
pub struct DefRule {
    pub head: LTerm,
    pub priority: u32,
    pub negated: bool,
}

/// 裁决:高优先级击败低优先级;negated 规则击败同结论正规则
pub fn settle(rules: &[DefRule]) -> im::HashSet<LTerm> {
    let mut prio: HashMap<LTerm, u32> = HashMap::new();
    let mut neg: HashMap<LTerm, u32> = HashMap::new();
    for r in rules {
        let map = if r.negated { &mut neg } else { &mut prio };
        map.entry(r.head.clone())
            .and_modify(|p| *p = (*p).max(r.priority))
            .or_insert(r.priority);
    }
    prio
        .into_iter()
        .filter(|(h, p)| neg.get(h).map_or(true, |np| *np <= *p))
        .map(|(h, _)| h)
        .collect()
}

// ── 5.7 模糊逻辑编程 ──

/// 模糊事实:真值度 [0,1]
#[derive(Debug, Clone)]
pub struct FuzzyFact {
    pub atom: LTerm,
    pub degree: f64,
}

pub fn degree(facts: &[FuzzyFact], atom: &LTerm) -> f64 {
    facts
        .iter()
        .find(|f| &f.atom == atom)
        .map(|f| f.degree)
        .unwrap_or(0.0)
}

/// 合取(min)
pub fn fuzzy_and(facts: &[FuzzyFact], atoms: &[LTerm]) -> f64 {
    atoms.iter().map(|a| degree(facts, a)).fold(1.0, f64::min)
}

/// 析取(max)
pub fn fuzzy_or(facts: &[FuzzyFact], atoms: &[LTerm]) -> f64 {
    atoms.iter().map(|a| degree(facts, a)).fold(0.0, f64::max)
}

// ── 5.8 表格逻辑编程(Tabled) ──

/// 表格化解器:记忆 + 进行中标记,使左递归终止
#[derive(Debug, Clone)]
pub struct Tabler {
    facts: im::HashSet<LTerm>,
    rules: Vec<(LTerm, Vec<LTerm>)>,
    memo: HashMap<LTerm, bool>,
    in_progress: im::HashSet<LTerm>,
}

impl Tabler {
    pub fn new(facts: &[LTerm], rules: Vec<(LTerm, Vec<LTerm>)>) -> Self {
        Tabler {
            facts: facts.iter().cloned().collect(),
            rules,
            memo: HashMap::new(),
            in_progress: im::HashSet::new(),
        }
    }

    pub fn prove(&mut self, goal: &LTerm) -> bool {
        if let Some(&r) = self.memo.get(goal) {
            return r;
        }
        if self.in_progress.contains(goal) {
            return false; // 左递归:正在求,暂时假设失败
        }
        if self.facts.contains(goal) {
            self.memo.insert(goal.clone(), true);
            return true;
        }
        self.in_progress.insert(goal.clone());
        // 取出匹配规则的体(避免同时借用 self.rules 与 self.prove)
        let bodies: Vec<Vec<LTerm>> = self
            .rules
            .iter()
            .filter(|(h, _)| h == goal)
            .map(|(_, b)| b.clone())
            .collect();
        let mut result = false;
        for body in bodies {
            if body.iter().all(|b| self.prove(b)) {
                result = true;
                break;
            }
        }
        self.in_progress.remove(goal);
        self.memo.insert(goal.clone(), result);
        result
    }
}

// ── 5.10 响应式逻辑编程 ──

/// 信号(FRP 值源)
#[derive(Debug, Clone)]
pub struct Signal<T: Clone> {
    pub value: T,
}

/// 响应式规则:从信号派生
#[derive(Debug, Clone)]
pub struct ReactiveRule<T: Clone, U: Clone> {
    pub derive: fn(&T) -> U,
}

impl<T: Clone, U: Clone> ReactiveRule<T, U> {
    pub fn eval(&self, sig: &Signal<T>) -> U {
        (self.derive)(&sig.value)
    }
}

// ── 5.11 情境逻辑编程 ──

/// 情境:名 + 父情境 + 规则集
#[derive(Debug, Clone)]
pub struct Context {
    pub name: Symbol,
    pub parent: Option<Symbol>,
    pub rules: im::HashSet<LTerm>,
}

/// 情境知识库:子情境继承父情境,同名谓词按情境隔离
#[derive(Debug, Clone)]
pub struct ContextKb {
    pub contexts: HashMap<Symbol, Context>,
}

impl ContextKb {
    pub fn query(&self, ctx: &Symbol, atom: &LTerm) -> bool {
        let mut cur = Some(ctx.clone());
        while let Some(c) = cur {
            match self.contexts.get(&c) {
                Some(ctxt) => {
                    if ctxt.rules.contains(atom) {
                        return true;
                    }
                    cur = ctxt.parent.clone();
                }
                None => break,
            }
        }
        false
    }
}

// ── 5.12 模态逻辑编程 ──

/// 可能世界与可达关系
#[derive(Debug, Clone)]
pub struct ModalKb {
    pub reach: Vec<(Symbol, Symbol)>,
    pub truths: HashMap<(Symbol, LTerm), bool>,
}

impl ModalKb {
    /// possible:存在可达世界使原子成立
    pub fn possible(&self, w: &Symbol, atom: &LTerm) -> bool {
        self.reach
            .iter()
            .any(|(from, to)| from == w && self.truths.get(&(to.clone(), atom.clone())) == Some(&true))
    }
    /// necessary:所有可达世界原子均成立
    pub fn necessary(&self, w: &Symbol, atom: &LTerm) -> bool {
        self.reach.iter().all(|(from, to)| {
            from != w || self.truths.get(&(to.clone(), atom.clone())) == Some(&true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(n: &i64) -> bool {
        *n > 0
    }

    #[test]
    fn test_higher_order() {
        assert!(call(pos, &3));
        let filtered: Vec<&i64> = filter_by(pos, &[1, -2, 3]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_ilp_induce() {
        let a = LTerm::atom("a");
        let b = LTerm::atom("b");
        let hyp = induce(&[a.clone(), b.clone()], &[b.clone()]);
        assert_eq!(hyp, vec![a]);
    }

    #[test]
    fn test_plp_marginal() {
        let heads = LTerm::atom("heads");
        let facts = vec![ProbFact { atom: heads.clone(), prob: 0.3 }];
        assert!((marginal(&heads, &facts) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_plp_independence() {
        let a = LTerm::atom("a");
        let b = LTerm::atom("b");
        let facts = vec![
            ProbFact { atom: a.clone(), prob: 0.5 },
            ProbFact { atom: b.clone(), prob: 0.5 },
        ];
        // P(a) = 0.5 与独立性语义一致
        assert!((marginal(&a, &facts) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_temporal() {
        let a = LTerm::atom("a");
        let b = LTerm::atom("b");
        let kb = TemporalKb { facts: vec![(0, a), (1, b.clone())] };
        assert!(kb.eventually(&b));
        assert!(kb.next(0, &b));
    }

    #[test]
    fn test_description_logic() {
        let ont = Ontology {
            subsumes: vec![(Symbol::new("Man"), Symbol::new("Person"))],
        };
        assert!(ont.is_instance(&Symbol::new("Man"), &Symbol::new("Person")));
        assert!(!ont.is_instance(&Symbol::new("Dog"), &Symbol::new("Person")));
    }

    #[test]
    fn test_defeasible() {
        let a = LTerm::atom("a");
        let rules = vec![
            DefRule { head: a.clone(), priority: 1, negated: false },
            DefRule { head: a.clone(), priority: 2, negated: true },
        ];
        // 更高优先级的否定规则击败 a
        assert!(!settle(&rules).contains(&a));
    }

    #[test]
    fn test_fuzzy() {
        let a = LTerm::atom("A");
        let b = LTerm::atom("B");
        let facts = vec![
            FuzzyFact { atom: a.clone(), degree: 0.7 },
            FuzzyFact { atom: b.clone(), degree: 0.5 },
        ];
        assert!((fuzzy_and(&facts, &[a.clone(), b.clone()]) - 0.5).abs() < 1e-9);
        assert!((fuzzy_or(&facts, &[a.clone(), b.clone()]) - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_tabled_terminates() {
        // 左递归 p :- p 使朴素递归无限循环;表格化终止
        let p = LTerm::atom("p");
        let mut t = Tabler::new(&[], vec![(p.clone(), vec![p.clone()])]);
        assert!(!t.prove(&p));
    }

    #[test]
    fn test_tabled_success() {
        let a = LTerm::atom("a");
        let b = LTerm::atom("b");
        let mut t = Tabler::new(&[a.clone()], vec![(b.clone(), vec![a.clone()])]);
        assert!(t.prove(&b));
    }

    #[test]
    fn test_reactive() {
        let rule: ReactiveRule<i64, i64> = ReactiveRule { derive: |x| x * 2 };
        let sig = Signal { value: 21 };
        assert_eq!(rule.eval(&sig), 42);
    }

    #[test]
    fn test_contextual() {
        let a = LTerm::atom("a");
        let mut kb = ContextKb { contexts: HashMap::new() };
        let mut parent_rules = im::HashSet::new();
        parent_rules.insert(a.clone());
        kb.contexts.insert(
            Symbol::new("parent"),
            Context { name: Symbol::new("parent"), parent: None, rules: parent_rules },
        );
        kb.contexts.insert(
            Symbol::new("child"),
            Context { name: Symbol::new("child"), parent: Some(Symbol::new("parent")), rules: im::HashSet::new() },
        );
        // 子情境继承父情境
        assert!(kb.query(&Symbol::new("child"), &a));
    }

    #[test]
    fn test_modal() {
        let w1 = Symbol::new("w1");
        let w2 = Symbol::new("w2");
        let p = LTerm::atom("p");
        let mut truths = HashMap::new();
        truths.insert((w2.clone(), p.clone()), true);
        let kb = ModalKb { reach: vec![(w1.clone(), w2.clone())], truths };
        assert!(kb.possible(&w1, &p));
    }
}
