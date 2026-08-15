# aspect-oriented-programming

## Purpose

定义基于编译器纯声明式 MOP 的面向切面编程(AOP):切面/切入点/建议的声明与编织,辅助 OOP 方法组合,直到端到端可用。

## Requirements

### Requirement: 切面定义

程序 SHALL 支持切面定义:`aspect` 声明切面、`pointcut` 声明切入点(按名/注解匹配)、`advice` 声明建议(before/after/around);切面 SHALL 作用于 OOP 方法(泛型函数/类型类方法)。

#### Scenario: 切面声明与匹配

- **WHEN** 声明 aspect/pointcut/advice 并指向某方法
- **THEN** 切入点匹配该方法,建议在方法调用时生效

### Requirement: AOP 编织

建议 SHALL 编织:before 在方法前执行、after 在方法后执行、around 包裹方法体;编织结果 SHALL 与直接方法组合(`:before/:after/:around` + `call-next-method`)语义一致。

#### Scenario: around 编织

- **WHEN** around 建议包裹方法并调用
- **THEN** 建议包裹方法体,结果与 `:around` 方法组合等价

#### Scenario: before/after 编织顺序

- **WHEN** before 与 after 建议作用于同一方法并调用
- **THEN** 按 before → 方法体 → after 顺序执行

### Requirement: 编译器纯声明式 MOP 驱动

AOP 编织 SHALL 由编译器纯声明式 MOP 驱动:切面 SHALL 在编译期经 MOP(反射 + 元对象)解析并编织,不依赖运行时反射;编织 SHALL 为纯函数变换(程序 → 程序)。

#### Scenario: 编译期编织

- **WHEN** 切面作用于编译期可见的方法
- **THEN** 编译期即完成编织,运行时无动态反射

### Requirement: comptime 纯声明式 MOP 编织

AOP SHALL 由 `comptime` + 纯声明式 MOP 驱动，在编译期完成编织：切面 SHALL 可声明 pointcut（作用于哪些泛型方法）与 around/before/after advice；编织 SHALL 发生在 desugar/特化阶段并写入 Core AST，`--desugar` 输出 SHALL 可见编织后的方法链；`call-next-method` SHALL 继续指向内层方法链。编织失败 SHALL 报告编译错误，运行时不得动态反射。

#### Scenario: 编译期编织可见

- **WHEN** 声明作用于某泛型方法的 around 切面并以 `--desugar` 运行
- **THEN** 输出包含编织后的方法体（切面代码在编译期已进入方法链）

#### Scenario: call-next-method 链保持

- **WHEN** 多个 around/before/after 切面作用于同一方法并调用 `call-next-method`，以 `--run` 运行
- **THEN** 执行顺序与语义为 around(注册序)→before→primary→after，结果与未编织的运行时分发语义等价

### Requirement: AOP 辅助 OOP 语义保持

AOP 编织 SHALL 保持 OOP 语义：before/after SHALL 不改变 primary 返回值；around SHALL 可包装并修改结果；泛型特化（§22.4）SHALL 与编织结果等价；未命中 pointcut 的方法 SHALL 完全不受影响；切面引用未定义方法/不合法 pointcut SHALL 显式报错。

#### Scenario: 结果语义

- **WHEN** 声明 around 切面将方法结果翻倍并调用该方法，以 `--run` 运行
- **THEN** 返回翻倍结果，before/after 切面不影响该结果

#### Scenario: 未命中方法不受影响

- **WHEN** pointcut 未命中的泛型方法被调用
- **THEN** 行为与无切面时完全一致

#### Scenario: 非法 pointcut 报错

- **WHEN** 切面 pointcut 指向未定义方法或不合法模式，以 `--typecheck`/`--desugar` 运行
- **THEN** 报告明确编译错误
