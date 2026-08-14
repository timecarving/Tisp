## ADDED Requirements

### Requirement: 尾调用优化

解释器 SHALL 对尾调用做消除(TCO):尾位置的递归调用 SHALL 复用当前栈帧(不随递归深度增长),深递归(如 `(sum-to 100000)`)SHALL 不栈溢出;尾位置之外的递归 SHALL 保持原语义(仍可能受限)。`--run` 结果 SHALL 与未优化一致。

#### Scenario: 尾递归不溢出

- **WHEN** 程序以尾递归实现 `(sum-to 100000)` 并以 `--run` 执行
- **THEN** 返回正确结果,不栈溢出

#### Scenario: 非尾递归语义不变

- **WHEN** 非尾位置递归程序以 `--run` 执行
- **THEN** 结果与优化前一致(行为不因 TCO 改变)

### Requirement: 多顶层表达式递归

含多个顶层表达式且顶层调用递归函数的程序 SHALL 正常执行(修复 `__top__` 的 `Do` 包装栈溢出):多个 `(println ...)` 等顶层表达式与递归调用并存时,`--run` SHALL 不栈溢出且依次求值。

#### Scenario: 多顶层表达式 + 递归

- **WHEN** 程序含两个以上顶层表达式且其中调用递归函数(如 fib),以 `--run` 执行
- **THEN** 全部顶层表达式依次求值,递归结果正确,不栈溢出
