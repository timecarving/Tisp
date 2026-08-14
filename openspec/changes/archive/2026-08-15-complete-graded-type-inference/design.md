## Context

`grade_check.rs` 已实现:0/1/ω 线性检查、依赖等级 r+s 传播(`collect_dependent_type_usage`)、`□_r A` 参数经 `effective_grade` 消去(用 r 作等级)、符号等级经 `grade_le` 返回 `None` 时收集 `GradeInequality`。`grades.rs` 已有 `grade_add/grade_mul/grade_le/check_cost_bound`。`z3_bridge.rs` 已有 `verify_implication(premises, conclusion) -> VerifyOutcome`(Unsat=恒真/Sat=反例/Unknown)与 `format_counterexample`。

两处占位待填:`type_infer::resolve_modal_grade`(死代码 + Var→ω)与 `liquid_verify::verify_grade_inequalities`(空 stub)。动机见 proposal.md - Why。

## Goals / Non-Goals

**Goals:**
- §11:按使用次数推导 `□_r` 的 r(引入规则),并接线到等级检查管线
- §19:符号等级不等式经 Z3 真实判定(verified / 明确警告带反例 / violated),替换静默 stub

**Non-Goals:**
- 不改 `grade_le` 的折叠能力(属 §10,已 ✅)
- 不做调用点等级实例化的全追踪(大特性;定义点判定即满足 spec「报错或明确警告」)
- 不改 spec 能力需求

## Decisions

1. **§11 使用感知推导**:新增 `resolve_modal_grade_with_usage(ty, &HashMap<Symbol,u64>) -> Type`。`ModalOp::Necessity(Grade::Var(v))` 若 `v` 在使用表中有计数,替换为 `Grade::Nat(count)`(Nat 精确计数);否则默认 `ω`。保留原 `resolve_modal_grade(ty)` 作空表委托(向后兼容)。在 `grade_check::check_def` 检查体后,用 `usage_env` 解析 `def.ty` 中的模态等级并存入 `GradeChecker::resolved_modal_types: HashMap<Symbol, Type>`,暴露给上层(反射/`--typecheck` 展示)。

2. **§19 结构化不等式**:`GradeInequality { grade: Grade, count: u64, span }`(替换 `var: String`)。收集处直接存 `other.clone()` 而非 `format!("{:?}")`。

3. **§19 Z3 判定语义**:对每条 `count ≤ grade`,翻译 `grade_to_smt(grade)` 为 SMT(自由变量声明 `Int`,并断言 `≥ 0` 自然数),用 `verify_implication([], "(<= count <grade_smt>)")`:
   - `Unsat` → 恒真 → `verified`
   - `Sat(model)` → 存在反例(等级欠约束)→ `warned` 并记 `errors` 一条「依赖等级 `grade` 未约束使用次数 `count`(反例 `<format_counterexample>`)」
   - `Unknown` 或 z3 不可用 → `warned`(降级)
   这条把「静默 `warned += 1`」升级为「可证恒真 / 明确反例警告」,是对 spec「明确警告(符号/不可判定时)」的兑现。

4. **`grade_to_smt` 放哪**:`tisp-backend`(`liquid_verify.rs` 内私有函数)——SMT 是后端关注点,与 `expr_to_smt` 同层;`tisp-middle` 不引入 z3 依赖。

## Risks / Trade-offs

- [§11 推导改变 `resolve_modal_grade` 签名] → 保留原函数委托空表,既有测试/调用不受影响
- [§19 每条不等式多一次 Z3 往返] → 不等式数量少(仅符号等级),可接受;z3 不可用走降级
- [自由等级变量 `count ≤ n` 恒有反例 n=count-1] → 这正是「明确警告」要告知用户的欠约束事实,非误报
