## Why

8 类编程范式（数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流）目前以宿主库函数 + `pf-*` 简化投影存在：副作用未统一走代数效应/单子、源码表面不完整、部分输入静默返回默认值；AOP 仅有 `pf-aop-weave` 演示，`comptime` 只做语法包装，未实现编译期纯声明式 MOP 编织。本变更要求：在**纯声明式副作用管理（代数效应 + 单子）**下把这些范式做到源码可写、类型/效应/等级可查、端到端可运行、错误显式，并让 **comptime + MOP 驱动的 AOP 真实编织 OOP 方法**，全部全链路可用前不关闭。

## What Changes

- 8 类编程范式各自获得完整源码表面与效应语义：数组操作（创建/索引/切片/逐元素 map/reduce/沿轴求和）为纯函数；栈、状态机、数据驱动经 `State` 效应或 `mlet/get-m/put-m/pure` 单子线程管理；自动机与符号编程为 Pure；基于流编程接入 `Signal`/FRP 效应。每个操作都有 type_infer/effect_infer/grade_check 签名，非法输入显式报错。
- 删除 `pf-*` 简化投影作为这些范式的公开执行路径：`pf-*` 仅保留为设施级别名，语义必须与完整内置一致（或移除别名并更新调用方）。
- 效果系统层面：为 Stack/StateMachine/DataDriven 注册声明式效应操作；单处理器单子降级（§12.6）对这些范式真实走直接状态线程（不只是计数）；多处理器保持 handler 语义。
- `comptime` 真实语义：comptime 表达式在编译期求值，可读写 MOP 知识库（GetKB/SetKB 效应），结果内联为常量/构造后的 Core 表达式；编译期求值失败 SHALL 报告编译错误。
- AOP 经 comptime 纯声明式 MOP 编织 OOP：定义切面（pointcut + around/before/after advice），在 desugar/特化阶段对 `defgeneric`/`defmethod` 方法链完成编译期编织；`call-next-method` 继续指向内层链；编织结果与运行时分发语义等价，且经 `--desugar` 可见。
- 为每个范式新增端到端示例与回归测试；验收矩阵覆盖 8 范式 + AOP/MOP 全链路（`--typecheck` + `--run` + 非法输入拒绝）。

## Capabilities

### New Capabilities

（无新增能力目录；全部落在既有能力上）

### Modified Capabilities

- `programming-paradigms`: 8 类范式的行为契约升级为「纯声明式副作用管理 + 完整源码表面 + 错误显式 + 全链路可用」；替换/补全既有简化投影语义。
- `aspect-oriented-programming`: AOP 编织从运行时/演示级改为 comptime 纯声明式 MOP 驱动、编译期编织 OOP 方法链。
- `meta-object-protocol`: comptime 编译期求值语义；GetKB/SetKB 在编译期经效应可编程；元程序可修改编译期可见规则集并影响后续编译。
- `paradigm-integration`: 8 范式与 AOP 设施的效应/单子元数据必须真实作用于执行路径；组合必须经共享抽象并保持声明式。
- `toolchain-and-macros`: §12.6 Monad 优化路径从「检测计数」扩展到范式状态线程真实编译；`comptime` 进入工具链全链路（读取→脱糖→编译期 MOP 求值→检查→执行）。

## Impact

- 代码：`tisp-runtime`（programming/aop/facility/full_chain）、`tisp-backend/src/interpreter.rs`（范式内置 + comptime + AOP 编织执行）、`tisp-frontend/src/desugar.rs`（新源码形式/comptime 编织）、`tisp-middle`（type/effect/grade 签名、effect_compile 单子降级、specialize 与编织交互）、`tisp-cli`（comptime 报告）。
- 行为收紧：原 `pf-*` 静默默认值（如 `pf-dfa-accept` 的 sum%2、`pf-aop-weave` 的 +100）在完整语义上线后不再作为正确性来源。
- 测试：每范式至少一个 `.tisp` 示例 + 非法输入拒绝测试 + AOP/MOP 编织端到端测试；保持 `cargo test --workspace` 全绿、零警告。
- 文档：README/PLAN/standard_doc/CHANGELOG 与 OpenSpec 主规范同步。
