# programming-paradigms

## Purpose

定义 8 类编程范式(数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流)的行为契约:全部以纯声明式副作用管理(代数效应 + 单子)实现,使这些范式在 Tisp 中端到端可用。

## Requirements

### Requirement: 数组编程

数组 SHALL 作为一等类型:多维数组可创建、索引、切片,并按元素/沿轴执行映射、折叠、扫描等批量操作;数组操作 SHALL 为纯函数(原数组不可变),批量副作用 SHALL 经代数效应/单子管理。

#### Scenario: 逐元素与归约

- **WHEN** 创建二维数组并执行 map/reduce(如逐元素加 1 后沿轴求和)
- **THEN** 返回正确的数组/标量,原数组保持不变

#### Scenario: 切片与形状

- **WHEN** 对数组切片并查询形状(维度/长度)
- **THEN** 返回正确的子数组与形状,越界索引报错

### Requirement: 栈编程

程序 SHALL 支持栈式求值:数据栈与栈操作(压栈/弹栈/交换/复制/旋转)SHALL 可用;栈操作 SHALL 组合为纯函数(栈 → 栈 变换),无隐式变异。

#### Scenario: 栈操作组合

- **WHEN** 以栈操作序列(如 dup/swap/+)作用于初始栈
- **THEN** 栈顶结果正确,整个变换为纯函数

### Requirement: 连接式编程

程序 SHALL 支持点自由(concatenative)组合:函数 SHALL 可经连接与组合子(compose/apply/branch)串联,无需显式参数传递;组合结果 SHALL 与显式管道等价。

#### Scenario: 点自由组合

- **WHEN** 以连接式组合子串联多个函数并求值
- **THEN** 结果与显式函数应用等价

### Requirement: 符号编程

程序 SHALL 支持符号表达式:符号 SHALL 可声明为未求值/惰性;符号表达式 SHALL 可构造、模式匹配、代换(变量替换)、化简与求值。

#### Scenario: 符号代换与化简

- **WHEN** 构造符号表达式(如 `(+ x 1)`)并代换 x=2 后化简求值
- **THEN** 得到化简结果 3

#### Scenario: 符号模式匹配

- **WHEN** 对符号表达式执行 match 按结构分支
- **THEN** 按符号结构正确分支

### Requirement: 自动机编程

程序 SHALL 支持有限自动机/下推自动机:状态与转移表 SHALL 可声明,输入串 SHALL 可被识别(接受/拒绝);自动机 SHALL 可组合(并/串/星)。

#### Scenario: DFA 识别

- **WHEN** 声明 DFA 转移表并以输入串运行
- **THEN** 正确判定接受/拒绝

#### Scenario: 自动机组合

- **WHEN** 组合两个自动机(并/串)
- **THEN** 组合结果正确识别对应语言

### Requirement: 状态机编程

程序 SHALL 支持显式状态机:状态/事件/转移/动作 SHALL 可声明;事件序列 SHALL 驱动状态转移并触发动作(entry/exit/transition);非法转移 SHALL 报错。

#### Scenario: 事件驱动转移

- **WHEN** 声明状态机并以事件序列驱动
- **THEN** 状态正确转移,动作按序触发

#### Scenario: 非法转移报错

- **WHEN** 触发未声明的事件-状态组合
- **THEN** 报错,状态不改变

### Requirement: 数据驱动编程

程序 SHALL 支持数据驱动:行为由数据(表/规则/分发)决定而非硬编码;数据表 SHALL 可驱动控制流与分发(查表/策略/解释器);仅改数据不改代码即可改变行为。

#### Scenario: 表驱动分发

- **WHEN** 以数据表驱动分发(查表选择行为)
- **THEN** 行为随数据变化,无需改代码

### Requirement: 基于流编程

程序 SHALL 支持基于流的数据流编程:节点(源/变换/汇)SHALL 可组合为数据流网络;流 SHALL 惰性/增量求值;数据流网络 SHALL 与既有 FRP/时序流一致。

#### Scenario: 数据流网络

- **WHEN** 组合源→变换→汇的数据流网络并求值
- **THEN** 数据经网络正确流转,与显式管道等价

#### Scenario: 惰性流

- **WHEN** 数据流为无限/惰性流并取前 n 项
- **THEN** 正确产出前 n 项,不卡死

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
