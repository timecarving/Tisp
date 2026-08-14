## ADDED Requirements

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

## MODIFIED Requirements

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
