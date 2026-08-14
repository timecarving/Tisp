## Context

Tisp 已有:统一 `def` + 六维注解、`defpred`/CLP/ALP/Search 效应回溯(§21)、`State s`(get/put)与 `Reflect`(quote/drop/lift)效应(§12.3)、宏(§24)、defclass/defgeneric(§22-23)、FRP 信号(§12.3 Signal)、分级模态(§11)与时序类型(§18)。本变更在纯声明式约束下新增 EVOLP/DLP/MOP 与 12 类 LP 范式,全部以「规则/约束/对象 = ADT」为统一数据基底。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 把规则、约束、项传播、OOP 对象建模为 tisp-core 中的一等 ADT 值(可绑定/传递/匹配/构造)。
- 实现不可变 `Program` 与纯函数演化(foldl),稳定模型不动点求值,DLP 状态序列与动态稳定模型。
- 以 GetKB/SetKB 效应操作 + handler 元解释器实现 MOP,编译期元编程为主、运行时 handler 为回退。
- 设计 `Ref a` 引用与 `ref`/`deref`/`set!` 的 State 效应 + 线性/分级等级所有权。
- 以组合优先原则覆盖 12 类 LP 范式,尽量复用既有特性、仅少量新增原语。

**Non-Goals:**
- 不重写既有 `defpred`/CLP/ALP 引擎——在 `tisp-runtime/logic.rs` 之上叠加新层。
- 不做生产级概率推理引擎(PLP 仅需正确的边际概率语义,不追求 MCMC 采样性能)。
- 不改变默认构建——新增求解器与范式以既有 feature 门控惯例隔离,LLVM/Z3 不变。
- 不引入命令式逃逸:所有演化/引用操作保持引用透明或经既有 `Unsafe`/效应门控。

## Decisions

### D1: 统一 ADT 数据模型(规则/约束/项/对象)

新增 `tisp-core` 类型:
- `Program` = 不可变规则集(`im::HashSet<Rule>` + 稳定模型元数据),`Rule` = `{ head: Term, body: Vec<Literal>, evol: Vec<EvolInstr> }`,其中 `EvolInstr = Assert(Rule) | Retract(RuleId)`。
- 约束 `Constrain`、统一项 `Term`、对象 `Object` 均复用既有 `CoreExpr`/`Type`/ADT 表示,新增数据构造器暴露为字面量,不新建平行类型系统。

**理由**:复用 `im` 与既有 AST 使「数据 = 程序结构」天然成立,避免第二套表示;`foldl` 可直接作用于 `Program`。
**备选**:独立 `KnowledgeBase` 类型(被否——与 defpred 已有子句表示割裂,增删查语义不一致)。

### D2: 稳定模型求解器(约化 + 最小模型 + 不动点)

在 `tisp-runtime/evolp.rs` 实现:
- grounding:子句按当前 `Program` 地面化。
- Gelfond-Lifschitz 约化:删除体含被否定字面量的规则,得到正程序。
- 最小模型:正程序的最小 Herbrand 模型(不动点迭代 `T_P ↑ ω`)。
- EVOLP 不动点:按时间点迭代,演化指令逐点改写 `Program`,直到 `Program == evolve(Program)` 收敛。

**理由**:稳定模型语义是 EVOLP/DLP 的定义性语义,约化+最小模型是标准可判定的实现;不动点用递归实现、可选 `fix` 组合子(与既有 Search/CLP 求值一致)。
**备选**:直接 SAT/SMT 编码(被否——z3 需 feature,默认构建不可用;grounding 规模可控时不动点更简单)。

### D3: DLP 动态稳定模型算法

`DLProgram = Vec<Program>`(状态序列),更新 = 追加新状态。动态稳定模型按定义分两步:
1. 对每个 `Pi` 拒绝所有「被后续状态 Pj(j>i) 缺省否定」的规则;
2. 对剩余规则做约化,求所得程序的最小模型。

**理由**:忠实于用户给出的拒绝/接受语义定义;序列不可变,拒绝判定为纯函数,可用 `foldl`/`scan` 表达。
**备选**:增量维护已求动态稳定模型(被否——初始实现以正确性为先,增量作为后续优化)。

### D4: MOP = 效应操作 + handler 元解释器

- 扩展效应操作枚举:新增 `GetKB` / `SetKB(Program)`,声明于 KB 效应行。
- handler 捕获 `GetKB`/`SetKB` 并解释其语义(读写当前 `Program`),即元解释器;与 §12 效应框架、§27.7 `Reflect` 一致。
- **编译期优先**:宏展开/部分求值在编译期解析对「编译期可见」规则集的 GetKB/SetKB 与反射,产出静态结果;仅当 KB 编译期不可见时回退运行时 handler。

**理由**:满足「元编程不需 Runtime,编译期即够」,同时保留 handler 元解释器作回退,不破坏既有效应语义。
**备选**:纯编译期宏展开(被否——丢失运行时动态 KB 的能力);纯运行时反射(被否——与用户约束相悖)。

### D5: State Effect 引用管理

- 新增 `Ref a` 类型;`ref`/`deref`/`set!` 建模为 `State (Ref a)` 效应操作(复用 §12.3 `State s`)。
- 所有权:创建引用得 1 级(线性)能力,`set!` 消费写端、`deref` 读端按等级计数;ω 级引用可共享读;违反线性/等级 SHALL 报编译错误(接入既有 QTT grade_check)。

**理由**:复用既有 `State` 效应与 QTT 等级,引用安全由类型系统而非运行时保证。
**备选**:独立 `Ref` 效应(被否——State 语义已覆盖;独立效应徒增效应行复杂度)。

### D6: 12 类 LP 范式组合优先映射

| 范式 | 组合/新增 | 关键既有特性 |
|------|-----------|--------------|
| 高阶 LP | 组合(谓词=一等值) | 函数一等值 + `call` |
| ILP | 组合 + `induce` 内置 | 规则即数据(D1) + Search |
| PLP | 新增少量(分布语义) | nondet + 数值 + 事实标注 |
| 时序 LP | 组合 | §18 时序类型 + 时间索引事实 |
| 描述逻辑 | 组合 | 类型类 + 子类型 + 逻辑项 |
| 可废止 LP | 组合 + 优先级字段 | 泛型方法组合(§22.3) + committed-choice |
| 模糊 LP | 组合 + 真值度 | 分级/等级 + min/max |
| Tabled LP | 新增少量(记忆表) | 递归谓词 + 缓存 |
| 一体化基底 | 组合(形式化互操作) | 静态类型 + OOP + 并发(§22/§27) |
| 响应式 LP | 组合 | FRP 信号(§12.3)+ 代数效应 |
| 情境 LP | 组合 | 模块(§25)+ Reader 效应 |
| 模态 LP | 组合 | §11 分级模态 □_r/◇_ε |

**理由**:遵循用户「可用既有特性组合则组合、否则少量新增」的约束;新增原语仅 `induce`、PLP 分布、优先级字段、真值度、表格化记忆表五处。
**备选**:每范式独立新特性(被否——违背组合优先,爆炸性扩张 AST/求值器)。

## Risks / Trade-offs

- [稳定模型 grounding 组合爆炸] → 地面化限定于当前 Program 的规则与查询项,后续以索引/tabling 优化。
- [缺省否定(NAF)与既有 committed-choice/回溯交互复杂] → 稳定模型求值与 `defpred` 回溯分离为独立求解器,避免语义纠缠。
- [引用线性等级与既有 State 的 get/put 冲突] → 引用操作单独声明等级契约,复用 grade_check 而非新增检查 pass。
- [PLP 概率推理精确性 vs 性能] → 默认精确枚举(有限 grounding),标注为「可替换为采样」的扩展点。
- [12 范式一次性落地风险高] → tasks 按范式拆分为独立可交付步骤,每个范式可独立合入与测试。

## Migration Plan

全部为增量新增(新类型/新内置/新模块),不改动既有 `defpred`/CLP/ALP 语义,无破坏性迁移。实现按 capability 顺序(ADT → EVOLP/DLP → MOP/State → 范式)逐层提交,每层保持 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- PLP 概率推理默认采用精确枚举还是带参数切换的采样:可延后,不影响 spec 语义(两者均满足边际概率契约)。
- Tabled LP 的记忆策略(SLG 悬挂-恢复 vs 朴素全记忆):可延后,spec 仅要求「左递归终止 + 子目标复用」。
