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
