# 液态类型(Z3)完整化 — 任务清单

规范依据:`specs/liquid-types/spec.md`;方案依据:`design.md`。

## 1. 依赖清理与桥基础

- [x] 1.1 从 workspace `Cargo.toml` 移除 `z3 = "0.12"` 依赖,从 `tisp-backend/Cargo.toml` 移除 `z3` feature 与可选依赖声明;`cargo check --workspace` 零警告
- [x] 1.2 确认 `Z3Bridge::new()` 探测失败路径(spawn 错误返回 Err),补单元测试:z3 存在时 declare/assert/check-sat 往返可用

## 2. 前端解析(desugar)

- [x] 2.1 desugar 解析精化类型 `{x : T | pred}` → `Type::Refined`(参考 §8.2 refined 模式已有 `:`/`|` 语法处理);`--desugar` 输出保留 Refined 节点与谓词
- [x] 2.2 `defn` 解析 `:requires`/`:ensures` 写入 `CoreDef.requires/ensures`;多个 `:requires` 合取为 `Predicate::And`;`result` 保留为 ensures 中的变量名
- [x] 2.3 desugar 单元测试:精化类型注解与多契约定义输出正确(参照 `examples/liquid-types-test.tisp` 现有用例)

## 3. 谓词与表达式 → SMT 翻译器

- [x] 3.1 实现 `Predicate → Option<String>`(SMT-LIB2):比较/算术(+ - * / %)/and/or/not/implies/forall/exists(声明 Int 绑定变量)/abs/一元负号/positive?/neg?/even?/odd? 展开/整数与布尔字面量;未知谓词函数返回 None
- [x] 3.2 实现 `CoreExprNode → Option<String>`(实参与函数体):字面量/变量/算术/`if → ite`;含用户函数调用、effect、非 i64 类型返回 None
- [x] 3.3 翻译器单元测试:嵌套算术、量词、ite、不可翻译边界(返回 None)

## 4. Z3Bridge 扩展

- [x] 4.1 新增 `verify_implication(premises: &[String], conclusion: &str) -> Result<VerifyOutcome>`:`push` → 声明自由变量(收集翻译项中标识符)→ 断言前提与 `(not conclusion)` → `check-sat`;返回 `Sat(model)/Unsat/Unknown`,失败路径 `pop` 恢复
- [x] 4.2 反例格式化:从 `get_model` 提取整数绑定,输出 `x = -1, d = 0` 风格字符串
- [x] 4.3 单元测试(z3 二进制存在时运行):蕴含通过、反例提取、unknown 路径

## 5. LiquidVerifier 驱动(liquid_types.rs)

- [x] 5.1 新增 `LiquidVerifier`:构造时探测 z3(spawn 失败 → 降级模式);`LiquidReport { verified, violated, warned, degraded }`;降级模式沿用 `LiquidChecker::check_predicate` 常量折叠
- [x] 5.2 调用点验证:遍历调用,实参项 ⇒ 参数精化谓词;违反 → 错误(span = 实参位置 + 反例);不可翻译 → warned
- [x] 5.3 返回精化验证:函数体(if→ite)⇒ 返回精化;违反 → 错误(span = defn 位置 + 反例)
- [x] 5.4 契约验证:`requires 合取 ∧ body` ⇒ `ensures`(result 绑定为函数体项);违反 → 错误(span = 契约注解位置 + 反例);unknown → warned
- [x] 5.5 单元测试:三类验证项的通过与违反、反例消息格式、降级模式计数

## 6. CLI 接线

- [x] 6.1 `--typecheck` 在现有分析(类型/效果/等级/模式/确定性/区域/优化)后运行 `LiquidVerifier`,打印统计与违反明细;存在违反时以非零退出码返回错误
- [x] 6.2 REPL `:type` 不触发求解(保持轻量);`--typecheck` 无 z3 时输出降级提示

## 7. 测试与示例

- [x] 7.1 更新 `examples/liquid-types-test.tisp`:通过型用例全跑通(合法调用、契约满足、返回精化满足);移除「仅解析不检查」注释
- [x] 7.2 新增负面用例(违反参数精化/返回精化/requires/ensures 各一,注释说明预期报错),用 `--typecheck` 验证报错与反例
- [x] 7.3 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告

## 8. 文档同步

- [x] 8.1 `CHANGELOG.md` 记录:精化类型/契约解析、Z3 验证、z3 crate 依赖移除
- [x] 8.2 `standard_doc/01-language-core.md` 与 `standard_doc/03-reference.md` 增补精化类型与契约语法(若已有章节则更新)
- [x] 8.3 `standard_doc/04-implementation-status.md` 第 15 章 ⚠️ → ✅(含 file:line 证据)
- [x] 8.4 `README.md` 实现状态概览同步(液态类型从「部分实现」移入「全链路可用」)
