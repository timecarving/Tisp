# logic-and-verification

## Purpose

补全逻辑编程与协议验证能力(§20-21/§28):CLP 域间约束传播、ALP 溯因真实化、协议攻击搜索(dolev-yao)与多参与方会话类型(MPST),兑现「验证 = 探索所有路径的 effect handler」的设计承诺。

## Requirements

### Requirement: CLP 域间约束传播

`constrain` SHALL 将不等式/等式约束编译为真实域传播(替换恒真 propagator):约束 SHALL 收缩相关变量域,产生域冲突 SHALL 使搜索失败(回溯);`solve-all` 结果 SHALL 与传播后的域一致。

#### Scenario: 域间传播收缩

- **WHEN** 两个变量经 `constrain` 施加 `(< x y)` 且 x 域 `[1,10]`、y 域 `[1,10]`,随后 `label` 枚举
- **THEN** 解集不包含违反 `x < y` 的组合

#### Scenario: 冲突导致失败

- **WHEN** 约束 `(< x y)` 与 `(> x y)` 同时施加于同一对变量并求解
- **THEN** 搜索失败(无解),不产生错误结果

### Requirement: ALP 溯因真实化

`abduce` SHALL 返回可验证的假设集:每个假设 SHALL 与目标一致(可满足性检查),假设缺失 SHALL 报告目标不可满足的原因;结果 SHALL 为假设列表而非占位字符串。

#### Scenario: 溯因假设集

- **WHEN** 对缺少事实的目标执行 `abduce`
- **THEN** 返回非空假设列表,且追加假设后目标可满足

### Requirement: 协议攻击搜索与等价检查

`find-attack` SHALL 在有限会话深度内搜索协议攻击(角色偏离、机密泄露),命中 SHALL 返回攻击轨迹;`check-equivalence` SHALL 比较两个协议进程的观察等价;搜索深度 SHALL 由参数限制。

#### Scenario: 找到攻击轨迹

- **WHEN** 对含漏洞协议执行 `find-attack`(深度 20)
- **THEN** 返回攻击轨迹(角色序列),或明确报告无攻击

#### Scenario: 等价检查结果

- **WHEN** 对两个行为等价/不等价进程执行 `check-equivalence`
- **THEN** 返回等价结论(等价或给出区分轨迹)

### Requirement: 多参与方会话类型(MPST)

defsession SHALL 支持多参与方协议(`:role` 标注),角色投影 SHALL 为单方会话类型;协议体 SHALL 在 `--desugar` 输出中保留(不得丢弃),类型级协议检查 SHALL 拒绝错序操作。

#### Scenario: 角色投影

- **WHEN** 定义含两个角色及交互序列的 defsession 协议
- **THEN** `--desugar` 输出保留完整协议结构,各角色投影正确

#### Scenario: 协议顺序违规

- **WHEN** 会话操作顺序违反协议(如未接收先发送),以 `--typecheck` 运行
- **THEN** 报告协议违反错误
