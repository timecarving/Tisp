# liquid-types

## Purpose

定义 Tisp 液态类型(§15)的端到端行为:精化类型 `{x : T | pred}` 与函数契约(`:requires`/`:ensures`)的解析、编译期验证与诊断,使「通过类型检查即保证无运行时类型错误」的承诺覆盖精化维度。

## Requirements

### Requirement: 精化类型语法与表示

编译器 SHALL 解析精化类型语法 `{x : T | pred}`(x 为绑定变量,T 为基础类型,pred 为引用 x 的谓词),并表示为可在类型显示、合一与替换中正确传播的类型。`--desugar` 输出的 Core AST 中 SHALL 保留精化类型,不得丢弃谓词。

#### Scenario: 解析精化类型

- **WHEN** 源文件包含 `(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 ...)` 且以 `--desugar` 运行
- **THEN** 输出的类型注解包含 `Refined` 节点与谓词 `(>= n 0)`,不报解析错误

#### Scenario: 类型显示保留谓词

- **WHEN** 精化类型出现在 `--typecheck` 或 REPL `:type` 的类型输出中
- **THEN** 输出以 `{x : T | pred}` 形式包含完整谓词文本

### Requirement: 函数契约声明

`defn` SHALL 接受 `:requires <pred>` 与 `:ensures <pred>` 注解,多个 `:requires` 语义上合取;`result` 在 `:ensures` 中 SHALL 绑定为函数返回值。契约 SHALL 随定义持久化(Core AST 保留),`--desugar` 输出中可见。

#### Scenario: 解析多个契约

- **WHEN** 源文件为 `(defn safe [a b] :requires (>= a b) :requires (> b 0) :ensures (> result 0) (+ a b))`
- **THEN** `--desugar` 输出中该定义带两个 requires 合取与一条 ensures,无解析错误

### Requirement: 精化类型边界验证

编译器 SHALL 在 `--typecheck` 阶段验证:精化参数的实际调用实参满足谓词;函数体所有返回路径满足返回精化类型。违反 SHALL 产生编译错误并附源位置与反例(若可求解)。对无法静态判定(未知谓词函数、超出现有理论)的情况,编译器 SHALL 发出警告并放行,而非静默通过。

#### Scenario: 调用违反参数精化

- **WHEN** 存在 `(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 x)` 且源文件调用 `(sqrt -1)` 并以 `--typecheck` 运行
- **THEN** 报告精化类型违反错误,消息包含谓词 `(>= n 0)` 与反例(如 `x = -1`),退出码非零

#### Scenario: 合法调用通过

- **WHEN** 同一函数被 `(sqrt 9)` 调用并以 `--typecheck` 运行
- **THEN** 不报告精化错误,输出验证通过信息

#### Scenario: 返回精化类型违反

- **WHEN** 存在 `(defn abs [x : i64] -> {n : i64 | (>= n 0)} x)`(直接返回参数,参数可为负)并以 `--typecheck` 运行
- **THEN** 报告返回值不满足精化类型的错误,并给出反例

#### Scenario: 返回精化类型满足

- **WHEN** 存在 `(defn abs [x : i64] -> {n : i64 | (>= n 0)} (if (>= x 0) x (- x)))` 并以 `--typecheck` 运行
- **THEN** 不报告精化错误(两分支均验证通过)

#### Scenario: 未知谓词函数放行并警告

- **WHEN** 精化谓词引用非内置谓词函数且无法静态判定,以 `--typecheck` 运行
- **THEN** 不产生错误,产生一条警告说明该谓词未验证

### Requirement: 契约验证

编译器 SHALL 在 `--typecheck` 阶段验证契约:调用点满足 `:requires`(否则报告违反,含反例);对 `:ensures`,验证函数体满足 `requires ⇒ ensures` 蕴含。无 z3 求解器时按降级规则处理(见下条)。

#### Scenario: 违反 requires

- **WHEN** 存在 `(defn divide [n d] :requires (!= d 0) n)` 且源文件调用 `(divide 1 0)` 并以 `--typecheck` 运行
- **THEN** 报告契约违反错误,消息包含谓词 `(!= d 0)` 与反例 `d = 0`

#### Scenario: 违反 ensures

- **WHEN** 存在 `(defn add-pos [x y] :ensures (> result 0) (+ x y))` 且存在调用 `(add-pos -5 -3)`(或常量折叠可判定),以 `--typecheck` 运行
- **THEN** 报告 ensures 违反错误,消息包含反例

#### Scenario: 契约满足

- **WHEN** 存在 `(defn add-pos [x y] :requires (> x 0) :requires (> y 0) :ensures (> result 0) (+ x y))` 且所有调用实参为正,以 `--typecheck` 运行
- **THEN** 不报告契约错误

### Requirement: 求解降级与诊断

液态验证 SHALL 对求解器可用性自适应:找不到 `z3` 可执行文件时,验证 SHALL 降级为仅常量折叠检查(不产生错误),并在 `--typecheck` 输出中提示求解器不可用。验证过程 SHALL 有可见的统计输出(验证项数、违反数、降级/跳过数)。

#### Scenario: 无求解器时降级

- **WHEN** 系统 PATH 中无 `z3`,`--typecheck` 运行含精化类型的文件
- **THEN** 不因缺求解器产生错误,输出包含求解器不可用提示与降级说明

#### Scenario: 验证统计输出

- **WHEN** `--typecheck` 运行含精化类型与契约的文件
- **THEN** 输出包含验证项数与违反计数(违反数可为 0)
