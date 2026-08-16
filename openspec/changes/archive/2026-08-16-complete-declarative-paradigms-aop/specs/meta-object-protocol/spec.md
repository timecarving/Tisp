## ADDED Requirements

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
