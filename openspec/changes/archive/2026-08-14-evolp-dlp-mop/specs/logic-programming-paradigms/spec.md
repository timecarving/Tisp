## Purpose

在纯声明式、静态类型、函数-并发-内存管理(依赖/分级线性类型)约束下,以组合优先原则覆盖 12 类逻辑编程范式:高阶、归纳(ILP)、概率(PLP)、时序、描述、可废止、模糊、表格化、一体化基底、响应式、情境、模态。

## ADDED Requirements

### Requirement: 高阶逻辑编程

谓词/规则 SHALL 可作为高阶值:谓词可绑定、作为参数传递、由高阶组合子(如 `call`、`map-pred`)调用;组合谓词 SHALL 与直接定义等价。

#### Scenario: 谓词作参数

- **WHEN** 程序把谓词作为参数传给高阶组合子并对列表逐项调用
- **THEN** 结果与逐项直接调用该谓词一致

#### Scenario: 组合谓词

- **WHEN** 程序用高阶组合子组合两个谓词(如合取/析取)并求解
- **THEN** 行为与显式定义组合谓词一致

### Requirement: 归纳逻辑编程(ILP)

程序 SHALL 支持从示例归纳规则:给定正/负例与背景知识,`induce` SHALL 生成覆盖正例、排除负例的假设规则集;结果 SHALL 可执行(可被普通逻辑引擎查询)。

#### Scenario: 归纳覆盖假设

- **WHEN** 以正例(如 `parent(a,b)`)与负例运行 `induce`
- **THEN** 返回覆盖正例、排除负例的规则,追加后可用引擎查询验证

#### Scenario: 假设可执行

- **WHEN** 归纳出的规则集被加入程序并查询
- **THEN** 查询结果与示例一致

### Requirement: 概率逻辑编程(PLP)

程序 SHALL 支持概率事实与规则:`prob` 标注事实概率;求值 SHALL 产生带概率的解释(分布),查询 SHALL 返回某目标的边际概率;概率 SHALL 满足归一化与独立性语义。

#### Scenario: 概率事实边际概率

- **WHEN** 概率事实 `0.3::heads` 且查询 heads
- **THEN** 返回边际概率 0.3

#### Scenario: 组合概率

- **WHEN** 多个独立概率事实经规则组合后查询
- **THEN** 结果概率按独立性语义计算,非错误叠加

### Requirement: 时序逻辑编程

程序 SHALL 支持时间索引事实与 LTL 时序算子:事实 SHALL 带时间点,查询 SHALL 可用 `next`/`always`/`eventually`/`until` 等算子;时序求值 SHALL 与既有时序类型(§18)一致。

#### Scenario: 时间索引查询

- **WHEN** 事实序列含 t=0/t=1 的状态,以 `eventually P` 查询
- **THEN** 返回存在时刻使 P 成立的真值

#### Scenario: 与时序类型一致

- **WHEN** 时序逻辑程序与时序流/模态混用,以 `--typecheck` 运行
- **THEN** 时态类型检查通过,语义一致

### Requirement: 描述逻辑编程

程序 SHALL 支持概念与角色作为类型/约束:概念(如 `Person`)SHALL 可声明并参与子概念关系(如 `Man ⊑ Person`);角色 SHALL 约束个体间关系;查询 SHALL 经描述逻辑语义(概念满足/角色推理)求值。

#### Scenario: 子概念推理

- **WHEN** 声明 `Man ⊑ Person` 且个体 `x` 是 `Man`,查询 `Person(x)`
- **THEN** 推理得出 `Person(x)` 成立

#### Scenario: 角色约束

- **WHEN** 声明角色 `hasParent` 与概念约束,查询个体关系
- **THEN** 按描述逻辑语义返回满足约束的个体

### Requirement: 可废止逻辑编程

程序 SHALL 支持带优先级/击败者的可废止规则:规则 SHALL 可声明优先级;冲突规则 SHALL 按优先级/击败者选择;结论 SHALL 记录可废止性(可被更强规则推翻)。

#### Scenario: 优先级裁决冲突

- **WHEN** 两条规则结论冲突且优先级不同,求值
- **THEN** 高优先级规则胜出,低优先级结论被击败

#### Scenario: 可废止结论

- **WHEN** 默认规则在无例外时成立、有例外规则时被推翻
- **THEN** 结论随例外规则出现而废止,符合可废止语义

### Requirement: 模糊逻辑编程

程序 SHALL 支持模糊真值:事实/规则 SHALL 可带真值度(如 `0.7::likes(a,b)`);求值 SHALL 按模糊逻辑连接词(min/max/补)组合真值度;查询 SHALL 返回真值度而非布尔。

#### Scenario: 模糊真值组合

- **WHEN** 事实 `0.7::A` 与规则 `A :- B`、`0.5::B` 求值
- **THEN** A 的真值度按 min 组合(0.5),非布尔

#### Scenario: 模糊查询返回真值度

- **WHEN** 查询模糊目标
- **THEN** 返回 [0,1] 真值度,而非真/假

### Requirement: 表格逻辑编程(Tabled)

程序 SHALL 支持表格化(tabling)求值:递归谓词 SHALL 记忆已解目标(表格),重复子目标 SHALL 复用结果;带表格化的左递归程序 SHALL 终止(不无限循环)。

#### Scenario: 表格化终止

- **WHEN** 左递归谓词(如可达性)以表格化求值
- **THEN** 终止并返回完整解集,不无限递归

#### Scenario: 子目标复用

- **WHEN** 同一子目标多次出现
- **THEN** 表格复用已解结果,结果一致且更高效

### Requirement: 静态类型-函数-OOP-并发一体化基底

逻辑编程 SHALL 与既有静态类型、函数、OOP、并发特性一体化:谓词 SHALL 有静态类型签名;逻辑子句 SHALL 可与函数/OOP 定义互操作;并发进程 SHALL 可作为逻辑项参与统一。

#### Scenario: 谓词静态类型

- **WHEN** 谓词以类型签名声明并以错误类型调用,以 `--typecheck` 运行
- **THEN** 报告类型错误

#### Scenario: 逻辑-函数-OOP 互操作

- **WHEN** 逻辑子句调用函数/OOP 方法且类型匹配
- **THEN** 类型检查通过,求值正确

### Requirement: 代数效应 FRP 响应式逻辑编程

逻辑程序 SHALL 可作为响应式逻辑程序:基于 FRP 信号与代数效应,规则 SHALL 可订阅/响应信号变化;规则集 SHALL 随信号流更新(响应式规则);求值 SHALL 与 FRP 语义一致。

#### Scenario: 信号驱动规则

- **WHEN** 规则订阅某信号且信号值变化
- **THEN** 规则按新值重新求值,结果随信号更新

#### Scenario: 响应式规则集

- **WHEN** 规则集随信号流动态更新并查询
- **THEN** 查询结果反映最新信号状态

### Requirement: 情境逻辑编程

程序 SHALL 支持情境(上下文)逻辑:规则 SHALL 可声明所在情境;查询 SHALL 可指定情境;情境 SHALL 按层次组织(子情境继承父情境规则);不同情境同名谓词 SHALL 隔离。

#### Scenario: 情境隔离

- **WHEN** 两个情境定义同名谓词并分别在各自情境查询
- **THEN** 各返回本情境的结果,互不干扰

#### Scenario: 情境继承

- **WHEN** 子情境未定义某谓词而父情境已定义,在子情境查询
- **THEN** 继承父情境规则,结果正确

### Requirement: 模态逻辑编程

程序 SHALL 支持模态逻辑推理:`possible`/`necessary` 模态 SHALL 作用于逻辑目标;求值 SHALL 按可能世界语义(可达关系)判定;模态 SHALL 与既有分级模态(□_r/◇_ε,§11)一致。

#### Scenario: 可能世界判定

- **WHEN** 目标在某可达世界成立,查询 `possible goal`
- **THEN** 返回真;不在任何可达世界成立则返回假

#### Scenario: 与分级模态一致

- **WHEN** 模态逻辑程序与分级模态类型混用,以 `--typecheck` 运行
- **THEN** 模态类型检查通过,语义一致
