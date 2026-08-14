## Why

Tisp 已具备强静态类型、纯声明式、进程演算与代数效应核心,但仍有若干「部分实现(⚠️)/未实现」特性停留在语义残缺或占位(密码学 XOR 占位、inkwell 闭包真代码生成缺、□_r/◇_ε 无推理、HoTT 完整立方填充缺、Cohesive 完整同伦模型缺、时序 □_t 语义保证缺、Cost 全推导缺、编译期区域逃逸检查缺、EVOLP/DLP/MOP 语言表面接线缺),且缺少数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流等 8 类编程范式,以及用于辅助 OOP 的 AOP。本变更把这些缺口补齐到「全链路可用」,并以纯声明式副作用管理(代数效应 + 单子)落地 8 类编程范式与 MOP 驱动的 AOP,使「一切皆 ADT + 演算 > 代数效应」的最终形态端到端可用。

## What Changes

- **补齐 ⚠️/⬜ 特性到全链路可用**:分级模态 □_r/◇_ε 推理、Cost 注解全推导(渐近代价)、HoTT 完整立方填充(多维 Kan)、Cohesive 完整同伦模型(adjoint-triple)、时序 □_t 稳定类型语义保证(因果性/生产率/空间回收)、编译期区域逃逸检查、inkwell 函数/闭包真代码生成、密码学原语真实实现(AES/ChaCha20/SHA-256 替换 XOR 占位)、EVOLP/DLP/MOP 语言表面接线、统一约束求解与演算统一收尾。
- **8 类编程范式(组合优先,纯声明式副作用管理)**:数组编程、栈编程、连接式编程、符号编程、自动机编程、状态机编程、数据驱动编程、基于流编程,全部经代数效应/单子管理副作用,实现到全链路可用。
- **AOP(基于编译器纯声明式 MOP)辅助 OOP**:在编译期 MOP 之上实现切面(Aspect/Pointcut/Advice)编织,辅助 OOP 方法组合,直到全链路可用。

## Capabilities

### New Capabilities

- `programming-paradigms`: 8 类编程范式(数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流),以代数效应 + 单子管理副作用的纯声明式实现。
- `aspect-oriented-programming`: 基于编译器纯声明式 MOP 的 AOP(切面/切入点/建议编织),辅助 OOP。

### Modified Capabilities

(无——「补齐 ⚠️ 特性」为既有需求的实现完成,不改变 spec 级行为;新能力以新增 capability 承载。)

## Impact

- **tisp-core**:新增数组/栈/符号/自动机/状态机等数据与 AST 节点;`Type` 扩展数组/多维类型。
- **tisp-frontend**:desugar 新增 8 类范式语法与 AOP(`deftrait`-风格 `aspect`/`pointcut`/`advice`)的脱糖。
- **tisp-middle**:效应/等级推断扩展(数组切片、状态机转移、AOP 编织的效应行);补齐 □_r/◇_ε 推理、Cost 全推导、区域逃逸检查。
- **tisp-backend**:interpreter 新增 8 类范式内置;inkwell 闭包真代码生成;密码学真原语。
- **tisp-runtime**:新增 `array`/`concatenative`/`symbolic`/`automata`/`statemachine`/`datadriven`/`stream`/`aop` 模块。
- **docs/spec.md**:新增/扩展编程范式与 AOP 章节;同步 `standard_doc/` 与 `CHANGELOG.md`。
