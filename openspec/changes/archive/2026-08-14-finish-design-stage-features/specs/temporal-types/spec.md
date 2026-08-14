## Purpose

定义时序类型(§18)的端到端行为:时序模态(⃝/□_t/◇_t)的运行时语义、时态属性作为类型(LTL-as-types)与多时钟系统,替换 ClockNew 字面量占位,使「时序流 = 类型保证的因果性」可用。

## ADDED Requirements

### Requirement: 时序模态运行时语义

时序模态 SHALL 有真实运行时语义(替换 ClockNew 字面量占位):`(next A)` 值 SHALL 在下一时刻可用;`(always A)`/`(eventually A)` SHALL 有流式判定语义;`advance`/`delay` 等既有流操作 SHALL 与模态一致。`--run` 结果 SHALL 与时刻语义一致。

#### Scenario: next 值求值

- **WHEN** 程序创建 `(next A)` 值并推进一个时刻后访问,以 `--run` 执行
- **THEN** 返回 A 在下一时刻的值(与 `delay`/`advance` 一致)

#### Scenario: always/eventually 判定

- **WHEN** 程序对有限流执行 `(always P)`/`(eventually P)` 判定,以 `--run` 执行
- **THEN** 返回符合时刻语义的布尔结论

### Requirement: LTL-as-types

时态属性 SHALL 可作为类型:表达式类型携带时态保证(如 `(next T)` 返回下一时刻可用的 T);`--typecheck` SHALL 检查时态类型与流操作的匹配(如 `advance` 的输入/输出时态类型),违反 SHALL 为编译错误。

#### Scenario: 时态类型检查

- **WHEN** 程序含 `(next T)` 类型标注且流操作与时态匹配,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 时态类型违规

- **WHEN** 流操作违反时态类型(如非 next 值被当下一时刻值),以 `--typecheck` 运行
- **THEN** 报告时态类型错误

### Requirement: 多时钟

Clock 类型类 SHALL 支持多时钟:`(clock name rate)` 声明 SHALL 注册时钟;跨时钟重采样 SHALL 可执行(采样/保持);时钟不匹配的流操作 SHALL 报错。

#### Scenario: 多时钟重采样

- **WHEN** 两个不同速率时钟的流执行重采样,以 `--run` 运行
- **THEN** 返回按目标时钟速率采样的结果

#### Scenario: 时钟不匹配报错

- **WHEN** 不同时钟的流被直接混合(无重采样),以 `--typecheck` 运行
- **THEN** 报告时钟不匹配错误
