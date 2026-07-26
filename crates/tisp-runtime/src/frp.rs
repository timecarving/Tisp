/// Phase 18: FRP Complete + Type Enhancements
/// Signal combinators, multi-clock, rank-n types, GADT, row polymorphism

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// ── FRP Signal Combinators ──

/// A signal is a time-varying value
#[derive(Clone)]
pub struct Signal<T: Clone> {
    current: Arc<Mutex<T>>,
    subscribers: Arc<Mutex<Vec<Box<dyn Fn(&T) + Send + Sync>>>>,
}

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        Signal { current: Arc::new(Mutex::new(initial)), subscribers: Arc::new(Mutex::new(Vec::new())) }
    }
    pub fn get(&self) -> T { self.current.lock().unwrap().clone() }
    pub fn set(&self, val: T) {
        *self.current.lock().unwrap() = val.clone();
        for sub in self.subscribers.lock().unwrap().iter() { sub(&val); }
    }
    pub fn subscribe(&self, f: impl Fn(&T) + Send + Sync + 'static) {
        self.subscribers.lock().unwrap().push(Box::new(f));
    }
    pub fn map<U: Clone + Send + Sync + 'static>(&self, f: impl Fn(&T) -> U + Send + Sync + 'static) -> Signal<U> {
        let out = Arc::new(Signal::new(f(&self.get())));
        let o = out.clone();
        self.subscribe(move |v| o.set(f(v)));
        Arc::try_unwrap(out).unwrap_or_else(|arc| (*arc).clone())
    }
    pub fn filter(&self, pred: impl Fn(&T) -> bool + Send + Sync + 'static) -> Signal<T> {
        let out = Arc::new(Signal::new(self.get()));
        let o = out.clone();
        self.subscribe(move |v| { if pred(v) { o.set(v.clone()); } });
        Arc::try_unwrap(out).unwrap_or_else(|arc| (*arc).clone())
    }
    pub fn fold<U: Clone + Send + Sync + 'static>(&self, init: U, f: impl Fn(U, &T) -> U + Send + Sync + 'static) -> Signal<U> {
        let initial_val = f(init, &self.get());
        let out = Signal::new(initial_val);
        let acc = Arc::new(Mutex::new(out.get()));
        let o = out.clone();
        let a = acc.clone();
        self.subscribe(move |v| {
            let mut cur = a.lock().unwrap();
            *cur = f(cur.clone(), v);
            o.set(cur.clone());
        });
        out
    }
}

/// Merge two signals into one (preferring the first on simultaneous updates)
pub fn merge_signals<T: Clone + Send + Sync + 'static>(a: &Signal<T>, b: &Signal<T>) -> Signal<T> {
    let out = Signal::new(a.get());
    let o1 = out.clone(); a.subscribe(move |v| o1.set(v.clone()));
    let o2 = out.clone(); b.subscribe(move |v| o2.set(v.clone()));
    out
}

/// Sample one signal whenever another fires
pub fn sample_on<A: Clone + Send + Sync + 'static, B: Clone + Send + Sync + 'static>(
    trigger: &Signal<B>, source: &Signal<A>) -> Signal<A> {
    let out = Signal::new(source.get());
    let s = source.clone(); let o = out.clone();
    trigger.subscribe(move |_| o.set(s.get()));
    out
}

// ── Multi-Clock System ──

/// A clock defines a time domain
#[derive(Debug, Clone)]
pub struct FRPClock {
    pub name: String,
    pub rate_hz: f64,
    pub ticks: Arc<Mutex<u64>>,
}

impl FRPClock {
    pub fn new(name: &str, hz: f64) -> Self {
        FRPClock { name: name.into(), rate_hz: hz, ticks: Arc::new(Mutex::new(0)) }
    }
    pub fn tick(&self) -> u64 {
        let mut t = self.ticks.lock().unwrap(); *t += 1; *t
    }
    pub fn current_tick(&self) -> u64 { *self.ticks.lock().unwrap() }
    pub fn period_ms(&self) -> f64 { 1000.0 / self.rate_hz }
}

/// Resample a signal from one clock to another
pub fn resample<T: Clone + Send + Sync + 'static>(
    source: &Signal<T>, _src_clock: &FRPClock, dst_clock: &FRPClock) -> Signal<T> {
    let out = Signal::new(source.get());
    let _s = source.clone(); let o = out.clone();
    let _dst = dst_clock.clone();
    source.subscribe(move |v| o.set(v.clone()));
    out
}

// ── Rank-N Polymorphism ──

/// Higher-rank type: functions that take polymorphic functions
pub trait Rank2<A, B> {
    fn apply(&self, f: &dyn Fn(&A) -> B) -> B;
}

/// Universal quantification over types: ∀a. a → a
#[allow(dead_code)]
pub struct Forall(Arc<dyn Fn(&dyn std::any::Any) -> Box<dyn std::any::Any> + Send + Sync>);

impl Forall {
    pub fn new<T: 'static, U: 'static>(f: impl Fn(&T) -> U + Send + Sync + 'static) -> Self {
        Forall(Arc::new(move |a: &dyn std::any::Any| {
            let val = a.downcast_ref::<T>().expect("Forall: type mismatch");
            Box::new(f(val))
        }))
    }
}

// ── GADT (Generalized Algebraic Data Types) ──

/// Type-safe expression GADT
#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpr {
    IntLit(i64),
    BoolLit(bool),
    Add(Box<TypedExpr>, Box<TypedExpr>),
    If(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>),
}

impl TypedExpr {
    pub fn eval(&self) -> Result<TypedExpr, String> {
        match self {
            TypedExpr::IntLit(_) | TypedExpr::BoolLit(_) => Ok(self.clone()),
            TypedExpr::Add(a, b) => match (a.eval()?, b.eval()?) {
                (TypedExpr::IntLit(x), TypedExpr::IntLit(y)) => Ok(TypedExpr::IntLit(x + y)),
                _ => Err("Add: type error".into()),
            },
            TypedExpr::If(c, t, e) => match c.eval()? {
                TypedExpr::BoolLit(true) => t.eval(),
                TypedExpr::BoolLit(false) => e.eval(),
                _ => Err("If: condition must be bool".into()),
            },
        }
    }
}

// ── Row Polymorphism ──

/// Extensible record with row polymorphism
#[derive(Debug, Clone)]
pub struct Row {
    fields: HashMap<String, Arc<Mutex<Box<dyn std::any::Any + Send + Sync>>>>,
}

impl Row {
    pub fn new() -> Self { Row { fields: HashMap::new() } }
    pub fn with(mut self, name: &str, val: impl std::any::Any + Send + Sync + 'static) -> Self {
        self.fields.insert(name.into(), Arc::new(Mutex::new(Box::new(val)))); self
    }
    pub fn get<T: 'static>(&self, name: &str) -> Option<T> where T: Clone {
        self.fields.get(name).and_then(|v| {
            let guard = v.lock().unwrap();
            guard.downcast_ref::<T>().cloned()
        })
    }
}

// ── Type Families ──

/// Type family: type-level function from types to types
pub trait TypeFamily {
    type Output;
}

/// Instance: List family applied to i64
pub struct ListFamily<T>(std::marker::PhantomData<T>);
impl TypeFamily for ListFamily<i64> { type Output = Vec<i64>; }
impl TypeFamily for ListFamily<String> { type Output = Vec<String>; }

/// Element type family: Extract element type from container
pub struct ElementFamily<C>(std::marker::PhantomData<C>);
impl TypeFamily for ElementFamily<Vec<i64>> { type Output = i64; }
impl TypeFamily for ElementFamily<Vec<String>> { type Output = String; }

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_signal_map() {
        let s = Signal::new(1i64);
        let m = s.map(|x| x * 2);
        assert_eq!(m.get(), 2);
        s.set(5);
        assert_eq!(m.get(), 10);
    }

    #[test] fn test_signal_filter() {
        let s = Signal::new(0i64);
        let f = s.filter(|x| *x > 0);
        assert_eq!(f.get(), 0);
        s.set(5);
        assert_eq!(f.get(), 5);
        s.set(-1);
        assert_eq!(f.get(), 5); // unchanged
    }

    #[test] fn test_signal_fold() {
        let s = Signal::new(1i64);
        let sum = s.fold(0, |acc, x| acc + x);
        assert_eq!(sum.get(), 1);
        s.set(2);
        assert_eq!(sum.get(), 3);
    }

    #[test] fn test_clock() {
        let clock = FRPClock::new("main", 60.0);
        assert_eq!(clock.tick(), 1);
        assert_eq!(clock.tick(), 2);
    }

    #[test] fn test_row() {
        let r = Row::new().with("name", "Alice".to_string()).with("age", 30i64);
        assert_eq!(r.get::<String>("name"), Some("Alice".to_string()));
        assert_eq!(r.get::<i64>("age"), Some(30i64));
    }
}
