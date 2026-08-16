## ADDED Requirements

### Requirement: 八范式纯声明式副作用管理

8 类编程范式 SHALL 在纯声明式副作用管理下运行：数组、符号、自动机为 Pure（不可变纯函数）；栈、状态机、数据驱动的状态变更 SHALL 经 `State` 效应操作或 monadic（`mlet`/`get-m`/`put-m`/`pure`）表达；基于流 SHALL 经 `Signal`/FRP 效应。`--typecheck` 输出的效应行 SHALL 与实际操作一致；纯代码未经 handler/声明调用状态类操作 SHALL 被拒绝。

#### Scenario: 效应行与操作一致

- **WHEN** 程序执行栈操作（如 push/pop）与状态机驱动，以 `--typecheck` 运行
- **THEN** 相关定义的效应行包含 `State` 或对应的 monadic 声明，纯函数定义不含状态效应

#### Scenario: 纯代码拒绝状态操作

- **WHEN** 声明为 Pure 的函数直接调用栈 push/状态机 drive 且无 handler，以 `--typecheck` 运行
- **THEN** 报告效应缺失错误

### Requirement: 八范式完整源码表面

每类范式 SHALL 具备完整源码表面并可端到端运行：数组（创建/形状/索引/切片/逐元素 map/沿轴 reduce）；栈（new/push/pop/peek/dup/swap/rotate）；连接式（compose/apply/branch 点自由组合）；符号（构造/匹配/代换/化简/求值）；自动机（DFA 声明/识别，未声明符号报错）；状态机（状态/事件/转移/动作，非法转移报错且状态不变）；数据驱动（表驱动分发，仅改数据可改行为）；基于流（源/变换/过滤/取前 n/汇，惰性不卡死）。非法输入 SHALL 显式报错，不得静默返回默认值。

#### Scenario: 八范式端到端

- **WHEN** 对 8 类范式各自的示例程序依次执行 `--typecheck` 与 `--run`
- **THEN** 全部通过且输出与各范式语义一致，无 panic、无占位输出

#### Scenario: 非法输入显式报错

- **WHEN** 数组越界索引、DFA 收到未声明符号、状态机触发非法转移、符号表达式含未代换自由变量求值
- **THEN** 各自报告明确错误，不静默继续或返回 0

### Requirement: 单子优化路径真实生效

单处理器 `State` 范式代码 SHALL 被编译/执行为直接状态线程（零开销状态传递，而非仅计数）：结果 SHALL 与 effect handler 语义一致；`--run` SHALL 报告实际走状态传递路径；多处理器/不可降级情形 SHALL 保持 handler 语义。

#### Scenario: 栈单子降级等价

- **WHEN** 以单处理器 handler 管理数据栈并执行 push/pop 序列，以 `--run` 运行
- **THEN** 结果与 effect 风格一致，且报告走直接状态线程（非仅计数）

#### Scenario: 不可降级保持原义

- **WHEN** 嵌套/多处理器状态范式代码不满足降级条件
- **THEN** 按 handler 语义执行，结果正确
