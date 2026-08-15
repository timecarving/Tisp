## ADDED Requirements

### Requirement: 推断签名回填反射

类型推断完成后的签名 SHALL 回填到运行时反射表：对未显式标注类型/效果/等级/模式/确定性的定义，`type-of`/`effects-of`/`grade-of`/`mode-of`/`determinism-of` SHALL 返回推断结果而非运行时标签或默认常量；对显式标注的定义 SHALL 返回标注与检查后的最终类型。

#### Scenario: 未标注定义反射推断类型

- **WHEN** 程序定义 `(defn add [x y] (+ x y))` 并在 `--run` 中执行 `(type-of "add")`
- **THEN** 返回 `add` 的推断类型（与 `--typecheck` 输出一致，如 `i64 -> i64 -> i64`），而非 `String`

#### Scenario: 值表达式反射

- **WHEN** 程序对值表达式执行 `type-of`（如 `(type-of 42)`）
- **THEN** 返回该表达式的静态类型 `i64`（或语言规定的等价表示），而非运行时值标签

### Requirement: 依赖类型运行时语义显式

Π/Σ 类型在运行时 SHALL 有明确定义的求值语义（依赖参数按编译期/擦除规则处理，返回体按作用域求值）；不支持的依赖运行时形态 SHALL 报告明确错误，不得静默求值为函数体或占位值。

#### Scenario: Pi 值求值

- **WHEN** 程序构造并应用 Π 类型值且其运行时形态受支持，以 `--run` 执行
- **THEN** 按声明语义返回结果

#### Scenario: 不支持形态显式报错

- **WHEN** 程序使用运行时不可表示的依赖形态
- **THEN** 报告明确的「不支持」错误，不静默返回部分值
