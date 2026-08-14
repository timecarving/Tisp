# evolp-dlp

## Purpose

实现演化逻辑编程(EVOLP)与动态逻辑编程(DLP):程序为不可变值序列,规则携带 assert/retract 演化指令,求值迭代稳定模型至不动点,动态稳定模型按拒绝/接受语义定义。

## Requirements

### Requirement: EVOLP 演化指令

规则 SHALL 可携带演化指令:`assert`(添加规则)与 `retract`(删除规则);演化指令 SHALL 在该时间点生效并影响后续时间点的程序状态。

#### Scenario: assert 添加规则

- **WHEN** 规则 r 携带 `assert(s)` 指令且 r 在时间点 t 成立
- **THEN** 时间点 t+1 的程序包含 s

#### Scenario: retract 删除规则

- **WHEN** 规则 r 携带 `retract(s)` 指令且 r 在时间点 t 成立
- **THEN** 时间点 t+1 的程序不再包含 s

### Requirement: 不可变 Program 与纯函数演化

程序当前状态 SHALL 表示为不可变值 `Program`(当前生效的普通规则集合);演化操作 SHALL 为纯函数(`Program -> Program`);整个演化过程 SHALL 可经 `foldl` 折叠实现。

#### Scenario: foldl 折叠演化

- **WHEN** 程序以 `(foldl evolve P0 [指令序列...])` 演化
- **THEN** 得到与逐时间点演化一致的程序,`P0` 保持不可变(未被破坏)

#### Scenario: 演化纯函数

- **WHEN** 演化函数应用到同一 Program 两次
- **THEN** 结果相同(无隐藏状态)

### Requirement: EVOLP 稳定模型不动点

EVOLP 求值 SHALL 在每个时间点迭代计算稳定模型,直到程序不再变化(不动点);稳定模型由约化 + 最小模型求得;不动点结果 SHALL 为最终生效规则集。

#### Scenario: 不动点收敛

- **WHEN** 演化指令序列经若干时间点后规则集不再变化,求值
- **THEN** 返回不动点规则集,且模型满足稳定模型条件

#### Scenario: 稳定模型判定

- **WHEN** 程序含否定规则(缺省否定),求稳定模型
- **THEN** 返回满足稳定模型语义的解释(约化 + 最小模型),而非朴素枚举

### Requirement: DLP 状态序列

知识库 SHALL 视为状态序列 `P1,…,Pn`,每个状态是一个普通逻辑程序;更新操作 SHALL 通过向序列末尾追加新状态实现。

#### Scenario: 追加状态更新

- **WHEN** 对动态程序序列追加新状态 P(n+1) 并查询
- **THEN** 查询按最新状态序列求值,历史状态保留

### Requirement: 动态稳定模型

给定动态程序序列 P1,…,Pn,解释 M 是动态稳定模型,当且仅当:对每个状态 Pi 拒绝所有被后续状态否定的规则;对剩余规则应用约化,M 是所得程序的最小模型。

#### Scenario: 拒绝被否定规则

- **WHEN** 状态 Pi 含规则 r,且后续状态 Pj 否定 r(缺省否定),求动态稳定模型
- **THEN** r 被拒绝,动态稳定模型与拒绝/接受语义一致

#### Scenario: 约化最小模型

- **WHEN** 拒绝操作后对剩余规则做约化并求最小模型
- **THEN** 返回该最小模型,结果与动态稳定模型定义一致
