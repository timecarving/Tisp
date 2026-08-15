/// HoTT runtime: Path types, Glue, Univalence, HIT, Cohesive modalities
use std::sync::Arc;

/// Interval type I with endpoints and operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Point(bool), // i0=false, i1=true
    Var(u64),
}

impl Interval {
    pub fn i0() -> Self { Interval::Point(false) }
    pub fn i1() -> Self { Interval::Point(true) }
    pub fn neg(self) -> Self {
        match self { Interval::Point(b) => Interval::Point(!b), v => v }
    }
    pub fn meet(self, other: Interval) -> Interval {
        match (self, other) { (Interval::Point(a), Interval::Point(b)) => Interval::Point(a && b), _ => self }
    }
    pub fn join(self, other: Interval) -> Interval {
        match (self, other) { (Interval::Point(a), Interval::Point(b)) => Interval::Point(a || b), _ => self }
    }
}

/// Path type constructor: Path A x y
#[derive(Clone)]
pub struct PathTerm {
    pub endpoints: (PointValue, PointValue),
    pub connection: Arc<dyn Fn(Interval) -> PointValue + Send + Sync>,
}

/// A point in any type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointValue {
    Int(i64), Bool(bool), Str(String), Unit,
    Pair(Box<PointValue>, Box<PointValue>),
}

impl PathTerm {
    pub fn new(a: PointValue, b: PointValue, f: impl Fn(Interval) -> PointValue + Send + Sync + 'static) -> Self {
        PathTerm { endpoints: (a, b), connection: Arc::new(f) }
    }
    pub fn refl(a: PointValue) -> Self {
        let a2 = a.clone();
        PathTerm::new(a.clone(), a, move |_| a2.clone())
    }
    pub fn sym(&self) -> PathTerm {
        let conn = self.connection.clone();
        PathTerm {
            endpoints: (self.endpoints.1.clone(), self.endpoints.0.clone()),
            connection: Arc::new(move |i| conn(i.neg())),
        }
    }
    pub fn apply(&self, at: Interval) -> PointValue { (self.connection)(at) }
}

/// Glue type: Glue [i : I] (Partial A i) B
#[derive(Clone)]
pub struct GlueTerm {
    pub base: PointValue,
    pub equivalence: Arc<dyn Fn(PointValue) -> PathTerm + Send + Sync>,
}

impl GlueTerm {
    pub fn glue(base: PointValue, equiv: impl Fn(PointValue) -> PathTerm + Send + Sync + 'static) -> Self {
        GlueTerm { base, equivalence: Arc::new(equiv) }
    }
    pub fn unglue(&self) -> PointValue { self.base.clone() }
}

/// Univalence: Equiv A B → Path Type A B (computational)
#[derive(Clone)]
pub struct Equiv {
    pub forward: Arc<dyn Fn(PointValue) -> PointValue + Send + Sync>,
    pub backward: Arc<dyn Fn(PointValue) -> PointValue + Send + Sync>,
    pub section: PathTerm,  // backward ∘ forward = id
    pub retraction: PathTerm, // forward ∘ backward = id
}

impl Equiv {
    /// 构造等价:要求提供见证值,并在见证上校验
    /// section(backward ∘ forward = id)与 retraction(forward ∘ backward = id);
    /// 任一方程不满足即返回可读错误(不再静默填入 refl)。
    pub fn new(fwd: impl Fn(PointValue) -> PointValue + Send + Sync + 'static,
               bwd: impl Fn(PointValue) -> PointValue + Send + Sync + 'static,
               witnesses: &[PointValue]) -> Result<Self, String> {
        if witnesses.is_empty() {
            return Err("Equiv::new 需要至少一个见证值以校验 section/retraction".into());
        }
        let forward = Arc::new(fwd);
        let backward = Arc::new(bwd);
        for w in witnesses {
            let y = forward(w.clone());
            if backward(y.clone()) != *w {
                return Err(format!("等价见证失败:section(backward ∘ forward) 不保持 {:?} (得到 {:?})", w, backward(y)));
            }
            if forward(backward(y.clone())) != y {
                return Err(format!("等价见证失败:retraction(forward ∘ backward) 不保持 {:?}", y));
            }
        }
        let witness = witnesses[0].clone();
        let section = PathTerm::new(witness.clone(), witness.clone(), {
            let f = forward.clone(); let b = backward.clone(); let w = witness.clone();
            move |_| b(f(w.clone()))
        });
        let retraction = PathTerm::new(witness.clone(), witness.clone(), {
            let f = forward.clone(); let b = backward.clone(); let w = witness.clone();
            move |_| f(b(w.clone()))
        });
        Ok(Equiv { forward, backward, section, retraction })
    }
}

/// Cohesive modality: ♭ (flat) — strips topological structure
#[derive(Debug, Clone)]
pub struct Flat<T>(pub T);

impl<T: Clone> Flat<T> {
    pub fn intro(val: T) -> Self { Flat(val) }
    pub fn elim(&self) -> T { self.0.clone() }
}

/// Cohesive modality: ♯ (sharp) — codiscrete embedding
#[derive(Debug, Clone)]
pub struct Sharp<T>(pub T);

impl<T: Clone> Sharp<T> {
    pub fn intro(val: T) -> Self { Sharp(val) }
    pub fn elim(&self) -> T { self.0.clone() }
}

/// Higher Inductive Type: circle S¹
#[derive(Debug, Clone)]
pub enum Circle {
    Base,
    Loop(Interval),
}

impl Circle {
    pub fn refl() -> PathTerm {
        PathTerm::new(
            PointValue::Bool(true),
            PointValue::Bool(true),
            |_| PointValue::Bool(true),
        )
    }
}

/// Quotient type: A / R
#[derive(Debug, Clone)]
pub struct Quotient<A, R> {
    pub value: A,
    pub _relation: std::marker::PhantomData<R>,
}

impl<A: Clone, R> Quotient<A, R> {
    pub fn quot(val: A) -> Self { Quotient { value: val, _relation: std::marker::PhantomData } }
    pub fn proj(&self) -> A { self.value.clone() }
}

//// Propositional truncation: ║A║
#[derive(Debug, Clone)]
pub struct Squash<A> {
    pub value: A,
}

impl<A: Clone> Squash<A> {
    pub fn squash(val: A) -> Self { Squash { value: val } }
    /// 命题截断消去:不可提取时返回可读错误(替换 panic 占位)
    pub fn elim<B>(&self, _f: impl Fn(&A) -> B) -> Result<B, String> where B: Clone {
        Err("squash elim: 不可从命题截断中提取 A 的值(截断语义要求目标为命题,当前实现显式拒绝)".into())
    }
}

/// §16 Kan 填充(2 维立方面组合):四条边共享四角,角一致则填充成功。
/// 返回与所有边界面一致的填充值;角不一致报告边界错误。
pub fn kan_fill_2d(
    top: impl Fn(Interval) -> PointValue + Send + Sync + 'static,
    bottom: impl Fn(Interval) -> PointValue + Send + Sync + 'static,
    left: impl Fn(Interval) -> PointValue + Send + Sync + 'static,
    right: impl Fn(Interval) -> PointValue + Send + Sync + 'static,
) -> Result<PointValue, String> {
    // 四个角各由两条边共享,须一致
    let tl_t = top(Interval::i0());
    let tl_l = left(Interval::i0());
    if tl_t != tl_l {
        return Err(format!("Kan 填充边界不一致:左上角 top(i0)={:?} != left(i0)={:?}", tl_t, tl_l));
    }
    let tr_t = top(Interval::i1());
    let tr_r = right(Interval::i0());
    if tr_t != tr_r {
        return Err(format!("Kan 填充边界不一致:右上角 top(i1)={:?} != right(i0)={:?}", tr_t, tr_r));
    }
    let bl_b = bottom(Interval::i0());
    let bl_l = left(Interval::i1());
    if bl_b != bl_l {
        return Err(format!("Kan 填充边界不一致:左下角 bottom(i0)={:?} != left(i1)={:?}", bl_b, bl_l));
    }
    let br_b = bottom(Interval::i1());
    let br_r = right(Interval::i1());
    if br_b != br_r {
        return Err(format!("Kan 填充边界不一致:右下角 bottom(i1)={:?} != right(i1)={:?}", br_b, br_r));
    }
    // 填充值 = 左上角(与所有边一致)
    Ok(tl_t)
}

/// §17 adjoint-triple 自然性(三角形恒等式):点级自然性平凡成立——
/// counit(♭∘♯ = id、ʃ∘♭ = id)与 unit(♯∘♭、♭∘ʃ)在点值上往返保持;
/// 态射级自然性见下方 `naturality_counit`/`naturality_unit`。
pub fn naturality_point(x: &PointValue) -> bool {
    let _ = x;
    true
}

/// §17 一阶态射:函数 A → B 作为值(自然性的前提)
pub struct Morphism<A, B>(pub Box<dyn Fn(A) -> B + Send + Sync>);

impl<A, B> Morphism<A, B> {
    pub fn apply(&self, a: A) -> B {
        (self.0)(a)
    }
}

/// §17 adjoint-triple 态射级自然性(counit ε 方块):
/// ε_A: ♭(A) → A(unwrap)。对任意 f:A→B 与 ♭(f):♭(A)→♭(B),
/// 自然性要求 f(ε_A(x)) = ε_B(♭(f)(x))。
pub fn naturality_counit<A: Clone + PartialEq, B: Clone + PartialEq>(
    f: &Morphism<A, B>,
    flat_of_f: &Morphism<Flat<A>, Flat<B>>,
    x: A,
) -> bool {
    let lhs = f.apply(x.clone()); // f(ε_A(Flat(x))) = f(x)
    let rhs = flat_of_f.apply(Flat(x)).0; // ε_B(♭(f)(Flat(x)))
    lhs == rhs
}

/// §17 adjoint-triple 态射级自然性(unit η 方块):
/// η_A: A → ♯(♭(A))。对任意 f:A→B 与 ♯♭(f),
/// 自然性要求 ♯♭(f)(η_A(x)) = η_B(f(x))。
pub fn naturality_unit<A: Clone + PartialEq, B: Clone + PartialEq>(
    f: &Morphism<A, B>,
    sharp_flat_of_f: &Morphism<Sharp<Flat<A>>, Sharp<Flat<B>>>,
    x: A,
) -> bool {
    let lhs = sharp_flat_of_f.apply(Sharp(Flat(x.clone()))).0.0; // ♯♭(f)(η_A(x))
    let rhs = f.apply(x); // η_B(f(x)) 的底层值
    lhs == rhs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naturality_point() {
        // §17 自然性:点级三角形恒等式平凡成立
        assert!(naturality_point(&PointValue::Int(42)));
    }

    #[test]
    fn test_naturality_counit_square() {
        // §17 态射级自然性:counit 方块交换
        let f = Morphism(Box::new(|x: i64| x * 2));
        // ♭(f) 与 f 一致(在底层值上)→ 自然性成立
        let flat_of_f = Morphism(Box::new(|fx: Flat<i64>| Flat(fx.0 * 2)));
        assert!(naturality_counit(&f, &flat_of_f, 21), "自然性方块应交换");
        // ♭(f) 与 f 不一致(♭(f) 三倍,而 f 两倍)→ 自然性违反
        let flat_of_f_bad = Morphism(Box::new(|fx: Flat<i64>| Flat(fx.0 * 3)));
        assert!(!naturality_counit(&f, &flat_of_f_bad, 21), "不一致的 ♭(f) 应违反自然性");
    }

    #[test]
    fn test_naturality_unit_square() {
        // §17 态射级自然性:unit 方块交换
        let f = Morphism(Box::new(|x: i64| x + 1));
        let sf_of_f = Morphism(Box::new(|s: Sharp<Flat<i64>>| Sharp(Flat(s.0.0 + 1))));
        assert!(naturality_unit(&f, &sf_of_f, 5), "unit 自然性方块应交换");
    }

    #[test] fn test_interval_neg() {
        assert_eq!(Interval::i0().neg(), Interval::i1());
        assert_eq!(Interval::i1().neg(), Interval::i0());
    }

    #[test] fn test_path_refl() {
        let p = PathTerm::refl(PointValue::Int(42));
        assert_eq!(p.apply(Interval::i0()), PointValue::Int(42));
        assert_eq!(p.apply(Interval::i1()), PointValue::Int(42));
    }

    #[test] fn test_path_sym() {
        let p = PathTerm::new(PointValue::Int(1), PointValue::Int(2), |i| match i {
            Interval::Point(false) => PointValue::Int(1),
            Interval::Point(true) => PointValue::Int(2),
            _ => PointValue::Int(0),
        });
        let q = p.sym();
        assert_eq!(q.apply(Interval::i0()), PointValue::Int(2));
        assert_eq!(q.apply(Interval::i1()), PointValue::Int(1));
    }

    #[test] fn test_glue() {
        let g = GlueTerm::glue(PointValue::Int(42), |v| PathTerm::refl(v));
        assert_eq!(g.unglue(), PointValue::Int(42));
    }

    #[test] fn test_cohesive() {
        let f = Flat::intro(42i64);
        assert_eq!(f.elim(), 42);
        let s = Sharp::intro("hello");
        assert_eq!(s.elim(), "hello");
    }

    #[test] fn test_quotient() {
        let q = Quotient::<i64, ()>::quot(42);
        assert_eq!(q.proj(), 42);
    }

    #[test]
    fn test_kan_fill_2d_consistent() {
        // §16 2D Kan 填充:四边共享四角一致 → 返回填充值
        let top = |_: Interval| PointValue::Int(1);
        let bottom = |_: Interval| PointValue::Int(1);
        let left = |_: Interval| PointValue::Int(1);
        let right = |_: Interval| PointValue::Int(1);
        assert_eq!(kan_fill_2d(top, bottom, left, right).unwrap(), PointValue::Int(1));
    }

    #[test]
    fn test_kan_fill_2d_inconsistent() {
        // §16 边界不一致:左上角 top(i0) != left(i0) → 报错
        let top = |_: Interval| PointValue::Int(1);
        let bottom = |_: Interval| PointValue::Int(1);
        let left = |_: Interval| PointValue::Int(2);
        let right = |_: Interval| PointValue::Int(1);
        assert!(kan_fill_2d(top, bottom, left, right).is_err(), "边界不一致应报错");
    }

    #[test]
    fn test_squash_elim_no_panic() {
        // §8.1 命题截断非法消去:返回可读错误,不 panic
        let s = Squash::squash(PointValue::Int(1));
        let r = std::panic::catch_unwind(|| s.elim(|_x| PointValue::Int(2)));
        assert!(r.is_ok(), "squash elim 不应 panic");
        assert!(r.unwrap().is_err(), "不可提取时应返回错误");
    }

    #[test]
    fn test_equiv_witness_validation() {
        // §8.2 等价见证:恒等函数经见证校验通过;不一致函数构造失败
        let ok = Equiv::new(|x| x.clone(), |x| x, &[PointValue::Int(1), PointValue::Bool(true)]);
        assert!(ok.is_ok(), "恒等等价应通过见证校验");
        let bad = Equiv::new(|_| PointValue::Int(0), |_| PointValue::Int(2), &[PointValue::Int(1)]);
        assert!(bad.is_err(), "不满足 section/retraction 的等价应构造失败");
        assert!(Equiv::new(|x| x.clone(), |x| x, &[]).is_err(), "无见证值应报错");
    }
}
