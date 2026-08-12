# Tisp Language Specification

**Version:** 0.1.0  
**Status:** Design specification for implementation

> 状态符号(与 standard_doc/04-implementation-status.md 一致):✅ 完全实现 | ⚠️ 部分实现 | ⬜ 仅设计(2026-08)

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Design Philosophy](#2-design-philosophy)
3. [Lexical Structure](#3-lexical-structure)
4. [Data Structures](#4-data-structures)
5. [Expressions](#5-expressions)
6. [Definitions](#6-definitions)
7. [Algebraic Data Types](#7-algebraic-data-types)
8. [Pattern Matching](#8-pattern-matching)
9. [Type System Overview](#9-type-system-overview)
10. [Quantitative Type Theory (QTT)](#10-quantitative-type-theory)
11. [Graded Modal Types](#11-graded-modal-types)
12. [Effect System](#12-effect-system)
13. [Mode System](#13-mode-system)
14. [Determinism](#14-determinism)
15. [Liquid Types](#15-liquid-types)
16. [Homotopy Type Theory](#16-homotopy-type-theory)
17. [Cohesive HoTT](#17-cohesive-hott)
18. [Temporal Types](#18-temporal-types)
19. [Dependent Graded Types](#19-dependent-graded-types)
20. [Session Types](#20-session-types)
21. [Logic Programming](#21-logic-programming)
22. [Generic Functions (OOP)](#22-generic-functions)
23. [Typeclasses](#23-typeclasses)
24. [Macros](#24-macros)
25. [Module System](#25-module-system)
26. [FFI & System-Level Programming](#26-ffi--system-level-programming)
27. [Process Calculi](#27-process-calculi)
28. [Verification](#28-verification)
29. [Built-in Functions](#29-built-in-functions)
30. [Compiler Pragmas](#30-compiler-pragmas)

---

## 1. Introduction ✅

Tisp is a **pure declarative**, **unified-method**, **multi-paradigm**, **high-performance**, **system-level** Lisp dialect.

### Core Identity

- **Lisp-1**: Functions and variables share a single namespace.
- **Pure declarative**: All computation is expressed as annotated relations. No imperative escape hatch.
- **Unified method**: All paradigms emerge from a single `def` form with six annotation dimensions.
- **Multi-paradigm**: Functional, logic, concurrent, reactive, and system-level programming are all first-class.
- **High-performance**: Compiles to LLVM IR with aggressive optimization. No GC.
- **System-level**: FFI, raw pointers, manual memory management through the effect system.

### Reader Principle

**Everything in the Reader is a first-class citizen.** Types, effects, grades, modes, determinism, and regions are all runtime-manipulable values. There is no "compile-time only" syntax.

---

## 2. Design Philosophy ⚠️

### Principle 1: Everything is an Annotated Relation

All definitions are `def` + six-dimensional annotations. Paradigms emerge from annotation values:

| Paradigm | Annotation Difference |
|----------|----------------------|
| Pure function | d=det, ε=· |
| Stateful computation | ε={State} |
| Logic predicate | d=nondet, m contains free→ground |
| Generic method | polymorphic type + specialization |
| System-level operation | ε={Unsafe}, @1 |
| FRP signal | ε={Signal} |

### Principle 2: Effects are the Universal Glue

State, errors, search, IO, unsafe operations, signals — all are effects. Monad is the optimization path for performance-sensitive code; the compiler automatically degrades from effect handlers to monadic encoding when safe.

### Principle 3: Annotations Constrain Each Other, Unified Solving

Type/effect/region/grade/mode/determinism are projections of the same constraint system. The compiler solves all dimensions simultaneously via fixpoint iteration.

### Principle 4: Declarative Throughout

No imperative escape hatch. System-level programming = declaring resource constraints + Unsafe effect gating.

### Principle 5: Syntax Sugar Reduces Cognitive Load

`def` is the core. `defn`, `defpred`, `defgeneric` are shorthand for common annotation combinations. All sugar mechanically desugars to `def`.

### Principle 6: Organic Composition

- Logic programming = Search effect + nondet determinism
- OOP = type specialization + method combination
- FRP = Signal effect
- System-level = Unsafe effect + manual regions
- No independent subsystems.

### Principle 7: Calculi are Communication Patterns

All process calculi (π, ρ, ambient, κ, spi, applied π, SKI) are specializations of the Communication effect family. Encoding relations = transformation functions between effect handlers. Verification = an effect handler that explores all paths.

**Calculi over Algebraic Effects**: Calculi (process calculi, logic search, temporal streams, state transitions) are the *abstract core* of the language — users express programs in terms of calculi. Algebraic effects (handle/perform) are the *encoding and verification substrate*: every calculus is expressible as an effect family (Search effect = logic programming, Communication effect = process calculi, Signal effect = FRP), and handlers encode transformations between calculi. Effect handlers are therefore implementation machinery, not the primary abstraction; all high-level constructs decompose into "calculus + effect handler" combinations.

### Principle 8: Strong Static Typing

Tisp is a **strongly statically typed** language: all types are checked at compile time via type inference with polymorphism (`--typecheck`, REPL `:type`), and a program that passes checking is guaranteed free of runtime type errors. Types, effects, grades, modes, determinism, and regions are projections of one unified constraint system solved by fixpoint iteration (Principle 3). Consistent with the Reader Principle, type expressions are first-class runtime-manipulable values — static checking and runtime reflection are two faces of the same type language.

---

## 3. Lexical Structure ⚠️

### 3.1 Character Set

UTF-8 encoded source files.

### 3.2 Whitespace and Comments

```
;; line comment — extends to end of line
#| block comment |#
```

Whitespace: space, tab, newline, form feed. Used as token separators.

### 3.3 Identifiers

```
ident     = start-char rest-char*
start-char = letter | _ | ! | - | ? | + | * | / | < | > | = | & | %
rest-char  = start-char | digit | . | :
```

Reserved words: `true`, `false`, `nil`.

### 3.4 Literals

| Literal | Syntax | Type |
|---------|--------|------|
| Integer | `42`, `-7` | `Int` |
| Float | `3.14`, `-2.5e10` | `Float` |
| Boolean | `true`, `false` | `Bool` |
| String | `"hello\nworld"` | `String` |
| Character | `\a`, `\newline`, `\space`, `\tab` | `Char` |
| Keyword | `:name`, `:version` | `Keyword` |
| Nil | `nil` | `Nil` |

### 3.5 Special Characters

```
( ) [ ] { } ' ` ~ ~@ # @
```

---

## 4. Data Structures ⚠️

All data structures are **immutable** (persistent).

### 4.1 List

```clojure
'(1 2 3)          ; quoted list
(list 1 2 3)      ; constructor
```

Type: `(List a)` — singly-linked persistent list.

### 4.2 Vector

```clojure
[1 2 3]           ; vector literal
```

Type: `(Vec a)` — persistent vector (HAMT-based, O(log32 n) access).

### 4.3 Map

```clojure
{:name "Tisp" :version 1}
```

Type: `(Map k v)` — persistent hash map.

### 4.4 Set

```clojure
#{1 2 3}
```

Type: `(Set a)` — persistent hash set.

### 4.5 Tuple

```clojure
(Pair 1 "hello")
```

Type: `(a × b)` — product type.

### 4.6 Unit

```clojure
Unit
```

Type: `Unit` — the unit type with one value.

---

## 5. Expressions ⚠️

### 5.1 Literals

Literals evaluate to themselves.

### 5.2 Variable Reference

```clojure
x                 ; look up x in current scope
```

### 5.3 Function Application

```clojure
(f x y z)         ; apply f to x, y, z
```

### 5.4 Lambda

```clojure
(fn [x y] (+ x y))
(fn [x : Int, y : Int] -> Int (+ x y))   ; with type annotations
```

### 5.5 Let Binding

```clojure
(let [x 1
      y 2]
  (+ x y))
```

### 5.6 If Expression

```clojure
(if condition then-expr else-expr)
```

### 5.7 Cond Expression

```clojure
(cond
  (> x 0)  "positive"
  (< x 0)  "negative"
  :else    "zero")
```

### 5.8 Threading Macros

```clojure
(-> x f g)        ; ≡ (g (f x))
(->> xs f g)      ; ≡ (g (f xs))
(as-> x name (f name) (g name))
(some-> x f g)    ; short-circuit on nil
```

### 5.9 Type Annotation

```clojure
(ann expr Type)   ; assert expr has type Type
```

---

## 6. Definitions ⚠️

### 6.1 Value Definition

```clojure
(def x 42)
(def x : Int 42)                    ; with type annotation
```

### 6.2 Function Definition

```clojure
(defn name [params] body)
(defn name : Type [params] -> RetType body)
```

Desugars to:
```clojure
(def name (fn [params] body))
```

### 6.3 Multi-arity Functions

```clojure
(defn greet
  ([name] (str "Hello, " name))
  ([greeting name] (str greeting ", " name)))
```

### 6.4 Recursive and Mutual Recursion

```clojure
(defn factorial [n]
  (if (<= n 1) 1 (* n (factorial (- n 1)))))
```

### 6.5 Private Definitions

```clojure
(defn- helper [x] ...)    ; not exported
(def- secret 42)
```

### 6.6 The Unified `def` Form

All definitions desugar to:

```clojure
(def name [params] ->[ε, ρ, @r, m, d] return-type
  body)
```

Where:
- `ε` — effect row
- `ρ` — region
- `@r` — grade (usage)
- `m` — mode
- `d` — determinism

---

## 7. Algebraic Data Types ⚠️

### 7.1 Data Type Definition

```clojure
(defdata (Maybe a)
  (Nothing)
  (Just [a]))

(defdata (Tree a)
  (Leaf [a])
  (Branch [(Tree a), (Tree a)]))
```

### 7.2 Record Syntax

```clojure
(defdata Person
  (MkPerson {name : String, age : Int}))
```

### 7.3 GADT

```clojure
(defdata (Expr a)
  (IntLit  [Int]        -> (Expr Int))
  (BoolLit [Bool]       -> (Expr Bool))
  (Add     [(Expr Int), (Expr Int)] -> (Expr Int))
  (If      [(Expr Bool), (Expr a), (Expr a)] -> (Expr a)))
```

### 7.4 Higher Inductive Types (HIT)

```clojure
(defdata-hit S1
  (base)
  (loop [i : I]
    :boundary [(i = i0) -> base
               (i = i1) -> base]))
```

### 7.5 Deriving

```clojure
(defdata Color (Red | Green | Blue)
  :deriving [Eq, Ord, Show])
```

---

## 8. Pattern Matching ⚠️

### 8.1 Match Expression

```clojure
(match value
  pattern1 result1
  pattern2 result2
  _        default)
```

### 8.2 Pattern Syntax

```clojure
42                          ; literal pattern
x                           ; variable binding
(Pair x y)                  ; constructor pattern
[head . tail]               ; cons pattern
_                           ; wildcard
(when pattern guard)        ; guard
(or pat1 pat2)              ; or-pattern
{x : Int | (> x 0)}        ; refined pattern
```

### 8.3 if-let / when-let

```clojure
(if-let [x (maybe-value)]
  (use x)
  (default))

(when-let [x (maybe-value)]
  (use x))
```

### 8.4 Exhaustiveness Checking

The compiler verifies that all possible values are covered. Missing cases produce a compile error.

---

## 9. Type System Overview ⚠️

Tisp's type system is a **unified modal dependent type theory** organized in layers:

```
Layer 5: Cohesive HoTT (ʃ ⊣ ♭ ⊣ ♯)
Layer 4: Temporal Types (⃝, □_t, ◇_t)
Layer 3: Session Types (binary + multiparty + dependent)
Layer 2: Liquid Types ({x : T | p(x)})
Layer 1: Graded Modal Types (□_r, ◇_ε)
Layer 0: QTT (multiplicity 0, 1, ω)
```

Plus the base type system:
- HM + rank-n polymorphism
- Higher-kinded types
- GADT + existential types
- Row polymorphism
- Type families + associated types
- Subtyping (effect, region, grade, determinism)

### Unified Type Annotation Syntax

```clojure
value : type [:annotation ...]

;; Full annotation:
f : (Int ->[{IO, State Int}, ρ_heap, @1, in, det] Bool)
;;        ↑effects   ↑region  ↑grade ↑mode ↑determinism
```

### Types as First-Class Values

Every type construct has a corresponding runtime value:

```clojure
Int                    ; → Type value
{IO, State}            ; → EffectRow value
@1                     ; → Grade value
:det                   ; → Determinism value
:in                    ; → Mode value
```

---

## 10. Quantitative Type Theory (QTT) ✅

Every binding has a **multiplicity**: `0`, `1`, or `ω`.

### 10.1 Multiplicity Semantics

| Multiplicity | Meaning | Runtime Behavior |
|-------------|---------|-----------------|
| `0` | Erased | Not passed, not allocated |
| `1` | Linear (used exactly once) | Move semantics, no copy |
| `ω` | Unrestricted | Traditional (ref-counted/region) |

### 10.2 Default Rules

- Explicit binding → `ω`
- Implicit binding → `0` (erased)

### 10.3 Syntax

```clojure
;; Explicit multiplicity
(defn ignore-n [{0 n : Nat}, xs : (Vec a n)] -> Nat
  (length-vec xs))

(defn consume [{1 x : a}] -> (Maybe a)
  (Just x))

;; Implicit = erased
(defn vlen [{0 n : Nat}, xs : (Vec a n)] -> Nat
  n)    ; n is known at compile time, erased at runtime
```

### 10.4 Linear Resource Protocols

```clojure
(defdata Door (MkDoor [DoorState]))

(defn open-door [{1 d : Door}] -> Door
  (MkDoor Open))

(defn close-door [{1 d : Door}] -> Door
  (MkDoor Closed))

;; Usage: linear guarantee prevents reuse of old state
(defn use-door [{1 d : Door}] -> Door
  (-> d open-door close-door))
```

### 10.5 Parametricity via 0-multiplicity

```clojure
;; Truly parametric: a is 0-multiplicity → cannot pattern match
(defn truly-parametric [{0 a : Type}, x : a] -> a
  x)    ; only possible implementation

;; Not truly parametric: a is ω → can pattern match
(defn not-parametric [{ω a : Type}, x : a] -> Bool
  (match a
    Int    true
    String false
    _      false))
```

---

### 10.4 依赖等级(实现,2026-08)

等级可为**编译期数值表达式**:数字 `(5 x : a)` → Nat(5)、符号 `(n x : a)` → Var(n)(绑定自类型参数,如 `(Vec i64 n)` 的 n)、复合 `((+ n 1) x : a)` → Add。检查语义为**使用计数 ≤ 等级**(上界,参考 Idris 2);数字等级常量折叠检查,符号等级可常量判定时检查、不可判定时警告放行;分支合并取计数上界。`0/1/ω` 为特例(0 擦除/1 恰好一次/ω 不限)。

## 11. Graded Modal Types ⚠️

QTT's {0, 1, ω} is a special case. Graded modal types generalize to **arbitrary semirings**.

### 11.1 Resource Algebra Declaration

```clojure
(defresource-algebra Nat
  :semiring (+ 0 * 1)
  :order <=)

(defresource-algebra Sec
  :lattice (join Public Private)
  :order (Public <= Private))

(defresource-algebra Cost
  :semiring (+ 0 * 1)
  :order <=
  :asymptotic true)
```

### 11.2 Graded Necessity (□_r)

```clojure
;; □_n A = value usable exactly n times
(defn apply-n [{n : Nat}, f : (a -> a) @[n], x : a @[n]] -> a
  (match n
    Z     x
    (S k) (apply-n k f (f x))))

;; Security lattice
(defn classify [{level : Sec}, x : a] -> (□_level a)
  ...)
```

### 11.3 Graded Possibility (◇_ε) = Effect System

```clojure
;; ◇_{IO} Int = computation returning Int with IO effect
(defn read-int [] -> (◇_{IO} Int)
  ...)
```

### 11.4 Built-in Resource Algebras

- **Nat**: Exact usage counting
- **Sec**: Security lattice (Public ≤ Private)
- **Cost**: Asymptotic cost analysis (Big-O)

---

## 12. Effect System ✅

### 12.1 Effect Declaration

```clojure
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))

(defeffect Except e
  (throw [e] -> (∀ [a] a)))

(defeffect IO
  (read-line [] -> String)
  (print-line [String] -> Unit))
```

### 12.2 Effect Handler

```clojure
(defn run-state [init f]
  (handle (f)
    (State s)
    (get [] [k s] (k s s))
    (put [v] [k _s] (k Unit v))))
```

### 12.3 Built-in Effects

| Effect | Operations | Description |
|--------|-----------|-------------|
| `State s` | get, put | Mutable state |
| `Reader r` | ask, local | Read-only environment |
| `Writer w` | tell | Accumulating output |
| `Except e` | throw | Error handling |
| `IO` | read-line, print-line, ... | Input/output |
| `Search` | choose | Backtracking search |
| `Unsafe` | ptr-read, ptr-write, ... | System-level operations |
| `Channel a` | send, recv, new, par | π-calculus communication |
| `Ambient` | enter, exit, open | Mobile ambients |
| `Reflect` | quote, drop, lift | ρ-calculus reflection |
| `Reaction` | bind, unbind, react | κ-calculus chemistry |
| `Signal` | subscribe, emit | FRP signals |
| `Session` | send, recv, fork | Session-typed channels |

> 实现注:表中部分操作(如 Channel 的 par/rep、Signal 的 subscribe/emit)在实现中为独立 Core AST 节点,而非 effect 操作;`--run` 走对应节点语义。

### 12.4 Effect Rows

```clojure
;; Pure (empty effect row)
f : (Int ->[·] Int)

;; Single effect
g : (Int ->[{IO}] Unit)

;; Multiple effects
h : (Int ->[{IO, State Int}] Bool)

;; Open effect row (polymorphic)
k : (a ->[{ε | Log}] b)    ; adds Log to whatever effects already present
```

### 12.5 Effect Subtyping

```
· ⊆ {IO} ⊆ {IO, State} ⊆ {IO, State, Except}
```

A pure function can be used where an effectful one is expected.

### 12.6 Monad as Optimization Path

```clojure
;; Effect style (flexible, dispatch overhead)
(handle body (State s) ...)

;; Monadic style (restricted, zero overhead)
(defn hot-path [s : State] -> (Int × State)
  (mlet [x  (get-m)
         _  (put-m (+ x 1))
         y  (get-m)]
    (pure (* y 2))))
```

The compiler detects single-handler, no-nesting patterns and automatically compiles to direct state-passing code.

---

## 13. Mode System ⚠️

Mercury-style instantiation tracking.

### 13.1 Modes

| Mode | Meaning |
|------|---------|
| `:in` | Input (ground → ground) |
| `:out` | Output (free → ground) |
| `:ground` | Fully instantiated |
| `:free` | Uninstantiated variable |
| `:crisp` | Indifferent to cohesive structure |
| `:cohesive` | Respects cohesive structure |

### 13.2 Mode Declarations

```clojure
(defpred append [List a :in, List a :in, List a :out] :det
  ([[] Ys Ys])
  ([([X . Xs] Ys [X . Zs])]
   (append Xs Ys Zs)))
```

### 13.3 Multi-mode Predicates

```clojure
;; Same predicate, different calling modes
(defpred member [a :free, List a :ground] :nondet
  ([X [X . _]])
  ([X [_ . Xs]] (member X Xs)))

(defpred member [a :ground, List a :ground] :semidet
  ([X [X . _]])
  ([X [_ . Xs]] (member X Xs)))
```

---

## 14. Determinism ✅

### 14.1 Determinism Categories

| Determinism | Can Fail? | Max Solutions |
|-------------|-----------|---------------|
| `:det` | No | 1 |
| `:semidet` | Yes | 1 |
| `:multi` | No | Many |
| `:nondet` | Yes | Many |
| `:cc_multi` | No | 1 (committed) |
| `:cc_nondet` | Yes | 1 (committed) |
| `:failure` | Yes | 0 |
| `:erroneous` | N/A | Never returns |

### 14.2 Determinism Inference Rules

- **Conjunction**: can fail if either can fail; many solutions if either has many
- **Disjunction**: can fail if both can fail; many solutions if either has many
- **Negation**: det ↔ failure, semidet → semidet
- **If-then-else**: combines condition, then, and else determinisms

### 14.3 Committed Choice

```clojure
(defpred first-solution [Goal :in, Result :out] :cc_nondet
  ...)
```

`cc_multi` and `cc_nondet` commit to the first solution found.

---

## 15. Liquid Types ✅

### 15.1 Refinement Types

```clojure
;; {x : T | predicate}(实现支持整数域 i64)
(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64
  x)

(sqrt 9)      ; OK
(sqrt -1)     ; 编译错误:实参不满足精化,反例 x = -1
```

### 15.2 返回精化验证(路径敏感)

```clojure
(defn abs [x : i64] -> {n : i64 | (>= n 0)}
  (if (>= x 0) x (- 0 x)))
;; 验证器以 if→ite 路径敏感方式验证两分支均满足返回精化
```

> 实现状态:精化类型与契约经 Z3(SMT-LIB2)求解验证(调用点/返回/契约,违反带反例);谓词自动推断(Liquid Type Inference)未实现。

### 15.3 Design by Contract

```clojure
(defn transfer [from : Account, to : Account, amount : {n : Int | (> n 0)}]
  -> (Account × Account)
  :requires (> (balance from) amount)
  :ensures  (= (+ (balance result.1) (balance result.2))
               (+ (balance from) (balance to)))
  ...)
```

### 15.4 Interaction with QTT

Refinement predicates can use 0-multiplicity variables (verified at compile time, erased at runtime):

```clojure
(defn divide [n : Int, d : {x : Int | (!= x 0)}] -> Int
  "d != 0 verified at compile time, no runtime check needed"
  (quot n d))
```

---

## 16. Homotopy Type Theory ⚠️

Enabled with `--cubical` compiler flag.

### 16.1 Interval Type

```clojure
;; I has two endpoints: i0, i1
;; Operations: ~i (negation), i ∧ j (meet), i ∨ j (join)
```

### 16.2 Path Types

```clojure
;; Path A x y = path from x to y in A
;; Parameterized by interval: (fn [i : I] -> A) where (f i0) = x, (f i1) = y

(defn refl [x : A] -> (Path A x x)
  (fn [i] x))

(defn sym [p : (Path A x y)] -> (Path A y x)
  (fn [i] (p (~ i))))
```

### 16.3 Function Extensionality (Provable, not axiom!)

```clojure
(defn fun-ext [{0 A B : Type}, f g : (A -> B),
               h : (∀ [x : A] (Path B (f x) (g x)))]
  -> (Path (A -> B) f g)
  (fn [i] (fn [x] (h x i))))
```

### 16.4 Higher Inductive Types

```clojure
(defdata-hit S1
  (base)
  (loop [i : I]
    :boundary [(i = i0) -> base (i = i1) -> base]))

(defdata-hit (Quotient {0 A : Type} [R : (A -> A -> Type)])
  (quot [A])
  (quot-eq [x y : A, p : (R x y), i : I]
    :boundary [(i = i0) -> (quot x) (i = i1) -> (quot y)]))
```

### 16.5 Univalence

```clojure
;; ua : Equiv A B → Path Type A B
;; transport (ua e) x ≡ e.f x   ← computational!
(defn ua [{0 A B : Type}, e : (Equiv A B)] -> (Path Type A B)
  ...)
```

---

## 17. Cohesive HoTT ⚠️

> 实现注(2026-08):ʃ(shape)以最小可区分语义落地(返回 Shape 容器,路径端点);`crisp` 上下文检查已实现;完整同伦语义(ʃ 形状代数、路径连通计算)未实现。

Enabled with `--cohesion` compiler flag.

### 17.1 Adjoint Triple: ʃ ⊣ ♭ ⊣ ♯

| Modality | Kind | Description |
|----------|------|-------------|
| `♭` (flat) | Comonadic | Strips topological/smooth structure, leaves discrete points |
| `♯` (sharp) | Monadic | Embeds as codiscrete space |
| `ʃ` (shape) | Monadic | Extracts homotopy shape |

### 17.2 Crisp/Cohesive Contexts

```clojure
;; @♭ x : A  — x is crisp (indifferent to cohesive structure)
;; x : A     — x is cohesive (varies continuously)

(defn continuous-map [f : (A -> B)]
  -> (♭ A -> ♭ B)
  ...)
```

### 17.3 Practical Applications

```clojure
;; Distinguish continuous vs discrete signals
(defn sample [{0 rate : Real}, sig : (ContinuousSignal a)]
  -> (DiscreteSignal a)
  ...)

;; Differential privacy
(defn dp-mechanism [{0 ε : Real}, f : (A -> B)]
  -> (♭ A -> B)
  ...)
```

---

## 18. Temporal Types ⚠️

### 18.1 Temporal Modalities

| Operator | Meaning |
|----------|---------|
| `⃝ A` | Value of type A available at next time step |
| `□_t A` | Value of type A available at all time steps (stable) |
| `◇_t A` | Value of type A available at some future time step |

### 18.2 Stream Type

```clojure
(defdata (Stream a)
  (::: [a, (⃝ (Stream a))]))
;; Stream a ≅ a × ⃝(Stream a)
```

### 18.3 Stable Types

Types that don't change over time. Can safely cross time steps.

```clojure
;; Int, Bool, String are Stable
;; (Stream a) is NOT Stable
;; (a -> b) is NOT Stable (closure may capture temporal values)
```

### 18.4 Causality, Productivity, No Space Leaks

- **Causality**: Current output depends only on current and past inputs
- **Productivity**: Every stream element computable in finite time (guarded recursion)
- **No space leaks**: ⃝ A values safely GC'd after two time steps

### 18.5 LTL as Types

```clojure
;; □_t P = "P holds at all time steps"
;; ◇_t P = "P holds at some time step"

(defn no-negative-balance [account : (Stream Account)]
  -> (□_t {bal : Int | (>= bal 0)})
  "Guarantee balance is always non-negative"
  ...)
```

### 18.6 Multi-clock

```clojure
(deftype-class Clock [cl]
  (type Time cl)
  (type Tag cl)
  (init-clock [cl] -> (RunningClock (Time cl) (Tag cl))))
```

---

## 19. Dependent Graded Types ⚠️

### 19.1 Graded Dependent Function (Π_r)

```clojure
;; (Π [x : A]_r -> B x)
;; r = usage of x in body
;; r=0: x only in types (erased parameter)
;; r=1: x used exactly once
;; r=ω: x unrestricted

(defn vhead [{0 n : Nat}, xs : (Vec a (S n))] -> a
  (match xs (VCons x _) x))
```

### 19.2 Graded Dependent Pair (Σ_r)

```clojure
;; (Σ [x : A]_r × B x)
;; r = usage of x after projection
```

### 19.3 Grade Propagation

If `f : (Π [x : A]_r -> B x)` and `x` appears in `B x` with grade `s`, then total usage of `x` = `r + s` (semiring addition).

---

## 20. Session Types ⚠️

### 20.1 Binary Session Types

```clojure
(deftype CalcProto
  (Send Int (Recv Bool End)))

(defn calc-client [ch : (Chan CalcProto)] ->[Channel] Unit
  (send ch 42)
  (let [result (recv ch)]
    (println "Result:" result)))
```

### 20.2 Dependent Session Types

```clojure
(deftype AuthProto
  (Send String
    (Choice
      (Label :ok (Send Token End))
      (Label :err (Send String End)))))
```

### 20.3 Multiparty Session Types (MPST)

```clojure
(defglobal-type RingProtocol [a b c]
  (a -> b : Int
    (b -> c : Int
      (c -> a : Int End))))

;; Projection:
;; RingProtocol ↾ a = !b<Int>. ?c<Int>. End
;; RingProtocol ↾ b = ?a<Int>. !c<Int>. End
```

---

## 21. Logic Programming ⚠️

### 21.1 Predicate Definition

```clojure
(defpred append [List a :in, List a :in, List a :out] :det
  ([[] Ys Ys])
  ([([X . Xs] Ys [X . Zs])]
   (append Xs Ys Zs)))
```

### 21.2 Unification and Backtracking

```clojure
(defpred member [a :free, List a :ground] :nondet
  ([X [X . _]])
  ([X [_ . Xs]] (member X Xs)))
```

### 21.3 Search Effect Integration

Logic programming = Search effect + nondet determinism.

```clojure
;; nondet predicates compile to functions with Search effect
;; Handler chooses search strategy:
(handle (member x [1 2 3 4])
  (Search)
  (choose [xs] [k]
    (fold (fn [acc x] (or (k x) acc)) false xs)))
```

### 21.4 Search Strategies

```clojure
;; DFS (default)
(handle body (Search) (choose [xs] [k] (dfs xs k)))

;; BFS
(handle body (Search) (choose [xs] [k] (bfs xs k)))

;; Iterative deepening
(handle body (Search) (choose [xs] [k] (id-dfs xs k 10)))
```

### 21.5 Constraint Logic Programming (CLP)

CLP(FD) — Constraint Logic Programming over Finite Domains — extends logic programming with domain variables and constraint propagation.

```clojure
;; Create a finite domain variable: x in [1..10]
(defn solve-sudoku []
  (domain x 1 10)
  (domain y 1 10)
  (domain z 1 10)
  ;; x < y constraint (propagates domains)
  (constrain (< x y))
  ;; All different constraint
  (all-diff [x y z])
  ;; Label: enumerate remaining domains
  (label x)
  (println x y z))
```

| Operation | Syntax | Semantics |
|-----------|--------|-----------|
| `domain` | `(domain var lo hi)` | Create FD variable with range [lo..hi] |
| `constrain` | `(constrain expr)` | Add constraint propagator |
| `label` | `(label var)` | Enumerate domain values of var |
| `all-diff` | `(all-diff [vars...])` | All variables pairwise distinct |

### 21.6 Abductive Logic Programming (ALP)

Abduction generates hypotheses that explain observations. Given a goal and abducible variables, the engine enumerates possible explanations.

```clojure
;; Abduce: find values of x, y that satisfy the goal
(defn find-explanation []
  (abduce (unify x y) x y))
;; Returns: "abduced-N" where N = number of possible explanations
```

| Operation | Syntax | Semantics |
|-----------|--------|-----------|
| `abduce` | `(abduce goal abducible ...)` | Generate hypotheses for abducibles to satisfy goal |

---

## 22. Generic Functions ⚠️

### 22.1 Declaration

```clojure
(defgeneric area [shape] -> Float)
(defgeneric collide [a b] -> Bool)    ; multi-dispatch
```

### 22.2 Method Definition

```clojure
(defmethod area [(c Circle)]
  (* pi (pow (radius c) 2)))

(defmethod collide [(c Circle) (r Rectangle)]
  (circle-rect-collision? c r))
```

### 22.3 Method Combination

```clojure
(defgeneric describe [obj] -> String
  :combination :around)

(defmethod describe :around [(s Shape)]
  (str "<shape: " (call-next-method) ">"))

(defmethod describe :primary [(c Circle)]
  (str "circle(r=" (radius c) ")"))
```

### 22.4 Compile-time Specialization

```clojure
;; area(Circle) → direct call to area_circle (zero overhead)
;; collide(Circle, Rectangle) → direct call to collide_circle_rect
;; Unknown calls → vtable lookup fallback
```

---

## 23. Typeclasses ✅

### 23.1 Declaration

```clojure
(defclass Eq a
  (== [a, a] -> Bool))

(defclass (Eq a) => (Ord a)
  (compare [a, a] -> Ordering))

(defclass Functor [f : (* -> *)]
  (fmap [(a -> b), (f a)] -> (f b)))
```

### 23.2 Instance

```clojure
(definstance (Eq Int)
  (== [a b] (prim-eq-int a b)))

(definstance (Functor Maybe)
  (fmap [f m]
    (match m
      (Nothing) (Nothing)
      (Just x)  (Just (f x)))))
```

### 23.3 Functional Dependencies

```clojure
(defclass Collection [c]
  :fun-deps [c -> (Elem c)]
  (type Elem c)
  (empty [] -> c)
  (insert [(Elem c), c] -> c))
```

---

## 24. Macros ⚠️

### 24.1 defmacro

```clojure
(defmacro unless [test body]
  `(if (not ~test) ~body nil))
```

### 24.2 Syntax Quote

```clojure
`(+ 1 ~x)          ; unquote x
`(list ~@items)    ; unquote-splice
```

### 24.3 Hygiene

Macros are hygienic by default. Use `gensym` for fresh names when needed.

---

## 25. Module System ✅

### 25.1 Namespace

```clojure
(ns my-app.core
  (:require [tisp.core :as core]
            [my-app.utils :refer [helper]]))
```

### 25.2 Import/Export

```clojure
(ns my-app.utils
  (:export [helper format-date]))
```

---

## 26. FFI & System-Level Programming ⚠️

### 26.1 External Function Declaration

```clojure
(defextern "malloc" [Int] -> (Ptr Unit) :effect [Unsafe])
(defextern "free" [(Ptr Unit) @1] -> Unit :effect [Unsafe])
(defextern "memcpy" [(Ptr Unit), (Ptr Unit), Int] -> Unit :effect [Unsafe])
```

### 26.2 Raw Pointers

```clojure
(defn ptr-read [{1 p : (Ptr a)}] ->[Unsafe] a
  "Read raw pointer, consuming it (linear)")

(defn ptr-write [{1 p : (Ptr a)}, {1 v : a}] ->[Unsafe] Unit
  "Write to raw pointer, consuming both")
```

### 26.3 Manual Region Management

```clojure
(defn with-region [f : (Region ->[ε] a)] ->[ε] a
  "Create region, run f, deallocate region on exit")

(defn region-alloc [r : Region @ω, {1 v : a}] -> (Ptr a)
  "Allocate in specified region")
```

### 26.4 Unsafe Effect Gating

All system-level operations require the `Unsafe` effect. Code using `Unsafe` must be explicitly annotated and cannot be called from pure code without a handler.

---

## 27. Process Calculi ✅

All process calculi are specializations of the Communication effect family.

### 27.1 SKI Combinators

Built-in. Also a compilation target for closure conversion.

```clojure
;; S f g x = (f x) (g x)
;; K x y = x
;; I x = x = (S K K) x
```

### 27.2 π-calculus (Channel Effect)

```clojure
(defeffect Channel a
  (new  []          -> (Chan a))
  (send [Chan a, a] -> Unit)
  (recv [Chan a]    -> a)
  (par  [Proc, Proc] -> Proc)
  (rep  [Proc]      -> Proc))
```

### 27.3 Async π-calculus

```clojure
(defeffect AsyncChannel a
  (send [Chan a, a] -> Unit)    ; non-blocking
  (recv [Chan a]    -> a))      ; blocking
```

### 27.4 Applied π-calculus

```clojure
(defeffect AppliedChannel a
  ;; inherits Channel operations, plus:
  (encrypt  [a, Key]    -> Cipher)
  (decrypt  [Cipher, Key] -> a)
  (sign     [a, PrivKey] -> Signature)
  (verify   [Signature, PubKey] -> Bool)
  (hash     [a]         -> Hash))
```

### 27.5 spi-calculus

```clojure
(defeffect SecureChannel a
  ;; inherits AppliedChannel, plus:
  (secret   [a]         -> Unit)
  (commit   [a, b]      -> Unit)
  (check    [a]         -> Bool))
```

### 27.6 Safe Ambients

```clojure
(defeffect Ambient a
  (amb      [Name, Proc]     -> Ambient)
  (enter    [Name]           -> Cap)
  (exit     [Name]           -> Cap)
  (open     [Name]           -> Cap)
  (co-enter [Name]           -> Cap)
  (co-exit  [Name]           -> Cap)
  (co-open  [Name]           -> Cap))
```

### 27.7 ρ-calculus (Reflective)

```clojure
(defeffect Reflect a
  (quote    [Proc]   -> Name)    ; ⌜P⌝ process → name
  (drop     [Name]   -> Proc)    ; ⌊x⌋ name → process
  (lift     [Chan, Proc] -> Unit) ; x⌈P⌉ send reference on channel
  (par      [Proc, Proc] -> Proc))
```

### 27.8 κ-calculus (Chemical)

```clojure
(defeffect Reaction
  (complex  [(List Site)] -> Complex)
  (site     [Name, State] -> Site)
  (bind     [Site, Site]  -> Unit)
  (unbind   [Site, Site]  -> Unit)
  (react    [Rule]        -> Proc))
```

### 27.9 ς-calculus

Encoded via OOP (generic functions + method combination).

### 27.10 Calculus Encodings

```clojure
;; Each calculus can encode into the previous:
(defn pi-to-ski [p : PiProc] -> SKITerm ...)
(defn async-to-sync [p : AsyncPiProc] -> PiProc ...)
(defn applied-to-pi [p : AppliedPiProc, theory : EqTheory] -> PiProc ...)
(defn rho-to-pi [p : RhoProc] -> PiProc ...)
(defn ambient-to-pi [p : AmbientProc] -> PiProc ...)
```

---

## 28. Verification ⚠️

**Verification and execution are the same code with different effect handlers.**

### 28.1 Property Specification

```clojure
(defprop (reachable [state : State]) : Bool)
(defprop (safe [bad-state : State]) : Bool
  (not (reachable bad-state)))
(defprop (secret [s : Term] [attacker-knowledge : (Set Term)]) : Bool
  (not (derivable s attacker-knowledge)))
```

### 28.2 Model Checking

```clojure
(verify ns-protocol
  :properties [(secret session-key attacker-knowledge)
               (auth alice bob session)]
  :model :dolev-yao
  :sessions :unbounded)
```

### 28.3 Equivalence Checking

```clojure
(check-equivalence protocol-v1 protocol-v2
  :equivalence :barbed-congruence)
```

### 28.4 Attack Reconstruction

```clojure
(find-attack protocol
  :target (secret key)
  :attacker :dolev-yao)
```

### 28.5 Verification = Effect Handler

```clojure
;; Normal execution: choose one path
(handle protocol (Channel a)
  (send [ch v] [k] (runtime-send ch v) (k Unit)))

;; Verification: explore all paths
(handle protocol (Channel a)
  (send [ch v] [k] (verify-send ch v) (k Unit))
  (recv [ch] [k]
    (let [all-possible (all-possible-receives ch)]
      (fold (fn [acc v] (or (k v) acc)) false all-possible))))
```

---

## 29. Built-in Functions ⚠️

### 29.1 Arithmetic

`+`, `-`, `*`, `/`, `mod`, `pow`, `abs`, `min`, `max`

### 29.2 Comparison

`=`, `<`, `>`, `<=`, `>=`, `not=`

### 29.3 Boolean

`and`, `or`, `not`

### 29.4 String Operations

`str`, `str-len`, `str-sub`, `str-concat`, `str-split`, `str-join`

### 29.5 Collection Operations

`map`, `filter`, `reduce`, `foldl`, `foldr`, `take`, `drop`, `reverse`, `sort`, `length`, `nth`, `first`, `rest`, `cons`, `append`, `concat`

### 29.6 IO Operations

`println`, `print`, `read-line`, `slurp`, `spit`

### 29.7 Type Operations

`type-of`, `effects-of`, `determinism-of`, `grade-of`, `mode-of`

---

## 30. Compiler Pragmas ⚠️

### 30.1 Inline Hint

```clojure
(inline! (small-fn x y))
```

### 30.2 Specialization Hint

```clojure
(specialize! map [Int, (List Int)])
```

### 30.3 Optimization Level

```clojure
(opt-level 3)    ; 0-3
```

### 30.4 Warning Control

```clojure
(suppress-warning :unused-variable)
```

---

## Appendix A: Complete Syntax BNF

```
program     = form*
form        = expr
expr        = literal | symbol | keyword | list | vector | map | set
            | quote | syntax-quote | unquote | unquote-splice
literal     = integer | float | string | char | boolean | nil
integer     = -? [0-9]+
float       = -? [0-9]+ '.' [0-9]+ ([eE] [+-]? [0-9]+)?
string      = '"' ([^"\\] | '\\' .)* '"'
char        = '\' (char-name | .)
char-name   = 'newline' | 'space' | 'tab'
boolean     = 'true' | 'false'
nil         = 'nil'
keyword     = ':' ident
symbol      = ident
ident       = start-char rest-char*
list        = '(' expr* ')'
vector      = '[' expr* ']'
map         = '{' (expr expr)* '}'
set         = '#' '{' expr* '}'
quote       = "'" expr
syntax-quote = '`' expr
unquote     = '~' expr
unquote-splice = '~@' expr
```

## Appendix B: Reserved Words

```
true false nil
def defn defn- def- defdata defdata-hit defpred defgeneric defmethod
defclass definstance defeffect defmacro defextern defglobal-type
defresource-algebra defprop defsession typefamily
fn let if cond match when unless
handle verify verify! check-equivalence find-attack
ns require use import refer
inline! specialize! opt-level suppress-warning
ann quote syntax-quote
fresh search solve-all find-all abduce constrain domain label
send recv close chan spawn
flat sharp shape crisp reflect-type gensym
```

## Appendix C: Operator Precedence

All operators are prefix (Lisp-style). No infix precedence needed.

## Appendix D: Effect System Formal Rules

```
[Pure]    Γ ⊢ e : τ        ─────────────────    Γ ⊢ e : τ [·]

[Perform] (op : τ₁ → τ₂) ∈ ε    Γ ⊢ x : τ₁
         ─────────────────────────────────
         Γ ⊢ (perform op x) : τ₂ [ε]

[Handle]  Γ ⊢ e : τ₁ [ε ∪ ε']    Γ, x:τ₁ ⊢ h : τ₂ [ε]
         ───────────────────────────────────────────
         Γ ⊢ (handle e handler) : τ₂ [ε]

[Sub]     Γ ⊢ e : τ [ε₁]    ε₁ ⊆ ε₂
         ─────────────────────────────
         Γ ⊢ e : τ [ε₂]
```

## Appendix E: Grade System Formal Rules

```
[Var]     x : [A]_r ∈ Γ    ─────────    Γ ⊢ x : A  (uses r of x)

[Promote] Γ ⊢ e : A    all x ∈ Γ have grade 0
         ─────────────────────────────
         Γ ⊢ [e] : □_r A

[Demote]  Γ ⊢ e : □_r A    x : [A]_r ∈ Γ'
         ─────────────────────────────────
         Γ ⊢ let [x] = e in body : B
```

## Appendix F: Example Program Index

See `examples/` directory:

| 文件 | 运行结果 | 说明 |
|------|---------|------|
| `hello.tisp` | `Hello, Tisp!` | 顶层表达式 |
| `fibonacci.tisp` | `55` | 递归 + 顶层 println |
| `adt-test.tisp` | `just/nothing/3` | ADT + match |
| `advanced-test.tisp` | `43/120` | 高阶函数 + 闭包 |
| `run-test.tisp` | `Hello from Tisp!` / `42` | 综合 |
| `type-infer-test.tisp` | `43` | 类型推断 |
| `state-effect.tisp` | `3` | 效果系统(state) |
| `logic-test.tisp` | `OK` | 逻辑编程(fresh/==/search) |
| `frp-counter.tisp` | 定义型(无入口) | FRP 流(:::/⃝/advance) |
| `logic-search.tisp` | 部分支持 | Search effect + Mercury 自由变量 |
| `liquid-types-test.tisp` | 验证通过(8 verified) | 液态类型(精化/契约,需 z3) |
| `liquid-types-violations.tisp` | 预期报错(退出码非零) | 液态类型负面用例 |
| `phase5-test.tisp` | 定义型(无入口) | 洞/确定性 |
| `_qtt-test.tisp` | 定义型(无入口) | QTT |
