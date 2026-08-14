## ADDED Requirements

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

## MODIFIED Requirements

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
