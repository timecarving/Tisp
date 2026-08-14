## MODIFIED Requirements

### Requirement: CLP 域间约束传播

`constrain` SHALL 将约束编译为真实域传播(替换恒真 propagator):约束 SHALL 收缩相关变量域,产生域冲突 SHALL 使搜索失败(回溯);`solve-all` 结果 SHALL 与传播后的域一致。**算术约束 SHALL 编译为传播器——线性(加减比较)已实现,非线性(乘/除/模)与全局约束(全不同等)SHALL 支持**,`label` 枚举 SHALL 与传播后的域一致。

#### Scenario: 域间传播收缩

- **WHEN** 两个变量经 `constrain` 施加 `(< x y)` 且 x 域 `[1,10]`、y 域 `[1,10]`,随后 `label` 枚举
- **THEN** 解集不包含违反 `x < y` 的组合

#### Scenario: 冲突导致失败

- **WHEN** 约束 `(< x y)` 与 `(> x y)` 同时施加于同一对变量并求解
- **THEN** 搜索失败(无解),不产生错误结果

#### Scenario: 乘法约束传播

- **WHEN** `(constrain (= (* x y) 12))` 且 x 域 `[1,6]`、y 域 `[1,6]`,随后 `label` 枚举
- **THEN** 解集仅含满足 x·y = 12 的组合(如 (2,6)、(3,4))

#### Scenario: 全不同约束

- **WHEN** `(constrain (all-different x y z))` 且三变量共享域 `[1,3]`,随后 `label` 枚举
- **THEN** 解为三变量的排列(无重复值)

### Requirement: ALP 溯因真实化

`abduce` SHALL 返回可验证的假设集:每个假设 SHALL 与目标一致(可满足性检查),假设缺失 SHALL 报告目标不可满足的原因;结果 SHALL 为假设列表而非占位字符串。**多解解释 SHALL 可枚举**(`abduce-all` 风格或返回全部一致解释),每解释 SHALL 独立可验证。

#### Scenario: 溯因假设集

- **WHEN** 对缺少事实的目标执行 `abduce`
- **THEN** 返回非空假设列表,且追加假设后目标可满足

#### Scenario: 多解解释枚举

- **WHEN** 目标可由多组不同假设满足,执行多解溯因
- **THEN** 返回全部一致解释(每组假设独立满足目标)

#### Scenario: 不可满足原因

- **WHEN** 目标在任何假设下都不可满足,执行 `abduce`
- **THEN** 报告目标不可满足及原因(而非空列表或占位)
