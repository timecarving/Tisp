## 1. §11 模态等级按使用次数推导

- [x] 1.1 `type_infer::resolve_modal_grade_with_usage(ty, usage)` 推导 `□_r` 中 `Grade::Var` → `Nat(count)`,保留原函数委托
- [x] 1.2 `grade_check::check_def` 检查体后用 usage 解析 def.ty 模态等级,存 `resolved_modal_types`
- [x] 1.3 单元测试:推导 Nat(count) + 不可推断默认 ω + 递归嵌套解析

## 2. §19 符号等级 Z3 判定

- [x] 2.1 `GradeInequality` 改为 `{ grade: Grade, count: u64, span }`(替换 `var: String`)
- [x] 2.2 `liquid_verify` 新增 `grade_to_smt`(Var/Nat/Add/Mul/Zero/One/Omega)
- [x] 2.3 `verify_grade_inequalities` 真实 Z3 判定:Unsat→verified、Sat→明确警告带反例、Unknown/降级→warned
- [x] 2.4 单元测试:恒真(Add(Nat(5),Var n) 计数 3)→verified、欠约束(Var n 计数 3)→warned 带反例、降级路径

## 3. 文档与收尾

- [x] 3.1 `04-implementation-status.md` §11/§19 ⚠️→✅ + `docs/spec.md` 标题符号 + `PLAN.md`/`README.md`/`CHANGELOG.md` 同步
- [x] 3.2 `cargo check --workspace` 零警告 + `cargo test --workspace` 全绿
