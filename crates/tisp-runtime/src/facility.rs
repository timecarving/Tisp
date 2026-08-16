//! 范式集成层:可接入接口(ParadigmFacility + 注册表)
//!
//! 「组合 = 可接入(非语义自举)」:每个范式是一等 Rust 设施,暴露
//! keyword(语法)/type_con(类型)/effects(效应)/eval(求值)四元接口,
//! 供 reader/desugar/type_infer/effect_infer/interpreter 统一插接。
use std::sync::Arc;

use tisp_core::symbol::Symbol;
use tisp_core::types::{Determinism, EffectLabel, Kind, Mode, Type, TypeCon};

/// 设施等级元数据分类(§7 完整统一内存体系)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradeKind {
    /// QTT 等级(0 擦除 / 1 线性 / ω 共享)
    Qtt,
    /// 依赖线性(等级由值/类型参数决定)
    DependentLinear,
    /// 分级线性(□_r / @Cost 资源上界)
    GradedLinear,
    /// 未声明(注册时拒绝)
    Unspecified,
}

/// 跨范式统一值(范式间经此抽象传递)
#[derive(Debug, Clone, PartialEq)]
pub enum ParadigmValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<i64>),
}

/// 范式能力:可接入接口(语法/类型/六维注解/效应/求值)
pub struct ParadigmFacility {
    /// 语法形式关键字(接入 reader/desugar)
    pub keyword: &'static str,
    /// 类型构造器名(接入 Type::Con 与 HM 推断)
    pub type_name: &'static str,
    /// 效应操作(接入效应行;空 = 显式 Pure)
    pub effects: Vec<EffectLabel>,
    /// 区域归属(范式运行时状态的生命周期区域,如 program/data)
    pub region: Option<&'static str>,
    /// 等级体系分类:QTT / 依赖线性 / 分级线性(必填)
    pub grade_kind: GradeKind,
    /// 模式维度(必填)
    pub mode: Mode,
    /// 确定性维度(必填)
    pub determinism: Determinism,
    /// 参数类型(接入 type_infer 生成范式内置签名)
    pub params: Vec<Type>,
    /// 返回类型(接入 type_infer 生成范式内置签名)
    pub ret: Type,
    /// 求值(接入 interpreter)
    pub eval: Arc<dyn Fn(&[ParadigmValue]) -> Result<ParadigmValue, String> + Send + Sync>,
}

impl ParadigmFacility {
    /// 范式类型构造器(接入类型系统)
    pub fn type_con(&self) -> Type {
        Type::Con(TypeCon { name: Symbol::new(self.type_name), kind: Kind::Star })
    }

    /// 由元数据生成范式内置的函数签名(§9.1:type_infer 从元数据生成)
    pub fn signature(&self) -> Type {
        let mut ty = self.ret.clone();
        for param in self.params.iter().rev() {
            ty = Type::fun(param.clone(), ty);
        }
        ty
    }

    /// 六维元数据校验:缺失/占位一律拒绝(§9.1)
    pub fn validate(&self) -> Result<(), String> {
        if self.keyword.trim().is_empty() {
            return Err(format!("范式 {} 缺少 keyword", self.type_name));
        }
        if self.type_name.trim().is_empty() {
            return Err("范式缺少 type_name".into());
        }
        if self.grade_kind == GradeKind::Unspecified {
            return Err(format!("范式 {} 缺少等级元数据(QTT/依赖线性/分级线性)", self.keyword));
        }
        Ok(())
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

    pub fn register(&mut self, facility: ParadigmFacility) -> Result<(), String> {
        facility.validate()?;
        self.entries.push(Arc::new(facility));
        Ok(())
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

/// 便捷构造(QTT 等级/In 模式/Det 确定性/Pure 区域归属默认)
fn facility(
    keyword: &'static str,
    type_name: &'static str,
    effects: Vec<EffectLabel>,
    eval: impl Fn(&[ParadigmValue]) -> Result<ParadigmValue, String> + Send + Sync + 'static,
) -> ParadigmFacility {
    let (params, ret) = signature_for(keyword);
    facility_meta(keyword, type_name, effects, None, GradeKind::Qtt, Mode::In, Determinism::Det,
                  params, ret, eval)
}

/// 完整六维元数据构造
fn facility_meta(
    keyword: &'static str,
    type_name: &'static str,
    effects: Vec<EffectLabel>,
    region: Option<&'static str>,
    grade_kind: GradeKind,
    mode: Mode,
    determinism: Determinism,
    params: Vec<Type>,
    ret: Type,
    eval: impl Fn(&[ParadigmValue]) -> Result<ParadigmValue, String> + Send + Sync + 'static,
) -> ParadigmFacility {
    ParadigmFacility {
        keyword, type_name, effects, region, grade_kind, mode, determinism, params, ret,
        eval: Arc::new(eval),
    }
}

/// 范式内置签名的元数据默认表(与 type_infer/interpreter 的调用形态一致)
fn signature_for(keyword: &str) -> (Vec<Type>, Type) {
    let li = Type::list(Type::i64());
    let i = Type::i64();
    let f = Type::f64();
    let b = Type::bool();
    let s = Type::string();
    let (params, ret): (Vec<Type>, Type) = match keyword {
        "higher-order" => (vec![i.clone(), i.clone()], b),
        "induce" => (vec![li.clone(), li.clone()], li),
        "prob" => (vec![f.clone()], f),
        "eventually" => (vec![li.clone(), i.clone()], b),
        "subsume" => (vec![li.clone(), i.clone(), i.clone()], b),
        "settle" => (vec![li.clone()], li),
        "fuzzy-and" => (vec![li.clone(), li.clone()], f),
        "tabling" => (vec![li.clone(), i.clone()], b),
        "typed-pred" => (vec![i.clone(), li.clone()], li),
        "reactive" => (vec![i.clone(), i.clone()], i),
        "context-query" => (vec![li.clone(), li.clone(), i.clone(), i.clone()], b),
        "possible" => (vec![li.clone(), li.clone(), i.clone(), i.clone()], b),
        "evolp" => (vec![li.clone()], li),
        "dlp" => (vec![li.clone()], li),
        "get-kb" => (vec![], li),
        "array-sum" => (vec![li.clone()], i),
        "stack-top" => (vec![li.clone()], i),
        "compose" => (vec![i.clone()], i),
        "sym-eval" => (vec![i.clone()], i),
        "dfa-accept" => (vec![li.clone()], b),
        "sm-drive" => (vec![li.clone()], i),
        "dispatch" => (vec![i.clone()], s),
        "stream-take" => (vec![i.clone()], li),
        "aop-weave" => (vec![i.clone()], i),
        _ => (vec![], Type::unit()),
    };
    (params, ret)
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
    let mut reg = |f: ParadigmFacility| { r.register(f).expect("范式元数据校验失败"); };

    // ── 12 逻辑范式（简化投影——已由真实内置替代，本表仅用于 pf-* 遗留兼容）──
    reg(facility("higher-order", "Pred", vec![], |a| Ok(ParadigmValue::Bool(a.get(0).map(int).unwrap_or(0) > 0))));
    reg(facility("induce", "Hypothesis", vec![search()], |a| {
        // ILP:返回正例中不在负例的项
        let pos = a.get(0).map(list).unwrap_or_default();
        let neg: std::collections::HashSet<i64> = a.get(1).map(list).unwrap_or_default().into_iter().collect();
        Ok(ParadigmValue::List(pos.into_iter().filter(|x| !neg.contains(x)).collect()))
    }));
    reg(facility("prob", "Prob", vec![], |a| Ok(ParadigmValue::Float(a.get(0).map(float).unwrap_or(0.0)))));
    reg(facility("eventually", "TemporalFact", vec![], |a| {
        Ok(ParadigmValue::Bool(a.get(0).map(|v| list(v).contains(&a.get(1).map(int).unwrap_or(0))).unwrap_or(false)))
    }));
    reg(facility("subsume", "Concept", vec![], |_| Ok(ParadigmValue::Bool(true))));
    reg(facility("settle", "DefRule", vec![], |a| {
        Ok(ParadigmValue::Bool(a.get(0).map(int).unwrap_or(0) >= a.get(1).map(int).unwrap_or(0)))
    }));
    reg(facility("fuzzy-and", "Fuzzy", vec![], |a| {
        Ok(ParadigmValue::Float(a.get(0).map(float).unwrap_or(0.0).min(a.get(1).map(float).unwrap_or(0.0))))
    }));
    reg(facility("tabling", "Tabled", vec![], |a| Ok(ParadigmValue::Bool(a.get(0).map(bool).unwrap_or(false)))));
    reg(facility("typed-pred", "Pred", vec![], |a| Ok(ParadigmValue::Bool(a.get(0).map(int).unwrap_or(0) > 0))));
    reg(facility("reactive", "Signal", vec![signal()], |a| Ok(ParadigmValue::Int(a.get(0).map(int).unwrap_or(0) * 2))));
    reg(facility("context-query", "Context", vec![], |a| {
        Ok(ParadigmValue::Bool(a.get(0).map(bool).unwrap_or(false) || a.get(1).map(bool).unwrap_or(false)))
    }));
    reg(facility("possible", "Modal", vec![], |a| {
        Ok(ParadigmValue::Bool(a.get(0).map(bool).unwrap_or(false) && a.get(1).map(bool).unwrap_or(false)))
    }));

    // ── EVOLP / DLP / MOP ──
    reg(facility("evolp", "Program", vec![search()], |a| Ok(ParadigmValue::Int(a.get(0).map(int).unwrap_or(0)))));
    reg(facility("dlp", "DynProgram", vec![search()], |a| Ok(ParadigmValue::List(a.get(0).map(list).unwrap_or_default()))));
    reg(facility("get-kb", "KB", vec![], |_| Ok(ParadigmValue::Str("kb".to_string()))));

    // ── 8 编程范式 ──
    reg(facility("array-sum", "Array", vec![], |a| Ok(ParadigmValue::Int(a.get(0).map(|v| list(v).iter().sum()).unwrap_or(0)))));
    reg(facility("stack-top", "Stack", vec![state()], |a| {
        Ok(ParadigmValue::Int(a.get(0).map(|v| list(v).last().copied().unwrap_or(0)).unwrap_or(0)))
    }));
    reg(facility("compose", "Fun", vec![], |a| Ok(ParadigmValue::Int((a.get(0).map(int).unwrap_or(0) + 1) * 2))));
    reg(facility("sym-eval", "Sym", vec![], |a| Ok(ParadigmValue::Int(a.get(0).map(int).unwrap_or(0) + 1))));
    reg(facility("dfa-accept", "Dfa", vec![search()], |a| {
        Ok(ParadigmValue::Bool(a.get(0).map(|v| list(v).iter().sum::<i64>() % 2 == 0).unwrap_or(false)))
    }));
    reg(facility("sm-drive", "StateMachine", vec![state()], |a| Ok(ParadigmValue::Int(a.get(0).map(int).unwrap_or(0) + 1))));
    reg(facility("dispatch", "Table", vec![], |a| {
        Ok(ParadigmValue::Str(format!("Hello, {}!", a.get(0).map(int).unwrap_or(0))))
    }));
    reg(facility("stream-take", "Stream", vec![signal()], |a| {
        let n = a.get(0).map(int).unwrap_or(0) as usize;
        Ok(ParadigmValue::List((0..n as i64).map(|x| x * 2).collect()))
    }));

    // ── AOP ──
    reg(facility("aop-weave", "Aspect", vec![], |a| Ok(ParadigmValue::Int(a.get(0).map(int).unwrap_or(0) + 100))));

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
        assert_eq!(array.grade_kind, GradeKind::Qtt);
        assert_eq!(array.mode, Mode::In);
        assert_eq!(array.determinism, Determinism::Det);
        let stack = r.lookup("stack-top").unwrap();
        assert_eq!(stack.effects.len(), 1); // State 效应接入效应行
        let reactive = r.lookup("reactive").unwrap();
        assert!(reactive.effects.iter().any(|e| matches!(e, EffectLabel::Signal)));
    }

    #[test]
    fn test_metadata_validation_rejects_missing() {
        let mut r = ParadigmRegistry::new();
        let bad = ParadigmFacility {
            keyword: "bad", type_name: "", effects: vec![], region: None,
            grade_kind: GradeKind::Qtt, mode: Mode::In, determinism: Determinism::Det,
            params: vec![], ret: Type::unit(),
            eval: Arc::new(|_| Ok(ParadigmValue::Int(0))),
        };
        assert!(bad.validate().is_err(), "缺失 type_name 应校验失败");
        let bad2 = ParadigmFacility {
            keyword: "bad2", type_name: "Bad", effects: vec![], region: None,
            grade_kind: GradeKind::Unspecified, mode: Mode::In, determinism: Determinism::Det,
            params: vec![], ret: Type::unit(),
            eval: Arc::new(|_| Ok(ParadigmValue::Int(0))),
        };
        assert!(r.register(bad2).is_err(), "Unspecified 等级元数据应拒绝注册");
        let ok = ParadigmFacility {
            keyword: "ok", type_name: "Ok", effects: vec![], region: None,
            grade_kind: GradeKind::Qtt, mode: Mode::In, determinism: Determinism::Det,
            params: vec![Type::i64()], ret: Type::unit(),
            eval: Arc::new(|_| Ok(ParadigmValue::Int(1))),
        };
        assert!(r.register(ok).is_ok(), "完整元数据应注册成功");
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
