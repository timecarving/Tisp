# paradigm-integration

## Purpose

定义 21 个范式(12 逻辑 + 8 编程 + AOP)的一等化接入契约:每个范式实现为 Rust 中带集成接口的一等设施,通过共享抽象(值/类型/效应/等级)与其他必要特性插接组合,并贯通到全链路,直到端到端可用。

## Requirements

### Requirement: 范式一等化接口

每个范式(12 逻辑 + 8 编程 + AOP)SHALL 实现为一等 Rust 设施,暴露清晰的集成接口(公开 trait / 模块 API / AST 节点 / 类型构造 / 效应操作);其他特性 SHALL 能经该接口调用范式能力,而非复制其内部逻辑。

#### Scenario: 接口可调用

- **WHEN** 另一特性(如类型检查器或解释器)经范式暴露的 trait/API 调用其能力(如驱动自动机、执行符号化简、求解逻辑目标)
- **THEN** 调用返回正确结果,接口签名稳定,不依赖范式内部表示

#### Scenario: 语义由范式自身实现

- **WHEN** 范式求值其核心语义(如稳定模型求解、DFA 识别、数组归约)
- **THEN** 语义由范式自身求值器产出,而非以「组合其他特性」替代

### Requirement: 组合 = 共享抽象接入

范式的组合 SHALL 通过统一接入抽象(值 `Value`、类型 `Type`、效应 `EffectRow`、等级 `Grade`)实现:范式间、范式与其他特性间 SHALL 经该抽象互操作;每个范式 SHALL 声明其类型构造与效应操作,接入统一的类型/效应检查。

#### Scenario: 跨范式组合

- **WHEN** 组合两个范式(如数组编程 + 逻辑编程,或基于流 + FRP)经共享值/类型/效应抽象协作
- **THEN** 组合正确工作,值在范式间传递、效应在统一效应行中追踪

#### Scenario: 副作用接入效应行

- **WHEN** 范式副作用(栈顶/状态机态/搜索/流缓冲)触发
- **THEN** 副作用作为声明式效应操作接入效应行,纯代码未经 handler 无法触发

### Requirement: 全链路接线

每个范式 SHALL 从 Tisp 源码可书写,并贯通全链路:lexer/reader 识别范式形式 → desugar 生成 `CoreExprNode` → 类型/效应/等级推断 → 解释器求值;`--run` SHALL 端到端给出正确结果。

#### Scenario: 源码端到端求值

- **WHEN** 源码书写某范式(如声明 DFA 并识别输入、声明状态机并驱动事件、声明符号表达式并化简)
- **THEN** `--typecheck` 通过且 `--run` 端到端返回正确结果

#### Scenario: 类型检查接入

- **WHEN** 范式值以错误类型使用(如把数组当自动机),以 `--typecheck` 运行
- **THEN** 报告类型错误(范式类型参与 HM 推断)

### Requirement: 设施元数据强制

每个经 ParadigmRegistry/Facility 注册的范式设施 SHALL 携带完整的六维元数据（类型构造、效应行、区域、等级、模式、确定性）与声明式来源标记；缺失或占位元数据的设施 SHALL 不得注册成功；type_infer 对范式内置的签名 SHALL 从该元数据生成，而非手写单态补丁。

#### Scenario: 元数据完整注册

- **WHEN** 注册全部范式设施并以 `--typecheck` 编译调用范式内置的程序
- **THEN** 每个设施的六维元数据齐全，类型/效应/确定性检查结果与元数据一致

#### Scenario: 缺失元数据拒绝

- **WHEN** 某设施未声明效应或等级元数据而尝试注册
- **THEN** 注册失败并报告缺项，不得以默认占位放行

### Requirement: 范式执行经统一内存跟踪

范式设施的执行 SHALL 经统一内存体系约束：设施句柄与状态 SHALL 携带 QTT 等级（0/1/ω）并参与 grade_check；值依赖结构经依赖线性类型检查；资源上界经 `□_r`/`@Cost` 判定；`Unsafe` 访问经效应门控；底层分配与回收经统一内存入口并由 `--run` 区域统计反映。重复执行同一范式程序 SHALL 不累积未回收状态。

#### Scenario: 分配回收一致

- **WHEN** 同一含范式状态的程序连续执行两次
- **THEN** 每次的区域统计一致（分配与回收配对），无跨次泄漏

#### Scenario: 设施句柄等级检查

- **WHEN** 范式设施句柄以线性等级使用后复用，以 `--typecheck` 运行
- **THEN** 报告等级违反，设施元数据中的等级信息与检查结果一致

#### Scenario: 设施资源上界

- **WHEN** 设施声明 `□_r`/`@Cost` 资源上界且调用超界，以 `--typecheck` 运行
- **THEN** 报告资源违反或明确警告

#### Scenario: 设施 Unsafe 门控

- **WHEN** 设施内部存储被裸指针访问且无 `Unsafe` 效应声明，以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误

### Requirement: 范式副作用接入共享效应行

8 类范式与 AOP 的状态副作用 SHALL 经共享效应行执行：栈/状态机/数据驱动操作 SHALL 归属 `State`；基于流 SHALL 归属 `Signal`；数组/符号/自动机 SHALL 为 Pure；AOP 编织后的方法链 SHALL 保留 primary 方法原有效应行并追加切面声明的效应。`effect_infer` 结果 SHALL 与运行时行为一致，不得以设施简化投影绕过。

#### Scenario: 跨范式效应组合

- **WHEN** 程序组合栈编程 + 基于流编程，以 `--typecheck` 运行
- **THEN** 效应行同时包含 `State` 与 `Signal`，纯代码部分保持 Pure

#### Scenario: AOP 效应行合成

- **WHEN** around 切面声明 `State` 效应并包裹 Pure 方法，以 `--typecheck` 运行
- **THEN** 编织后方法链效应行为 `State`，与切面声明一致

### Requirement: 范式设施语义不能绕过

`pf-*` 设施别名与完整范式内置的语义 SHALL 一致：对同一输入 SHALL 产生同一结果与同一错误；任何简化投影（sum%2、+100、默认 0）SHALL 不得作为 `--run` 的公开正确性来源；调用设施别名 SHALL 与调用完整内置走同一效应/类型检查。

#### Scenario: 别名等价

- **WHEN** 同一程序分别经 `pf-*` 别名与完整内置执行同一范式操作
- **THEN** 结果、效应行与错误行为完全一致

#### Scenario: 简化投影失效

- **WHEN** 运行依赖旧简化投影语义（如 `pf-dfa-accept` 的 sum%2、`pf-aop-weave` 的 +100）的程序
- **THEN** 输出为完整语义结果（或对不再支持的形式显式报错），不静默给出旧投影值
