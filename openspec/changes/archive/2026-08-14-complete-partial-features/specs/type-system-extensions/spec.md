## MODIFIED Requirements

### Requirement: 依赖类型等级传播

Π/Σ 类型 SHALL 携带等级维并在检查中传播(§19.1 r+s 规则):函数应用的等级约束 SHALL 参与 grade_check,违反 SHALL 为编译错误。数字等级与可折叠复合等级 SHALL 常量检查;符号等级(如 `(n x : T)`)的使用次数 SHALL 记录诊断信息(自由等级变量无法静态判定,`--typecheck` 输出含使用次数的警告);等级变量有具体常量值(经实例化传播)时 SHALL 严格检查。`(pi (x : T) R)` 语法保持兼容。

#### Scenario: 等级传播通过

- **WHEN** 依赖函数应用的等级线性使用满足 r+s 规则,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 等级违规报错

- **WHEN** 依赖绑定被使用次数超过其等级允许,以 `--typecheck` 运行
- **THEN** 报告等级违反错误

#### Scenario: 符号等级诊断警告

- **WHEN** `(n x : T)` 的 x 在函数体内使用 count 次(等级变量 n 自由),以 `--typecheck` 运行
- **THEN** 类型检查通过,输出含使用次数的诊断警告(自由等级变量不可静态判定,不误报违反)

#### Scenario: 常量等级严格检查

- **WHEN** 复合等级经常量折叠可判定(如 `((+ n 1) x : T)` 中 n 实例化为常量)且 count 超过等级,以 `--typecheck` 运行
- **THEN** 报告等级违反错误(常量可判定路径严格)

### Requirement: 类型一等值

`Type` SHALL 是一等运行时值(**`Value::Type` 变体**):运行时 SHALL 能获取表达式的类型(`reflect-type` 风格内置),类型值 SHALL 可绑定、传递、比较与匹配;`--typecheck` 通过的程序 SHALL 保持无运行时类型错误。

#### Scenario: 运行时类型反射

- **WHEN** 程序调用类型反射内置获取某表达式类型并以 `--run` 执行
- **THEN** 返回类型值(非字符串),与静态推断类型一致

#### Scenario: 类型值传递与比较

- **WHEN** 类型值绑定到变量、作为参数传递并与其他类型值比较,以 `--run` 执行
- **THEN** 类型值保持相等性语义(相同类型相等、不同类型不等)

### Requirement: 类型族与关联类型

编译器 SHALL 解析类型族声明与实例(`:type` 关联类型),在类型推断中简化类型族应用(依据实例归约),**支持多模式实例与 rewrite 简化规则**(单模式匹配归约已实现);无法归约时保留为悬挂应用并 SHALL 报错(未定义实例)。`--desugar` 输出 SHALL 保留类型族节点。

#### Scenario: 类型族归约

- **WHEN** 定义类型族 `(typefamily Elem (List a) a)` 风格声明并使用 `Elem (List i64)` 标注
- **THEN** 类型推断将 `Elem (List i64)` 归约为 `i64`,类型检查通过

#### Scenario: 多模式实例归约

- **WHEN** 类型族声明多个实例模式(如 `(typefamily Len (List a) i64)` 与 `(typefamily Len (Vec a n) i64)` 风格)且应用匹配第二个模式
- **THEN** 按匹配模式归约,类型检查通过

#### Scenario: 未定义实例报错

- **WHEN** 类型族应用无匹配实例且无法归约,以 `--typecheck` 运行
- **THEN** 报告类型族实例缺失错误

### Requirement: Mercury 多模式谓词

defpred SHALL 支持多模式声明(如 `:mode (i, o)` 与 `(o, i)` 组合);调用点 SHALL 按实参实例化状态(free/ground)选择可用模式;无匹配模式 SHALL 为编译错误;**未声明 `:mode` 的谓词 SHALL 自动推断其模式**(多模式自动推断,替换「仅显式签名」);`--typecheck` 输出各谓词模式签名。

#### Scenario: 多模式调用成功

- **WHEN** 谓词声明 `(i, o)` 与 `(o, i)` 两种模式,分别以全 ground 实参与含 free 实参调用
- **THEN** 两种调用均通过类型检查并正确执行

#### Scenario: 无匹配模式报错

- **WHEN** 调用点实参实例化状态与谓词全部声明模式均不兼容,以 `--typecheck` 运行
- **THEN** 报告模式错误,列出可用模式

#### Scenario: 模式自动推断

- **WHEN** 未声明 `:mode` 的谓词被以 ground 与 free 实参分别调用,以 `--typecheck` 运行
- **THEN** 自动推断模式并接受合法调用(不要求显式声明)

## ADDED Requirements

### Requirement: 隐式绑定默认 0 级

§10.2 隐式绑定(未显式标注等级的绑定)SHALL 默认 0 级:运行时擦除,不参与求值;若隐式绑定被运行时引用,SHALL 报编译错误(擦除语义)。**0 级语法 `{0 x : T}` 的擦除与引用报错 SHALL 完整可用**(实施修正:无等级标注的 Map 参数存在语法歧义(无法区分等级符号与绑定名),自动默认 0 不落地,记录为已知限制);显式标注等级(0/1/ω/表达式)不受影响。

#### Scenario: 0 级绑定擦除

- **WHEN** `{0 x : T}` 绑定未被运行时引用,以 `--run` 执行
- **THEN** 正常执行,绑定不占运行环境

#### Scenario: 0 级绑定运行时引用报错

- **WHEN** `{0 x : T}` 绑定被表达式引用(运行时需要其值),以 `--typecheck` 运行
- **THEN** 报告擦除违反错误
