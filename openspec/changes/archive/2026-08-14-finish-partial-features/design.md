## Context

17 条「部分实现」的缺失半边已逐条代码审计定位(见 `standard_doc/04-implementation-status.md` 与本次探索报告)。关键现状:`FunAnnotation`(types.rs:52-58)是六维注解的天然载体却从未从源码构造;`@`(lexer.rs:21)与 `□` 被 lexer 标记/缺失但 parser 拒绝;一批「真引擎」是死代码(`infer_modes`、`inline_state_passing`、`logic.rs` 的 `SearchEngine`/`BfsSearchEngine`、`grades.rs` 的 `Semiring`、`constraint.rs` 的 `AbductiveEngine`、`hott.rs`);若干「已实现」实为计数器/占位/运行时内置。分层依赖关系决定了实现顺序:语法层(lexer/parser)是 items 1/7/8 的硬前置。

## Goals / Non-Goals

**Goals:**
- 17 条 ⚠️ 全部推到 ✅:每条既有可观测行为(可 `--typecheck`/`--run`/`--desugar` 验证),又有对应单元测试。
- 打通语法 → 中间 → 运行时三层:六维注解、`□_r`、`{n:T}`、`:semiring` 从词法一路进到推断与求值。
- 复活或重写死代码,消除「代码存在但功能不存在」的假象。

**Non-Goals:**
- 不触碰 10 条 ⬜ 仅设计项(属 `implement-design-stage-features`),仅与其中重叠的「类型族 rewrite」「HIT 端点」在此承接并协调归属。
- 不改变已 ✅ 的章节语义(§10/12/13/14/15/23/25/27 的已工作部分保持兼容)。
- 不引入新的外部依赖(继续使用 logos/z3/inkwell 等既有栈)。

## Decisions

### D1. 语法层前置,统一走 `FunAnnotation`

lexer 补 `□` 与 `@` 已在的标注处理;parser 为 `->[...]` 括号注解、`{n : T}` 隐式绑定、`:semiring` 关键字形式各加一条解析路径。desugar 把六维注解填进 `FunAnnotation`(types.rs:52-58),`CoreDef` 增加 `region` 字段对齐 ε/ρ/@r/m/d。**理由**:单一载体避免六维在 def 签名、类型箭头、分级应用三处各写一套;`FunAnnotation` 已存在,复用它最小侵入。**备选**:新增独立 `Annotation` 结构——否定,会与现有 `FunAnnotation`/`Param` 重复。

### D2. 死代码「复活优先,声明确为桩则重写」

- `mode_analysis.rs::infer_modes`、`logic.rs::SearchEngine/BfsSearchEngine`、`grades.rs::Semiring/Order`、`constraint.rs::AbductiveEngine`:接线复用(引擎逻辑已有,补调用点与数据结构贯通)。
- `hott.rs`:被解释器内联版本替代,改由解释器引用该模块或在模块内补齐端点求解后接线。
- `effect_compile.rs::inline_state_passing`:当前是空壳,重写为真实状态传递。
**理由**:死代码多为上一轮已完成却未接线,复活成本低于重写;仅空壳(如 `inline_state_passing`)才重写。

### D3. 回溯统一为「续延 + 选择点 + 逐分支隔离」

把 `interpreter.rs` 的单遍 `Search` 节点改为续延式:维护选择点栈 + trail 恢复,`Match` 每臂成功推入独立 `bound_snapshot`,`find-all`/`solve-all` 逐分支收集(替换全局 `collect_mode` 累加器)。`logic.rs` 的 `Goal`/`disj`/`conj` 流引擎作为底层接入。**理由**:这是 items 14/15/16 的共同地基,一次改造同时修复递归多解、or 多解、结构化统一与 CLP/ALP 的域回退。**备选**:在现有单遍逻辑上打补丁——否定,补丁无法解决「分支互相污染」与「结构化值折叠为 Int(0)」。

### D4. deriving 从运行时内置改为 desugar 代码生成

`eq-*`/`show-*`/`ord-*` 由 desugar 在解析时生成进 Core 程序,`--desugar` 可见;按构造器声明序 + 字段逐项实现;含函数字段或未知 trait 报错。**理由**:spec(§7.5)要求派生是编译期生成;当前运行时内置既不进 `--desugar`,也无法对不可派生类型报错。

### D5. 特化键从「字面值」改为「构造器类型」并接入 `--run`

`specialize.rs` 由 `Pattern::Lit` 匹配改为 `Pattern::Con`(构造器类型)+ 多参数组合;特化结果替换进 `core_program` 供 `--run` 执行(当前仅 `--typecheck` 展示)。**理由**:spec(§22.4)示例 `area(Circle) → area_circle` 本身就是类型驱动;字面值特化无法覆盖构造器分派。

### D6. Monad 降级:monadic 形式先行落地

desugar 新增 `mlet`/`get-m`/`put-m`/`pure` 语法并编到状态传递 `Do` 链;`detect_single_handler` 从「非空」改为「单处理器且无嵌套」的真判定,满足时走状态传递路径。**理由**:monadic 形式是 spec 的显式零开销路径,比在 handler 上做降级更易验证语义等价;两者共用同一状态传递后端。

### D7. 模块可见性:轻量导出表 + 可见性字段

`CoreDef` 增 `visibility` 字段;`ns` 解析保留 `:refer` 列表,加载时按导出表过滤;私有 `defn-`/`def-` 仅在源模块可见。**理由**:最小侵入满足 §6.5/§25.2,不重写整个模块系统。

### D8. 观察等价:进程项的迹/互模拟检查

在 `process.rs` 为演算项加归约迹等价比较(比较通道 I/O 迹或 barbed 等价),接在编码结果与原项之间;修复 `SKI::reduce` 丢 `K` 负载。**理由**:5 个编码补 3 个后,「保持观察等价」需要可执行检查而非文档声称。

## Risks / Trade-offs

- [语法层改动破坏既有解析] → 新增 token/分支保持向后兼容,`cargo test --workspace` 全绿门槛;每条语法改动配套 parser/desugar 测试。
- [续延回溯重写 Search 影响现有单解语义] → 默认 DFS 首解行为不变,多解仅经 `find-all`/`solve-all` 暴露;现有单解测试保持通过。
- [复活死代码引入隐藏耦合] → 每接线一处补一处测试,避免「接线但不验证」重蹈覆辙。
- [deriving 从内置移到 desugar 改变 `--desugar` 输出] → 属 spec 要求的可见性提升,更新受影响测试与示例预期。
- [特化接入 `--run` 改变执行路径] → 特化结果语义与运行时分发保持一致,以对拍测试(特化 vs 非特化输出一致)兜底。
- [模块可见性过滤可能破坏现有跨文件示例] → 默认导出全部公开定义(不显式 `:export` 即公开),仅私有 `defn-` 不可见,最小化破坏面。

## Open Questions

- 「类型族 rewrite」与「HIT 端点方程」与进行中变更 `implement-design-stage-features`(任务 1.1/2.3)的最终归属,需在实现前协调确认(不影响本变更的 spec/task 结构,仅影响执行时的代码合并)。
