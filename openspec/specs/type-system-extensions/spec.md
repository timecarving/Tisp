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

`□_r`(分级必然)、`◇_ε`(分级可能,即效果行)与 `@[r]` 分级应用 SHALL 有语法与**推理**:声明资源代数后,`(□_r A)`/`(◇_ε A)` 类型与 `f @[r]` 分级应用 SHALL 可解析;分级应用 SHALL 参与等级检查(使用次数 ≤ r),违反 SHALL 为编译错误;`□_r`/`◇_ε` 的引入与消去 SHALL 参与类型推断(可推断时推导 r/ε,不可推断时取默认并警告)。

#### Scenario: 分级必然类型解析

- **WHEN** 源文件以 `(□_n a)` 形式标注分级必然类型并以 `--typecheck` 运行
- **THEN** 类型解析成功,分级信息进入类型

#### Scenario: 分级应用检查

- **WHEN** `f @[n]` 分级应用的使用次数超过等级 n,以 `--typecheck` 运行
- **THEN** 报告等级违反错误

#### Scenario: 分级可能推理

- **WHEN** 源文件以 `(◇_ε a)` 标注且 ε 可由上下文推导,以 `--typecheck` 运行
- **THEN** ε 被推导并进入类型,效果行检查通过

#### Scenario: 模态消去推理

- **WHEN** `□_r` 值被消去使用且使用次数可判定,以 `--typecheck` 运行
- **THEN** 推断 r 满足使用上界,或对不可判定情形明确警告放行

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

### Requirement: 类型类完整实例解析

类型类实例 SHALL 完整解析:分发器按参数类型经**约束求解**查实例(替换运行时字典特例);`:fun-deps`(函数依赖)SHALL 约束实例一致性——违反函数依赖的实例对(同输入不同输出)SHALL 报错;超类约束 SHALL 传播——实例须实现超类方法;kind 约束 SHALL 校验(实例类型 kind 与声明一致)。

#### Scenario: fun-deps 冲突检测

- **WHEN** 声明含 `:fun-deps` 的类型类且存在违反函数依赖的实例对,以 `--typecheck` 运行
- **THEN** 报告函数依赖冲突错误

#### Scenario: 超类约束传播

- **WHEN** 类型类声明超类且实例未实现超类方法,以 `--typecheck` 运行
- **THEN** 报告超类缺失错误

#### Scenario: kind 校验

- **WHEN** 实例类型 kind 与类型类声明不一致,以 `--typecheck` 运行
- **THEN** 报告 kind 错误

#### Scenario: 约束求解驱动查找

- **WHEN** 实例查找需解析重叠/间接约束(如经关联类型间接确定参数类型),以 `--typecheck` 运行
- **THEN** 约束求解器给出唯一实例,而非运行时字典特例失败

### Requirement: 依赖会话类型

会话类型 SHALL 支持值依赖(§20.2/20.3):会话协议 SHALL 引用依赖值(如通道消息携带长度依赖的负载 `(Vec i64 n)`);依赖会话的类型级协议检查 SHALL 拒绝违反协议的操作(与既有顺序检查一致)。

#### Scenario: 值依赖会话

- **WHEN** defsession 协议含值依赖(如发送依赖负载)且操作顺序合法,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 依赖会话违规

- **WHEN** 依赖会话操作顺序违反协议,以 `--typecheck` 运行
- **THEN** 报告协议违反错误

### Requirement: 依赖类型等级传播

Π/Σ 类型 SHALL 携带等级维并在检查中传播(§19.1 r+s 规则):函数应用的等级约束 SHALL 参与 grade_check,违反 SHALL 为编译错误。`(pi (x : T) R)` 语法保持兼容。

#### Scenario: 等级传播通过

- **WHEN** 依赖函数应用的等级线性使用满足 r+s 规则,以 `--typecheck` 运行
- **THEN** 类型检查通过

#### Scenario: 等级违规报错

- **WHEN** 依赖绑定被使用次数超过其等级允许,以 `--typecheck` 运行
- **THEN** 报告等级违反错误

### Requirement: 资源代数声明

`defresource-algebra` SHALL 解析 spec 关键字形式(`:semiring (+ 0 * 1)`、`:order <=`、`:lattice (join Public Private)`、`:asymptotic true`)为资源代数(单位元、二元运算、阶);`Cost` 注解 SHALL 有语法(`@Cost`)并在类型中携带代数语义并参与**完整代价推导**(Big-O 渐近分析:递归代价按代数运算复合、取渐近上界)——使用超过上界 SHALL 报错(可判定时)或明确警告(符号/不可判定时);未实现的运算 SHALL 报错而非静默通过。

#### Scenario: 资源代数解析

- **WHEN** 源文件声明资源代数与 Cost 注解,以 `--desugar` 运行
- **THEN** 输出保留代数结构与 Cost 标注,无解析错误

#### Scenario: 关键字形式解析

- **WHEN** 源文件以 `(defresource-algebra Cost :semiring (+ 0 * 1) :order <= :asymptotic true)` 声明并以 `--desugar` 运行
- **THEN** 输出保留半环、阶与 asymptotic 标记,无解析错误

#### Scenario: Cost 注解与推导

- **WHEN** 类型以 `@Cost` 标注代价上界且使用超过上界,以 `--typecheck` 运行
- **THEN** 报告代价违反,或对不可判定情形明确警告放行

#### Scenario: 渐近代价复合

- **WHEN** 递归函数经 `@Cost` 标注渐近代价(如 O(n))且递归调用代价按代数复合,以 `--typecheck` 运行
- **THEN** 代价推导出正确渐近上界(复合而非仅字面量检查)

### Requirement: committed-choice 运行时语义

CcMulti/CcNonDet 谓词 SHALL 在运行时实现承诺选择:求解到首个解后提交(cc),不再回溯重选;`--run` 行为与注解一致。

#### Scenario: 承诺选择提交

- **WHEN** cc 谓词含多个解分支且以 `--run` 执行
- **THEN** 只产出首个解并提交,不枚举其余分支

### Requirement: 统一六维约束求解

六维注解(type/effect/region/grade/mode/determinism)SHALL 由统一约束系统求解(共享约束图 + fixpoint 迭代),替换六个独立 pass 的串行检查:各维度约束 SHALL 相互可见并联合求解;求解结果 SHALL 同时满足全部维度;维度间冲突 SHALL 报告带跨维度上下文的错误。`--typecheck` 通过 SHALL 等价于统一约束系统无冲突解。

#### Scenario: 跨维度联合求解

- **WHEN** 函数的类型、效果、等级、确定性注解相互关联(如某分支 effect 与 grade 联合约束),以 `--typecheck` 运行
- **THEN** 统一求解给出同时满足各维度的解,而非各 pass 独立放行

#### Scenario: 跨维度冲突报错

- **WHEN** 两维度约束互相矛盾(如 effect 行与 determinism 冲突),以 `--typecheck` 运行
- **THEN** 报告含跨维度上下文的错误,而非首维度独立报错

### Requirement: 五维子类型格

除效果行子类型(§12.5)外,region/grade/mode/determinism 维 SHALL 有子类型关系:区域子类型(子区域值可作父区域)、等级子类型(更宽松等级可作更严等级)、确定性子类型(det ≤ semidet ≤ nondet 的期望序)、模式子类型;子类型 SHALL 参与类型检查(按协变/逆变位置),违反 SHALL 为编译错误。

#### Scenario: 确定性子类型通过

- **WHEN** det 函数用于期望 semidet/nondet 的位置,以 `--typecheck` 运行
- **THEN** 子类型检查通过

#### Scenario: 子类型违反报错

- **WHEN** 更宽松确定性被用于要求更严确定性的位置(如 nondet 用于期望 det),以 `--typecheck` 运行
- **THEN** 报告子类型违反错误

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
