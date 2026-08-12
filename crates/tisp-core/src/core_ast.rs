use crate::span::Span;
use crate::symbol::Symbol;
use crate::types::{Type, EffectRow, Grade, Mode, Determinism, Predicate, EffectLabel};
use crate::data::DataDecl;
use crate::effects::EffectDecl;

pub type CoreExpr = SpannedCore<CoreExprNode>;

#[derive(Debug, Clone)]
pub struct SpannedCore<T> {
    pub node: T,
    pub span: Span,
    pub ty: Option<Type>,
}

impl<T> SpannedCore<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span, ty: None }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.ty = Some(ty);
        self
    }
}

/// Method combination type for OOP dispatch (§22.3)
#[derive(Debug, Clone, PartialEq)]
pub enum MethodCategory {
    Primary,
    Around,
    Before,
    After,
}

#[derive(Debug, Clone)]
pub enum CoreExprNode {
    // ── Base ──
    Lit(Literal),
    Var(Symbol),
    Lam(Lambda),
    App(Box<CoreExpr>, Box<CoreExpr>),
    Let(Symbol, Option<Type>, Box<CoreExpr>, Box<CoreExpr>),
    If(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>),
    Match(Box<CoreExpr>, Vec<MatchArm>),
    Data(Symbol, Vec<CoreExpr>),
    Handle(Box<CoreExpr>, Handler),
    Perform(Symbol, Vec<CoreExpr>),
    Hole(Option<Symbol>),
    Do(Vec<CoreExpr>),

    // ── Logic Programming (Mercury-style) ──
    PredDef(Symbol, Vec<Param>, Vec<CoreExpr>),
    Fresh(Symbol),
    Unify(Box<CoreExpr>, Box<CoreExpr>),
    Search(Box<CoreExpr>),
    Commit(Box<CoreExpr>),
    Abduce(Box<CoreExpr>, Vec<Symbol>),

    // ── Constraint Logic ──
    Constrain(Box<CoreExpr>),
    Label(Box<CoreExpr>, Box<CoreExpr>),
    AllDifferent(Vec<CoreExpr>),
    Domain(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>),

    // ── Process Calculi (with effect tracking) ──
    /// Spawn with structured concurrency: returns JoinHandle
    Spawn(Box<CoreExpr>, Box<CoreExpr>),  // (body, handle_name)
    /// Create a new channel: ε={Channel}, grade=@1
    ChannelNew,
    /// Send on channel (linear): ε={Channel}, consumes channel@1
    ChannelSend(Box<CoreExpr>, Box<CoreExpr>),
    /// Receive from channel (linear): ε={Channel}, consumes channel@1
    ChannelRecv(Box<CoreExpr>),
    /// Async send (fire-and-forget): ε={Async}, value@1
    AsyncSend(Box<CoreExpr>, Box<CoreExpr>),
    /// Async receive (blocking): ε={Async}, consumes channel@1
    AsyncRecv(Box<CoreExpr>),
    /// Join a spawned thread (structured concurrency): ε={Spawn}
    Join(Box<CoreExpr>),
    AmbientNew(Symbol),
    AmbientEnter(Box<CoreExpr>, Box<CoreExpr>),
    AmbientExit(Box<CoreExpr>, Box<CoreExpr>),
    AmbientOpen(Box<CoreExpr>, Box<CoreExpr>),
    RhoQuote(Box<CoreExpr>),
    RhoDrop(Box<CoreExpr>),
    RhoLift(Box<CoreExpr>, Box<CoreExpr>),
    KappaBind(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>),
    KappaUnbind(Box<CoreExpr>, Box<CoreExpr>),
    KappaReact(Box<CoreExpr>),
    // Applied π-calculus (crypto)
    CryptoEncrypt(Box<CoreExpr>, Box<CoreExpr>),
    CryptoDecrypt(Box<CoreExpr>, Box<CoreExpr>),
    CryptoSign(Box<CoreExpr>, Box<CoreExpr>),
    CryptoVerify(Box<CoreExpr>, Box<CoreExpr>),
    CryptoHash(Box<CoreExpr>),
    // spi-calculus (security protocols)
    SpiSecret(Box<CoreExpr>),
    SpiCommit(Box<CoreExpr>, Box<CoreExpr>),
    SpiCheck(Box<CoreExpr>, Box<CoreExpr>),
    // SKI combinators
    SkiS, SkiK, SkiI,
    SkiApp(Box<CoreExpr>, Box<CoreExpr>),
    SkiReduce(Box<CoreExpr>),
    // ς-calculus (object calculus)
    SigmaInvoke(Box<CoreExpr>, Box<CoreExpr>),
    SigmaUpdate(Box<CoreExpr>, Box<CoreExpr>),

    // ── HoTT ──
    IntervalEndpoint(bool),
    PathLam(Symbol, Box<CoreExpr>),
    PathApp(Box<CoreExpr>, Box<CoreExpr>),
    HComp(Box<CoreExpr>),
    Transp(Box<Type>, Box<CoreExpr>, Box<CoreExpr>),
    FlatMod(Box<CoreExpr>),
    SharpMod(Box<CoreExpr>),
    /// §17 ʃ(Shape)模态:路径形状化(区间端点容器)
    ShapeMod(Box<CoreExpr>),
    /// §17 crisp 上下文标记(♭ 解包要求)
    CrispMod(Box<CoreExpr>),
    Glue(Box<CoreExpr>, Box<CoreExpr>),
    Unglue(Box<CoreExpr>),
    HitDef(Symbol, Vec<(Symbol, Vec<Param>)>),

    // ── FRP / Temporal ──
    SignalNew(Box<CoreExpr>),
    SignalMap(Box<CoreExpr>, Box<CoreExpr>),
    SignalFilter(Box<CoreExpr>, Box<CoreExpr>),
    SignalFold(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>),
    SignalMerge(Box<CoreExpr>, Box<CoreExpr>),
    Delay(Box<CoreExpr>),
    Advance(Box<CoreExpr>),
    Stable(Box<CoreExpr>),
    Unbox(Box<CoreExpr>),
    ClockNew(Symbol),

    // ── Metaprogramming ──
    Comptime(Box<CoreExpr>),
    CompilerMacroDef(Symbol, usize, Box<CoreExpr>),
    MetaQuery(Symbol),
    AdviceDef(Symbol, Box<CoreExpr>, Box<CoreExpr>),

    // ── Theorem Proving ──
    TheoremDef(Symbol, Box<CoreExpr>),
    ProofTactic(Symbol, Vec<CoreExpr>),
    Obligation(Box<CoreExpr>),

    // ── Memory / Region ──
    RegionNew(Symbol),
    RegionAlloc(Box<CoreExpr>, Box<CoreExpr>),
    RegionFree(Box<CoreExpr>),
    PtrRead(Box<CoreExpr>),
    PtrWrite(Box<CoreExpr>, Box<CoreExpr>),

    // ── Session types ──
    Session(SessionOp, Box<CoreExpr>),

    // ── Dependent types ──
    Pi(Symbol, Type, Box<CoreExpr>),
    Sigma(Symbol, Type, Box<CoreExpr>),

    // ── 类型标注与字段访问 ──
    Ann(Box<Type>, Box<CoreExpr>),
    FieldGet(Symbol, Box<CoreExpr>),

    // ── HoTT extended ──
    FunExt(Box<CoreExpr>),

    // ── Generic functions / OOP ──
    GenericDef(Symbol, Vec<Param>, Option<Type>),
    MethodDef(Symbol, MethodCategory, Vec<Pattern>, Box<CoreExpr>),

    // ── Typeclasses ──
    ClassDef(Symbol, Vec<Symbol>, Vec<(Symbol, Type)>),
    InstanceDef(Symbol, Vec<Type>, Vec<(Symbol, Box<CoreExpr>)>),

    // ── Macros ──
    MacroDef(Symbol, usize, Box<CoreExpr>),

    // ── Modules ──
    NSDef(Symbol, Vec<(Symbol, Symbol)>, Vec<Symbol>),

    // ── FFI ──
    ExternDef(Symbol, String, Vec<Type>, Option<Type>, Vec<EffectLabel>),
}

#[derive(Debug, Clone)]
pub enum Literal {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Char(char),
    Unit,
}

#[derive(Debug, Clone)]
pub struct Lambda {
    pub params: Vec<Param>,
    pub body: Box<CoreExpr>,
    pub ret_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Symbol,
    pub ty: Option<Type>,
    pub grade: Grade,
    pub mode: Mode,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<CoreExpr>>,
    pub body: Box<CoreExpr>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Var(Symbol),
    Lit(Literal),
    Con(Symbol, Vec<Pattern>),
    Tuple(Vec<Pattern>),
    /// 或模式 (or p1 p2 ...)(§8.2):任一子模式匹配即成功
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub effect_name: Symbol,
    pub type_args: Vec<Type>,
    pub clauses: Vec<HandlerClause>,
    pub return_clause: Option<Box<CoreExpr>>,
}

#[derive(Debug, Clone)]
pub struct HandlerClause {
    pub operation: Symbol,
    pub params: Vec<Symbol>,
    pub continuation: Symbol,
    pub state: Option<Symbol>,
    pub body: Box<CoreExpr>,
}

/// Session type operations
#[derive(Debug, Clone)]
pub enum SessionOp {
    Send,
    Recv,
    Close,
    Fork(Box<CoreExpr>),
}

#[derive(Debug, Clone)]
pub struct CoreProgram {
    pub data_decls: Vec<DataDecl>,
    pub effect_decls: Vec<EffectDecl>,
    /// 类型族实例(§9)
    pub type_families: Vec<crate::types::TypeFamilyInstance>,
    /// 资源代数声明(§11.1)
    pub resource_algebras: Vec<crate::types::ResourceAlgebra>,
    pub defs: Vec<CoreDef>,
}

#[derive(Debug, Clone)]
pub struct CoreDef {
    pub name: Symbol,
    pub ty: Option<Type>,
    pub effects: EffectRow,
    pub grade: Grade,
    pub mode: Mode,
    /// 多模式谓词签名(§13):每个元素是一个模式的参数 Mode 列表(如 [In, Out])
    pub mode_sigs: Vec<Vec<Mode>>,
    pub determinism: Determinism,
    pub body: CoreExpr,
    pub requires: Option<Predicate>,
    pub ensures: Option<Predicate>,
    pub span: Span,
}
