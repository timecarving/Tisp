## MODIFIED Requirements

### Requirement: Cohesive HoTT 语义层

`ʃ`(Shape)模态 SHALL 有真实语义:ʃ 类型值 SHALL 表示「形状化」数据(可计算路径连通信息);**ʃ 形状代数 SHALL 支持路径连通计算(区间端点 i0/i1 的连通关系判断),替换最小可区分语义**;`♭`/`♯` SHALL 在 crisp 上下文约束下求值而非无条件直通;`--typecheck` SHALL 检查 cohesive 模态的上下文合法性。

#### Scenario: ʃ 形状计算

- **WHEN** 程序对路径数据执行 ʃ 模态操作(路径连通判断)并以 `--run` 执行
- **THEN** 返回形状语义结果(如端点连通结论),与直通求值可区分

#### Scenario: crisp 上下文检查

- **WHEN** 在非 crisp 上下文使用 `♭` 解包,以 `--typecheck` 运行
- **THEN** 报告 cohesive 上下文错误

### Requirement: HIT 边界语义

defdata-hit 的 `:boundary` SHALL 被解析并检查:路径构造器 SHALL 验证端点与边界声明一致;边界违反 SHALL 为编译错误。**端点方程 SHALL 求解验证(边界声明的等式经约束求解确认一致),替换符号一致性检查**。

#### Scenario: 边界一致通过

- **WHEN** 定义 HIT 且路径构造器端点与 `:boundary` 声明一致
- **THEN** `--typecheck` 通过,`--run` 可构造端点值

#### Scenario: 边界违反报错

- **WHEN** 路径构造器端点与 `:boundary` 声明冲突(等式不可满足)
- **THEN** 报告边界违反错误

## ADDED Requirements

### Requirement: HComp/Transp 真实求值

同伦合成(HComp)与传输(Transp)SHALL 有非平凡语义(替换直通求值):HComp SHALL 沿路径填充计算边界一致的值;Transp SHALL 沿路径在纤维间传输值;`--run` 结果 SHALL 与端点语义一致(如 HComp 的边界值、Transp 的目标端点值)。

#### Scenario: HComp 边界填充

- **WHEN** 程序对路径执行 HComp(同伦合成)并以 `--run` 执行
- **THEN** 返回边界一致的值(与路径端点语义一致),而非原值直通

#### Scenario: Transp 纤维传输

- **WHEN** 程序沿路径执行 Transp(传输)并以 `--run` 执行
- **THEN** 返回目标端点的传输值(路径端点语义正确)
