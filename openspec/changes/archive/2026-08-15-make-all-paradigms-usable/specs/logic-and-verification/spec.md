## ADDED Requirements

### Requirement: 用户程序验证（--verify）

`--verify` SHALL 读取当前程序的验证声明（`defprop` 及程序定义的模型转换），在有限深度内检查目标可达性并输出结论与反例/证据 trace；SHALL 不得忽略输入程序、不得运行与输入无关的固定演示；程序不含验证声明或属性不可验证时 SHALL 报告明确错误。

#### Scenario: 验证用户属性

- **WHEN** 文件含 `defprop` 与可达目标声明，以 `--verify <file>` 运行
- **THEN** 输出该文件的验证结论（holds/不成立）与对应 trace，结论随文件内容变化

#### Scenario: 缺少验证声明报错

- **WHEN** 文件不含任何可验证属性，以 `--verify` 运行
- **THEN** 报告「无可验证属性」错误，不打印固定演示结果

### Requirement: 协议攻击搜索由用户协议驱动

`find-attack` SHALL 接受用户声明的协议模型（参与者动作、消息、机密与攻击者能力）作为输入并执行 dolev-yao 知识合成搜索；SHALL 不得内置唯一固定协议场景；搜索深度 SHALL 由调用参数限制，结果 SHALL 为攻击存在性结论与证据。

#### Scenario: 用户协议攻击搜索

- **WHEN** 程序声明一个含泄密步骤的协议模型并调用 `find-attack`（深度受限）
- **THEN** 返回攻击成立及证据；对安全协议返回无攻击结论

### Requirement: 会话运行时语义真实化

会话语法 `send`/`recv`/`close` SHALL 直接作用于指定通道并保留消息负载；协议状态 SHALL 按通道隔离存储；类型级协议检查 SHALL 从每个通道的首个操作开始判定（首个错序操作也必须报错）；`send!`/`recv!` 通道原语 SHALL 与声明的 `defsession` 协议共享同一顺序检查。

#### Scenario: 会话负载往返

- **WHEN** 程序以 `(send c 7)` 后 `(recv c)` 执行
- **THEN** `recv` 返回 7

#### Scenario: 首操作违规

- **WHEN** 协议要求先 send 后 recv，而程序首操作为 recv，以 `--typecheck` 运行
- **THEN** 报告协议顺序违反错误

#### Scenario: 多通道状态隔离

- **WHEN** 两个通道分别执行 send/recv，其一违反协议
- **THEN** 仅违规通道报错，另一通道协议状态不受污染
