## ADDED Requirements

### Requirement: def 六维注解

`def`/`defn` 签名 SHALL 支持统一六维注解 `->[ε, ρ, @r, m, d] Ret`(ε=效果行、ρ=区域、`@r`=等级、m=模式、d=确定性):各维度 SHALL 从源码解析并写入定义的函数注解(而非硬编码默认值);未标注的维度 SHALL 取默认值(纯效果、ω 等级、in 模式、det 确定性)。`--desugar` 输出 SHALL 保留已标注维度。

#### Scenario: 六维注解解析

- **WHEN** 函数以 `->[IO, ρ_heap, @1, out, nondet] Bool` 风格签名声明,以 `--desugar` 运行
- **THEN** 输出保留效果/区域/等级/模式/确定性标注,无解析错误

#### Scenario: 未标注维度取默认

- **WHEN** 函数签名仅标注返回类型 `-> Bool`(无六维),以 `--typecheck` 运行
- **THEN** 未标注维度取默认值,类型检查通过且行为与现状一致

### Requirement: QTT 隐式绑定默认等级

未显式标注等级的绑定 SHALL 默认取 0 级(擦除):`{n : T}` 形式(隐式绑定)SHALL 解析为 0 级绑定并在运行时擦除;显式标注(`{0 n : T}`/`{1 n : T}`/`{ω n : T}`)SHALL 保持原语义。

#### Scenario: 隐式绑定擦除

- **WHEN** 函数参数以 `{n : Nat}` 隐式形式声明(未标等级)并以 `--run` 执行
- **THEN** 该参数按 0 级处理(不求值、不占闭包环境),运行正确

#### Scenario: 显式等级不受影响

- **WHEN** 参数显式标注 `{1 x : T}` 且使用后再次引用,以 `--typecheck` 运行
- **THEN** 报告线性移动违反错误,与现状一致

### Requirement: 分级模态 □_r

`□_r`(分级必然)与 `@[r]` 分级应用 SHALL 有语法与推断:声明资源代数后,`(□_r A)` 类型与 `f @[r]` 分级应用 SHALL 可解析;分级应用 SHALL 参与等级检查(使用次数 ≤ r),违反 SHALL 为编译错误。

#### Scenario: 分级必然类型解析

- **WHEN** 源文件以 `(□_n a)` 形式标注分级必然类型并以 `--typecheck` 运行
- **THEN** 类型解析成功,分级信息进入类型

#### Scenario: 分级应用检查

- **WHEN** `f @[n]` 分级应用的使用次数超过等级 n,以 `--typecheck` 运行
- **THEN** 报告等级违反错误

## MODIFIED Requirements

### Requirement: Mercury 多模式谓词

defpred SHALL 支持多模式声明(如 `:mode (i, o)` 与 `(o, i)` 组合)与内联参数模式(`[List a :in, List a :out]`);调用点 SHALL 按实参实例化状态(free/ground)选择可用模式;未声明模式的谓词 SHALL 由函数体自动推断其模式;同名谓词 SHALL 支持以不同模式重复声明(多模式重载);无匹配模式 SHALL 为编译错误;`--typecheck` 输出各谓词模式签名。

#### Scenario: 多模式调用成功

- **WHEN** 谓词声明 `(i, o)` 与 `(o, i)` 两种模式,分别以全 ground 实参与含 free 实参调用
- **THEN** 两种调用均通过类型检查并正确执行

#### Scenario: 内联参数模式生效

- **WHEN** 谓词以 `[List a :in, List a :out]` 内联模式声明并以不匹配方向调用,以 `--typecheck` 运行
- **THEN** 报告模式错误,列出可用模式

#### Scenario: 自动模式推断

- **WHEN** 谓词未声明 `:mode`,以 ground 实参调用,以 `--typecheck` 运行
- **THEN** 由函数体推断出可用模式并据此检查调用点,输出推断的模式签名

#### Scenario: 同名多模式重载

- **WHEN** 同一谓词名以 `:nondet` 与 `:semidet` 两种模式声明并以相应方向调用
- **THEN** 按调用方向选择匹配声明,两者互不覆盖

#### Scenario: 无匹配模式报错

- **WHEN** 调用点实参实例化状态与谓词全部声明模式均不兼容,以 `--typecheck` 运行
- **THEN** 报告模式错误,列出可用模式

### Requirement: 类型族与关联类型

编译器 SHALL 解析类型族声明与实例(`:type` 关联类型),单一声明 SHALL 支持多个模式实例;在类型推断中简化类型族应用(依据实例归约,命中任一模式即归约并递归简化结果);SHALL 支持 `rewrite` 规则(实例间简化重写);无法归约时保留为悬挂应用并 SHALL 报错(未定义实例);`--desugar` 输出 SHALL 保留类型族节点。

#### Scenario: 类型族归约

- **WHEN** 定义类型族 `(typefamily Elem (List a) a)` 风格声明并使用 `Elem (List i64)` 标注
- **THEN** 类型推断将 `Elem (List i64)` 归约为 `i64`,类型检查通过

#### Scenario: 多模式实例匹配

- **WHEN** 类型族以多个模式实例声明(如 `(List a)` 与 `(Pair a b)`),应用 `(Pair i64 String)` 标注
- **THEN** 匹配第二个实例并归约,类型检查通过

#### Scenario: rewrite 规则归约

- **WHEN** 类型族声明 `rewrite` 简化规则且应用匹配该规则
- **THEN** 应用按 rewrite 规则归约到目标类型

#### Scenario: 未定义实例报错

- **WHEN** 类型族应用无匹配实例且无法归约,以 `--typecheck` 运行
- **THEN** 报告类型族实例缺失错误(而非误导性 unify 错误)

### Requirement: 类型一等值

`Type` SHALL 是一等运行时值(`Value::Type` 变体):运行时 SHALL 能获取表达式的类型(`reflect-type` 风格内置),类型值可绑定、传递、比较、打印与模式匹配;除类型外,效果行、等级、模式、确定性 SHALL 亦各有对应运行时值(`reflect-type`/`effects-of`/`grade-of`/`mode-of`/`determinism-of`);`--typecheck` 通过的程序 SHALL 保持无运行时类型错误。

#### Scenario: 运行时类型反射

- **WHEN** 程序调用类型反射内置获取某表达式类型并以 `--run` 执行
- **THEN** 返回可打印的类型值(如 `i64`),与静态推断类型一致

#### Scenario: 类型值打印

- **WHEN** 程序以 `println` 打印一个类型值并以 `--run` 执行
- **THEN** 输出该类型的可读表示(而非占位 `...`)

#### Scenario: 类型值模式匹配

- **WHEN** 程序对类型值执行 `match` 匹配类型构造(如 `Int` vs `String`)并以 `--run` 执行
- **THEN** 匹配按类型构造正确分支

#### Scenario: 六维信息反射

- **WHEN** 程序对已定义函数执行 `effects-of`/`grade-of`/`mode-of`/`determinism-of` 反射并以 `--run` 执行
- **THEN** 各返回该函数的真实效果行/等级/模式/确定性,与静态信息一致

### Requirement: 资源代数声明

`defresource-algebra` SHALL 解析 spec 关键字形式(`:semiring (+ 0 * 1)`、`:order <=`、`:lattice (join Public Private)`、`:asymptotic true`)为资源代数(单位元、二元运算、阶);`Cost` 注解 SHALL 在类型中携带代数语义并 SHALL 参与代价/复杂度检查;未实现的运算 SHALL 报错而非静默通过。

#### Scenario: 资源代数解析

- **WHEN** 源文件声明资源代数与 Cost 注解,以 `--desugar` 运行
- **THEN** 输出保留代数结构与 Cost 标注,无解析错误

#### Scenario: 关键字形式解析

- **WHEN** 源文件以 `(defresource-algebra Cost :semiring (+ 0 * 1) :order <= :asymptotic true)` 声明并以 `--desugar` 运行
- **THEN** 输出保留半环、阶与 asymptotic 标记,无解析错误

#### Scenario: Cost 注解检查

- **WHEN** 类型携带 `Cost` 代数语义且使用超过声明代价,以 `--typecheck` 运行
- **THEN** 报告代价违反,或明确该代数为可判定范围外的警告
