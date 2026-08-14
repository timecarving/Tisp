# 部分实现特性全链路化 — 设计

## Context

动机与范围见 `proposal.md`;行为要求见 3 个 MODIFIED delta(既有能力)。关键现状(2026-08 实测):

- **等级**:grade_check 对符号等级(count ≤ n)不可判定时警告放行;grades.rs 有半环;液态验证 `LiquidVerifier::verify_implication` 具备 SMT 整数比较能力(verify_implication 通用)
- **类型一等值**:`reflect-type` 返回字符串;`Value` 枚举 388+ 处引用(加变体改动面大)
- **类型族**:单模式匹配归约已实现(type_infer reduce_families)
- **多模式**:`:mode` 显式签名 + 调用点匹配已实现(mode_analysis)
- **CLP**:线性传播(add_lt/add_eq)已实现;constraint.rs 的 Domain/Propagator 机制可扩展
- **ALP**:abduce 一致性验证已实现(单解)
- **HoTT**:FlatMod/SharpMod 直通、ShapeMod 最小容器、HComp/Transp 直通;Z3 可用于端点方程
- 约束:182 测试全绿、零警告、默认构建可用(LLVM/Z3 feature 门控风格——Z3 是外部进程,无 feature 依赖)

## Goals / Non-Goals

**Goals**
- 相关能力 04 状态升级 ✅(全链路可用,含测试与示例)
- 符号等级严格验证、类型一等值、类型族多模式、多模式推断、隐式绑定 0 级、CLP 算术/全局约束、ALP 多解、Cohesive 形状代数、HIT 端点方程、HComp/Transp 真实求值

**Non-Goals**
- 非本变更范围的 ⚠️ 章(LLVM 真编译链、dlopen 全签名、宏 fn 参数卫生等)
- HoTT 的完全公理化语义(以「端点语义一致」为验收,不做完整立方类型论)
- 等级多态/等级推断(等级仍显式标注;本次做的是符号等级验证而非推断)

## Decisions

### D1:符号等级验证 — 实施修正:自由变量不可判定,诊断警告 + 常量严格

2026-08 实施验证发现:等级变量 n 是自由类型参数,「count ≤ n 恒真」对自由 n 永远不成立(任何 count 都有 n=0 反例)——Z3 严格验证的原始设计语义不成立。修正:数字/可折叠复合等级由 grade_check 常量检查(严格,已有);自由符号等级记录诊断警告(含使用次数),不误报;等级变量经实例化传播获得常量值时严格检查(列为后续,本变更不做实例化传播)。spec 场景已同步修正。

### D2:类型一等值 — Value::Type 变体 + 全量 match 补分支

`Value` 加 `Type(Type)` 变体;编译器穷尽性检查强制暴露全部 match 点(388 处引用,主要分布在 interpreter/process 转换);`reflect-type` 返回 `Value::Type`(兼容:显示/比较路径适配);`type-of` 等保持字符串。风险:改动面大 → 分两批(interpreter 求值路径 + 转换路径),编译器穷尽性兜底。
**备选**:保持字符串反射。否决:spec 明确要求 Value::Type。

### D3:类型族多模式与 rewrite — 实例表扩展 + 归约策略

TypeFamilyInstance 支持多实例;归约按「模式匹配 → rewrite 规则」顺序尝试(模式匹配已有;rewrite = 实例间的简化重写,如 `(Len (List a)) → 1 + (Len (List a 尾))` 风格——以「匹配即归约」实现,多实例按声明序);悬挂报错保持。
**风险**:rewrite 语义未在 spec 定义语法 → 以「多实例匹配归约」为落地语义(记录)。

### D4:多模式自动推断 — mode_analysis 扩展

未声明 `:mode` 的谓词:按调用点实参 free/ground 模式自动收集签名(每个不同调用形态注册一个模式);自动推断与显式声明共存(显式优先)。
**风险**:推断模式可能冲突 → 冲突时报错提示显式声明。

### D5:隐式绑定默认 0 级 — desugar 层默认等级

`desugar_params` 对无等级标注的绑定默认 `Grade::Zero`(§10.2);显式标注保持。**风险:改变大量现有程序行为**(所有未标注参数变 0 级→运行时引用报错)→ **需要精确限定**:仅「隐式绑定」= 无类型注解的参数?§10.2 的隐式绑定指类型级隐式参数。**保守落地**:仅对 `{...}` 无等级标注的 Map 参数默认 Zero;普通符号参数保持 Omega(避免大规模破坏)。

### D6:CLP 算术约束编译 — constraint.rs 扩展

Domain 已支持 retain/区间;新增传播器:
- 乘/除/模:对 `(= (* x y) c)` 类约束,域收缩(枚举候选对;除法域:y 域按 c/x 过滤;模域:同余过滤)
- all-different:变量间两两互斥传播(值冲突剔除)
实现为 ConstraintStore 的 add_mul/add_div/add_mod/add_all_different,接入 clp_constraint 的 op 分发。
**风险**:枚举复杂度 → 域小(教学级)可接受;标注复杂度边界。

### D7:ALP 多解枚举 — abduction.rs 扩展

`abduce-all` 返回全部一致解释(现有 generate_hypotheses 已枚举候选,验证器逐个验证并收集全部一致者);不可满足原因:全部候选验证失败时返回「无一致假设」+ 失败假设数。

### D8:Cohesive 形状代数 — Path 端点连通计算

ShapeMod 求值:对 Path 数据计算端点连通关系(端点 i0/i1 的值相等性 → 连通结论);`shape-connect` 风格内置返回连通布尔;替换最小容器语义。

### D9:HIT 端点方程求解 — 复用 Z3

`:boundary` 的等式经 `verify_implication` 验证可满足性(端点值代入方程);不可满足 → 边界违反报错;替换符号一致性检查。

### D10:HComp/Transp 真实求值 — 端点语义

HComp:沿路径填充——求路径在 i0/i1 的边界值,返回与边界一致的值(对路径 lam 求端点);Transp:沿路径传输——返回目标端点处的值(路径应用结果)。以「端点语义一致」为验收(不实现完整立方填充)。

## Risks / Trade-offs

- [Value::Type 改动面大] → 编译器穷尽性兜底 + 分批(求值/转换);182 测试回归
- [隐式绑定默认 0 破坏现有程序] → 保守限定(仅 Map 无标注参数);验证 04 测试
- [CLP 枚举复杂度] → 教学级域小;标注边界
- [HComp/Transp 语义近似] → 端点语义一致为验收;04 标注近似度
- [符号等级 Z3 依赖外部进程] → 无 z3 时降级警告放行(现状)

## Migration Plan

无部署概念。实施顺序:Z3 类(D1/D9)→ CLP/ALP(D6/D7)→ 类型系统(D2-D5)→ HoTT(D8/D10)→ 文档与 04 状态升级。回滚:git revert 对应提交。

## Open Questions

- 无。类型族 rewrite 语法与 Cohesive 形状语义以「最小可验收语义」落地并记录(不改变 spec/方案/任务拆分)。
