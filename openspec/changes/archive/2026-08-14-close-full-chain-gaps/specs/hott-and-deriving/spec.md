## ADDED Requirements

### Requirement: 完整立方填充

HIT 的立方组合 SHALL 完整落地:除既有 HComp(边界填充)与 Transp(端点传输)外,完整 Kan 填充(多维立方体面组合成)SHALL 可求值——任意维度区间面的组合 SHALL 返回与边界一致的立方值;不一致边界 SHALL 为类型错误;`--run` 结果 SHALL 与立方语义一致。

#### Scenario: 多维立方组合

- **WHEN** 程序构造多维(≥2 维)立方体并按面组合求值,以 `--run` 执行
- **THEN** 返回与所有边界面一致的立方值

#### Scenario: 边界不一致报错

- **WHEN** 立方组合的面与边界声明不一致,以 `--typecheck` 运行
- **THEN** 报告立方边界错误

## MODIFIED Requirements

### Requirement: Cohesive HoTT 语义层

`ʃ`(Shape)模态 SHALL 有真实语义:ʃ 类型值 SHALL 表示「形状化」数据;ʃ SHALL 支持**连通图代数**(节点=值,边=路径连通关系,`shape-graph` 返回连通图结构);`♭`(flat)SHALL 为 comonadic 语义——剥离拓扑/光滑结构,返回离散点(非无条件直通);`♯`(sharp)SHALL 为 monadic 语义——嵌入为 codiscrete 空间;`--typecheck` SHALL 检查 cohesive 模态的上下文合法性与 adjoint-triple(ʃ ⊣ ♭ ⊣ ♯)的模态组合。

#### Scenario: ʃ 形状计算

- **WHEN** 程序对数据执行 ʃ 模态操作(如路径连通判断)并以 `--run` 执行
- **THEN** 返回形状语义结果(与直通求值可区分)

#### Scenario: ʃ 连通图

- **WHEN** 程序对多个路径值执行 ʃ 连通图操作并以 `--run` 执行
- **THEN** 返回连通图结构(节点与连通边),端点连通信息正确

#### Scenario: crisp 上下文检查

- **WHEN** 在非 crisp 上下文使用 `♭` 解包,以 `--typecheck` 运行
- **THEN** 报告 cohesive 上下文错误

#### Scenario: flat 剥离结构

- **WHEN** 程序对含拓扑结构的值执行 `♭` 并以 `--run` 执行
- **THEN** 返回剥离结构的离散点(与直通求值可区分)

#### Scenario: sharp 嵌入 codiscrete

- **WHEN** 程序对离散值执行 `♯` 并以 `--run` 执行
- **THEN** 返回 codiscrete 空间值,模态组合符合 adjoint-triple 语义
