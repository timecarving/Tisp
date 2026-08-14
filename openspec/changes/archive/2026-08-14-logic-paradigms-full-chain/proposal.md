## Why

12 种逻辑编程子范式在 `paradigms.rs` 已有真实求解器,但 `facility.rs` 的 eval 多为「简化投影」(higher-order = `x>0`、prob = 恒等、eventually = 列表包含、reactive = `x*2`),仅部分范式(tabling/subsumme/evolp/dlp)接了专用内置。本变更把这 12 范式全部在**静态类型 + 纯声明式 + 统一内存管理(Grade + EffectRow)**约束下接线到真实求解器,直到全链路可用。

## What Changes

- **12 范式全链路接线**:高阶/归纳 ILP/概率 PLP/时序/描述/可废止/模糊/表格化/一体化基底/响应式/情境/模态,各配专用内置 + 类型签名 + 效应门控(State/Search/Signal),替换 `facility.rs` 的简化投影。
- **真实求解器接线**:prob→`marginal`、eventually→`TemporalKb`、reactive→`Signal`、settle→`DefRule`、fuzzy→`FuzzyFact`、induce→`induce`、context→`ContextKb`、possible→`ModalKb`、typed-pred→静态类型谓词、higher-order→谓词组合子。
- **类型/效应/等级接入**:各范式内置补单态签名(type_infer),副作用接入效应行(effect_infer 已注册 State/Search/Signal)。

## Capabilities

(无新增/修改能力——本变更为既有 `logic-programming-paradigms` 需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。)

## Impact

- **tisp-backend**:interpreter 注册 12 范式专用内置(经 `ParadigmRegistry` 分发到真实求解器)。
- **tisp-middle**:type_infer 补 12 范式类型签名;effect_infer 效应门控(已注册)。
- **tisp-runtime**:paradigms.rs 真实求解器作为求值内核。
- **standard_doc**:§31/§32 状态更新。
