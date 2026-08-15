## ADDED Requirements

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
