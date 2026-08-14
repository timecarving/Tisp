## Purpose

定义「一切皆 ADT」的统一基底:逻辑规则、约束、项传播与 OOP 对象 SHALL 成为可绑定、传递、匹配的一等 ADT 值,在纯声明式约束下使「数据 = 程序结构」全链路落地。

## ADDED Requirements

### Requirement: 规则即数据

`defpred` 子句 SHALL 可作为一等 ADT 值:规则(头/体/演化指令)SHALL 可绑定、传递、匹配与构造;程序对规则数据的操作(增删查)SHALL 与程序文本中的定义等价,不依赖编译器内部表示。

#### Scenario: 规则值绑定与匹配

- **WHEN** 程序以数据形式构造规则(如 `(rule (member X [X|_]) ())`)并 match 其头/体
- **THEN** 匹配按结构正确分支,规则可被绑定与传递

#### Scenario: 规则数据可增删

- **WHEN** 程序把规则加入/移出规则集合并以数据形式查询
- **THEN** 增删查结果与文本定义语义一致

### Requirement: 约束与项传播即数据

`constrain` 约束与统一项 SHALL 作为 ADT 值:约束表达式 SHALL 可构造、组合、传递与模式匹配;项传播(域收缩)SHALL 作为数据变换可在纯函数中折叠应用。

#### Scenario: 约束数据构造

- **WHEN** 程序以数据形式构造约束 `(< x y)` 并施加于域变量
- **THEN** 约束作为值参与传播,解集与内联约束一致

#### Scenario: 约束组合

- **WHEN** 程序以 `foldl` 组合多个约束数据并求解
- **THEN** 组合结果等价于逐条施加,解集一致

### Requirement: OOP 对象即数据

`defclass`/`defmethod` 的对象 SHALL 作为 ADT 数据:对象 SHALL 可构造、模式匹配、序列化与传递;方法分发 SHALL 基于对象的结构(类型构造器),而非隐藏的内部标识。

#### Scenario: 对象模式匹配

- **WHEN** 程序对对象执行 match 按类型构造分支
- **THEN** 按对象类型正确分支,方法与数据一致

#### Scenario: 对象即值传递

- **WHEN** 程序把对象作为参数/返回值传递并序列化打印
- **THEN** 对象保持结构可读表示,无隐式标识泄漏

### Requirement: 纯声明式不破坏

上述 ADT 化 SHALL 不引入命令式逃逸:规则/约束/对象操作 SHALL 均为纯函数(或经既有效应门控),`--typecheck` 通过的程序 SHALL 保持引用透明。

#### Scenario: 纯函数演化

- **WHEN** 程序以纯函数(如 `foldl`)演化规则集并求值
- **THEN** 无副作用泄漏,同一输入恒得同一输出
