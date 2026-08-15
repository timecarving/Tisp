## ADDED Requirements

### Requirement: 八范式可用性契约达标

8 类编程范式（数组、栈、连接式、符号、自动机、状态机、数据驱动、基于流）与 AOP 编织中的每一类 SHALL：从 Tisp 源码可声明并使用；其操作具备静态类型签名并参与类型检查；在合法输入上运行结果与范式语义一致；非法输入（越界索引、非法转移、非法 DFA 符号、未知切面）SHALL 显式报错；范式运行时状态 SHALL 纳入统一内存跟踪。

#### Scenario: 八范式端到端

- **WHEN** 对 8 类范式各自的示例程序依次执行 `--typecheck` 与 `--run`
- **THEN** 全部类型检查通过，运行结果正确，无占位值

#### Scenario: 非法输入显式报错

- **WHEN** 数组越界索引、状态机非法转移或 DFA 收到未声明符号
- **THEN** 报告明确错误，程序不静默继续

### Requirement: AOP 编织语义可验证

AOP 切面 SHALL 以纯声明式 MOP 声明并经编译期/加载期编织：around SHALL 包裹原方法且 `call-next-method` 进入内层链；before/after SHALL 不改变主方法结果；编织后的调用 SHALL 与未编织的等价调用在同一类型/效应/内存检查下执行。

#### Scenario: around 编织结果

- **WHEN** 声明 around 切面对方法翻倍并以 `--run` 调用
- **THEN** 返回包裹后的结果（原结果 × 2），`call-next-method` 返回内层原结果

#### Scenario: before/after 不影响结果

- **WHEN** before/after 切面各自返回非主结果值
- **THEN** 主方法结果保持不变
