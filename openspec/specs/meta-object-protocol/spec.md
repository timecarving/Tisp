# meta-object-protocol

## Purpose

实现元对象协议(MOP):知识库查询/更新(GetKB/SetKB)建模为 Effect 操作,Handler 充当元解释器;元编程能力在编译期即可满足,并设计 State Effect 完成引用管理。

## Requirements

### Requirement: GetKB/SetKB 效应操作

知识库查询/更新 SHALL 建模为效应操作:`GetKB` 读取当前知识库、`SetKB` 写入知识库;两者 SHALL 声明在效应行中,纯代码未经 handler SHALL 无法调用。

#### Scenario: 效应门控 KB 访问

- **WHEN** 纯代码(无对应效应)调用 GetKB,以 `--typecheck` 运行
- **THEN** 报告效应缺失错误

#### Scenario: 经 handler 读写 KB

- **WHEN** 程序以 handler 解释器执行含 GetKB/SetKB 的元程序
- **THEN** 读写按 handler 语义作用于知识库,结果正确

### Requirement: Handler 元解释器

元程序 SHALL 由 handler 充当元解释器执行:handler SHALL 捕获 GetKB/SetKB 操作并解释其语义;元解释 SHALL 与语言自身求值器行为一致(可反射、可扩展)。

#### Scenario: 元解释一致性

- **WHEN** handler 元解释器执行一段元程序并求值
- **THEN** 结果与直接求值等价,且可扩展新的解释行为

### Requirement: 编译期元编程

MOP 的元编程能力 SHALL 在编译期即可满足:宏展开/部分求值 SHALL 在编译期解析 GetKB/SetKB 与反射(如规则/约束数据的静态操作),运行时 SHALL 不强制依赖反射性运行时(运行时 handler 为回退路径)。

#### Scenario: 编译期解析元操作

- **WHEN** 元程序对已知(编译期可见)规则集执行 GetKB/SetKB 类静态操作
- **THEN** 编译期即解析出结果,不产生运行时动态变量

#### Scenario: 运行时回退

- **WHEN** 元程序操作编译期不可见的知识库
- **THEN** 回退到运行时 handler 元解释,行为一致

### Requirement: State Effect 引用管理

在既有 `State s`(get/put)之上设计可变引用:`ref` 创建引用、`deref` 读取、`set!` 写入;引用 SHALL 建模为 State 效应操作并以线性/分级等级约束所有权(引用使用后不可复用、别名受等级限制)。

#### Scenario: 线性引用读写

- **WHEN** 程序以线性引用 `{1 r : (Ref a)}` 创建/读写并消费
- **THEN** 读写正确,使用后引用不可复用

#### Scenario: 引用别名受等级约束

- **WHEN** 引用被多次读(deref)且等级标注为 1,以 `--typecheck` 运行
- **THEN** 报告等级违反;标注 ω 则可多次读
