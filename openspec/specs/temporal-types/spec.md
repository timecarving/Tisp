# temporal-types

## Purpose

定义时序类型(§18)的端到端行为:时序模态(⃝/□_t/◇_t)的运行时语义、时态属性作为类型(LTL-as-types)与多时钟系统,替换 ClockNew 字面量占位,使「时序流 = 类型保证的因果性」可用。

## Requirements

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

### Requirement: 时序语义保证

时序模态 SHALL 有语义保证(§18.3/18.4):`□_t A`(稳定类型)SHALL 表示「A 在所有时刻可用」且可安全跨时刻;因果性 SHALL 成立(当前输出仅依赖当前与过去输入);生产率 SHALL 成立(受保护递归——每个流元素有限时间内可计算);`⃝ A` 值 SHALL 在两个时刻后安全回收(无空间泄漏)。违反生产率/因果性的定义 SHALL 为类型错误。

#### Scenario: 稳定类型跨时刻

- **WHEN** `□_t Int` 标注的值被跨时刻使用(如进入下一时刻),以 `--typecheck` 运行
- **THEN** 类型检查通过(稳定类型可安全跨时刻)

#### Scenario: 非稳定类型跨时刻报错

- **WHEN** 非稳定类型(如 `(Stream a)` 或闭包捕获时序值)被标注跨时刻使用,以 `--typecheck` 运行
- **THEN** 报告稳定类型违反错误

#### Scenario: 受保护递归生产率

- **WHEN** 流定义经受保护递归(每个 cons 尾为 `⃝` 递归)以 `--typecheck` 运行
- **THEN** 通过生产率检查;非受保护递归(无 `⃝` 保护)报告生产率错误

### Requirement: lambda 返回类型注解

`fn`/lambda 的签名 SHALL 支持与 `defn` 一致的返回类型注解语法 `(fn [params] -> Ret body...)` 与六维变体 `(fn [params] ->[ε, ρ, @r, m, d] Ret body...)`；返回注解 SHALL 参与类型检查（推断体与 Ret 统一），未匹配时报告类型错误。

#### Scenario: lambda 注解解析

- **WHEN** 程序以 `(fn [acc : Int] -> Int (+ acc 1))` 书写 lambda，以 `--typecheck` 运行
- **THEN** 解析成功且返回类型为 Int，不把 `->`/`Int` 当作函数体表达式

#### Scenario: lambda 注解类型错误

- **WHEN** lambda 标注返回 Int 但函数体返回 String，以 `--typecheck` 运行
- **THEN** 报告返回类型不匹配错误

### Requirement: FRP 源码端到端可用

FRP/时序示例 SHALL 在 lambda 返回类型注解支持下通过全链路：`examples/frp-counter.tisp` SHALL 以 `--typecheck` 通过（无未绑定 `Stream` 错误）且其定义可被 `--run` 加载；流值打印 SHALL 输出可读结构而非占位标签；流取前 n 项 SHALL 返回正确序列。

#### Scenario: frp-counter 类型检查

- **WHEN** 以 `--typecheck examples/frp-counter.tisp` 运行
- **THEN** 类型检查通过，全部定义输出类型与效应签名

#### Scenario: 流可读输出

- **WHEN** 程序以 `println` 输出流取前 n 项的结果
- **THEN** 输出包含元素序列的可读表示，非 `<Cons>`/`...` 占位

#### Scenario: 流取值正确

- **WHEN** 程序执行 `(stream-take (stream 1) 5)`
- **THEN** 得到 `[1 2 3 4 5]` 序列
