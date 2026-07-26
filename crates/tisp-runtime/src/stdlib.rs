/// Phase 21: Stdlib + Toolchain — core standard library functions
use std::collections::{HashMap, HashSet};

// ── Core functions (pure, ε=·) ──

pub fn id<T: Clone>(x: T) -> T { x }
pub fn const_fn<T: Clone, U: Clone>(x: T, _y: U) -> T { x }
pub fn compose<A, B, C>(f: impl Fn(B) -> C, g: impl Fn(A) -> B) -> impl Fn(A) -> C {
    move |x| f(g(x))
}
pub fn flip<A, B, C>(f: impl Fn(A, B) -> C) -> impl Fn(B, A) -> C {
    move |b, a| f(a, b)
}

// ── Boolean operations ──
pub fn not(b: bool) -> bool { !b }
pub fn and(a: bool, b: bool) -> bool { a && b }
pub fn or(a: bool, b: bool) -> bool { a || b }
pub fn xor(a: bool, b: bool) -> bool { a ^ b }

// ── Integer arithmetic ──
pub fn abs_i64(x: i64) -> i64 { x.abs() }
pub fn min_i64(a: i64, b: i64) -> i64 { a.min(b) }
pub fn max_i64(a: i64, b: i64) -> i64 { a.max(b) }
pub fn pow_i64(base: i64, exp: u32) -> i64 { base.pow(exp) }

// ── Floating point ──
pub fn abs_f64(x: f64) -> f64 { x.abs() }
pub fn sin(x: f64) -> f64 { x.sin() }
pub fn cos(x: f64) -> f64 { x.cos() }
pub fn sqrt(x: f64) -> f64 { x.sqrt() }
pub fn log(x: f64) -> f64 { x.ln() }
pub fn exp(x: f64) -> f64 { x.exp() }
pub fn ceil(x: f64) -> i64 { x.ceil() as i64 }
pub fn floor(x: f64) -> i64 { x.floor() as i64 }

// ── String operations ──
pub fn str_len(s: &str) -> usize { s.len() }
pub fn str_concat(a: &str, b: &str) -> String { format!("{}{}", a, b) }
pub fn str_contains(s: &str, sub: &str) -> bool { s.contains(sub) }
pub fn str_to_upper(s: &str) -> String { s.to_uppercase() }
pub fn str_to_lower(s: &str) -> String { s.to_lowercase() }
pub fn str_trim(s: &str) -> String { s.trim().to_string() }

// ── Vector/Collection operations ──
pub fn vec_len<T>(v: &[T]) -> usize { v.len() }
pub fn vec_is_empty<T>(v: &[T]) -> bool { v.is_empty() }
pub fn vec_get<T: Clone>(v: &[T], idx: usize) -> Option<T> { v.get(idx).cloned() }
pub fn vec_concat<T: Clone>(a: &[T], b: &[T]) -> Vec<T> {
    let mut r = a.to_vec(); r.extend_from_slice(b); r
}

// ── Higher-order functions ──
pub fn map<A, B>(f: impl Fn(&A) -> B, xs: &[A]) -> Vec<B> { xs.iter().map(f).collect() }
pub fn filter<A>(pred: impl Fn(&A) -> bool, xs: &[A]) -> Vec<A> where A: Clone {
    xs.iter().filter(|x| pred(x)).cloned().collect()
}
pub fn reduce<A, B>(f: impl Fn(B, &A) -> B, init: B, xs: &[A]) -> B { xs.iter().fold(init, f) }
pub fn foldl<A, B>(f: impl Fn(B, &A) -> B, init: B, xs: &[A]) -> B { xs.iter().fold(init, f) }
pub fn foldr<A: Clone, B>(f: impl Fn(&A, B) -> B, init: B, xs: &[A]) -> B {
    xs.iter().rfold(init, |acc, x| f(x, acc))
}
pub fn take<A: Clone>(n: usize, xs: &[A]) -> Vec<A> { xs.iter().take(n).cloned().collect() }
pub fn drop<A: Clone>(n: usize, xs: &[A]) -> Vec<A> { xs.iter().skip(n).cloned().collect() }
pub fn reverse<A: Clone>(xs: &[A]) -> Vec<A> { let mut r = xs.to_vec(); r.reverse(); r }
pub fn zip<A: Clone, B: Clone>(a: &[A], b: &[B]) -> Vec<(A, B)> {
    a.iter().zip(b.iter()).map(|(x, y)| (x.clone(), y.clone())).collect()
}
pub fn range(start: i64, end: i64) -> Vec<i64> { (start..end).collect() }

// ── Type information ──
pub fn type_of<T: std::any::Any>(_: &T) -> String {
    std::any::type_name::<T>().to_string()
}

// ── Module system: simple namespace registry ──
pub struct Namespace {
    pub name: String,
    pub bindings: HashMap<String, String>,
    pub requires: HashSet<String>,
}

impl Namespace {
    pub fn new(name: &str) -> Self {
        Namespace { name: name.into(), bindings: HashMap::new(), requires: HashSet::new() }
    }
    pub fn require(&mut self, ns: &str) { self.requires.insert(ns.into()); }
    pub fn def(&mut self, name: &str, ty: &str) { self.bindings.insert(name.into(), ty.into()); }
}

// ── Package manager basics ──
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<(String, String)>,
    pub modules: Vec<String>,
}

impl Package {
    pub fn new(name: &str, version: &str) -> Self {
        Package { name: name.into(), version: version.into(), dependencies: Vec::new(), modules: Vec::new() }
    }
    pub fn add_dep(&mut self, name: &str, version: &str) {
        self.dependencies.push((name.into(), version.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_id() { assert_eq!(id(42), 42); }
    #[test] fn test_compose() {
        let f = compose(|x: i64| x * 2, |x: i64| x + 1);
        assert_eq!(f(3), 8); // (3+1)*2 = 8
    }
    #[test] fn test_map() { assert_eq!(map(|x| x * 2, &[1, 2, 3]), vec![2, 4, 6]); }
    #[test] fn test_filter() { assert_eq!(filter(|x| *x > 2, &[1, 2, 3, 4]), vec![3, 4]); }
    #[test] fn test_reduce() { assert_eq!(reduce(|a, x| a + x, 0, &[1, 2, 3]), 6); }
    #[test] fn test_zip() { assert_eq!(zip(&[1, 2], &["a", "b"]), vec![(1, "a"), (2, "b")]); }
    #[test] fn test_range() { assert_eq!(range(0, 5), vec![0, 1, 2, 3, 4]); }
    #[test] fn test_namespace() {
        let mut ns = Namespace::new("tisp.core");
        ns.require("tisp.collections");
        ns.def("id", "∀a. a → a");
        assert!(ns.requires.contains("tisp.collections"));
    }
}
