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

### Requirement: comptime 编译期求值

`(comptime expr)` SHALL 在编译期对 expr 求值并将结果内联进 Core AST：常量表达式 SHALL 折叠为字面量/构造后的值；comptime 内的 MOP 操作（GetKB/SetKB 效应）SHALL 在编译期执行并影响后续编译；编译期求值失败 SHALL 报告编译错误（含位置），不得延迟到运行时。

#### Scenario: 常量折叠

- **WHEN** 程序以 `(comptime (+ 1 2))` 出现在表达式位置，以 `--desugar` 运行
- **THEN** 输出中该位置为字面量 3，`--run` 结果与普通求值一致

#### Scenario: 编译期错误

- **WHEN** `(comptime (undefined-fn 1))` 编译期求值失败，以 `--typecheck`/`--desugar` 运行
- **THEN** 报告带位置的编译期求值错误，程序不进入执行

### Requirement: 编译期 MOP 知识库

comptime 上下文中的 `get-kb`/`set-kb` SHALL 读写编译期知识库：元程序 SHALL 可读取编译期可见的规则/方法集合、写入新事实或规则，并让同一编译单元后续的宏展开/切面编织/类型检查看到更新；运行时知识库与编译期知识库 SHALL 分离（编译期写入不泄漏为运行时状态）。

#### Scenario: 编译期 KB 影响后续编译

- **WHEN** comptime 元程序向 KB 写入一条规则，随后同文件定义引用该规则，以 `--typecheck` 运行
- **THEN** 后续定义可见该规则，类型检查/脱糖按更新后的 KB 进行

#### Scenario: 编译期与运行时分离

- **WHEN** comptime 写入 KB 后运行程序并调用运行时 `get-kb`
- **THEN** 运行时 KB 不包含 comptime 写入的编译期事实
