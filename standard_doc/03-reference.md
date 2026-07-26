# 03 — Tisp 参考手册

> 覆盖：内置函数表 · CLI 参考 · 类型系统附录 · Core AST 附录 · 实现状态矩阵

---

## 1. 内置函数表

### 算术运算

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `+` | `i64 → i64 → i64` | Pure | ✅ |
| `-` | `i64 → i64 → i64` | Pure | ✅ |
| `*` | `i64 → i64 → i64` | Pure | ✅ |
| `/` | `i64 → i64 → i64` | Pure | ✅ |

### 比较运算

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `=` | `∀a. a → a → bool` | Pure | ✅ |
| `<` | `i64 → i64 → bool` | Pure | ✅ |
| `<=` | `i64 → i64 → bool` | Pure | ✅ |
| `>` | `i64 → i64 → bool` | Pure | ✅ |
| `>=` | `i64 → i64 → bool` | Pure | ✅ |
| `!=` | `∀a. a → a → bool` | Pure | ✅ |

### 布尔运算

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `not` | `bool → bool` | Pure | ✅ |

### IO 运算

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `println` | `∀a. a → Unit` | `Closed([IO])` | ✅ |

### Channel 运算 (π-calculus) ✅

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `chan` | `() → Channel` | Channel | ✅ |
| `send` | `Channel → a → Unit` | Channel | ✅ |
| `recv` | `Channel → a` | Channel | ✅ |
| `spawn` | `body → JoinHandle` | Spawn | ✅ |

### FRP 运算 ✅

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `stream` | `a → (a → a) → (Stream a)` | Pure | ✅ |
| `stream-take` | `(Stream a) → i64 → (List a)` | Pure | ✅ |
| `delay` | `a → a` | Pure | ✅ |
| `advance` | `(Stream a) → (Stream a)` | Pure | ✅ |
| `clock` | `String → Clock` | Pure | ✅ |

### 逻辑编程运算 ✅

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `fresh` | `Symbol → LVar` | Search | ✅ |
| `unify` | `LVar → Value → ()` | Search | ✅ |
| `search` | `Goal → ()` | Search | ✅ |
| `commit` | `Goal → ()` | Search | ✅ |
| `abduce` | `Goal → [Symbol] → ()` | Search | ✅ |

### HoTT 运算 ⚠️

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `i0` | `I` | Pure | ✅ |
| `i1` | `I` | Pure | ✅ |
| `path-lam` | `Symbol → Body → Path` | Pure | ✅ |
| `path-apply` | `Path → I → Value` | Pure | ✅ |
| `hcomp` | `Value → Value` | Pure | ⚠️ (pass-through) |
| `transp` | `Type → I → a → a` | Pure | ⚠️ (pass-through) |
| `flat` | `a → ♭ a` | Pure | ✅ |
| `sharp` | `a → ♯ a` | Pure | ✅ |
| `glue` | `a → b → Glue a b` | Pure | ⚠️ (stub) |
| `unglue` | `Glue a b → a` | Pure | ⚠️ (stub) |

### 元编程运算 ⚠️

| 函数 | 签名 | 效果 | 状态 |
|------|------|------|------|
| `comptime` | `Expr → Value` | Meta | ⚠️ |
| `defmacro` | `Name → Arity → Body → ()` | Meta | ⚠️ |

---

## 2. CLI 参考

```
tisp [OPTIONS] [FILE]

参 数:
  FILE                    源码文件 (.tisp)

选 项:
  -e, --eval <EXPR>       对表达式求值
  --run                   运行程序（解释执行）
  --typecheck             完整类型检查、效果推断、等级检查、模式分析、确定性分析、
                          区域推断、优化统计
  --desugar               打印脱糖后的 Core AST
  --print-ast             打印 S-expression AST
  --print-tokens          打印词法 token 流
  --verify                运行 BFS 模型检查器
  --ir                    生成文本型 IR (stub)
  --compile               JIT 编译（需 LLVM 特性） ⬜
```

### 使用示例

```bash
# REPL 交互式
tisp

# 对文件进行类型检查
tisp --typecheck examples/adt-test.tisp

# 运行程序
tisp --run examples/run-test.tisp

# 查看脱糖结果
tisp --desugar examples/hello.tisp

# 查看 token 流
tisp --print-tokens examples/hello.tisp

# 模型检查
tisp --verify examples/hello.tisp

# 单行求值
tisp -e "(+ 21 21)"
```

---

## 3. 类型系统附录

### 3.1 Type 枚举完整定义 ✅

```rust
pub enum Type {
    Var(TypeVar),                                    // 类型变量 (?1, ?2, ...)
    Con(TypeCon),                                    // 类型构造器 (i64, bool, List, ...)
    App(Box<Type>, Box<Type>),                       // 类型应用 (List i64)
    Fun(Box<Type>, FunAnnotation, Box<Type>),        // 函数类型 (A ->[ε] B)
    Forall(Vec<TypeVar>, Box<Type>),                 // 全称量化 (∀a. a → a)
    Tuple(Vec<Type>),                                // 积类型 (A × B)
    Record(Vec<(Symbol, Type)>, Option<Box<Type>>),  // 可扩展记录
    Refined(Box<Type>, Box<Predicate>),              // 液态/Refinement 类型
    Path(Box<Type>, Box<Term>, Box<Term>),           // HoTT 路径类型
    Interval,                                        // HoTT 区间类型
    Session(Box<SessionType>),                       // 会话类型
    Modal(ModalOp, Box<Type>),                       // Graded Modal (□_r, ◇_ε)
    Temporal(TemporalOp, Box<Type>),                 // 时序 (⃝, □_t, ◇_t)
    Cohesive(CohesiveOp, Box<Type>),                 // Cohesive (♭, ♯, ʃ)
    Meta(Box<MetaType>),                             // 元类型 (Type, Effect, Grade, ...)
}
```

### 3.2 TypeVar / Kind / TypeCon ✅

```rust
pub struct TypeVar {
    pub name: Symbol,    // 显示名称 (e.g. "?42")
    pub kind: Kind,      // Kind 分类
    pub id: u64,         // 唯一 ID (用于合一)
}

pub enum Kind {
    Star,                    // 值类型: i64, bool, String
    Arrow(Box<Kind>, Box<Kind>), // 类型构造器: List :: * → *
    Effect, Grade, Region, Mode, Determinism, Session,
}

pub struct TypeCon {
    pub name: Symbol,    // 构造器名称
    pub kind: Kind,      // 构造器的 Kind
}
```

### 3.3 FunAnnotation ✅

```rust
pub struct FunAnnotation {
    pub effects: EffectRow,          // 效果行
    pub region: Option<RegionVar>,   // 区域变量
    pub grade: Grade,                // 使用等级 (QTT)
    pub mode: Mode,                  // Mercury 模式
    pub determinism: Determinism,    // 确定性类别
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
```

### 3.4 EffectRow ✅

```rust
pub enum EffectRow {
    Pure,                            // 无效果
    Open(Vec<EffectLabel>, Box<EffectRow>),  // 开放行
    Closed(Vec<EffectLabel>),        // 封闭行
    Var(u64),                        // 效果行变量
}
```

### 3.5 Grade / Mode / Determinism ✅

```rust
pub enum Grade { Zero, One, Omega }

pub enum Mode { In, Out, Free }

pub enum Determinism { Det, SemiDet, Multi, NonDet }
```

### 3.6 辅助类型 ✅

```rust
pub enum TemporalOp { Next, Always, Eventually }

pub enum CohesiveOp { Flat, Sharp, Shape }

pub enum MetaType { Type, Effect, Grade, Region, Mode, Determinism, Session, Property }
```

---

## 4. Core AST 附录

### 4.1 CoreExprNode 完整枚举 ✅

```rust
pub enum CoreExprNode {
    // ── 基础 ──
    Lit(Literal),                                // 字面量 (42, true, "hello")
    Var(Symbol),                                 // 变量引用
    Lam(Lambda),                                 // Lambda (fn [x] ...)
    App(Box<CoreExpr>, Box<CoreExpr>),           // 函数应用 (f a)
    Let(Symbol, Option<Type>, Box<CoreExpr>, Box<CoreExpr>),  // let 绑定
    If(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>),          // if 条件
    Match(Box<CoreExpr>, Vec<MatchArm>),         // 模式匹配
    Data(Symbol, Vec<CoreExpr>),                 // 构造器应用
    Handle(Box<CoreExpr>, Handler),              // 效果处理
    Perform(Symbol, Vec<CoreExpr>),              // 效果执行
    Hole(Option<Symbol>),                        // 类型洞 (typed hole)
    Do(Vec<CoreExpr>),                           // 顺序执行

    // ── 逻辑编程 ──
    PredDef(Symbol, Vec<Param>, Vec<CoreExpr>),
    Fresh(Symbol), Unify(Box, Box),
    Search(Box), Commit(Box), Abduce(Box, Vec<Symbol>),

    // ── 约束逻辑 ──
    Constrain(Box), Label(Box, Box),
    AllDifferent(Vec), Domain(Box, Box, Box),

    // ── 进程演算 ──
    Spawn(Box, Box), ChannelNew,
    ChannelSend(Box, Box), ChannelRecv(Box),
    AsyncSend(Box, Box), AsyncRecv(Box), Join(Box),
    AmbientNew(Symbol), AmbientEnter(Box, Box),
    AmbientExit(Box, Box), AmbientOpen(Box, Box),
    RhoQuote(Box), RhoDrop(Box), RhoLift(Box, Box),
    KappaBind(Box, Box, Box, Box), KappaUnbind(Box, Box), KappaReact(Box),
    CryptoEncrypt(Box, Box), CryptoDecrypt(Box, Box),
    CryptoSign(Box, Box), CryptoVerify(Box, Box), CryptoHash(Box),
    SpiSecret(Box), SpiCommit(Box, Box), SpiCheck(Box, Box),
    SkiS, SkiK, SkiI, SkiApp(Box, Box), SkiReduce(Box),
    SigmaInvoke(Box, Box), SigmaUpdate(Box, Box),

    // ── HoTT ──
    IntervalEndpoint(bool),
    PathLam(Symbol, Box), PathApp(Box, Box),
    HComp(Box), Transp(Box<Type>, Box, Box),
    FlatMod(Box), SharpMod(Box),
    Glue(Box, Box), Unglue(Box),
    HitDef(Symbol, Vec<(Symbol, Vec<Param>)>),

    // ── FRP / 时序 ──
    SignalNew(Box), SignalMap(Box, Box),
    SignalFilter(Box, Box), SignalFold(Box, Box, Box),
    SignalMerge(Box, Box), Delay(Box), Advance(Box),
    Stable(Box), Unbox(Box), ClockNew(Symbol),

    // ── 元编程 ──
    Comptime(Box), CompilerMacroDef(Symbol, usize, Box),
    MetaQuery(Symbol), AdviceDef(Symbol, Box, Box),

    // ── 定理证明 ──
    TheoremDef(Symbol, Box), ProofTactic(Symbol, Vec),
    Obligation(Box),

    // ── 内存/区域 ──
    RegionNew(Symbol), RegionAlloc(Box, Box),
    RegionFree(Box), PtrRead(Box), PtrWrite(Box, Box),

    // ── 会话类型 ──
    Session(SessionOp, Box),
}
```

### 4.2 关键辅助类型 ✅

```rust
pub enum Literal {
    I8(i8), I16(i16), I32(i32), I64(i64),
    U8(u8), U16(u16), U32(u32), U64(u64),
    F32(f32), F64(f64),
    Bool(bool), String(String), Char(char), Unit,
}

pub struct Lambda {
    pub params: Vec<Param>,
    pub body: Box<CoreExpr>,
    pub ret_type: Option<Type>,
}

pub struct Param {
    pub name: Symbol,
    pub ty: Option<Type>,
    pub grade: Grade,
    pub mode: Mode,
}

pub struct Handler {
    pub effect_name: Symbol,
    pub type_args: Vec<Type>,
    pub clauses: Vec<HandlerClause>,
    pub return_clause: Option<Box<CoreExpr>>,
}

pub struct HandlerClause {
    pub operation: Symbol,
    pub params: Vec<Symbol>,
    pub continuation: Symbol,
    pub state: Option<Symbol>,
    pub body: Box<CoreExpr>,
}

pub enum SessionOp { Send, Recv, Close, Fork(Box<CoreExpr>) }

pub struct CoreProgram {
    pub data_decls: Vec<DataDecl>,
    pub effect_decls: Vec<EffectDecl>,
    pub defs: Vec<CoreDef>,
}

pub struct CoreDef {
    pub name: Symbol,
    pub ty: Option<Type>,
    pub effects: EffectRow,
    pub grade: Grade,
    pub mode: Mode,
    pub determinism: Determinism,
    pub body: CoreExpr,
    pub requires: Option<Predicate>,
    pub ensures: Option<Predicate>,
}
```

---

## 5. 实现状态矩阵

### 前端 (tisp-frontend)

| 特性 | Lexer | Parser | Desugar | 总评 |
|------|-------|--------|---------|------|
| 字面量 (int, float, bool, string, nil) | ✅ | ✅ | ✅ | ✅ |
| 标识符 / 关键字 | ✅ | ✅ | ✅ | ✅ |
| 列表 / 向量 | ✅ | ✅ | ✅ | ✅ |
| Map / Set | ✅ | ✅ | ⚠️ | ⚠️ |
| `def` / `defn` | ✅ | ✅ | ✅ | ✅ |
| `defdata` / ADT | ✅ | ✅ | ✅ | ✅ |
| `match` | ✅ | ✅ | ✅ | ✅ |
| `let` | ✅ | ✅ | ✅ | ✅ |
| `if` | ✅ | ✅ | ✅ | ✅ |
| `fn` / lambda | ✅ | ✅ | ✅ | ✅ |
| `handle` / `perform` | ✅ | ✅ | ⚠️ | ⚠️ |
| `-> Type` 返回类型 | ✅ | ✅ | ✅ | ✅ |
| `:requires` / `:ensures` 合约 | ✅ | ✅ | ⚠️ | ⚠️ |
| 等级参数 `{grade name : Type}` | ⬜ | ⬜ | ⬜ | ⬜ |
| `defpred` | ⬜ | ⬜ | ⬜ | ⬜ |
| `defgeneric` / `defmethod` | ⬜ | ⬜ | ⬜ | ⬜ |
| 宏 / `defmacro` | ⬜ | ⬜ | ⬜ | ⬜ |
| `ns` / 模块系统 | ⬜ | ⬜ | ⬜ | ⬜ |

### 中端 (tisp-middle)

| 特性 | 类型推断 | 效果推断 | 等级检查 | 模式分析 | 确定性 | 区域 | 优化 | 总评 |
|------|---------|---------|---------|---------|--------|------|------|------|
| HM 类型推断 (Algorithm W) | ✅ | — | — | — | — | — | — | ✅ |
| Let-多态 | ✅ | — | — | — | — | — | — | ✅ |
| 递归函数 | ✅ | — | — | — | — | — | — | ✅ |
| ADT 构造器类型 | ✅ | — | — | — | — | — | — | ✅ |
| 模式匹配类型检查 | ✅ | — | — | — | — | — | — | ✅ |
| 效果行推断 | — | ✅ | — | — | — | — | — | ✅ |
| 效果子类型检查 | — | ✅ | — | — | — | — | — | ✅ |
| QTT multiplicity | — | — | ✅ | — | — | — | — | ✅ |
| 模式分析 (In/Out/Free) | — | — | — | ✅ | — | — | — | ✅ |
| 确定性推断 | — | — | — | — | ✅ | — | — | ✅ |
| 区域推断 | — | — | — | — | — | ⚠️ | — | ⚠️ |
| 液态类型 (Z3) | — | — | — | — | — | — | — | ⚠️ |
| 优化管线 (内联/折叠) | — | — | — | — | — | — | ✅ | ✅ |
| 子优化模块 | — | — | — | — | — | — | ⬜ | ⬜ |
| 效果编译 (evidence passing) | — | — | — | — | — | — | — | ⚠️ |

### 后端 (tisp-backend)

| 特性 | 解释器 | Codegen | 总评 |
|------|--------|---------|------|
| 树遍历解释器 | ✅ | — | ✅ |
| 算术/比较/布尔 | ✅ | — | ✅ |
| `println` / IO | ✅ | — | ✅ |
| ADT 构造/模式匹配 | ✅ | — | ✅ |
| 递归函数调用 | ✅ | — | ✅ |
| 模型检查器 (BFS) | ✅ | — | ✅ |
| Channel 操作 | ✅ | — | ✅ |
| FRP 流/时钟 | ✅ | — | ✅ |
| LLVM IR 生成 | — | ⬜ | ⬜ |
| HO 转换/lambda lifting | — | ⬜ | ⬜ |
| 尾调用优化 | — | ⬜ | ⬜ |

### 运行时 (tisp-runtime)

| 特性 | 实现 | 测试 | 总评 |
|------|------|------|------|
| 逻辑合一 | ✅ (436L) | ✅ (6 测试) | ✅ |
| 约束 (CLP) | ✅ (318L) | ✅ (4 测试) | ✅ |
| FRP 信号 | ✅ (230L) | ✅ (4 测试) | ✅ |
| HoTT | ✅ (195L) | ✅ (5 测试) | ✅ |
| 效果运行时 | ✅ (165L) | ✅ (2 测试) | ✅ |
| 进程 (π/ρ/κ/amb) | ✅ (238L) | ✅ (4 测试) | ✅ |
| 定理证明 | ✅ (212L) | ✅ (3 测试) | ✅ |
| 区域分配器 | ✅ (260L) | ✅ (4 测试) | ✅ |
| 持久化数据结构 | ✅ (94L) | — | ✅ |
| 并发逻辑 | ✅ (94L) | ✅ (2 测试) | ✅ |
| 依赖分级类型 | ✅ (321L) | ✅ (4 测试) | ✅ |
| 标准库 | ✅ (129L) | ✅ (8 测试) | ✅ |
| 元编程 | ✅ (112L) | ✅ (4 测试) | ✅ |

### 总统计

| 指标 | 数值 |
|------|------|
| 总代码行数 | 9,904 行 Rust |
| 单元测试 | 78 个（全部通过） |
| 类型检查通过的示例 | 6/12 |
| 编译警告 | 0 |
| Crate 数 | 6 |
| 核心 AST 节点 | 84 个 |
| 类型变体 | 16 个 |
| 实现的功能效果标签 | 16 个 |

---
