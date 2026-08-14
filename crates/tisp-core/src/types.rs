use crate::symbol::Symbol;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Type {
    Var(TypeVar),
    Con(TypeCon),
    App(Box<Type>, Box<Type>),
    Fun(Box<Type>, FunAnnotation, Box<Type>),
    Forall(Vec<TypeVar>, Box<Type>),
    Tuple(Vec<Type>),
    Record(Vec<(Symbol, Type)>, Option<Box<Type>>),
    Refined(Box<Type>, Box<Predicate>),
    Path(Box<Type>, Box<Term>, Box<Term>),
    Interval,
    Session(Box<SessionType>),
    Modal(ModalOp, Box<Type>),
    Temporal(TemporalOp, Box<Type>),
    Cohesive(CohesiveOp, Box<Type>),
    Meta(Box<MetaType>),
    /// 依赖函数类型 Π(x : T). R(§19.1)
    Pi(Symbol, Box<Type>, Box<Type>),
    /// 依赖对类型 Σ(x : T). R(§19.1)
    Sigma(Symbol, Box<Type>, Box<Type>),
    /// 类型 λ(tlambda,草稿 type-system):类型级抽象,参数类型 → 返回类型
    TLambda(Box<Type>, Box<Type>),
    /// 可变引用(§统一内存管理):Ref a 分级值(1 线性可变 / ω 共享读 / 0 擦除)
    Ref(Box<Type>),
    /// 裸指针(§统一内存管理):Ptr a 手动 Unsafe(1 级线性指针,经 Unsafe 门控)
    Ptr(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TypeVar {
    pub name: Symbol,
    pub kind: Kind,
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Star,
    Arrow(Box<Kind>, Box<Kind>),
    Effect,
    Grade,
    Region,
    Mode,
    Determinism,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TypeCon {
    pub name: Symbol,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FunAnnotation {
    pub effects: EffectRow,
    pub region: Option<RegionVar>,
    pub grade: Grade,
    pub mode: Mode,
    pub determinism: Determinism,
}

impl Default for FunAnnotation {
    fn default() -> Self {
        Self {
            effects: EffectRow::Pure,
            region: None,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EffectRow {
    Pure,
    Open(Vec<EffectLabel>, Box<EffectRow>),
    Closed(Vec<EffectLabel>),
    Var(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EffectLabel {
    Named(Symbol),
    State(Box<Type>),
    Reader(Box<Type>),
    Writer(Box<Type>),
    Except(Box<Type>),
    IO,
    Search,
    Unsafe,
    Channel(Box<Type>),
    Ambient,
    Reflect,
    Reaction,
    Signal,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Grade {
    Zero,
    One,
    Omega,
    Nat(u64),
    Add(Box<Grade>, Box<Grade>),
    Mul(Box<Grade>, Box<Grade>),
    Var(Symbol),
    Custom(Symbol, Box<Grade>),
}

/// 类型族实例(§9):(typefamily 名称 参数模式 结果)
/// 资源代数声明(§11.1):(defresource-algebra 名称 单位元 二元运算 阶)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceAlgebra {
    pub name: Symbol,
    /// 单位元显示文本(如 "0")
    pub unit: String,
    /// 二元运算名(如 "+")
    pub op: Symbol,
    /// 阶(preorder,可无)
    pub order: Option<Symbol>,
    /// §11.1 `:asymptotic true` — 渐近代价分析(Big-O)
    pub asymptotic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeFamilyInstance {
    pub name: Symbol,
    /// 参数模式(如 `(List a)` 中的 [Con(List), Var(a)])
    pub params: Vec<Type>,
    pub result: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    In,
    Out,
    Ground,
    Free,
    Crisp,
    Cohesive,
    Product(Box<Mode>, Box<Mode>),
    Var(Symbol),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Determinism {
    Det,
    SemiDet,
    Multi,
    NonDet,
    CcMulti,
    CcNonDet,
    Failure,
    Erroneous,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RegionVar {
    pub name: Symbol,
    pub id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Predicate {
    Lit(bool),
    Var(Symbol),
    App(Symbol, Vec<Predicate>),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    Implies(Box<Predicate>, Box<Predicate>),
    Forall(Symbol, Box<Predicate>),
    Exists(Symbol, Box<Predicate>),
    Cmp(CmpOp, Box<Term>, Box<Term>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Term {
    Lit(Lit),
    Var(Symbol),
    App(Symbol, Vec<Term>),
    BinOp(BinOp, Box<Term>, Box<Term>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Lit {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SessionType {
    Send(Box<Type>, Box<SessionType>),
    Recv(Box<Type>, Box<SessionType>),
    Choice(Vec<(Symbol, SessionType)>),
    Offer(Vec<(Symbol, SessionType)>),
    Rec(Symbol, Box<SessionType>),
    Var(Symbol),
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ModalOp {
    Necessity(Grade),
    Possibility(EffectRow),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TemporalOp {
    Next,
    Always,
    Eventually,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CohesiveOp {
    Flat,
    Sharp,
    Shape,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MetaType {
    Type,
    Effect,
    Grade,
    Region,
    Mode,
    Determinism,
    Session,
    Property,
}

impl Type {
    pub fn i8() -> Self {
        Type::Con(TypeCon { name: Symbol::new("i8"), kind: Kind::Star })
    }

    pub fn i16() -> Self {
        Type::Con(TypeCon { name: Symbol::new("i16"), kind: Kind::Star })
    }

    pub fn i32() -> Self {
        Type::Con(TypeCon { name: Symbol::new("i32"), kind: Kind::Star })
    }

    pub fn i64() -> Self {
        Type::Con(TypeCon { name: Symbol::new("i64"), kind: Kind::Star })
    }

    pub fn u8() -> Self {
        Type::Con(TypeCon { name: Symbol::new("u8"), kind: Kind::Star })
    }

    pub fn u16() -> Self {
        Type::Con(TypeCon { name: Symbol::new("u16"), kind: Kind::Star })
    }

    pub fn u32() -> Self {
        Type::Con(TypeCon { name: Symbol::new("u32"), kind: Kind::Star })
    }

    pub fn u64() -> Self {
        Type::Con(TypeCon { name: Symbol::new("u64"), kind: Kind::Star })
    }

    pub fn f32() -> Self {
        Type::Con(TypeCon { name: Symbol::new("f32"), kind: Kind::Star })
    }

    pub fn f64() -> Self {
        Type::Con(TypeCon { name: Symbol::new("f64"), kind: Kind::Star })
    }

    pub fn bool() -> Self {
        Type::Con(TypeCon { name: Symbol::new("bool"), kind: Kind::Star })
    }

    pub fn string() -> Self {
        Type::Con(TypeCon { name: Symbol::new("String"), kind: Kind::Star })
    }

    pub fn unit() -> Self {
        Type::Con(TypeCon { name: Symbol::new("Unit"), kind: Kind::Star })
    }

    pub fn list(elem: Type) -> Self {
        Type::App(
            Box::new(Type::Con(TypeCon { name: Symbol::new("List"), kind: Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star)) })),
            Box::new(elem),
        )
    }

    pub fn fun(param: Type, ret: Type) -> Self {
        Type::Fun(Box::new(param), FunAnnotation::default(), Box::new(ret))
    }

    pub fn fun_annotated(param: Type, ann: FunAnnotation, ret: Type) -> Self {
        Type::Fun(Box::new(param), ann, Box::new(ret))
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Var(v) => write!(f, "{}", v.name),
            Type::Con(c) => {
                match c.name.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" | "bool" | "String" | "Unit" => write!(f, "{}", c.name),
                    _ => write!(f, "{}", c.name),
                }
            }
            Type::App(fun, arg) => {
                if let Type::App(_, _) = arg.as_ref() {
                    write!(f, "({} {})", fun, arg)
                } else {
                    write!(f, "{} {}", fun, arg)
                }
            }
            Type::Fun(param, _, ret) => write!(f, "{} -> {}", param, ret),
            Type::Forall(vars, body) => {
                let vars_str: Vec<String> = vars.iter().map(|v| v.name.as_str().to_string()).collect();
                write!(f, "∀{}. {}", vars_str.join(" "), body)
            }
            Type::Tuple(ts) => {
                let ts_str: Vec<String> = ts.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({})", ts_str.join(" × "))
            }
            Type::Refined(base, pred) => write!(f, "{{x : {} | {:?}}}", base, pred),
            Type::Record(fields, rest) => {
                let f_str: Vec<String> = fields.iter().map(|(k, v)| format!("{} : {}", k, v)).collect();
                match rest {
                    Some(r) => write!(f, "{{ {} | {} }}", f_str.join(", "), r),
                    None => write!(f, "{{ {} }}", f_str.join(", ")),
                }
            }
            Type::Path(a, x, y) => write!(f, "Path({}, {:?}, {:?})", a, x, y),
            Type::Interval => write!(f, "I"),
            Type::Pi(name, dom, cod) => write!(f, "Π({} : {}). {}", name, dom, cod),
            Type::Sigma(name, dom, cod) => write!(f, "Σ({} : {}). {}", name, dom, cod),
            Type::TLambda(param, body) => write!(f, "{} => {}", param, body),
            Type::Ref(t) => write!(f, "Ref {}", t),
            Type::Ptr(t) => write!(f, "Ptr {}", t),
            _ => write!(f, "{:?}", self),
        }
    }
}
