## Why

`standard_doc/04-implementation-status.md` 仍标注 12 章 ⚠️(§2/§3/§5/§8/§11/§16/§17/§18/§19/§20/§26/§30/§32)与若干语义深度缺口。其中部分为**陈旧标记**(§3/§5/§8/§20 描述已「齐」却仍 ⚠️;§32 的「剩余范式」已由 `evolp-stable`/`dlp-stable`/`get-kb` 接线),其余为**真实深度缺口**(维度间 fixpoint 收敛、□_r/◇_ε 引入消去推导、N 维立方、Cohesive unit/自然性、时序因果性/空间回收、符号等级 Z3 验证、数据流逃逸分析、inkwell 闭包环境打包)。本变更把全部剩余特性实现到全链路可用,并把陈旧标记修正到位。

## What Changes

- **修正陈旧状态标记**:§3/§5/§8/§20 从 ⚠️ → ✅(描述已「齐」);§32 的「剩余范式」note 更新(evolp/dlp/get-kb 已接线);清理 §3 已知运行时局限(TCO + 多顶层表达式递归已修)。
- **维度间 fixpoint 迭代收敛**(§2):solve.rs 从「串行聚合」升级为「fixpoint 迭代至收敛」。
- **□_r/◇_ε 引入/消去等级推导**(§11):type_infer 在可推断时推导 r/ε。
- **N 维立方组合接线**(§16):interpreter HComp 扩展 N 维 Kan(≥2 维)。
- **Cohesive unit/自然性**(§17):♭/♯/ʃ adjoint-triple 的 unit 语义(♯∘♭、♭∘ʃ)。
- **时序因果性/空间回收**(§18):因果性检查 + 空间泄漏回收语义。
- **符号等级 Z3 验证**(§10/§19):符号等级不等式交 Z3 求解。
- **完整数据流逃逸分析**(§26):region_infer 从「返回值逃逸」升级为「完整数据流逃逸」。
- **inkwell 闭包环境打包**(§30):闭包捕获环境堆分配 display 层。

## Capabilities

(无新增/修改能力——本变更为既有需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。各特性的 spec 已存在于 `docs/spec.md` 与主规范中。)

## Impact

- **tisp-middle**:solve.rs(fixpoint)、type_infer(□_r/◇_ε 推导、符号等级)、region_infer(数据流逃逸)、grade_check(Z3 等级)。
- **tisp-backend**:interpreter(HComp N 维、Cohesive unit、时序因果)、codegen(inkwell 闭包环境)。
- **tisp-runtime**:hott.rs(立方/cohesive)、temporal.rs(空间回收)。
- **standard_doc**:⚠️→✅ 升级 + file:line 证据更新。
