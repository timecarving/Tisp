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
