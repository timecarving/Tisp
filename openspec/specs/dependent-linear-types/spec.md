# dependent-linear-types

## Purpose

定义依赖线性类型(§10 QTT 的推广)的端到端行为:等级可由编译期数值表达式(如向量长度 n)决定,资源使用次数受等级表达式约束,使「线性使用次数 = 数据结构规模」的依赖资源语义可用。

## Requirements

### Requirement: 依赖等级语法

graded 参数 SHALL 支持等级表达式:数字 `(5 x : a)` 解析为常量等级、符号 `(n x : a)` 解析为等级变量、复合表达式 `((+ n 1) x : a)` 解析为等级运算(Add/Mul)。现有 `{0 x : a}`/`{1 x : a}`/`{ω x : a}` 固定等级语法 SHALL 保持兼容;`--desugar` 输出 SHALL 保留等级表达式结构。

#### Scenario: 符号等级解析

- **WHEN** 源文件含 `(defn f [n (n x : i64)] ...)` 且以 `--desugar` 运行
- **THEN** 参数 x 的等级为 `Grade::Var(n)`,无解析错误

#### Scenario: 复合等级解析

- **WHEN** 源文件含 `(defn g [n ((+ n 1) x : i64)] ...)` 且以 `--desugar` 运行
- **THEN** 参数 x 的等级为 `Grade::Add(Var(n), Nat(1))`,无解析错误

### Requirement: 等级变量绑定

等级变量 SHALL 从依赖类型参数解析(如 `(Vec i64 n)` 的 n);未绑定等级变量 SHALL 为编译错误,错误消息指明该等级变量未绑定。

#### Scenario: 等级变量来自类型参数

- **WHEN** 函数签名含类型参数 n(经 `(Vec i64 n)` 类类型出现)且参数标注 `(n x : T)`,以 `--typecheck` 运行
- **THEN** 类型检查通过,等级变量 n 正确绑定

#### Scenario: 未绑定等级变量报错

- **WHEN** 参数标注 `(m x : T)` 而 m 未在任何类型参数中出现,以 `--typecheck` 运行
- **THEN** 报告未绑定等级变量错误

### Requirement: 使用计数受等级约束

等级表达式等级(非 ω)的绑定 SHALL 在使用计数 ≤ 等级表达式时通过检查(上界语义);数字等级按常量折叠判定,符号等级在可常量判定时检查、不可判定时警告放行。违反 SHALL 为编译错误并带 span。

#### Scenario: 数字等级满足

- **WHEN** `(5 x : i64)` 的 x 在函数体内使用 3 次,以 `--typecheck` 运行
- **THEN** 检查通过(3 ≤ 5)

#### Scenario: 数字等级违反

- **WHEN** `(3 x : i64)` 的 x 在函数体内使用 4 次,以 `--typecheck` 运行
- **THEN** 报告等级违反错误,消息含使用次数与等级

#### Scenario: 符号等级常量判定

- **WHEN** `(n x : i64)` 的 x 使用次数与 n 的常量值可比较(如 n 来自类型级常量),以 `--typecheck` 运行
- **THEN** 按 n 的常量值判定通过或报错

### Requirement: 分支合并

if/match 分支对依赖等级变量的使用计数 SHALL 取合并(各分支计数上界),任一分支超限 SHALL 报错;分支合并 SHALL 不破坏现有 0/1/ω 线性检查。

#### Scenario: 分支计数上界

- **WHEN** 依赖等级绑定在 if 的 then 分支使用 2 次、else 分支使用 1 次,等级为 `(3 x : T)`
- **THEN** 检查通过(上界 2 ≤ 3)

#### Scenario: 分支超限报错

- **WHEN** 依赖等级绑定在 then 分支使用 4 次(等级 3),以 `--typecheck` 运行
- **THEN** 报告等级违反错误

### Requirement: 固定等级兼容

0/1/ω 固定等级 SHALL 保持现有语义:0 级运行时擦除、1 级恰好一次、ω 不限制;引入依赖等级 SHALL 不改变既有程序行为。

#### Scenario: 既有程序无回归

- **WHEN** 含 `{0 x}`/`{1 x}`/`{ω x}` 的既有程序以 `--typecheck` 与 `--run` 运行
- **THEN** 行为与变更前一致(擦除/移动/无限制)
