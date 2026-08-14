## ADDED Requirements

### Requirement: 类型 λ(tlambda)

`A => B`(tlambda)与 `=> B`(无输入 tlambda)类型字面量 SHALL 有语义:tlambda SHALL 作为编译期变量/参数参与类型推导(静态语义,通过 `[]` 与类型系统通信);tlambda 值 SHALL 可在编译期绑定、传递与匹配,运行时 SHALL 不产生动态变量。

#### Scenario: tlambda 类型字面量解析

- **WHEN** 程序以 `A => B` 标注 tlambda 类型,以 `--typecheck` 运行
- **THEN** 类型解析成功,tlambda 进入类型

#### Scenario: tlambda 作编译期参数

- **WHEN** 多态定义以 tlambda 作为参数(如 `['a where AnyType]` 形式)并经 `[]` 应用类型实参
- **THEN** 类型实参经编译期绑定匹配,不产生运行时动态变量

#### Scenario: 无输入 tlambda

- **WHEN** 程序以 `=> B` 标注无输入 tlambda,以 `--typecheck` 运行
- **THEN** 解析为「直接产出类型 B」的 tlambda

### Requirement: 多态类型(defpoly + where)

`(defpoly Name [params where 约束...] body)` SHALL 定义带约束的多态类型:`where` 约束(如 `Number`、`BiggerThan[60]`)SHALL 参与编译期检查;应用 `[]` 提供类型实参 SHALL 按参数序匹配;不满足约束 SHALL 为编译错误。

#### Scenario: defpoly 定义与匹配

- **WHEN** `(defpoly Demo ['a 'b 'c where Number] ...)` 并以 `Demo[i64 f64 String]` 应用类型实参
- **THEN** 类型实参按参数序匹配,where 约束参与检查

#### Scenario: 约束违反报错

- **WHEN** 多态类型应用的类型实参不满足 `where` 约束(如 `BiggerThan[60]` 传 30)
- **THEN** 报告约束违反错误

### Requirement: conj/disj 类型字面量

`(conj A B)` SHALL 为乘积类型(等价现有 Tuple/Record 的别名);`(disj A B)` SHALL 为和类型,等价 `defdata` 的多构造器 ADT 形式(语法糖);类型字面量 `()` SHALL 为 `Unit`、`(A B C)` SHALL 为 Tuple、`A -> B` SHALL 为 lambda。

#### Scenario: conj 乘积类型

- **WHEN** 程序以 `(conj I32 F32)` 标注类型,以 `--typecheck` 运行
- **THEN** 解析为乘积类型(与 Tuple 等价)

#### Scenario: disj 和类型糖

- **WHEN** 程序以 `(disj A B)` 标注和类型,以 `--desugar` 运行
- **THEN** 脱糖为 `defdata` 多构造器 ADT 形式(构造器 A/B)

#### Scenario: 类型字面量补齐

- **WHEN** 程序使用 `()`(Unit)、`(A B C)`(Tuple)、`A -> B`(lambda)标注
- **THEN** 分别解析为对应类型,与既有语义一致

### Requirement: trait 语法糖

`deftrait`/`defabsmember`/`defmember`/`polytrait`/`(with ...)`/`(with-static ...)`/`(with-cons ...)` SHALL 为类型类系统的等价语法糖:`deftrait` SHALL 等价 `defclass`、`defabsmember` SHALL 等价抽象方法声明、`polytrait` SHALL 等价带类型参数的 `defclass`;`with` 系列 SHALL 等价实例方法绑定;行为 SHALL 与 `defclass`/`definstance` 一致。

#### Scenario: deftrait 等价 defclass

- **WHEN** 程序以 `deftrait` 声明 trait 并以 `--desugar` 运行
- **THEN** 脱糖为 `defclass`,实例查找行为一致

#### Scenario: polytrait 带参

- **WHEN** 程序以 `(polytrait ['a 'b] ...)` 声明多态 trait,应用 `[]` 提供类型实参
- **THEN** 等价带类型参数的 `defclass`,按实参匹配实例

#### Scenario: with 成员绑定

- **WHEN** 类型定义含 `(with Traits ...)`/`(with-static ...)` 成员绑定
- **THEN** 等价 `definstance` 方法绑定,运行时按类型分发
