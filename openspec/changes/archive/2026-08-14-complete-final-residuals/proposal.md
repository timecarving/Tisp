## Why

前几轮已把大部分特性补齐,但 3 项深语义仍是「残余」:§17 态射级自然性(需一阶态射表示)、§30 inkwell 闭包堆分配(llvm feature 门控)、§26 跨区域/全局别名分析。本变更**不把它们当作「残余」搁置**,而是一直实现到全链路可用。

## What Changes

- **§17 态射级自然性**:新增一阶态射表示(函数 `A → B` 作为值),实现 adjoint-triple 的自然变换方块(对任意 f:A→B,unit/counit 自然性交换)。
- **§30 inkwell 闭包堆分配**:codegen inkwell 层实现闭包环境堆分配 display 层(捕获自由变量打包为堆结构 + 函数指针),llvm feature 门控,llc 验证。
- **§26 跨区域/全局别名分析**:region_infer 从「闭包捕获/实参流入」升级为「跨区域/全局别名分析」(地址流图 + 逃逸点全覆盖)。

## Capabilities

(无新增/修改能力——本变更为既有需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。)

## Impact

- **tisp-runtime**:hott.rs(一阶态射 + 自然变换方块)。
- **tisp-backend**:codegen.rs(inkwell 闭包堆分配)、interpreter(态射自然性)。
- **tisp-middle**:region_infer.rs(跨区域/全局别名)。
- **standard_doc**:⚠️→✅ 升级。
