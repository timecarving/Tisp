# logic-and-verification

## Purpose

补全逻辑编程与协议验证能力(§20-21/§28):CLP 域间约束传播、ALP 溯因真实化、协议攻击搜索(dolev-yao)与多参与方会话类型(MPST),兑现「验证 = 探索所有路径的 effect handler」的设计承诺。

## Requirements

### Requirement: 任意谓词多解回溯

`defpred` 谓词 SHALL 支持 Prolog 式完整续延回溯(替换首解/0 解):对递归谓词、`or` 体与结构化参数,`find-all`/`solve-all` SHALL 经续延式选择点重入枚举全部解;每个分支的解 SHALL 隔离收集;自由变量 SHALL 经真实统一绑定;搜索策略(DFS/BFS)SHALL 可由 handler 选择。

#### Scenario: 递归谓词多解

- **WHEN** 以 `find-all` 查询递归 `member` 谓词(含 `:free` 输出参数)
- **THEN** 返回全部匹配解(如 `[1 2 3]`),而非首解或 0 解

#### Scenario: or 体多解

- **WHEN** 谓词体含 `or` 析取分支并以 `find-all` 求解
- **THEN** 返回所有分支解,而非短路首解

#### Scenario: 结构化值统一

- **WHEN** 谓词参数含结构化值(如 `(cons 1 2)`)并以统一绑定输出变量
- **THEN** 输出变量绑定到正确结构化值(非 `Int(0)` 折叠)

#### Scenario: 分支解隔离

- **WHEN** 谓词多子句且嵌套调用子谓词,以 `find-all` 求解
- **THEN** 各分支解互不污染,总数与语义一致

### Requirement: CLP 域间约束传播

`constrain` SHALL 将不等式/等式约束编译为真实域传播(替换恒真 propagator):约束 SHALL 收缩所有相关变量域(含结果变量),产生域冲突 SHALL 使搜索失败(回溯);SHALL 支持非线性约束(乘/除/模)与线性表达式(`(+ x 1)`/`(- x y)`);除法 SHALL 精确(截断除 SHALL 不把非整除结果判为满足);`solve-all` 结果 SHALL 与传播后的域一致。

#### Scenario: 域间传播收缩

- **WHEN** 两个变量经 `constrain` 施加 `(< x y)` 且 x 域 `[1,10]`、y 域 `[1,10]`,随后 `label` 枚举
- **THEN** 解集不包含违反 `x < y` 的组合

#### Scenario: 非线性约束收窄结果变量

- **WHEN** 约束 `(= (* x y) z)` 且 x、y 域受限,求解 z
- **THEN** z 域收窄到可能的乘积集合,`solve-all z` 结果与之相符

#### Scenario: 线性表达式约束

- **WHEN** 约束 `(< (+ x 1) y)` 施加于域变量
- **THEN** 编译为域传播,解集正确收缩

#### Scenario: 精确除法

- **WHEN** 约束 `(= (/ x y) z)` 且 `x=7, y=2`(非整除)
- **THEN** 不把截断值 3 判为满足;要么精确求解要么正确失败

#### Scenario: 冲突导致失败

- **WHEN** 约束 `(< x y)` 与 `(> x y)` 同时施加于同一对变量并求解
- **THEN** 搜索失败(无解),不产生错误结果

### Requirement: ALP 溯因真实化

`abduce` SHALL 返回可验证的假设集并枚举多个解释:每个假设 SHALL 与目标一致(可满足性检查)且 SHALL 尊重声明的 `domain`(越界假设 SHALL 被排除);假设缺失 SHALL 报告目标不可满足的原因;SHALL 支持对逻辑变量(而非仅整数)的溯因;结果 SHALL 为假设列表而非占位字符串。

#### Scenario: 溯因假设集

- **WHEN** 对缺少事实的目标执行 `abduce`
- **THEN** 返回非空假设列表,且追加假设后目标可满足

#### Scenario: 多解释枚举

- **WHEN** 目标存在多个一致假设并执行 `abduce`
- **THEN** 枚举出全部一致解释,数量与语义一致

#### Scenario: domain 约束假设

- **WHEN** 声明 `(domain x 1 3)` 后对约束 `(> x 1)` 执行 `abduce`
- **THEN** 仅返回 `x ∈ {2, 3}` 的假设(而非越界的 `x ∈ {4, 5}`)

#### Scenario: 逻辑变量溯因

- **WHEN** 对含逻辑变量等价关系的目标(如 `(abduce (unify x y) x y)`)执行 `abduce`
- **THEN** 返回使目标可满足的变量绑定假设

### Requirement: 协议攻击搜索与等价检查

`find-attack` SHALL 在有限会话深度内搜索协议攻击;dolev-yao 攻击者模型 SHALL 支持——标准攻击者(窃听/转发/篡改/合成消息)驱动的完整攻击搜索,替换场景搜索;`check-equivalence` SHALL 比较观察等价;搜索深度 SHALL 由参数限制。

#### Scenario: 找到攻击轨迹

- **WHEN** 对含漏洞协议执行 `find-attack`(深度 20)
- **THEN** 返回攻击轨迹(角色序列),或明确报告无攻击

#### Scenario: dolev-yao 攻击者合成

- **WHEN** 攻击者从已知消息合成新消息发起攻击(如重放/篡改/解密),执行 `find-attack`
- **THEN** 合成攻击被建模并给出攻击结论(成功或证明安全)

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
