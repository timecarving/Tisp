## MODIFIED Requirements

### Requirement: 宏卫生与 gensym

`defmacro` 展开 SHALL 避免捕获用户变量(卫生展开):展开中引入的绑定(含 `let`、`fn`/lambda 参数、`if-let`/`when-let` 绑定、`match` 模式变量)SHALL 不与调用点上下文冲突;syntax-quote 的 `~x`/`~@x` SHALL 正确绑定宏参数并参与展开;`gensym` 内置 SHALL 生成每次调用唯一的符号;同一宏展开多次 SHALL 产生互不冲突的符号。

#### Scenario: 无捕获展开

- **WHEN** 宏展开引入的绑定名与调用点同名变量并存
- **THEN** 两者互不干扰,展开后程序行为与卫生语义一致

#### Scenario: fn 参数卫生

- **WHEN** 宏模板内引入 `fn`/lambda 参数且调用点存在同名自由变量
- **THEN** 模板参数被重命名,不捕获调用点变量,展开后行为正确

#### Scenario: unquote 绑定宏参数

- **WHEN** 宏模板以 syntax-quote 的 `~x` 引用宏参数并以实参调用
- **THEN** `~x` 处替换为调用点实参,展开结果正确

#### Scenario: gensym 唯一性

- **WHEN** 同一宏在程序中展开两次且使用 gensym
- **THEN** 两次生成的符号互不相同,无变量冲突

### Requirement: 泛型编译期特化

GenericDef SHALL 在 middle 层被识别并可按参数**类型** monomorphize:对构造器类型(如 `Circle`)的调用 SHALL 特化为专用方法(不再走运行时分发);多参数调用 SHALL 按参数类型组合特化;特化 SHALL 作用于 `--run` 执行路径(非仅 `--typecheck` 展示);非特化调用保持运行时分发。`--typecheck` SHALL 报告特化数量。

#### Scenario: ground 类型特化

- **WHEN** 泛型函数以 `i64` 等具体类型调用且存在匹配方法,以 `--typecheck` 运行
- **THEN** 报告特化发生,运行结果与运行时分发一致

#### Scenario: 类型驱动特化

- **WHEN** 泛型函数以构造器类型实参(如 `area(circle)`)调用且存在匹配方法,以 `--run` 执行
- **THEN** 走特化路径,运行结果与运行时分发一致

#### Scenario: 多参数特化

- **WHEN** 多分派泛型函数以具体构造器类型组合调用(如 `collide(circle, rect)`),以 `--typecheck` 运行
- **THEN** 报告该调用特化,生成对应专用方法

#### Scenario: 非特化调用回退

- **WHEN** 泛型函数以无法静态判定类型的实参调用
- **THEN** 保持运行时分发,行为正确

### Requirement: 反射函数真实化

类型反射内置(`reflect-type`/`type-of`/`effects-of`/`determinism-of`/`grade-of`/`mode-of`)SHALL 返回真实静态类型与运行环境信息(名称、定义、参数、效果、等级、模式、确定性),替换硬编码占位;`type-of` SHALL 返回静态推断类型(而非运行时值标签);`--typecheck` 通过的程序反射结果 SHALL 与静态信息一致。

#### Scenario: 反射真实类型

- **WHEN** 程序对已定义函数执行类型反射并以 `--run` 运行
- **THEN** 返回该函数的真实签名(与 `--typecheck` 输出一致)

#### Scenario: 反射效果与模式

- **WHEN** 程序对含效果/模式的函数执行 `effects-of`/`mode-of` 反射并以 `--run` 运行
- **THEN** 分别返回真实效果行与模式,而非常量 `"Pure"`/`"in"`

#### Scenario: type-of 返回静态类型

- **WHEN** 程序对某表达式执行 `type-of` 并以 `--run` 运行
- **THEN** 返回该表达式的静态推断类型(如 `i64`),而非运行时值标签

### Requirement: Monad 优化路径接线

EffectCompiler SHALL 从「检测单处理器」扩展到「编译降级」:检测 SHALL 判定单处理器且无嵌套;对可安全降级的单 handle 代码,`--typecheck` 与 `--run` 生成/执行等价的直接状态传递(monadic)路径;monadic 风格(`mlet`/`get-m`/`put-m`/`pure`)SHALL 可解析并编译为零开销链;不可降级时保持 handler 语义,行为 SHALL 与未优化一致。

#### Scenario: 单处理器降级

- **WHEN** 单处理器 handle 代码满足降级条件并以 `--run` 执行
- **THEN** 结果与 handler 语义一致,且实际走状态传递路径(非仅计数)

#### Scenario: monadic 风格编译

- **WHEN** 程序以 `mlet`/`get-m`/`put-m`/`pure` 编写状态热路径并以 `--run` 执行
- **THEN** 解析成功并执行,结果与 effect 风格等价

#### Scenario: 不可降级保持原义

- **WHEN** 嵌套/多处理器 handle 不满足降级条件
- **THEN** 按 handler 语义执行,结果正确
