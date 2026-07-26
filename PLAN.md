# Tisp Compiler - Plan Specification

> **Last Updated**: Phase 3 completed
> **Status**: 4/13 phases complete
> **Next Phase**: Phase 4 - Liquid Types + Refinement Types + Z3 Integration

---

## 1. Project Overview

**Tisp** is a pure declarative, unified-method, multi-paradigm, high-performance, system-level Lisp-1 dialect. It compiles to LLVM IR via Rust (inkwell).

### Core Design Principles

1. **Everything is an Annotated Relation** — all definitions are `def` + 6-dimensional annotations (type, effect, region, grade, mode, determinism)
2. **Effects are Universal Glue** — State, errors, search, IO, unsafe ops, signals are all effects; Monad is the optimization path
3. **Annotations Constrain Each Other** — unified constraint solver via fixpoint iteration
4. **Reader = First-Class Citizens** — types, effects, grades, modes are all runtime-manipulable values
5. **Declarative Throughout** — no imperative escape hatch; system-level = Unsafe effect + manual regions
6. **Calculi are Communication Patterns** — π, ρ, ambient, κ, spi, SKI are all Communication effect specializations

### Key Design Decisions Already Made

| Decision | Choice |
|----------|--------|
| Implementation language | Rust |
| Compilation target | LLVM IR (via inkwell) |
| Type system base | HM + Effect Rows + Graded Types + QTT |
| Syntax style | Clojure data syntax + Scheme minimal core |
| Effect system | Algebraic effects + Koka-style evidence passing |
| Monad role | Optimization path for effects (not separate concept) |
| OOP | CLOS-style generic functions + compile-time specialization |
| Logic programming | Mercury-style (mode/determinism/typeclass) |
| FRP | Elm-style Signal graph model (as Signal effect) |
| Immutable data | `im` crate (persistent data structures) |
| Memory management | No GC; region inference + graded types |
| System-level | FFI + raw pointers + manual regions via Unsafe effect |
| Type naming | Rust-style: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool` |

---

## 2. Project Structure

```
/tmp/tisp/
├── Cargo.toml                    # Workspace root
├── PLAN.md                       # THIS FILE - continuation plan
├── PHASE2_SUMMARY.md             # Phase 2 completion notes
├── PHASE3_SUMMARY.md             # Phase 3 completion notes
├── docs/
│   └── spec.md                   # Complete language specification (30 chapters + 6 appendices)
├── crates/
│   ├── tisp-core/                # Core type definitions
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── span.rs           # Source location tracking
│   │       ├── symbol.rs         # Interned symbols (Arc<str>)
│   │       ├── ast.rs            # S-expression AST (surface)
│   │       ├── core_ast.rs       # Desugared Core AST
│   │       ├── types.rs          # Full type system definitions
│   │       ├── effects.rs        # Effect rows + handler types
│   │       ├── grades.rs         # Grade semiring operations
│   │       ├── modes.rs          # Mercury-style mode system
│   │       ├── determinism.rs    # Determinism categories
│   │       ├── regions.rs        # Region types + storage modes
│   │       └── data.rs           # ADT declarations + DataEnv
│   │
│   ├── tisp-frontend/            # Frontend
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lexer.rs          # logos-based lexer
│   │       ├── parser.rs         # Recursive descent parser
│   │       ├── reader.rs         # S-expression reader
│   │       └── desugar.rs        # S-expr → Core AST desugaring
│   │
│   ├── tisp-middle/              # Middle-end (type checking, analysis)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── type_infer.rs     # HM type inference (Algorithm W)
│   │       ├── effect_infer.rs   # Effect row inference
│   │       ├── grade_check.rs    # QTT multiplicity checking
│   │       ├── mode_analysis.rs  # STUB - Phase 5
│   │       ├── determinism_analysis.rs  # STUB - Phase 5
│   │       ├── region_infer.rs   # STUB - Phase 6
│   │       ├── effect_compile.rs # STUB - Phase 5
│   │       └── optimize/         # STUB - Phase 7
│   │           ├── mod.rs
│   │           ├── inline.rs
│   │           ├── strictness.rs
│   │           ├── deforest.rs
│   │           ├── effect_elim.rs
│   │           └── region_opt.rs
│   │
│   ├── tisp-backend/             # Backend (STUB - Phase 8)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── codegen.rs        # STUB
│   │       ├── closure.rs        # STUB
│   │       └── runtime_ffi.rs    # STUB
│   │
│   ├── tisp-runtime/             # Runtime (STUB - Phase 8)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── region.rs         # STUB
│   │       ├── persistent.rs     # STUB
│   │       ├── effect.rs         # STUB
│   │       └── builtin.rs        # STUB
│   │
│   └── tisp-cli/                 # CLI + REPL
│       └── src/
│           └── main.rs           # Working: --typecheck, --desugar, --print-ast, REPL
│
├── stdlib/                       # Standard library (empty - Phase 9)
├── tests/                        # Test infrastructure (empty)
│   ├── unit/
│   ├── integration/
│   └── snapshots/
└── examples/
    ├── hello.tisp
    ├── fibonacci.tisp
    ├── state-effect.tisp
    ├── logic-search.tisp
    ├── frp-counter.tisp
    ├── type-infer-test.tisp      # Phase 2 test
    └── adt-test.tisp             # Phase 3 test
```

---

## 3. Completed Phases (0-3)

### Phase 0: Infrastructure ✅
- Cargo workspace with 6 crates
- Complete `docs/spec.md` (30 chapters, 6 appendices)
- Lexer (logos), Parser (recursive descent), Reader
- REPL with rustyline
- CLI with `--typecheck`, `--desugar`, `--print-ast`, `--print-tokens` flags

### Phase 1: Frontend ✅
- S-expression AST with all Clojure-style data structures
- Desugaring: `def`, `defn`, `fn`, `let`, `if`, `match`, `handle`, `perform`
- Left-associative function application
- Vector/Map/Set literal desugaring to constructors

### Phase 2: Type System Core ✅
- **Algorithm W** (HM type inference) with:
  - Type variables, unification, occurs check
  - Let-polymorphism (generalization at let-bindings)
  - Recursive definition support (fresh var → unify → generalize)
- **Effect row inference** (union of subexpression effects)
- **QTT multiplicity checking** (0/1/ω usage tracking)
- Built-in types: `+`, `-`, `*`, `/`, `=`, `<`, `<=`, `println`
- Rust-style primitive types: `i8`..`i64`, `u8`..`u64`, `f32`, `f64`, `bool`

### Phase 3: ADT + Pattern Matching ✅
- **`defdata` syntax** with type parameters: `(defdata (Maybe a) (Nothing) (Just a))`
- **Constructor type generation** with proper polymorphic schemes
- **Pattern matching** with constructor patterns, wildcards, variables
- **Constructor application** type checking with instantiation
- **Smart field parsing** (distinguishes type params from field names)
- Key fix: type variable instantiation uses **names** not IDs to handle multiple type params

---

## 4. Remaining Phases (4-12)

### Phase 4: Liquid Types + Refinement Types + Z3
**Priority**: HIGH | **Estimated effort**: 2-3 weeks

#### What to implement:
1. **Refinement type syntax**: `{x : T | predicate}` in the type system
2. **Z3 SMT solver integration**: Use the `z3` crate for constraint solving
3. **Liquid type inference**: Automatically infer refinement predicates
4. **Contract system**: `:requires` and `:ensures` annotations on functions
5. **Integration with QTT**: Refinement predicates can use 0-multiplicity variables

#### Key files to modify:
- `crates/tisp-core/src/types.rs` — `Refined` variant already exists
- `crates/tisp-middle/src/` — new `liquid_types.rs` module
- `Cargo.toml` — add `z3` dependency

#### Test cases to create:
```clojure
;; Refinement types
(defn sqrt [x : {n : f64 | (>= n 0.0)}] -> f64 ...)

;; Liquid type inference
(defn abs [x : i64] -> {n : i64 | (>= n 0)}
  (if (>= x 0) x (- x)))

;; Contracts
(defn divide [n : i64, d : {x : i64 | (!= x 0)}] -> i64
  :requires true
  :ensures (= result (quot n d))
  (quot n d))
```

#### References:
- Liquid Types (Rondon et al., PLDI 2008)
- Z3 Rust bindings: `z3` crate on crates.io

---

### Phase 5: Effect Compilation + Mode/Determinism Analysis
**Priority**: HIGH | **Estimated effort**: 3-4 weeks

#### What to implement:
1. **Evidence passing translation** (Koka-style):
   - Transform effect handlers into evidence vectors
   - Tail-resumptive operation optimization
   - Monadic degradation for single-handler cases
2. **Mode analysis** (Mercury-style):
   - Track instantiation states (free → ground)
   - Producer/consumer analysis
   - Conjunct reordering for correct execution order
3. **Determinism analysis**:
   - Infer det/semidet/multi/nondet from code structure
   - Conjunction/disjunction/negation rules (already in `determinism.rs`)
   - Committed choice handling

#### Key files:
- `crates/tisp-middle/src/effect_compile.rs` — evidence passing
- `crates/tisp-middle/src/mode_analysis.rs` — mode inference
- `crates/tisp-middle/src/determinism_analysis.rs` — determinism inference

#### References:
- Generalized Evidence Passing (Xie & Leijen, ICFP 2021)
- Mercury mode system documentation
- Constraint-Based Mode Analysis of Mercury (PPDP 2002)

---

### Phase 6: Region Inference
**Priority**: MEDIUM | **Estimated effort**: 3-4 weeks

#### What to implement:
1. **Tofte-Talpin region inference**:
   - Region variable introduction and unification
   - `letregion` insertion
   - Finite vs infinite region classification
2. **Region representation inference**:
   - Multiplicity analysis (0, 1, ∞)
   - Storage mode analysis (AtTop, AtBot, Sat)
   - Scalar region elimination (word-size values)
3. **Region optimization**:
   - Region merging (same lifetime)
   - Region promotion (lift to outer region)
   - Tail recursion region reuse

#### Key files:
- `crates/tisp-middle/src/region_infer.rs`
- `crates/tisp-middle/src/optimize/region_opt.rs`
- `crates/tisp-core/src/regions.rs` — types already defined

#### References:
- Tofte-Talpin (TOPLAS 1998)
- ML Kit with Regions documentation

---

### Phase 7: Optimization Pipeline
**Priority**: MEDIUM | **Estimated effort**: 3-4 weeks

#### What to implement:
1. **Inlining**: small functions, single-use functions, tail-resumptive handlers
2. **Specialization**: generic function monomorphization, effect handler specialization
3. **Strictness analysis**: demand analysis, unused parameter elimination, CBV transform
4. **Deforestation**: stream fusion, build/augment fusion
5. **Effect elimination**: pure computation extraction, effect reordering, empty handler elimination

#### Key files:
- `crates/tisp-middle/src/optimize/` — all files are stubs

---

### Phase 8: LLVM Backend + Runtime
**Priority**: HIGH | **Estimated effort**: 4-6 weeks

#### What to implement:
1. **inkwell LLVM IR generation**:
   - Type mapping (Tisp types → LLVM types)
   - Function compilation
   - Control flow (if/match → LLVM branches)
   - Tail call optimization (`musttail`)
2. **Closure conversion + lambda lifting**:
   - Free variable capture
   - Environment allocation in regions
3. **Runtime library** (`tisp-runtime`):
   - Region allocator (finite: stack, infinite: page-linked list)
   - Effect runtime (evidence vectors, yield/resume)
   - Persistent data structure wrappers (`im` crate)
   - Built-in functions (arithmetic, string ops, IO)
4. **FFI bridge**:
   - C ABI compatibility
   - `defextern` compilation
   - Dynamic library loading

#### Key files:
- `crates/tisp-backend/src/codegen.rs`
- `crates/tisp-backend/src/closure.rs`
- `crates/tisp-runtime/src/`

#### Dependencies to add:
- `inkwell = { version = "0.9", features = ["llvm18-0"] }` (or latest LLVM)
- `libloading` for dynamic FFI

#### References:
- inkwell documentation: docs.rs/inkwell
- LLVM Kaleidoscope tutorial (Rust version)
- Create Your Own Programming Language with Rust (createlang.rs)

---

### Phase 9: Toolchain
**Priority**: MEDIUM | **Estimated effort**: 3-4 weeks

#### What to implement:
1. **Module system**: `ns`, `require`, `use`, `import`, `refer`
2. **Standard library**: core, collections, effects, io, logic, unsafe
3. **Macro system**: `defmacro`, syntax-quote, hygiene
4. **Error diagnostics**: ariadne-based error reporting with source spans
5. **Build system**: incremental compilation, caching

---

### Phase 10: HoTT + Cohesive HoTT + Session Types
**Priority**: LOW | **Estimated effort**: 4-6 weeks

#### What to implement:
1. **Interval type** `I` with `i0`, `i1`, `~i`, `i ∧ j`, `i ∨ j`
2. **Path types** `Path A x y`
3. **Homogeneous composition** `hcomp` and **transport** `transp`
4. **Glue types** and **univalence** `ua`
5. **Higher inductive types** (HIT) with path constructors
6. **Cohesive modalities**: `♭` (flat), `♯` (sharp), `ʃ` (shape)
7. **Session types**: binary, multiparty (MPST), dependent

#### Compiler flag: `--cubical`, `--cohesion`

#### References:
- Cubical Agda documentation
- CCHM (Cohen et al., 2017)
- Cohesive HoTT (Shulman, 2018)

---

### Phase 11 ████████████ ✅ 进程演算 + 验证
**Priority**: LOW | **Estimated effort**: 6-8 weeks

#### What to implement:
1. **Channel effect** (π-calculus): `new`, `send`, `recv`, `par`, `rep`
2. **AsyncChannel effect** (async π): non-blocking send
3. **AppliedChannel effect** (applied π): crypto primitives
4. **SecureChannel effect** (spi-calculus): secrecy, commitment
5. **Ambient effect** (safe ambients): `enter`, `exit`, `open` + co-capabilities
6. **Reflect effect** (ρ-calculus): `quote`, `drop`, `lift`
7. **Reaction effect** (κ-calculus): `bind`, `unbind`, `react`
8. **Verification engine**: model checking, equivalence checking, attack reconstruction
9. **Property specification language**: `defprop`, `verify`, `check-equivalence`, `find-attack`

#### Key insight: Verification = an effect handler that explores all paths

---

### Phase 12: Temporal Types (FRP)
**Priority**: LOW (future) | **Estimated effort**: 4-6 weeks

#### What to implement:
1. **Temporal modalities**: `⃝` (next), `□_t` (always), `◇_t` (eventually)
2. **Fitch-style ✓ token** for time step tracking
3. **Stable typeclass** for time-invariant types
4. **Guarded recursion** for productive stream definitions
5. **LTL as types**: temporal properties as refinement types
6. **Multi-clock** support via `Clock` typeclass
7. **Resampling** between different clock rates

#### References:
- Rattus (Bahr et al., POPL 2022)
- Rhine (Bärenz, Perez)

---

## 5. Type System Architecture (Current State)

```
Layer 5: Cohesive HoTT (ʃ ⊣ ♭ ⊣ ♯)          — Phase 10
Layer 4: Temporal Types (⃝, □_t, ◇_t)        — Phase 12
Layer 3: Session Types                         — Phase 10
Layer 2: Liquid Types ({x : T | p(x)})        — Phase 4
Layer 1: Graded Modal Types (□_r, ◇_ε)        — Phase 2 (basic)
Layer 0: QTT (multiplicity 0, 1, ω)           — Phase 2 ✅
```

Plus base type system:
- HM + rank-n polymorphism ✅
- Higher-kinded types (defined in types.rs, not yet used in inference)
- GADT + existential types (defined, not yet used)
- Row polymorphism (defined, not yet used)
- Type families + associated types (defined, not yet used)
- Subtyping (defined, not yet used)
- ADT + pattern matching ✅

---

## 6. Key Type Definitions (in `types.rs`)

The type system is already fully defined in `crates/tisp-core/src/types.rs`. All variants exist:

```rust
pub enum Type {
    Var(TypeVar),           // Type variable
    Con(TypeCon),           // Type constructor (i64, bool, List, ...)
    App(Box<Type>, Box<Type>),  // Type application (List i64)
    Fun(Box<Type>, FunAnnotation, Box<Type>),  // Function with 6-dim annotation
    Forall(Vec<TypeVar>, Box<Type>),  // Universal quantification
    Tuple(Vec<Type>),       // Product type
    Record(Vec<(Symbol, Type)>, Option<Box<Type>>),  // Extensible record
    Refined(Box<Type>, Box<Predicate>),  // Liquid/refinement type
    Path(Box<Type>, Box<Term>, Box<Term>),  // HoTT path type
    Interval,               // HoTT interval
    Session(Box<SessionType>),  // Session type
    Modal(ModalOp, Box<Type>),  // Graded modal (□_r, ◇_ε)
    Temporal(TemporalOp, Box<Type>),  // Temporal (⃝, □_t, ◇_t)
    Cohesive(CohesiveOp, Box<Type>),  // Cohesive (♭, ♯, ʃ)
    Meta(Box<MetaType>),    // Meta-types (Type, Effect, Grade, ...)
}
```

`FunAnnotation` carries all 6 dimensions:
```rust
pub struct FunAnnotation {
    pub effects: EffectRow,      // Effect row
    pub region: Option<RegionVar>,  // Region
    pub grade: Grade,            // Usage grade
    pub mode: Mode,              // Mercury mode
    pub determinism: Determinism, // Determinism category
}
```

---

## 7. Build & Test Commands

```bash
# Build
cd /tmp/tisp && cargo build

# Run type checker on a file
./target/debug/tisp --typecheck examples/adt-test.tisp

# Desugar and print Core AST
./target/debug/tisp --desugar examples/adt-test.tisp

# Print tokens
./target/debug/tisp --print-tokens examples/hello.tisp

# Print AST
./target/debug/tisp --print-ast examples/hello.tisp

# Start REPL
./target/debug/tisp

# Run tests (when implemented)
cargo test
```

---

## 8. Known Issues & Technical Debt

1. **`desugar_type` method is unused** — `desugar_type_with_params` replaced it but the old method remains. Remove it.
2. **No exhaustiveness checking** for pattern matching — Phase 4+ should add this
3. **Effect handlers are stubs** — `desugar_handle` creates a dummy handler; needs full parsing
4. **`perform` syntax not fully specified** — needs integration with effect declarations
5. **No error recovery** — parser fails on first error; should collect multiple errors
6. **Type printing is raw Debug format** — should pretty-print types for user-facing output
7. **No source file tracking** — spans are byte offsets but no filename association
8. **`substitute_vars` method is dead code** — replaced by `substitute_vars_by_name` in Phase 3

---

## 9. Dependencies

### Current (in Cargo.toml)
```toml
logos = "0.14"          # Lexer
im = "15"               # Persistent data structures
ariadne = "0.4"         # Error reporting
miette = "7"            # Error diagnostics
thiserror = "2"         # Error derive
serde = "1"             # Serialization
clap = "4"              # CLI
rustyline = "14"        # REPL
petgraph = "0.6"        # Graph algorithms
unicode-xid = "0.2"     # Unicode identifiers
```

### Planned
```toml
z3 = "..."              # SMT solver (Phase 4)
inkwell = "0.9"         # LLVM bindings (Phase 8)
libloading = "..."      # Dynamic FFI (Phase 8)
```

---

## 10. How to Continue

### Immediate next step: Phase 4

1. Add `z3` crate to `Cargo.toml`
2. Create `crates/tisp-middle/src/liquid_types.rs`
3. Implement refinement type checking:
   - Parse `{x : T | pred}` syntax in `desugar.rs`
   - Generate Z3 constraints from predicates
   - Verify predicates at function boundaries
4. Add `:requires` / `:ensures` to `defn` syntax
5. Create test file `examples/liquid-types-test.tisp`
6. Update `docs/spec.md` Chapter 15 with implementation details

### For any phase:
1. Read the corresponding chapter in `docs/spec.md`
2. Check the phase description in Section 4 above
3. Look at existing code in the relevant crate
4. Implement, test, update spec.md
5. Write a `PHASE{N}_SUMMARY.md`

---

## 11. Design Philosophy Reference

The full design philosophy is documented in `docs/spec.md` Section 2. Key points:

- **One declaration, six dimensions**: `def name [params] ->[ε, ρ, @r, m, d] ret body`
- **Effects are declaration interface, Monad is implementation path**
- **QTT is the foundation**: every binding has multiplicity (0, 1, ω)
- **Graded modal types generalize QTT** to arbitrary semirings
- **Liquid types refine values** with Z3-verifiable predicates
- **HoTT provides computational equality** (optional, `--cubical`)
- **Cohesive HoTT adds spatial structure** (optional, `--cohesion`)
- **Temporal types ensure causality/productivity** for FRP
- **Process calculi = Communication effect family**
- **Verification = effect handler exploring all paths**
