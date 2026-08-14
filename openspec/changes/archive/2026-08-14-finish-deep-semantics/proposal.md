## Why

前一轮已把大部分 ⚠️ 特性补齐,但仍有 5 项深语义缺口(均标注在 `standard_doc/04-implementation-status.md`):§16 N(≥3)维立方组合、§17 adjoint-triple 自然性、§18 空间回收、§26 完整别名分析、§30 inkwell 闭包堆分配。这些是深 HoTT/数据流/feature 门控特性,本变更逐项实现到全链路可用。

## What Changes

- **§16 N(≥3)维立方组合**:在 `hcomp-2d`(2 维)基础上泛化 N 维 Kan 填充(递归面组合 + 边界一致性)。
- **§17 adjoint-triple 自然性**:♭/♯/ʃ 的自然变换方块(unit/counit 的自然性条件)。
- **§18 空间回收**:`⃝`(next)值在两个时刻后回收(无空间泄漏)。
- **§26 完整别名分析**:region_infer 从「返回值/let 数据流逃逸」升级为「完整别名分析」(跨赋值/分支/闭包捕获的地址别名)。
- **§30 inkwell 闭包堆分配**:codegen 的 inkwell 层补闭包环境堆分配 display 层(llvm feature 门控)。

## Capabilities

(无新增/修改能力——本变更为既有需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。)

## Impact

- **tisp-backend**:interpreter(HComp N 维)、codegen(inkwell 闭包堆分配)、temporal(空间回收)。
- **tisp-middle**:region_infer(别名分析)。
- **tisp-runtime**:hott.rs(自然性、N 维立方)。
- **standard_doc**:⚠️→✅ 升级 + file:line 证据。
