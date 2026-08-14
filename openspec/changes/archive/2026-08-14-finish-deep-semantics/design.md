## Context

5 项深语义缺口已在 `standard_doc/04-implementation-status.md` 标注。本变更逐项补齐。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- N(≥3)维立方、adjoint-triple 自然性、空间回收、完整别名分析、inkwell 闭包堆分配 5 项实现到全链路可用。

**Non-Goals:**
- 不改动既有 ✅ 特性语义;不追求生产级性能。

## Decisions

### D1: N 维立方 = 递归面组合

`hcomp-2d` 泛化为 `hcomp-nd`:N 维立方由 2N 个 (N-1) 维面组合,递归到 1 维(端点);每维检查共享面的一致性。**理由**:递归泛化复用既有 1 维/2 维 Kan 语义。

### D2: 自然性 = 自然变换方块

adjoint-triple 的自然性条件:对任意态射 f,unit/counit 的自然变换方块交换。以「组合恒等式」形式实现(如 ♭(f)∘η = η'∘f),检查一致性。

### D3: 空间回收 = next 值生命周期

`⃝`(next)值在两个时刻(推进两次)后回收:运行时以「时刻计数器 + 回收队列」实现,`advance` 两次后值被回收。

### D4: 别名分析 = 地址流图

region_infer 构建「地址流图」(RegionAlloc → 绑定/分支/闭包捕获 → 使用),逃逸判定 = 地址可达「逃逸点」(返回/全局/跨区域)。**理由**:完整别名分析需流图,比「返回值/let 数据流」更全面。

### D5: inkwell 闭包堆分配

inkwell 层(仅 llvm feature)补闭包环境堆分配 display 层:闭包捕获环境打包为堆结构 + 函数指针;默认构建回退文本 IR 闭包标注。

## Risks / Trade-offs

- [N 维立方递归实现复杂] → 先 3 维(泛化 2 维),再推广 N;测试覆盖 2/3 维。
- [别名分析可能过度报错] → 保守策略:仅报「明确可达逃逸点」,不确定则放行。
- [空间回收可能破坏既有流语义] → 回收仅作用于「已 advance 两次」的值,不影响活跃值。
- [inkwell 闭包 llvm feature 门控] → 默认构建文本 IR,llvm 构建单独验证。

## Migration Plan

逐项实现,每项独立提交;全程 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- N 维立方是否需通用 N 维数组表示(初版递归泛化即可)。
- 自然性是否引入显式 2-范畴(初版以组合恒等式表示)。
