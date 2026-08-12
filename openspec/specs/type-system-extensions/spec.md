# type-system-extensions

## Purpose

补全 Tisp 类型系统深化能力(§9-14/§19):QTT 运行时语义、Mercury 多模式、类型族/关联类型、类型一等值、依赖类型等级传播、资源代数与 committed-choice 运行时行为,使「统一 def + 六维注解」的类型主线真正落地。

## Requirements

### Requirement: QTT 运行时擦除与移动

0 级(Zero)参数与绑定 SHALL 在运行时擦除(不进入闭包环境、不求值);1 级(One)值 SHALL 移动语义——使用后变量不可再引用,违反 SHALL 为编译错误。ω 级保持现状。

#### Scenario: 0 级参数擦除

- **WHEN** 函数含 0 级参数(如类型证据)且以 `--run` 执行
- **THEN** 该参数不求值、不占闭包环境,运行结果正确且无副作用泄漏

#### Scenario: 1 级值移动后复用报错

- **WHEN** 源文件在 1 级绑定使用后再次引用该绑定,以 `--typecheck` 运行
- **THEN** 报告编译错误,消息指明该值已被移动

### Requirement: Mercury 多模式谓词

defpred SHALL 支持多模式声明(如 `:mode (i, o)` 与 `(o, i)` 组合);调用点 SHALL 按实参实例化状态(free/ground)选择可用模式;无匹配模式 SHALL 为编译错误;`--typecheck` 输出各谓词模式签名。

#### Scenario: 多模式调用成功

- **WHEN** 谓词声明 `(i, o)` 与 `(o, i)` 两种模式,分别以全 ground 实参与含 free 实参调用
- **THEN** 两种调用均通过类型检查并正确执行

#### Scenario: 无匹配模式报错

- **WHEN** 调用点实参实例化状态与谓词全部声明模式均不兼容,以 `--typecheck` 运行
- **THEN** 报告模式错误,列出可用模式

### Requirement: 类型族与关联类型

编译器 SHALL 解析类型族声明与实例(`:type` 关联类型),在类型推断中简化类型族应用(依据实例归约),无法归约时保留为悬挂应用并 SHALL 报错(未定义实例)。`--desugar` 输出 SHALL 保留类型族节点。

#### Scenario: 类型族归约

- **WHEN** 定义类型族 `(typefamily Elem (List a) a)` 风格声明并使用 `Elem (List i64)` 标注
- **THEN** 类型推断将 `Elem (List i64)` 归约为 `i64`,类型检查通过

#### Scenario: 未定义实例报错

- **WHEN** 类型族应用无匹配实例且无法归约,以 `--typecheck` 运行
- **THEN** 报告类型族实例缺失错误

### Requirement: 类型一等值

`Type` SHALL 是一等运行时值(`Value::Type` 变体):运行时 SHALL 能获取表达式的类型(`reflect-type` 风格内置),类型值可绑定、传递与比较;`--typecheck` 通过的程序 SHALL 保持无运行时类型错误。

#### Scenario: 运行时类型反射

- **WHEN** 程序调用类型反射内置获取某表达式类型并以 `--run` 执行
- **THEN** 返回可打印的类型值(如 `i64`),与静态推断类型一致

### Requirement: 依赖类型等级传播

Π/Σ 类型 SHALL 携带等级维并在检查中传播(§19.1 r+s 规则):函数应用的等级约束 SHALL 参与 grade_check,违反 SHALL 为编译错误。`(pi (x : T) R)` 语法保持兼容。

#### Scenario: 等级传播通过

- **WHEN** 依赖函数应用的等级线性使用满足 r+s 规则,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 等级违规报错

- **WHEN** 依赖绑定被使用次数超过其等级允许,以 `--typecheck` 运行
- **THEN** 报告等级违反错误

### Requirement: 资源代数声明

`defresource-algebra` SHALL 解析为资源代数(单位元、二元运算、阶),`Cost` 注解 SHALL 在类型中携带代数语义;未实现的运算 SHALL 报错而非静默通过。

#### Scenario: 资源代数解析

- **WHEN** 源文件声明资源代数与 Cost 注解,以 `--desugar` 运行
- **THEN** 输出保留代数结构与 Cost 标注,无解析错误

### Requirement: committed-choice 运行时语义

CcMulti/CcNonDet 谓词 SHALL 在运行时实现承诺选择:求解到首个解后提交(cc),不再回溯重选;`--run` 行为与注解一致。

#### Scenario: 承诺选择提交

- **WHEN** cc 谓词含多个解分支且以 `--run` 执行
- **THEN** 只产出首个解并提交,不枚举其余分支
