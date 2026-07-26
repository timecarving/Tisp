# 02 — Tisp 高级特性

> 覆盖：QTT · 效果系统 · 模式/确定性/区域 · 液态类型 · HoTT · FRP · 逻辑编程 · 进程演算 · 验证

---

## 1. Quantitative Type Theory (QTT)

### 1.1 等级语义 ✅

每个绑定带有 **multiplicity（使用次数）**：

| 等级 | 符号 | 语义 |
|------|------|------|
| 0 | `@0` | **擦除** — 编译时使用，运行时不存在 |
| 1 | `@1` | **线性** — 恰好使用一次，不可复制 |
| ω | `@ω` (默认) | **无限制** — 可任意复制和使用 |

### 1.2 语法 ⚠️

```clojure
;; 函数参数等级标注（设计语法，语法未完整实现）
(defn f [{1 x : i64}] -> i64
  (* x 2))

;; QTT 等级检查 ✅（类型推断中 QTT multiplicity 已验证）
(defn vhead [{0 n : Nat}, xs : (Vec a (S n))] -> a
  (match xs (VCons x _) x))
;; n 的使用等级 = 0（仅用于类型）
```

> ⚠️ 等级语法 `{r name : Type}` 使用 set literal 写法，当前 desugar 无法解析。实际可用的等级检查通过类型系统隐式进行。

### 1.3 等级检查 ✅

QTT 等级检查在 `crates/tisp-middle/src/grade_check.rs` 中实现：

- 检查每个绑定是否按照声明的等级使用
- 支持等级余环（semiring）操作
- 0 与 1 之间的约束传播

---

## 2. 效果系统

### 2.1 效果行 (Effect Row) ✅

```clojure
Pure                    ; 无效果
Closed([IO])            ; 包含 IO 效果
Closed([IO, State])     ; 多个效果
```

效果行类型定义：

```rust
pub enum EffectRow {
    Pure,
    Open(Vec<EffectLabel>, Box<EffectRow>),
    Closed(Vec<EffectLabel>),
    Var(u64),
}
```

### 2.2 效果标签 ✅

```rust
pub enum EffectLabel {
    Named(Symbol),
    State(Box<Type>),
    Reader(Box<Type>),
    Writer(Box<Type>),
    Except(Box<Type>),
    IO,
    Search,
    Signal(Symbol),
    Channel,
    Async,
    Spawn,
    Unsafe,
    Ambient,
    Reflect,
    Reaction,
    Spi,
    Crypto,
}
```

### 2.3 内置效果函数 ✅

| 函数 | 签名 | 效果 |
|------|------|------|
| `println` | `∀a. a → Unit` | `Closed([IO])` |
| `+`, `-`, `*`, `/` | `i64 → i64 → i64` | `Pure` |
| `=`, `<`, `<=` | `i64 → i64 → bool` | `Pure` |

### 2.4 效果子类型 ✅

```rust
fn effect_subtype(a: &EffectRow, b: &EffectRow) -> bool {
    // Pure 接受任意效果（默认宽松）
    // Closed([xs]) ⊆ Closed([ys]) iff xs 所有元素在 ys 中
}
```

### 2.5 效果声明 ⬜

```clojure
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))

(defeffect Reader r
  (ask [] -> r))
```

### 2.6 效果处理器 ⚠️

```clojure
(handle body
  (State)
  (get [] [k] (k initial-value))
  (put [v] [k] ...))
```

> ⚠️ `handle` 语法已部分解析（`desugar_handle`），但效果处理器运行时未完全实现。

---

## 3. 模式系统 (Mercury-style)

### 3.1 模式值 ✅

```rust
pub enum Mode {
    In,      // 输入 — 参数已被实例化
    Out,     // 输出 — 参数将被产出/绑定
    Free,    // 自由 — 参数状态未知
}
```

### 3.2 模式分析 ✅

模式分析在 `crates/tisp-middle/src/mode_analysis.rs` 中实现：

- **count_usages**: 统计变量在表达式中的使用次数
- **find_first_binding**: 找到变量首次被绑定的位置
- **infer_mode_for_var**: 基于 usage + binding 位置推断模式

| 使用次数 | 首次绑定位置 | 推断模式 |
|---------|-------------|---------|
| 0 | (无) | `Free` (output parameter) |
| > 0 | `pos == 0` | `Out` (producer) |
| > 0 | `pos > 0` | `In` (consumer) |

---

## 4. 确定性分析

### 4.1 确定性类别 ✅

```rust
pub enum Determinism {
    Det,        // 恰好一个解，永不失败
    SemiDet,    // 最多一个解，可能失败
    Multi,      // 至少一个解（不可能失败）
    NonDet,     // 零或多个解
}
```

### 4.2 确定性分析 ✅

在 `crates/tisp-middle/src/determinism_analysis.rs` 中：

| 结构 | 规则 |
|------|------|
| `Lit` | `{can_fail: false, max_solutions: One}` |
| `App` | `conjunction(f_cat, a_cat)` |
| `If` | `conjunction(cond_cat, conjunction(then_cat, else_cat))` |
| `Match` | 各 arm 的确定性析取后与 scrutinee 合取 |
| `Handle` | 继承 body 的确定性 |

### 4.3 析取/合取运算 ✅

```rust
// 合取 (conjunction): A ∧ B — 两者都成功才成功
pub fn det_conjunction(a: &DetCategory, b: &DetCategory) -> DetCategory

// 析取 (disjunction): A ∨ B — 任一成功就成功
pub fn det_disjunction(a: &DetCategory, b: &DetCategory) -> DetCategory
```

---

## 5. 区域推断

### 5.1 区域类型 ✅

```rust
pub enum RegionKind {
    Finite,       // 栈分配，有限生命周期
    Infinite,     // 堆分配，页链接表
    Scalar,       // 标量（word-size 值），寄存器分配
}
```

### 5.2 区域推断 ⚠️

> ⚠️ 区域推断在 `crates/tisp-middle/src/region_infer.rs` 中仅实现基础框架（65 行）。区域运行时在 `crates/tisp-runtime/src/region.rs` 中完整实现（260 行）。

### 5.3 区域运行时 ✅

```
- 有限区域: 固定大小数组，reset 时清除
- 无限区域: 页链接表，动态增长
- 标量区域: 直接寄存器/栈传递，不走分配器
```

---

## 6. 液态类型 (Liquid Types)

### 6.1 Refinement 类型语法 ⚠️

```clojure
{x : T | predicate}
```

> ⚠️ `Type::Refined` 变体已定义，predicate 语法已解析，但 Z3 验证集成不完整。

### 6.2 Z3 集成 ⚠️

```
crates/tisp-backend/src/z3_bridge.rs  : 139 行 — Z3 进程交互桥
crates/tisp-middle/src/liquid_types.rs: 270 行 — 液态类型检查器
```

### 6.3 合约 ⚠️

语法已解析：
- `:requires pred` — 前置条件
- `:ensures pred` — 后置条件

---

## 7. Homotopy Type Theory (HoTT)

### 7.1 Interval 类型 ✅

```clojure
i0  ; → false (Interval endpoint 0)
i1  ; → true  (Interval endpoint 1)
```

### 7.2 Path 类型 ✅

```clojure
(path-lam i body)     ; 路径 lambda → CoreExprNode::PathLam(var, body)
(path-apply p i)      ; 路径应用 → CoreExprNode::PathApp(path, point)
```

### 7.3 Homogeneous Comp / Transport ⚠️

```clojure
(hcomp expr)          ; → CoreExprNode::HComp(expr)
(transp type fi a)    ; → CoreExprNode::Transp(type, fi, a)
```

> ⚠️ AST 节点存在，解释器 pass-through。

### 7.4 Cohesive Modalities ✅

```clojure
(flat expr)           ; 风格 flat modality → pass-through 解释
(sharp expr)          ; sharp modality → pass-through 解释
```

### 7.5 Glue / HIT ⚠️

```
Glue(Box<CoreExpr>, Box<CoreExpr>)  ; AST 节点存在
Unglue(Box<CoreExpr>)               ; AST 节点存在
HitDef(Symbol, Vec<(Symbol, Vec<Param>)>) ; AST 节点存在
```

> ⚠️ 以上节点已定义但解释器为 stub 实现。

---

## 8. 时序类型 (FRP — Functional Reactive Programming)

### 8.1 流 (Stream) ✅

```rust
pub struct Stream<T> {
    // 懒惰无限流，基于 thunks: Arc<Mutex<Option<Box<FnOnce>>>>
}
```

操作：`unfold`, `repeat`, `take`, `fold`, `next`, `now`

### 8.2 时钟 (Clock) ✅

```rust
pub struct Clock {
    pub name: String,
    pub tick_rate_hz: u64,
    pub current_tick: u64,
}
```

### 8.3 信号 (Signal) ✅

```rust
pub struct Signal<T> {
    // 可推可拉的响应式值
    // 支持 map, filter, fold, merge
}
```

### 8.4 FRP 内置函数 ✅

```clojure
(stream init step)     ; 创建流
(stream-take s n)      ; 取前 n 个元素
(delay val)            ; Fitch-style 时间步延迟
(advance s)            ; 推进到下一时间步
(clock name)           ; 创建具名时钟
```

### 8.5 时序类型系统 ⚠️

```
Type::Temporal(TemporalOp, Box<Type>)  ; 类型变体已定义
TemporalOp: Next | Always | Eventually
```

> ⚠️ 类型变体存在但类型推断中为 pass-through。

---

## 9. 逻辑编程

### 9.1 合一引擎 ✅

```rust
// crates/tisp-runtime/src/logic.rs (436 行)
pub struct UnificationEngine { /* trail + choice points + backtracking */ }

pub enum LogicValue {
    Var(u64),       // 逻辑变量
    Int(i64),       // 具体值
    Str(String),
    Cons(Box<LVar>, Box<LVar>),
    Nil,
}
```

### 9.2 回溯搜索 ✅

```rust
pub enum SearchResult {
    Solutions(Vec<Substitution>),
    NoSolution,
    SearchLimitReached,
}

pub fn dfs_search(goal: Goal, max_depth: usize) -> SearchResult
```

### 9.3 CLP(有限域) ✅

```rust
// crates/tisp-runtime/src/constraint.rs (318 行)
pub struct ConstraintStore {
    domains: HashMap<u64, Domain>,
    propagators: Vec<Propagator>,
}
```

CLP 操作已连接到解释器：

| 语法 | AST 节点 | 解释器实现 |
|------|---------|-----------|
| `(domain var lo hi)` | `Domain` | `clp_store.new_int_var(lo, hi)` — 创建 FD 变量 |
| `(constrain expr)` | `Constrain` | `clp_store.add_propagator(...)` — 加传播器 |
| `(label var)` | `Label` | `clp_store.label()` — 枚举域值 |
| `(all-diff [vars...])` | `AllDifferent` | `clp_store.add_all_different(...)` — 互异约束 |

### 9.4 ALP (Abduction) ✅

```rust
// crates/tisp-runtime/src/abduction.rs (新增)
pub struct AbductionEngine { /* 假设生成 + 一致性检查 */ }
pub struct Hypothesis { pub var: String, pub value: i64 }
pub struct Explanation { pub hypotheses: Vec<Hypothesis> }
```

| 语法 | AST 节点 | 解释器实现 |
|------|---------|-----------|
| `(abduce goal var ...)` | `Abduce` | `AbductionEngine::generate_hypotheses(...)` — 生成解释 |

### 9.5 并发逻辑编程 ✅

```rust
// crates/tisp-runtime/src/concurrent.rs (94 行)
// Guarded Horn Clauses, OR-并行, committed choice
pub struct ParallelEngine { max_threads: usize }
```

### 9.5 语法支持 ⚠️

```clojure
;; 设计语法（部分已实现 desugar）:
(defpred append [List a :in, List a :in, List a :out] :det
  ([[] Ys Ys])
  ([([X . Xs] Ys [X . Zs])]
   (append Xs Ys Zs)))

;; 可用语法（通过 Core AST 节点）:
(fresh x)               ; → CoreExprNode::Fresh(name)
(unify a b)             ; → CoreExprNode::Unify(a, b)
(search goal)           ; → CoreExprNode::Search(goal)
(commit goal)           ; → CoreExprNode::Commit(goal)
(abduce goal vars...)   ; → CoreExprNode::Abduce(goal, vars)
```

> ⚠️ 以上 AST 节点全部存在且脱糖实现，但完整的 Mercury-style 谓词语法未完全集成到 parser。

---

## 10. 进程演算

### 10.1 π-calculus (Channel Effect) ✅

运行时实现（`crates/tisp-runtime/src/process.rs`, 238 行）：

```rust
pub struct ProcessRuntime {
    channels: HashMap<Symbol, Channel>,
}
pub struct Channel { buffer: Arc<Mutex<Vec<Value>>> }
```

操作：`chan`, `send`, `recv` 作为解释器 built-in。

### 10.2 Async π-calculus ✅

```
CoreExprNode::AsyncSend(channel, value)
CoreExprNode::AsyncRecv(channel)
```

### 10.3 Applied π-calculus ✅

```
CoreExprNode::CryptoEncrypt(plaintext, key)
CoreExprNode::CryptoDecrypt(ciphertext, key)
CoreExprNode::CryptoSign(message, key)
CoreExprNode::CryptoVerify(message, signature)
CoreExprNode::CryptoHash(data)
```

### 10.4 spi-calculus ✅

```
CoreExprNode::SpiSecret(value)
CoreExprNode::SpiCommit(a, b)
CoreExprNode::SpiCheck(a, b)
```

### 10.5 Safe Ambients ✅

```
CoreExprNode::AmbientNew(name)
CoreExprNode::AmbientEnter(ambient, capability)
CoreExprNode::AmbientExit(ambient, capability)
CoreExprNode::AmbientOpen(ambient, capability)
```

### 10.6 ρ-calculus (Reflective) ✅

```
CoreExprNode::RhoQuote(process)
CoreExprNode::RhoDrop(name)
CoreExprNode::RhoLift(channel, process)
```

### 10.7 κ-calculus (Chemical) ✅

```
CoreExprNode::KappaBind(complex, site1, site2, value)
CoreExprNode::KappaUnbind(complex, site)
CoreExprNode::KappaReact(rule)
```

### 10.8 SKI / ς-calculus ⚠️

```
CoreExprNode::SkiS
CoreExprNode::SkiK
CoreExprNode::SkiI
CoreExprNode::SkiApp(term, arg)
CoreExprNode::SkiReduce(term)
CoreExprNode::SigmaInvoke(obj, method)
CoreExprNode::SigmaUpdate(obj, method)
```

> ⚠️ AST 节点存在，运行时实现为 stub。

---

## 11. 验证引擎

### 11.1 模型检查器 ✅

```rust
// crates/tisp-backend/src/process.rs: ModelChecker
pub struct ModelChecker<S> {
    max_depth: usize,
}

impl<S: Clone + Eq + Hash + Debug> ModelChecker<S> {
    pub fn check_reachability(
        initial: S,
        target: impl Fn(&S) -> bool,
        transitions: impl Fn(&S) -> Vec<S>,
    ) -> VerificationResult
}
```

### 11.2 BFS 状态空间探索 ✅

```
--verify examples/hello.tisp

; verification result:
;   property holds: true
;   search depth: 3
;   trace: depth 0: 0 → depth 1: 1 → depth 2: 3 → depth 3: 5
```

### 11.3 属性规范语言 ⬜

```clojure
;; 设计阶段:
(defprop (reachable [state : State]) : Bool)
(defprop (safe [bad-state : State]) : Bool
  (not (reachable bad-state)))
```

---

## 12. 元编程

### 12.1 Compile-time 求值 ⚠️

```
CoreExprNode::Comptime(expr)        ; 编译时求值
CoreExprNode::CompilerMacroDef(...) ; 编译器宏
CoreExprNode::MetaQuery(Symbol)     ; 元查询
```

### 12.2 AOP (Aspect-Oriented Programming) ⚠️

```
CoreExprNode::AdviceDef(pointcut, advice, body)
```

> ⚠️ 以上 AST 节点全部存在，运行时部分实现。

---

## 13. 定理证明

### 13.1 证明状态 ✅

```rust
// crates/tisp-runtime/src/theorem.rs (212 行)
pub struct ProofState {
    pub goals: Vec<Goal>,
    pub assumptions: Vec<Term>,
}

pub struct Goal {
    pub context: Vec<(String, Term)>,
    pub target: Term,
}
```

### 13.2 策略 (Tactics) ✅

```
intro   : 引入假设 → Term::Pi(x, a, b) 处理
reflexivity : 自反性 → Term::Rel 处理
```

### 13.3 AST 节点 ✅

```
CoreExprNode::TheoremDef(name, body)
CoreExprNode::ProofTactic(name, args)
CoreExprNode::Obligation(goal)
```

---
