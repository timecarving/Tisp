# hott-and-deriving

## Purpose

补全 HoTT 体系与数据声明小件(§7/§16-17/§27.10):Cohesive HoTT 语义层、HIT 边界语义、deriving 自动派生与演算互编码,使同伦类型理论与演算统一思想从「类型存在」推进到「语义可用」。

## Requirements

### Requirement: Cohesive HoTT 语义层

`ʃ`(Shape)模态 SHALL 有真实语义:ʃ 类型值 SHALL 表示「形状化」数据(可计算路径连通信息);`♭`/`♯` SHALL 在 crisp 上下文约束下求值而非无条件直通;`--typecheck` SHALL 检查 cohesive 模态的上下文合法性。

#### Scenario: ʃ 形状计算

- **WHEN** 程序对数据执行 ʃ 模态操作(如路径连通判断)并以 `--run` 执行
- **THEN** 返回形状语义结果(与直通求值可区分)

#### Scenario: crisp 上下文检查

- **WHEN** 在非 crisp 上下文使用 `♭` 解包,以 `--typecheck` 运行
- **THEN** 报告 cohesive 上下文错误

### Requirement: HIT 边界语义

defdata-hit 的 `:boundary` SHALL 被解析并检查:路径构造器 SHALL 验证端点与边界声明一致;边界违反 SHALL 为编译错误。

#### Scenario: 边界一致通过

- **WHEN** 定义 HIT 且路径构造器端点与 `:boundary` 声明一致
- **THEN** `--typecheck` 通过,`--run` 可构造端点值

#### Scenario: 边界违反报错

- **WHEN** 路径构造器端点与 `:boundary` 声明冲突
- **THEN** 报告边界违反错误

### Requirement: deriving 自动派生

`:deriving` SHALL 生成 Eq/Ord/Show 实现(非仅收集名字):生成实例 SHALL 支持结构比较、排序与打印;`--desugar` 输出 SHALL 可见生成的函数;无法派生(如含函数字段)SHALL 报错。

#### Scenario: 派生比较与打印

- **WHEN** 数据类型声明 `:deriving (Eq, Show)` 且值参与比较/打印,以 `--run` 执行
- **THEN** 结构相等判断与打印输出正确

#### Scenario: 不可派生报错

- **WHEN** 含函数类型字段的数据声明 `:deriving Eq`
- **THEN** 报告无法派生错误

### Requirement: 演算互编码

编译器 SHALL 提供演算间转换(§27.10):π 进程 SHALL 可编码为 SKI 组合子、ambient 能力 SHALL 可编码为通道操作;编码结果 SHALL 可执行且保持观察等价。

#### Scenario: π→SKI 编码

- **WHEN** 对简单 π 进程执行 pi-to-ski 转换并运行
- **THEN** 编码后的 SKI 组合程序行为与原始进程一致

#### Scenario: ambient 能力编码

- **WHEN** 对含 enter/exit 的 ambient 进程执行能力编码并运行
- **THEN** 编码结果保持原移动语义
