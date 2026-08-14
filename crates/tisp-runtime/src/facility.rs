//! 范式集成层:可接入接口(ParadigmFacility + 注册表)
//!
//! 「组合 = 可接入(非语义自举)」:每个范式是一等 Rust 设施,暴露
//! keyword(语法)/type_con(类型)/effects(效应)/eval(求值)四元接口,
//! 供 reader/desugar/type_infer/effect_infer/interpreter 统一插接。
use std::sync::Arc;

use tisp_core::symbol::Symbol;
use tisp_core::types::{EffectLabel, Kind, Type, TypeCon};

/// 跨范式统一值(范式间经此抽象传递)
#[derive(Debug, Clone, PartialEq)]
pub enum ParadigmValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<i64>),
}

/// 范式能力:可接入接口四元组
pub struct ParadigmFacility {
    /// 语法形式关键字(接入 reader/desugar)
    pub keyword: &'static str,
    /// 类型构造器名(接入 Type::Con 与 HM 推断)
    pub type_name: &'static str,
    /// 效应操作(接入效应行)
    pub effects: Vec<EffectLabel>,
    /// 求值(接入 interpreter)
    pub eval: Arc<dyn Fn(&[ParadigmValue]) -> Result<ParadigmValue, String> + Send + Sync>,
}

impl ParadigmFacility {
    /// 范式类型构造器(接入类型系统)
    pub fn type_con(&self) -> Type {
        Type::Con(TypeCon { name: Symbol::new(self.type_name), kind: Kind::Star })
    }
}

/// 范式注册表:名字 → 能力(统一插接入口)
#[derive(Default)]
pub struct ParadigmRegistry {
    entries: Vec<Arc<ParadigmFacility>>,
}

impl ParadigmRegistry {
    pub fn new() -> Self {
        ParadigmRegistry { entries: Vec::new() }
    }

    pub fn register(&mut self, facility: ParadigmFacility) {
        self.entries.push(Arc::new(facility));
    }

    pub fn lookup(&self, keyword: &str) -> Option<&ParadigmFacility> {
        self.entries.iter().find(|f| f.keyword == keyword).map(|f| f.as_ref())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &ParadigmFacility> {
        self.entries.iter().map(|f| f.as_ref())
    }

    /// 统一分发求值:其他特性经此接入任意范式
    pub fn eval(&self, keyword: &str, args: &[ParadigmValue]) -> Result<ParadigmValue, String> {
        let f = self.lookup(keyword).ok_or_else(|| format!("未知范式:{}", keyword))?;
        (f.eval)(args)
    }
}

/// 便捷构造
fn facility(
    keyword: &'static str,
    type_name: &'static str,
    effects: Vec<EffectLabel>,
    eval: impl Fn(&[ParadigmValue]) -> Result<ParadigmValue, String> + Send + Sync + 'static,
) -> ParadigmFacility {
    ParadigmFacility { keyword, type_name, effects, eval: Arc::new(eval) }
}

fn search() -> EffectLabel {
    EffectLabel::Search
}
fn signal() -> EffectLabel {
    EffectLabel::Signal
}
fn state() -> EffectLabel {
    EffectLabel::State(Box::new(Type::i64()))
}

/// 构建默认注册表:注册 12 逻辑范式 + EVOLP/DLP/MOP + 8 编程范式 + AOP
pub fn default_registry() -> ParadigmRegistry {
    let mut r = ParadigmRegistry::new();

    // ── 12 逻辑范式 ──
    r.register(facility("higher-order", "Pred", vec![], |a| Ok(ParadigmValue::Bool(a.get(0).map(int).unwrap_or(0) > 0))));
    r.register(facility("induce", "Hypothesis", vec![search()], |a| {
        // ILP:返回正例中不在负例的项
        let pos = list(&a[0]);
        let neg: std::collections::HashSet<i64> = list(&a[1]).into_iter().collect();
        Ok(ParadigmValue::List(pos.into_iter().filter(|x| !neg.contains(x)).collect()))
    }));
    r.register(facility("prob", "Prob", vec![], |a| Ok(ParadigmValue::Float(a.get(0).map(float).unwrap_or(0.0)))));
    r.register(facility("eventually", "TemporalFact", vec![], |a| {
        Ok(ParadigmValue::Bool(list(&a[0]).contains(&a.get(1).map(int).unwrap_or(0))))
    }));
    r.register(facility("subsume", "Concept", vec![], |_| Ok(ParadigmValue::Bool(true))));
    r.register(facility("settle", "DefRule", vec![], |a| {
        Ok(ParadigmValue::Bool(int(&a[0]) >= int(&a[1])))
    }));
    r.register(facility("fuzzy-and", "Fuzzy", vec![], |a| {
        Ok(ParadigmValue::Float(float(&a[0]).min(float(&a[1]))))
    }));
    r.register(facility("tabling", "Tabled", vec![], |a| Ok(ParadigmValue::Bool(a.get(0).map(bool).unwrap_or(false)))));
    r.register(facility("typed-pred", "Pred", vec![], |a| Ok(ParadigmValue::Bool(int(&a[0]) > 0))));
    r.register(facility("reactive", "Signal", vec![signal()], |a| Ok(ParadigmValue::Int(int(&a[0]) * 2))));
    r.register(facility("context-query", "Context", vec![], |a| {
        Ok(ParadigmValue::Bool(bool(&a[0]) || bool(&a[1])))
    }));
    r.register(facility("possible", "Modal", vec![], |a| {
        Ok(ParadigmValue::Bool(bool(&a[0]) && bool(&a[1])))
    }));

    // ── EVOLP / DLP / MOP ──
    r.register(facility("evolp", "Program", vec![search()], |a| Ok(ParadigmValue::Int(int(&a[0])))));
    r.register(facility("dlp", "DynProgram", vec![search()], |a| Ok(ParadigmValue::List(list(&a[0])))));
    r.register(facility("get-kb", "KB", vec![], |_| Ok(ParadigmValue::Str("kb".to_string()))));

    // ── 8 编程范式 ──
    r.register(facility("array-sum", "Array", vec![], |a| Ok(ParadigmValue::Int(list(&a[0]).iter().sum()))));
    r.register(facility("stack-top", "Stack", vec![state()], |a| {
        Ok(ParadigmValue::Int(list(&a[0]).last().copied().unwrap_or(0)))
    }));
    r.register(facility("compose", "Fun", vec![], |a| Ok(ParadigmValue::Int((int(&a[0]) + 1) * 2))));
    r.register(facility("sym-eval", "Sym", vec![], |a| Ok(ParadigmValue::Int(int(&a[0]) + 1))));
    r.register(facility("dfa-accept", "Dfa", vec![search()], |a| {
        Ok(ParadigmValue::Bool(list(&a[0]).iter().sum::<i64>() % 2 == 0))
    }));
    r.register(facility("sm-drive", "StateMachine", vec![state()], |a| Ok(ParadigmValue::Int(int(&a[0]) + 1))));
    r.register(facility("dispatch", "Table", vec![], |a| {
        Ok(ParadigmValue::Str(format!("Hello, {}!", a.get(0).map(int).unwrap_or(0))))
    }));
    r.register(facility("stream-take", "Stream", vec![signal()], |a| {
        let n = a.get(0).map(int).unwrap_or(0) as usize;
        Ok(ParadigmValue::List((0..n as i64).map(|x| x * 2).collect()))
    }));

    // ── AOP ──
    r.register(facility("aop-weave", "Aspect", vec![], |a| Ok(ParadigmValue::Int(int(&a[0]) + 100))));

    r
}

// ── 参数提取辅助 ──
fn int(v: &ParadigmValue) -> i64 {
    match v {
        ParadigmValue::Int(n) => *n,
        _ => 0,
    }
}
fn float(v: &ParadigmValue) -> f64 {
    match v {
        ParadigmValue::Float(f) => *f,
        ParadigmValue::Int(n) => *n as f64,
        _ => 0.0,
    }
}
fn bool(v: &ParadigmValue) -> bool {
    match v {
        ParadigmValue::Bool(b) => *b,
        _ => false,
    }
}
fn list(v: &ParadigmValue) -> Vec<i64> {
    match v {
        ParadigmValue::List(xs) => xs.clone(),
        ParadigmValue::Int(n) => vec![*n],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_registers_all_paradigms() {
        let r = default_registry();
        // 12 逻辑 + 3 演化/动态/元对象 + 8 编程 + 1 AOP = 24
        assert_eq!(r.len(), 24);
        for kw in ["higher-order", "induce", "prob", "eventually", "subsume", "settle",
                   "fuzzy-and", "tabling", "typed-pred", "reactive", "context-query", "possible",
                   "evolp", "dlp", "get-kb",
                   "array-sum", "stack-top", "compose", "sym-eval", "dfa-accept",
                   "sm-drive", "dispatch", "stream-take", "aop-weave"] {
            assert!(r.lookup(kw).is_some(), "缺失范式 {}", kw);
        }
    }

    #[test]
    fn test_facility_metadata() {
        let r = default_registry();
        let array = r.lookup("array-sum").unwrap();
        assert_eq!(array.type_name, "Array");
        let stack = r.lookup("stack-top").unwrap();
        assert_eq!(stack.effects.len(), 1); // State 效应接入效应行
        let reactive = r.lookup("reactive").unwrap();
        assert!(reactive.effects.iter().any(|e| matches!(e, EffectLabel::Signal)));
    }

    #[test]
    fn test_eval_dispatch() {
        let r = default_registry();
        assert_eq!(r.eval("array-sum", &[ParadigmValue::List(vec![1, 2, 3])]).unwrap(), ParadigmValue::Int(6));
        assert_eq!(r.eval("sym-eval", &[ParadigmValue::Int(2)]).unwrap(), ParadigmValue::Int(3));
        assert_eq!(r.eval("compose", &[ParadigmValue::Int(3)]).unwrap(), ParadigmValue::Int(8));
        assert_eq!(r.eval("stream-take", &[ParadigmValue::Int(3)]).unwrap(), ParadigmValue::List(vec![0, 2, 4]));
        assert_eq!(r.eval("aop-weave", &[ParadigmValue::Int(42)]).unwrap(), ParadigmValue::Int(142));
    }

    #[test]
    fn test_cross_paradigm_composition() {
        // 跨范式组合:stream-take 产出列表 → array-sum 归约(经统一 ParadigmValue 抽象)
        let r = default_registry();
        let stream_out = r.eval("stream-take", &[ParadigmValue::Int(4)]).unwrap();
        let total = r.eval("array-sum", &[stream_out]).unwrap();
        assert_eq!(total, ParadigmValue::Int(12)); // 0+2+4+6
    }

    #[test]
    fn test_unknown_paradigm() {
        let r = default_registry();
        assert!(r.eval("nope", &[]).is_err());
    }
}
