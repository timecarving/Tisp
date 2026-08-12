# 依赖线性类型系统 — 设计

## Context

动机与范围见 `proposal.md`;行为要求见 `specs/dependent-linear-types/spec.md`。关键现状(2026-08 实测):

- `Grade` 枚举已定义代数:`Zero/One/Omega/Nat(u64)/Add/Mul/Var(Symbol)/Custom`(types.rs:99-108);`grades.rs` 有半环运算(Add/Mul 对 Nat 折叠);`depgraded.rs` 有 `grade_value` 求值(Zero→0、Nat→n 等)
- `desugar_graded_param`(desugar.rs)只接受 `Expr::Int(0)`/`Int(1)`/`Sym("ω"|"omega")`——数字 >1、符号、复合表达式全部报「grade must be 0, 1, or ω」
- `grade_check.rs` 只处理 `Grade::Zero/One/Omega`:One 恰好一次(含 if 分支 snapshot/merge)、Zero 豁免、Omega 不限制;`Nat/Add/Mul/Var` 落入 `_ => {}` 不检查
- `count_var_in_type` 依赖计数机制已有(§19.1 等级传播轮)
- 液态类型基础设施(Z3 验证 + 警告放行策略)可复用

## Goals / Non-Goals

**Goals**
- 等级表达式语法(Nat/Add/Mul/Var)全链路解析 → 检查 → 显示
- 使用计数 ≤ 等级表达式(上界语义)的检查,数字等级常量折叠、符号等级常量可判定时检查
- 与现有 0/1/ω 语义完全兼容(零回归)

**Non-Goals**
- 符号等级不等式的 Z3 验证(可判定的常量场景先行;符号不等式列为后续,复用液态基础设施)
- 等级多态/等级推断(用户显式标注等级)
- 运行时复制语义(等级 >1 的资源仍按引用传递,检查层特性;运行时擦除仅对 Zero 保持现状)

## Decisions

### D1:检查语义 — 上界(使用次数 ≤ 等级),与 Idris 2 数量语义一致

`(5 x : T)` 的 x 可用最多 5 次;`(n x : T)` 的 x 可用最多 n 次。备选「恰好 n 次」否决:分支/条件中「恰好」要求各分支都恰好 n 次,实现与使用都过于严格;上界语义与 Idris 2 一致,且 0/1/ω 是特例(Zero=恰好 0、One=恰好 1 由现有恰好检查保留、Omega=无穷)。

### D2:语法 — 复用 graded 参数位置,等级表达式三形态

`desugar_graded_param` 扩展:
- `Expr::Int(n)` → `Nat(n)`(n ≥ 0;现 0/1 路径并入)
- `Expr::Sym(s)` → `Var(s)`(s 非 ω/omega)
- `Expr::List([op, a, b])` 且 op ∈ {+,-,*,/} → `Add/Sub/Mul/Div`(负结果在检查时按 0 处理或报错)
- 复合嵌套递归解析
`{5 x : a}`(Map 形式)与 `(5 x : a)`(List 形式)均支持(现有两种形式已并存)。

### D3:等级变量绑定 — 类型参数扫描

等级变量合法性:函数签名中的类型参数(经 `(Vec i64 n)`、`(pi (n : T) R)` 等出现的类型级符号)集合。实现:desugar 时收集 def 类型注解中的类型级符号 → `Grade::Var` 检查;或 type_infer 阶段统一检查(等级 Var 出现时,若不在「类型参数集合」报错)。选 type_infer:desugar 不持有全局类型信息;具体做法——`check_def` 时收集 def.ty/params 中的类型变量名,`grade_check` 检查 `Grade::Var(n)` 是否在该集合(需要把集合传入 grade_check)。

### D4:检查算法 — 常量折叠 + 符号比较 + 警告放行

`grade_check` 对非固定等级:
1. **计数**:现有 usage 计数(One 恰好一次逻辑保持)
2. **等级求值**:`grade_value(grade) -> Option<u64>`(depgraded 已有,扩展 Add/Mul 折叠)——等级中无 Var 时得常量
3. **比较**:`count ≤ grade_value`;违反报错(span 指向绑定处)
4. **符号等级**:Var 出现在等级中时——若 Var 可绑定到常量(类型级常量传播:参数类型 `(Vec i64 n)` 中 n 的实例化?本变更不做实例化传播)→ 常量判定;否则**警告放行**(与液态类型「未知谓词警告放行」策略一致,不误报)
5. **分支合并**:现有 if snapshot/merge 机制扩展——合并计数取各分支 max(上界)

### D5:运行时 — 擦除保持 Zero,等级求值接线

0 级擦除现状不变(Zero 唯一擦除条件);`Nat(n>0)/Var/Add/Mul` 等级不擦除(值参与运行)。`grade-of` 内置扩展:返回等级表达式显示(Nat 数字、Var 符号)。`depgraded.rs` 的 `grade_value` 保持(检查层复用)。

### D6:与液态类型的关系 — 后续集成点,本变更不做

符号等级不等式(count ≤ n)的严格验证可用 Z3(SMT 整数比较)——液态验证的 `verify_implication` 已具备此能力;本变更以「常量可判定 + 警告放行」落地,Z3 集成列为后续变更(design 层面记录接口:grade_check 可产出不等式约束供 LiquidVerifier 消费)。

## Risks / Trade-offs

- [分支合并破坏现有 One 检查] → 现有 if 分支逻辑(快照/恢复/合并)保持不变,只扩展计数合并;既有 32 个 frontend + grade_check 测试回归
- [符号等级误放行导致漏检] → 有意的保守策略(与液态「未知谓词放行」一致);警告可见;Z3 集成后续收紧
- [等级语法与现有 `{0 x}` 冲突] → 0/1/ω 走原路径,数字 >1 走新路径,无语法歧义
- [grade_value 溢出/负等级] → 折叠结果饱和或按违反处理(常量负等级报错)

## Migration Plan

无部署概念。实施顺序:语法(desugar)→ 检查(grade_check)→ 运行时(grade-of/擦除适配)→ 测试与文档。回滚:git revert 对应提交。既有程序零行为变化(0/1/ω 路径不变)。

## Open Questions

- 无。符号等级 Z3 验证、等级推断、运行时复制语义均为后续优化,不影响本变更的 spec/方案/任务拆分。
