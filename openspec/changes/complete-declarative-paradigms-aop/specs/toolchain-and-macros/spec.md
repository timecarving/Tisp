## ADDED Requirements

### Requirement: 范式 monadic 状态链真实编译

`mlet`/`get-m`/`put-m`/`pure` SHALL 在 8 类范式的状态线程中真实可用：monadic 风格编写的栈/状态机/数据驱动程序 SHALL 经单处理器检测降级为直接状态传递并执行；结果 SHALL 与代数效应风格等价；`--run` SHALL 报告降级数量与实际路径，而非仅语法接受。

#### Scenario: monadic 栈程序

- **WHEN** 以 `mlet`/`get-m`/`put-m`/`pure` 编写数据栈操作并以 `--run` 执行
- **THEN** 执行结果与 effect 风格一致，且报告走直接状态线程

#### Scenario: 效应风格等价

- **WHEN** 同一栈/状态机程序分别以 handle/perform 与 monadic 风格编写并运行
- **THEN** 两种风格输出一致

### Requirement: comptime 工具链全链路

`comptime` SHALL 贯通工具链：lexer/reader 识别 → desugar 执行编译期求值与 MOP 编织 → `--desugar` 输出内联/编织后的 Core AST → `--typecheck` 检查内联后的程序 → `--run` 执行内联后的程序；`--run` 在执行前 SHALL 已完成 comptime 阶段，运行时不再次求值。

#### Scenario: 全链路 comptime

- **WHEN** 程序含 `comptime` 常量表达式，依次以 `--desugar`/`--typecheck`/`--run` 运行
- **THEN** desugar 显示内联结果，typecheck 通过，run 输出与内联语义一致

#### Scenario: 运行时不重复求值

- **WHEN** comptime 表达式带副作用（如编译期 KB 写入），以 `--run` 执行
- **THEN** 副作用只发生一次（编译期），运行阶段不重复执行
