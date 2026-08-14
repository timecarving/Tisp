## MODIFIED Requirements

### Requirement: HIT 边界语义

defdata-hit 的 `:boundary` SHALL 被解析为结构化边界子句(guard → 目标)并检查:路径构造器 SHALL 验证每个端点(`i := i0`/`i := i1`)经代入后钉到唯一一致的目标;端点被钉到不同目标 SHALL 为编译错误;spec 的 `(i = i0) -> base` 语法 SHALL 被接受。`--run` SHALL 可构造端点值(而非占位)。

#### Scenario: 边界一致通过

- **WHEN** 定义 HIT 且路径构造器端点与 `:boundary` 声明一致
- **THEN** `--typecheck` 通过,`--run` 可构造端点值

#### Scenario: 端点唯一一致性

- **WHEN** `:boundary` 两条子句把同一端点(`i0`)钉到不同构造器,以 `--typecheck` 运行
- **THEN** 报告边界不一致错误

#### Scenario: spec 语法可解析

- **WHEN** 以 `(loop [i : I] :boundary [(i = i0) -> base (i = i1) -> base])` 声明 HIT,以 `--typecheck` 运行
- **THEN** 解析成功(区间变量 `i` 被识别),边界检查通过

#### Scenario: 边界违反报错

- **WHEN** 路径构造器端点与 `:boundary` 声明冲突
- **THEN** 报告边界违反错误

### Requirement: deriving 自动派生

`:deriving` SHALL 生成 Eq/Ord/Show 实现(非仅收集名字,且非运行时内置):生成实例 SHALL 支持结构比较、排序与打印;`ord-*` SHALL 按构造器声明序与字段逐项比较;`--desugar` 输出 SHALL 可见生成的函数;无法派生(如含函数字段)SHALL 报错;未识别 trait SHALL 报错而非静默忽略。

#### Scenario: 派生比较与打印

- **WHEN** 数据类型声明 `:deriving (Eq, Show)` 且值参与比较/打印,以 `--run` 执行
- **THEN** 结构相等判断与打印输出正确

#### Scenario: 派生 Ord 排序

- **WHEN** 数据类型声明 `:deriving Ord` 且两个值参与 `<` 比较,以 `--run` 执行
- **THEN** 按构造器声明序与字段序返回正确排序结果

#### Scenario: 未知 trait 报错

- **WHEN** 数据类型声明 `:deriving (Foo)`(未识别 trait)
- **THEN** 报告未知 trait 错误,而非静默忽略

#### Scenario: 不可派生报错

- **WHEN** 含函数类型字段的数据声明 `:deriving Eq`
- **THEN** 报告无法派生错误

### Requirement: 演算互编码

编译器 SHALL 提供演算间转换(§27.10):π 进程 SHALL 可编码为 SKI 组合子、async 进程 SHALL 可编码为同步 π、applied-π SHALL 可编码为 π、ρ 进程 SHALL 可编码为 π、ambient 能力 SHALL 可编码为通道操作;编码结果 SHALL 可执行;编码 SHALL 保持观察等价(SKI 归约 SHALL 保留被丢弃的常量负载),并 SHALL 支持对原进程与编码结果的观察等价检查。

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
