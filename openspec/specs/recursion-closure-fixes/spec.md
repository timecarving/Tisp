# recursion-closure-fixes

## Purpose

消除递归与闭包的类型检查缺陷:前向引用、相互递归、let 内递归、递归返回闭包 SHALL 通过类型检查,且 `--typecheck` 与 `--run` 对同一程序的接受行为 SHALL 一致(定义顺序无关)。

## Requirements

### Requirement: 前向引用与相互递归

编译器 SHALL 支持函数使用在前、定义在后(前向引用);相互递归的函数 SHALL 通过类型检查。定义顺序 SHALL 不影响 `--typecheck` 结果。

#### Scenario: 前向引用

- **WHEN** `(defn main [] (foo 1))` 在前、`(defn foo [x] (+ x 1))` 在后,以 `--typecheck` 运行
- **THEN** 类型检查通过(不再报 unbound variable)

#### Scenario: 相互递归

- **WHEN** `is-even` 与 `is-odd` 互相调用(定义顺序任意),以 `--typecheck` 运行
- **THEN** 类型检查通过,`--run` 结果正确

### Requirement: let 内递归

`(let [f (fn ... (f ...))] ...)` 局部递归绑定 SHALL 通过类型检查:f 在自身值推断期间可见,推断结果 SHALL 与递归使用一致。

#### Scenario: 局部递归函数

- **WHEN** `(let [fact (fn [n] (if (= n 0) 1 (* n (fact (- n 1)))))] (fact 5))` 以 `--typecheck` 运行
- **THEN** 类型检查通过(不再报 unbound variable: fact),`--run` 输出 120

### Requirement: 递归返回闭包

有限类型的递归定义返回闭包(如 `i64 -> (i64 -> i64)`)SHALL 通过类型推导;无限类型(自引用返回,如 `T = Unit -> T`)SHALL 被类型检查拒绝(occurs check 正确行为)。

#### Scenario: 有限类型递归返回闭包

- **WHEN** `(defn make-adder-n [n] (if (= n 0) (fn [x] x) (fn [x] ((make-adder-n (- n 1)) (+ x 1)))))` 以 `--typecheck` 运行
- **THEN** 类型检查通过,`--run` 调用结果正确

#### Scenario: 无限类型被拒绝

- **WHEN** 自引用返回闭包(如 `(fn [] (make-countdown (- n 1)))` 且 base 返回 `(fn [] 0)`)以 `--typecheck` 运行
- **THEN** 报告类型错误(无限类型,不 panic、不误放行)

### Requirement: 行为一致性

`--typecheck` 拒绝的程序 SHALL 在 `--run` 中也报错(或同为运行时错误);`--typecheck` 接受的程序 SHALL 在 `--run` 中按语义执行。定义顺序 SHALL 不再造成两阶段行为差异。

#### Scenario: 定义顺序一致性

- **WHEN** 同一程序(使用在前、定义在后)分别以 `--typecheck` 与 `--run` 运行
- **THEN** 两阶段均接受,`--run` 结果正确

#### Scenario: 真实类型错误仍被拒绝

- **WHEN** 程序含真实类型错误(如把 i64 当函数调用)且以 `--typecheck` 运行
- **THEN** 仍报告类型错误(修复不引入误放行)
