# 液态类型(Z3)完整化 — 设计

## Context

动机与范围见 `proposal.md`;可观察行为要求见 `specs/liquid-types/spec.md`。现状关键事实(2026-08 实测):

- `Type::Refined(base, pred)` 与 `Predicate`/`Term` 结构完整(`crates/tisp-core/src/types.rs`)
- `LiquidChecker`(`liquid_types.rs`)仅做常量折叠,变量谓词与未知谓词一律 `Ok(true)` 放行;`TypeInfer::verify_refinements` 用它,`predicate not satisfied` 错误无 span、无反例
- `Z3Bridge`(`z3_bridge.rs`)是 SMT-LIB2 外部进程桥(declare/assert/push/pop/check-sat/get-model),**无任何调用方**(死代码),且未 feature 门控
- `z3 = "0.12"` crate 依赖存在但从未 `use`;`tisp-backend` 有 `z3` feature
- desugar 不解析 `:requires`/`:ensures`(`CoreDef` 字段恒 `None`);`{x : T | pred}` 精化语法未解析
- 项目约定:LLVM/Z3 相关代码 feature 门控、勿破坏默认构建、零警告、简体中文注释

## Goals / Non-Goals

**Goals**

- 打通「精化类型/契约 → 谓词 → SMT → 求解 → 诊断」完整链路,验证失败为编译错误(带反例)
- 无 z3 二进制时行为与现状兼容(仅常量折叠),不因缺求解器报错
- 保持 `cargo check --workspace` 零警告、默认构建可用

**Non-Goals**

- 完整的 Liquid Type Inference(谓词自动推断,§15.2 的「编译器自动验证分支」仅以 if→ite 路径敏感方式覆盖,不做谓词综合)
- 精化类型参与合一(合一仍按基础类型展开 Refined,避免类型系统行为扩大)
- 浮点/字符串精化(第一版仅整数域 i64 与布尔)
- 求解性能工程(无缓存/增量复用,标注为后续优化)

## Decisions

### D1:Z3 接入 — 保留外部进程桥,移除未用的 `z3` crate 依赖

现状 `Z3Bridge` 已是外部 `z3` 二进制 + SMT-LIB2(stdin/stdout),零编译期依赖、可运行时探测;`z3` crate 从未使用,且绑定需要 C++ 库链接——与 llvm-sys 的静态库教训相同(项目已吃过亏,见 Cargo.toml 注释)。

**方案**:删除 workspace 与 tisp-backend 中的 `z3` 依赖及 `z3` feature(它当前没门控任何真实代码);`Z3Bridge` 保留为无条件编译,构造时探测失败返回错误,调用方降级。
**备选**:改用 `z3` crate(Rust 绑定)。否决:链接复杂度高、引入与现有桥重复的实现、无行为收益。
**备选**:完全移除 Z3、只做常量折叠。否决:spec 要求真实求解。

### D2:验证架构 — 独立 `LiquidVerifier` 驱动,不改 type_infer 主干

`TypeInfer` 中的 `liquid_checker` 字段保留(常量折叠路径,现有 `verify_refinements` 行为不变,作为降级模式)。新增验证驱动模块(扩展 `liquid_types.rs`):

- 输入:`CoreProgram` + 类型推断产物(每个 def 的签名类型)
- 三种验证项,统一「可翻译才验证,否则警告放行」:
  1. **调用点**:实参表达式翻译为 SMT 项 ⇒ 参数精化谓词
  2. **返回精化**:函数体翻译(if → `ite`,路径敏感)⇒ 返回精化谓词
  3. **契约**:`(requires 1 ∧ … ∧ requires n ∧ body约束)` ⇒ `ensures`(result 绑定为函数体翻译项)
- 输出:`LiquidReport { verified, violated, warned, degraded }`,CLI 打印统计与每个违反项(span + 反例)

**备选**:把验证逻辑塞进 `TypeInfer` 的检查点。否决:type_infer 已 600+ 行,SMT 逻辑与其解耦;CLI 顺序控制更清晰。

### D3:翻译子集与验证语义

**谓词→SMT**(`Predicate` → SMT-LIB2 字符串):比较(> < >= <= = !=)、算术(+ - * / %)、布尔连接(and/or/not/implies)、量词(forall/exists,声明绑定变量)、已知一元函数(abs、一元负号、positive?/neg?/even?/odd? 展开)、整数/布尔字面量。未知谓词函数 → 不可翻译 → 警告放行。

**表达式→SMT**(实参/函数体):字面量、变量、算术、`if → ite`、已知纯函数。含调用其他用户函数、effect、非整数类型 → 不可翻译 → 警告放行。

**验证语义**:对谓词 P(自由变量 x₁…xₙ)验证恒真 ⟺ `push; (assert (not P)); check-sat`:
- `unsat` → 通过
- `sat` → `get-model` 提取反例,格式化为 `x = -1, d = 0` 附进错误消息
- `unknown` → 警告放行(不误报)

调用点蕴含:`push; (assert (not (=> 实参约束 参数精化))); check-sat`——实参翻译项即约束,无需绑定具体值。

### D4:降级策略(无 z3 / 不可翻译)

`LiquidVerifier` 构造时探测 z3:`Command::new("z3")` spawn 失败 ⟹ 降级模式,所有验证项跳过,`report.degraded` 计数,CLI 输出「z3 求解器不可用,液态验证降级为常量折叠」。常量折叠沿用 `LiquidChecker::check_predicate`(现状行为,`TypeInfer` 内不变)。降级模式下 `:requires`/`:ensures` 已解析但仅常量折叠可判时报告。

### D5:错误诊断

违反项经 miette 输出,错误消息格式:`精化/契约违反: <谓词>,反例: x = -1` + span(实参位置/返回位置/defn 契约位置)。`verify_contract` 现有 `Span::dummy()` 改为真实 span(defn 的契约注解位置)。

## Risks / Trade-offs

- [z3 二进制缺失或版本差异] → 构造探测 + 降级模式 + CLI 提示;文档注明 `apt install z3`
- [SMT 翻译不完备导致漏检(未知函数放行)] → 有意的保守选择(不误报);警告可见,后续可扩充函数符号表
- [求解性能:每个调用点一次 `push/check-sat/pop`] → 第一版可接受;缓存同形状验证项列为后续优化(不阻塞)
- [求解器返回 unknown 误判] → 按「未验证」警告处理,绝不据此报错
- [移除 `z3` crate 依赖改 Cargo.lock] → 无代码使用,仅锁文件与 Cargo.toml 删除,风险为零
- [误报:验证合法程序失败] → 仅「可翻译子集」参与验证,失败需给出反例,可人工复核

## Migration Plan

无部署概念。落地顺序:依赖清理 → desugar 解析(精化类型 + 契约)→ 翻译器 → Z3Bridge 扩展与验证驱动 → CLI 接线 → 测试与示例 → 文档(CHANGELOG、standard_doc/01、04-implementation-status §15 ⚠️→✅)。回滚:git revert 对应提交即可。

## Open Questions

- 无。所有可延后问题(缓存、浮点支持、谓词综合)均为后续优化,不影响本变更的 spec/方案/任务拆分。
