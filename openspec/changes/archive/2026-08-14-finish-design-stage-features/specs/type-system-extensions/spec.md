## ADDED Requirements

### Requirement: 类型类完整实例解析

类型类实例 SHALL 完整解析:分发器按参数类型查实例(SHALL 保持);`:fun-deps`(函数依赖)SHALL 约束实例一致性——违反函数依赖的实例对(同输入不同输出)SHALL 报错;超类约束 SHALL 传播——实例须实现超类方法;kind 约束 SHALL 校验(实例类型 kind 与声明一致)。

#### Scenario: fun-deps 冲突检测

- **WHEN** 声明含 `:fun-deps` 的类型类且存在违反函数依赖的实例对,以 `--typecheck` 运行
- **THEN** 报告函数依赖冲突错误

#### Scenario: 超类约束传播

- **WHEN** 类型类声明超类且实例未实现超类方法,以 `--typecheck` 运行
- **THEN** 报告超类缺失错误

#### Scenario: kind 校验

- **WHEN** 实例类型 kind 与类型类声明不一致,以 `--typecheck` 运行
- **THEN** 报告 kind 错误

### Requirement: 依赖会话类型

会话类型 SHALL 支持值依赖(§20.2/20.3):会话协议 SHALL 引用依赖值(如通道消息携带长度依赖的负载 `(Vec i64 n)`);依赖会话的类型级协议检查 SHALL 拒绝违反协议的操作(与既有顺序检查一致)。

#### Scenario: 值依赖会话

- **WHEN** defsession 协议含值依赖(如发送依赖负载)且操作顺序合法,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 依赖会话违规

- **WHEN** 依赖会话操作顺序违反协议,以 `--typecheck` 运行
- **THEN** 报告协议违反错误

## MODIFIED Requirements

### Requirement: 资源代数声明

`defresource-algebra` SHALL 解析为资源代数(单位元、二元运算、阶),`Cost` 注解 SHALL 在类型中携带代数语义;`Cost` 注解 SHALL 有语法并参与代价/复杂度推导——声明了 Cost 代数的类型 SHALL 可标注代价上界,使用超过上界 SHALL 报错(可判定时)或明确警告(符号/不可判定时);未实现的运算 SHALL 报错而非静默通过。

#### Scenario: 资源代数解析

- **WHEN** 源文件声明资源代数与 Cost 注解,以 `--desugar` 运行
- **THEN** 输出保留代数结构与 Cost 标注,无解析错误

#### Scenario: Cost 注解与推导

- **WHEN** 类型以 `@Cost` 标注代价上界且使用超过上界,以 `--typecheck` 运行
- **THEN** 报告代价违反,或对不可判定情形明确警告放行
