## Context

`paradigms.rs` 有 12 范式的真实求解器;`facility.rs` 是简化投影。本变更把 12 范式接线到真实求解器。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 12 范式全链路:源码 → 类型/效应/等级 → 解释器求值(经真实求解器)。

**Non-Goals:**
- 不改动既有 ✅ 范式(表格化/描述逻辑已接专用内置);仅补其余范式的接线。

## Decisions

### D1: 专用内置 + ParadigmRegistry 分发

每个范式一个专用内置(如 `plp-marginal`、`temporal-eventually`、`reactive-eval`),经 `ParadigmRegistry::eval` 分发到真实求解器。**理由**:统一入口,复用可接入接口。

### D2: 扁平编码 + 真实求解器

各范式用扁平 int/float 列表编码输入,构造真实求解器结构并求值:
- prob → `marginal(query, facts)`;eventually → `TemporalKb::eventually`
- reactive → `Signal` + 派生;settle → `DefRule::settle`;fuzzy → `fuzzy_and`
- induce → `induce`;context → `ContextKb::query`;possible → `ModalKb::possible`
- typed-pred → 静态类型谓词;higher-order → `call`

### D3: 类型/效应接入

各内置补单态签名(type_infer);副作用接入效应行(effect_infer 已注册 State/Search/Signal)。

## Risks / Trade-offs

- [扁平编码对结构化输入(如 ProbFact 的 (atom,prob) 对)不优雅] → 用 `[atom prob atom prob ...]` 扁平对编码。
- [12 范式一次性接线工作量大] → 逐个范式接线,每个独立可测可合入。

## Migration Plan

逐范式接线,每步 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- 各范式是否需要 LLVM 降级(初版 `--run` 端到端即可)。
