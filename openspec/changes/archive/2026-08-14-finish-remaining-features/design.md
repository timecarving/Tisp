## Context

`standard_doc/04-implementation-status.md` 的 ⚠️ 项分两类:①陈旧标记(实现已「齐」但未升 ✅);②真实深度缺口。本变更逐项补齐。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 修正陈旧标记(§3/§5/§8/§20 → ✅;§32 note 更新;清理陈旧运行时局限)。
- 补齐真实深度缺口(fixpoint、□_r/◇_ε 推导、N 维立方、Cohesive unit、时序因果、Z3 等级、数据流逃逸、inkwell 闭包)。

**Non-Goals:**
- 不改动既有 ✅ 特性语义。
- 不追求生产级性能,以正确性 + 端到端可用为先。

## Decisions

### D1: 修正陈旧标记 = 审计优先

先审计各 ⚠️ 章的实际代码,凡描述「齐」且实现存在即升 ✅(附 file:line 证据);仅真实缺口进入实现。**理由**:避免为「已实现但误标 ⚠️」的项浪费实现工作。

### D2: fixpoint 迭代收敛(solve.rs)

`solve.rs` 从「串行运行六 pass」升级为「fixpoint 循环:迭代运行各 pass,把新约束写入共享约束图,直到无新冲突」。**理由**:兑现 §2「统一约束求解」的 fixpoint 语义。

### D3: □_r/◇_ε 引入消去推导(type_infer)

`(□_r A)` 类型消去时,若 r 可推导则推导、否则默认 ω 并警告;`(◇_ε A)` 同理推导 ε。**理由**:补 §11 的「引入/消去推理」缺口。

### D4: N 维立方 + Cohesive unit(interpreter/hott)

HComp 扩展 ≥2 维 Kan 填充(复用 hott.rs `kan_fill_2d`,泛化 N 维);♯∘♭ 与 ♭∘ʃ 的 unit 语义(嵌入)。**理由**:补 §16/§17 的立方/同伦深度缺口。

### D5: 时序因果性 + Z3 等级

时序因果性检查(输出仅依赖当前/过去);符号等级不等式交 Z3(有 z3 时)求解。**理由**:补 §18/§19 语义缺口。

### D6: 数据流逃逸 + inkwell 闭包

region_infer 从「返回值逃逸」升级为「数据流逃逸(跟踪分配地址流向)」;codegen 补 inkwell 闭包环境打包。**理由**:补 §26/§30 缺口。

## Risks / Trade-offs

- [fixpoint 迭代可能不收敛] → 设最大迭代上限,超限报告并停止(不无限循环)。
- [Z3 等级依赖 z3 feature] → 无 z3 时降级常量折叠,与既有液态类型一致。
- [数据流逃逸分析复杂] → 先做「分配地址是否被 return/捕获」的轻量数据流,不做完整别名分析。
- [inkwell 闭包环境 llvm feature 门控] → 默认构建下用文本 IR 闭包标注(已做),inkwell 层 feature 门控。

## Migration Plan

逐项补齐,每项独立提交;全程 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- N 维立方填充是否需通用 N 维表示(初版 2 维 + 递归泛化即可)。
- Cohesive unit 是否引入 ∞-groupoid 表示(初版以 adjoint 组合语义为准)。
