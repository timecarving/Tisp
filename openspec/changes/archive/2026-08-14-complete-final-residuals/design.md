## Context

3 项残余深语义已在 `standard_doc/04-implementation-status.md` 标注。本变更把它们实现到全链路可用。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 态射级自然性(一阶态射 + 自然变换方块)、inkwell 闭包堆分配、跨区域/全局别名分析 3 项实现到全链路可用。

**Non-Goals:**
- 不改动既有 ✅ 特性语义。

## Decisions

### D1: 一阶态射 = 函数值 + 自然性方块

新增 `Morphism<A,B>`(函数 `A→B` 作为值);自然性 = 对任意态射 f 检查 unit/counit 方块交换:
`♭(f) ∘ η_A = η_B ∘ f`、`ε_B ∘ ♭(♯(f)) = f ∘ ε_A`。
**理由**:一阶态射表示是自然性的前提;方块交换是可判定的恒等式检查。

### D2: inkwell 闭包堆分配

inkwell 层(仅 llvm feature)把闭包捕获环境打包为堆结构 + 函数指针;`llc` 验证;默认构建回退文本 IR 闭包标注(已做)。
**理由**:闭包环境打包是「闭包真代码生成」的完整形态。

### D3: 跨区域/全局别名分析

region_infer 构建「地址流图」(RegionAlloc → 绑定/分支/闭包捕获/跨区域赋值/全局 → 逃逸点);逃逸判定 = 地址可达「逃逸点」。
**理由**:完整别名分析需流图覆盖跨区域/全局。

## Risks / Trade-offs

- [自然性需态射表示,改动面大] → 态射为轻量 wrapper,自然性为独立 check,不影响既有值语义。
- [inkwell 闭包 llvm feature 门控] → 默认构建文本 IR,llvm 构建单独验证 llc。
- [跨区域/全局别名可能过度报错] → 保守:仅报「明确可达逃逸点」。

## Migration Plan

逐项实现,每项独立提交;全程 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- 自然性是否需要显式 2-范畴(初版以方块交换恒等式表示)。
- 跨区域别名是否需要区域标注语法(初版以地址流图 + 逃逸点判定)。
