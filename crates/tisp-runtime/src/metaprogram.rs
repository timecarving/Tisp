/// Phase 19: Metaprogramming — CompileTime MOP + AOP + Compiler Macros
use std::sync::Arc;
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

// ── CompileTime computation ──
#[derive(Debug, Clone)] pub struct CompileTime<T>(pub T);
impl<T> CompileTime<T> { pub fn eval(val: T) -> Self { CompileTime(val) } pub fn get(&self) -> &T { &self.0 } }

// ── Type reflection ──
#[derive(Debug, Clone, PartialEq)] pub struct TypeInfo { pub name: String, pub size: usize, pub align: usize }
#[derive(Debug, Clone)] pub enum TypeKind { Int, Float, Bool, String, Struct(Vec<(String, TypeInfo)>), Enum(Vec<String>), Ptr(Box<TypeInfo>) }
pub type CodeGen = Arc<dyn Fn(&TypeInfo) -> Vec<u8> + Send + Sync>;

// ── Clojure-style MOP ──
#[derive(Debug, Clone)] pub struct Meta {
    pub doc: Option<String>, pub deprecated: Option<String>,
    pub author: Option<String>, pub since: Option<String>, pub custom: HashMap<String, String>,
}
impl Meta {
    pub fn new() -> Self { Meta { doc: None, deprecated: None, author: None, since: None, custom: HashMap::new() } }
    pub fn with_doc(mut self, d: &str) -> Self { self.doc = Some(d.into()); self }
    pub fn with_deprecated(mut self, d: &str) -> Self { self.deprecated = Some(d.into()); self }
    pub fn with_author(mut self, a: &str) -> Self { self.author = Some(a.into()); self }
    pub fn get(&self, k: &str) -> Option<&String> { self.custom.get(k) }
    pub fn set(&mut self, k: &str, v: &str) { self.custom.insert(k.into(), v.into()); }
}

#[derive(Debug, Clone)] pub struct MetaSymbol { pub name: String, pub meta: Meta }

pub trait Protocol { fn name(&self) -> &str; fn method_count(&self) -> usize; }
#[derive(Debug, Clone)] pub struct ExtensibleProtocol { pub name: String, pub methods: Vec<String> }
impl Protocol for ExtensibleProtocol { fn name(&self) -> &str { &self.name } fn method_count(&self) -> usize { self.methods.len() } }

#[derive(Debug, Clone, PartialEq)] pub enum CompileTimeValue { Int(i64), Float(f64), Bool(bool), Str(String), Type(TypeInfo), Nil }
#[derive(Clone)] pub struct TypeExtension {
    pub type_name: String, pub protocol: ExtensibleProtocol,
    pub impls: HashMap<String, Arc<dyn Fn(&[CompileTimeValue]) -> CompileTimeValue + Send + Sync>>,
}

// ── AOP ──
#[derive(Debug, Clone)] pub struct Pointcut { pub name_pattern: Option<String>, pub annotation: Option<String> }
impl Pointcut {
    pub fn by_name(p: &str) -> Self { Pointcut { name_pattern: Some(p.into()), annotation: None } }
    pub fn matches_name(&self, n: &str) -> bool { self.name_pattern.as_ref().map_or(true, |p| n.contains(p.as_str())) }
}

#[derive(Debug, Clone)] pub struct AdviceContext { pub fn_name: String, pub args: Vec<CompileTimeValue>, pub meta: Meta }

#[derive(Clone)] pub enum Advice {
    Before(Arc<dyn Fn(&AdviceContext) + Send + Sync>),
    After(Arc<dyn Fn(&AdviceContext) + Send + Sync>),
    Around(Arc<dyn Fn(&AdviceContext, &dyn Fn() -> CompileTimeValue) -> CompileTimeValue + Send + Sync>),
}

#[derive(Clone)] pub struct Aspect { pub pointcut: Pointcut, pub advice: Advice }

pub struct AspectWeaver { aspects: Vec<Aspect> }
impl AspectWeaver {
    pub fn new() -> Self { AspectWeaver { aspects: Vec::new() } }
    pub fn add_aspect(&mut self, a: Aspect) { self.aspects.push(a); }
    pub fn weave_before(&self, ctx: &AdviceContext) {
        for a in &self.aspects { if a.pointcut.matches_name(&ctx.fn_name) { if let Advice::Before(f) = &a.advice { f(ctx); } } }
    }
    pub fn weave_after(&self, ctx: &AdviceContext) {
        for a in &self.aspects { if a.pointcut.matches_name(&ctx.fn_name) { if let Advice::After(f) = &a.advice { f(ctx); } } }
    }
    pub fn weave_around(&self, ctx: &AdviceContext, p: &dyn Fn() -> CompileTimeValue) -> Option<CompileTimeValue> {
        for a in &self.aspects { if a.pointcut.matches_name(&ctx.fn_name) { if let Advice::Around(f) = &a.advice { return Some(f(ctx, p)); } } }
        None
    }
}

// ── Compiler Macros ──
#[derive(Debug, Clone)] pub struct CompileTimeEnv {
    pub target_arch: String, pub opt_level: u8, pub debug: bool,
    pub type_info: HashMap<String, TypeInfo>, pub defined: Vec<String>,
}
impl CompileTimeEnv {
    pub fn new() -> Self { CompileTimeEnv { target_arch: std::env::consts::ARCH.into(), opt_level: 2, debug: false, type_info: HashMap::new(), defined: vec![] } }
    pub fn is_const(&self, v: &CompileTimeValue) -> bool { matches!(v, CompileTimeValue::Int(_) | CompileTimeValue::Float(_) | CompileTimeValue::Bool(_)) }
    pub fn const_val(&self, v: &CompileTimeValue) -> Option<i64> { match v { CompileTimeValue::Int(n) => Some(*n), _ => None } }
}

#[derive(Clone)] pub struct CompilerMacro { pub name: String, pub arity: usize, pub expander: Arc<dyn Fn(&[CompileTimeValue], &CompileTimeEnv) -> Option<CompileTimeValue> + Send + Sync> }
pub struct MacroRegistry { macros: Vec<CompilerMacro> }
impl MacroRegistry {
    pub fn new() -> Self { MacroRegistry { macros: Vec::new() } }
    pub fn register(&mut self, name: &str, arity: usize, f: impl Fn(&[CompileTimeValue], &CompileTimeEnv) -> Option<CompileTimeValue> + Send + Sync + 'static) {
        self.macros.push(CompilerMacro { name: name.into(), arity, expander: Arc::new(f) });
    }
    pub fn try_expand(&self, name: &str, args: &[CompileTimeValue], env: &CompileTimeEnv) -> Option<CompileTimeValue> {
        for m in &self.macros { if m.name == name && m.arity == args.len() { return (m.expander)(args, env); } } None
    }
}
pub fn comptime<T: Clone>(f: impl FnOnce() -> T) -> CompileTime<T> { CompileTime::eval(f()) }

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn test_comptime() { assert_eq!(*comptime(|| 42).get(), 42); }
    #[test] fn test_meta() { let m = Meta::new().with_doc("adds two numbers").with_author("Tisp"); assert!(m.doc.is_some()); }
    #[test] fn test_macro() {
        let mut r = MacroRegistry::new(); let e = CompileTimeEnv::new();
        r.register("+", 2, |a, env| if env.is_const(&a[0]) && env.const_val(&a[0]) == Some(0) { Some(a[1].clone()) } else { None });
        assert_eq!(r.try_expand("+", &[CompileTimeValue::Int(0), CompileTimeValue::Int(42)], &e), Some(CompileTimeValue::Int(42)));
    }
    #[test] fn test_aop() {
        let mut w = AspectWeaver::new(); let logged = Arc::new(Mutex::new(false)); let l2 = logged.clone();
        w.add_aspect(Aspect { pointcut: Pointcut::by_name("write"), advice: Advice::Before(Arc::new(move |_| { *l2.lock().unwrap() = true; })) });
        w.weave_before(&AdviceContext { fn_name: "write-file".into(), args: vec![], meta: Meta::new() });
        assert!(*logged.lock().unwrap());
    }
}
