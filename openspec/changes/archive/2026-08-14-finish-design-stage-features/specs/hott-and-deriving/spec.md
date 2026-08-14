## ADDED Requirements

### Requirement: fun-ext / 幺半等价

(§16.3-16.5)函数外延性(fun-ext)与幺半等价 SHALL 可计算:同一点态相等的函数 SHALL 经 fun-ext 视为相等;幺半群(Monoid)等价 SHALL 可判定(结合律/单位元性质验证,枚举三元组)。`--run` 结果 SHALL 与等价语义一致(有限域限定)。

#### Scenario: fun-ext 判定

- **WHEN** 两个函数对所有输入点态相等,程序执行 fun-ext 判定并以 `--run` 运行
- **THEN** 返回等价结论(true)

#### Scenario: 幺半等价验证

- **WHEN** 程序验证某运算的幺半群性质(结合律/单位元),以 `--run` 运行
- **THEN** 返回性质验证结论(满足或给出反例)

## MODIFIED Requirements

### Requirement: Cohesive HoTT 语义层

`ʃ`(Shape)模态 SHALL 有真实语义:ʃ 类型值 SHALL 表示「形状化」数据;ʃ SHALL 支持**连通图代数**(路径连通计算已实现,补连通图:节点=值,边=路径连通关系,`shape-graph` 返回连通图结构);`♭`/`♯` SHALL 在 crisp 上下文约束下求值而非无条件直通;`--typecheck` SHALL 检查 cohesive 模态的上下文合法性。

#### Scenario: ʃ 形状计算

- **WHEN** 程序对数据执行 ʃ 模态操作(如路径连通判断)并以 `--run` 执行
- **THEN** 返回形状语义结果(与直通求值可区分)

#### Scenario: ʃ 连通图

- **WHEN** 程序对多个路径值执行 ʃ 连通图操作并以 `--run` 执行
- **THEN** 返回连通图结构(节点与连通边),端点连通信息正确

#### Scenario: crisp 上下文检查

- **WHEN** 在非 crisp 上下文使用 `♭` 解包,以 `--typecheck` 运行
- **THEN** 报告 cohesive 上下文错误

### Requirement: HIT 边界语义

defdata-hit 的 `:boundary` SHALL 被解析并检查:路径构造器 SHALL 验证端点与边界声明一致;端点方程 SHALL 完整求解(常量端点判定已实现,补符号端点可满足性验证);边界违反 SHALL 为编译错误;`--run` SHALL 经 hott.rs 运行时模块构造端点值(替换内联占位)。

#### Scenario: 边界一致通过

- **WHEN** 定义 HIT 且路径构造器端点与 `:boundary` 声明一致
- **THEN** `--typecheck` 通过,`--run` 可构造端点值

#### Scenario: 端点方程求解

- **WHEN** 边界等式含符号端点且不可满足,以 `--typecheck` 运行
- **THEN** 报告边界违反错误

#### Scenario: 边界违反报错

- **WHEN** 路径构造器端点与 `:boundary` 声明冲突
- **THEN** 报告边界违反错误

### Requirement: deriving 自动派生

`:deriving` SHALL 在 desugar 阶段生成 Eq/Ord/Show 实现(替换运行时内置):生成实例 SHALL 支持结构比较、排序与打印;`--desugar` 输出 SHALL 可见生成的函数;无法派生(如含函数字段)SHALL 报错;未识别 trait SHALL 报错而非静默忽略。

#### Scenario: 派生比较与打印

- **WHEN** 数据类型声明 `:deriving (Eq, Show)` 且值参与比较/打印,以 `--run` 执行
- **THEN** 结构相等判断与打印输出正确

#### Scenario: desugar 可见生成函数

- **WHEN** 数据类型声明 `:deriving (Eq, Ord, Show)`,以 `--desugar` 运行
- **THEN** 输出包含生成的 `eq-*`/`ord-*`/`show-*` 函数定义

#### Scenario: 未知 trait 报错

- **WHEN** 数据类型声明 `:deriving (Foo)`(未识别 trait)
- **THEN** 报告未知 trait 错误,而非静默忽略

#### Scenario: 不可派生报错

- **WHEN** 含函数类型字段的数据声明 `:deriving Eq`
- **THEN** 报告无法派生错误

### Requirement: 演算互编码

编译器 SHALL 提供演算间转换(§27.10):π 进程 SHALL 可编码为 SKI 组合子、async SHALL 可编码为同步 π、applied-π SHALL 可编码为 π、ρ SHALL 可编码为 π、ambient 能力 SHALL 可编码为通道操作;编码结果 SHALL 可执行;编码 SHALL 保持观察等价,并 SHALL 支持全演算互模拟/barbed 等价检查(替换 π→SKI 特例)。

#### Scenario: π→SKI 编码

- **WHEN** 对简单 π 进程执行 pi-to-ski 转换并运行
- **THEN** 编码后的 SKI 组合程序行为与原始进程一致

#### Scenario: 三缺编码补齐

- **WHEN** 对 async-π / applied-π / ρ 进程执行对应编码转换
- **THEN** 转换存在且结果可执行,行为与源进程一致

#### Scenario: ambient 能力编码

- **WHEN** 对含 enter/exit 的 ambient 进程执行能力编码并运行
- **THEN** 编码结果保持原移动语义

#### Scenario: 观察等价检查

- **WHEN** 对原进程与其编码结果执行观察等价检查
- **THEN** 返回等价结论,或给出区分轨迹
