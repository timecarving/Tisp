## Purpose

补全宏、工具链与系统级能力(§12.6/§22-26/§29):宏卫生与 gensym、泛型编译期特化、真实动态库 FFI、反射函数真实化与 Monad 优化路径接线,使「宏/FFI/优化」工具链从骨架变为可用。

## ADDED Requirements

### Requirement: 宏卫生与 gensym

`defmacro` 展开 SHALL 避免捕获用户变量(卫生展开):展开中引入的符号 SHALL 不与调用点上下文冲突;`gensym` 内置 SHALL 生成每次调用唯一的符号;同一宏展开多次 SHALL 产生互不冲突的符号。

#### Scenario: 无捕获展开

- **WHEN** 宏展开引入的绑定名与调用点同名变量并存
- **THEN** 两者互不干扰,展开后程序行为与卫生语义一致

#### Scenario: gensym 唯一性

- **WHEN** 同一宏在程序中展开两次且使用 gensym
- **THEN** 两次生成的符号互不相同,无变量冲突

### Requirement: 泛型编译期特化

GenericDef SHALL 在 middle 层被识别并可按参数类型 monomorphize:对 ground 类型的调用 SHALL 特化为专用方法(不再走运行时分发);非特化调用保持运行时分发。`--typecheck` SHALL 报告特化数量。

#### Scenario: ground 类型特化

- **WHEN** 泛型函数以 `i64` 等具体类型调用且存在匹配方法,以 `--typecheck` 运行
- **THEN** 报告特化发生,运行结果与运行时分发一致

### Requirement: 真实动态库 FFI

`defextern` SHALL 支持经动态库加载的真实函数(`libloading`):按名称从指定库解析符号并按 C ABI 调用;库不存在或符号缺失 SHALL 在调用时报明确错误;现有模拟函数表 SHALL 保留为回退。

#### Scenario: 动态库调用

- **WHEN** 声明外部函数指向 libc(如 `abs`)并以 `--run` 执行
- **THEN** 调用返回真实 libc 结果

#### Scenario: 符号缺失报错

- **WHEN** 声明的外部符号在库中不存在并调用
- **THEN** 报告符号解析错误,不崩溃

### Requirement: 反射函数真实化

类型反射内置(`reflect-type` 等)SHALL 返回真实静态类型与运行环境信息(名称、定义、参数、效果),替换硬编码占位;`--typecheck` 通过的程序反射结果 SHALL 与静态信息一致。

#### Scenario: 反射真实类型

- **WHEN** 程序对已定义函数执行类型反射并以 `--run` 运行
- **THEN** 返回该函数的真实签名(与 `--typecheck` 输出一致)

### Requirement: Monad 优化路径接线

EffectCompiler SHALL 从「检测单处理器」扩展到「编译降级」:对可安全降级的单 handle 代码,`--typecheck` 与 `--run` 生成/执行等价的直接状态传递(monadic)路径;不可降级时保持 handler 语义,行为 SHALL 与未优化一致。

#### Scenario: 单处理器降级

- **WHEN** 单处理器 handle 代码满足降级条件并以 `--run` 执行
- **THEN** 结果与 handler 语义一致,且输出标注优化路径生效

#### Scenario: 不可降级保持原义

- **WHEN** 嵌套/多处理器 handle 不满足降级条件
- **THEN** 按 handler 语义执行,结果正确
