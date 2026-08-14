## Why

唯二仍标 ⚠️ 的两章:§11 Graded Modal Types 的「完整可推断情形推导(按使用次数推导 r/ε)」与 §19 Dependent Graded Types 的「符号等级判定(严格 Z3 验证)」尚未落地。当前:

- `type_infer::resolve_modal_grade` 是**死代码**(只自递归 + 测试,未接线到管线),且 `Grade::Var` 一律默认 `ω`,不做按使用次数的引入推导;
- `liquid_verify::verify_grade_inequalities` 是**空 stub**(`warned += 1`),`GradeInequality.var` 存的是 `{:?}` 调试字符串,符号等级「警告放行」实为静默丢弃,未真正经 Z3 判定。

本变更把这两处从「近似/占位」升级为真实推导与真实 Z3 判定,使 §11/§19 全链路可用(源码 → 类型/等级推断 → Z3 判定 → 求值),升为 ✅。

## What Changes

- **§11 引入推导**:`resolve_modal_grade` 增加按使用次数的推导——`□_r A` 中 `Grade::Var` 从实际使用计数推导为 `Nat(count)`(Nat 代数精确计数);`◇_ε A` 的 ε 即效应行(已具体)。接线到 `grade_check` 管线(计算使用后解析模态等级)。
- **§19 Z3 判定**:`GradeInequality` 从 `var: String` 改为携带结构化 `Grade` 表达式 + 使用次数;`verify_grade_inequalities` 用 Z3 判定每条 `count ≤ grade`:可证恒真 → verified,存在反例(等级欠约束)→ 明确警告并给出反例,矛盾 → violated。替换空 stub。
- **等级→SMT 翻译**:新增 `grade_to_smt`,把 `Grade`(Var/Nat/Add/Mul/Zero/One/Omega)翻译为 SMT 表达式(自由变量声明为 `Int ≥ 0`)。
- **测试与文档**:§11 推导 + §19 Z3 判定单元测试;`04-implementation-status.md` §11/§19 ⚠️→✅;`docs/spec.md`/`PLAN.md`/`README.md`/`CHANGELOG.md` 同步。

## Capabilities

(无新增/修改能力——本变更为既有 `type-system-extensions`/`dependent-linear-types` 需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。)

## Impact

- **tisp-middle**:`type_infer.rs`(`resolve_modal_grade` 使用感知推导)、`grade_check.rs`(`GradeInequality` 结构化 + 接线推导)
- **tisp-backend**:`liquid_verify.rs`(`verify_grade_inequalities` 真实 Z3 判定)、`z3_bridge.rs`(复用)
- **tisp-core**:`grades.rs`(可能补 `grade_to_smt` 或放 backend)
- **文档**:`standard_doc/04-implementation-status.md`、`docs/spec.md`、`PLAN.md`、`README.md`、`CHANGELOG.md`
