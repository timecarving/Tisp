# 依赖线性类型系统 — 任务清单

规范依据:`specs/dependent-linear-types/spec.md`;方案依据:`design.md`。

## 1. 等级表达式语法(desugar)

- [x] 1.1 `desugar_graded_param` 扩展:数字等级 `(5 x : a)` → `Grade::Nat(5)`;0/1/ω 走原路径;`--desugar` 保留等级结构
- [x] 1.2 符号等级 `(n x : a)` → `Grade::Var(n)`(ω/omega 除外);复合等级 `((+ n 1) x : a)` → `Grade::Add/Mul/Sub/Div` 递归解析
- [x] 1.3 desugar 测试:数字/符号/复合等级解析正确;非法等级(负数字/未知运算)报错

## 2. 等级检查(grade_check)

- [x] 2.1 `grade_value` 常量求值(复用 depgraded,扩展 Add/Mul 折叠;含 Var 返回 None);数字等级常量折叠
- [x] 2.2 使用计数 ≤ 等级检查:非固定等级绑定的计数超限报错(span 定位);`0/1/ω` 路径不变
- [x] 2.3 等级变量集合传入:type_infer 收集 def 签名中的类型级符号,grade_check 校验 `Grade::Var` 未绑定报错
- [x] 2.4 分支合并:if/match 分支对依赖等级绑定的计数取上界(现有 snapshot/merge 扩展);超限报错
- [x] 2.5 符号等级不可判定时警告放行(与液态策略一致);负等级常量报错
- [x] 2.6 grade_check 测试:数字满足/违反、符号常量判定、分支上界、未绑定等级变量

## 3. 运行时接线

- [x] 3.1 `grade-of` 内置扩展:返回等级表达式显示(Nat 数字/Var 符号/复合)
- [x] 3.2 擦除适配确认:Nat(>0)/Var/Add/Mul 等级不擦除,Zero 擦除不变;回归验证既有 QTT 测试
- [x] 3.3 运行时测试:grade-of 输出、擦除行为无回归

## 4. 示例与文档

- [x] 4.1 示例:`examples/dependent-linear-test.tisp`(向量长度级线性使用:通过型 + 违反型注释)
- [x] 4.2 `standard_doc/01-language-core.md`:§10 QTT 增补依赖等级语法与语义
- [x] 4.3 `standard_doc/04-implementation-status.md`:§10 QTT 更新(依赖等级已实现、Z3 集成未做);未实现清单同步
- [x] 4.4 `docs/spec.md` §10 状态同步;`CHANGELOG.md` 记录;README 测试数与示例同步
- [x] 4.5 最终验证:`cargo test --workspace` 全绿、`cargo check --workspace` 零警告、示例 `--typecheck`/`--run` 抽查
