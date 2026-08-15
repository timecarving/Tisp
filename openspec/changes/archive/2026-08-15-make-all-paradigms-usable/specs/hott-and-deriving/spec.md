## ADDED Requirements

### Requirement: HoTT 运行时无 panic 且语义显式

HoTT 运行时原语 SHALL 对所有可达输入返回结果或显式错误，不得 panic：命题截断（squash）的消去在不可提取情形 SHALL 返回明确错误值/错误诊断；等价（Equiv）的 section/retraction SHALL 由构造参数提供并校验，未提供或与 fwd/bwd 不一致时 SHALL 报错，不得静默填入 refl；HComp/Transp/立方填充的边界不一致 SHALL 报告可读错误。

#### Scenario: 非法消去显式报错

- **WHEN** 程序对 squash 值执行不可提取的消去，以 `--run` 执行
- **THEN** 报告明确错误，进程不 panic

#### Scenario: 等价见证校验

- **WHEN** 构造 Equiv 时提供的 section/retraction 与 forward/backward 不一致
- **THEN** 报告等价见证错误，不静默接受

#### Scenario: 边界错误可读

- **WHEN** hcomp 输入边界不一致，以 `--run` 执行
- **THEN** 返回/报告包含边界信息的错误，不 panic

### Requirement: 演算编码端到端可执行

演算互编码（π→SKI、async→π、applied→π、ρ→π、ambient→通道）SHALL 从 Tisp 源码可调用并返回可执行结果；编码后的结果 SHALL 可继续在解释器中执行；编码失败（无法编码的构造）SHALL 报告错误。

#### Scenario: 编码结果可执行

- **WHEN** 程序对声明的进程执行互编码并运行编码结果
- **THEN** 输出与源进程一致的行为，或对不可编码构造报告明确错误

#### Scenario: 观察等价结论

- **WHEN** 对源进程与编码结果执行观察等价检查
- **THEN** 返回等价结论或给出区分轨迹，不返回固定演示结论
