## Context

当前大量特性已实现到「运行时模块 + 语义助手」层,但未贯通语言表面(lexer/reader/desugar/type_infer/effect/grade/interpreter/codegen),在 `standard_doc/04-implementation-status.md` 标注 ⚠️。本变更把这些特性**完全实现**到全链路可用。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 每个 ⚠️/⬜ 特性贯通全链路:源码可书写 → 类型/效应/等级正确 → `--run` 正确求值 → (可选)`--ir` 生成。
- 24 个范式(EVOLP/DLP/MOP + 12 逻辑 + 8 编程 + AOP)从 Tisp 源码端到端可用。
- 实现状态 ⚠️ → ✅,`standard_doc` 同步。

**Non-Goals:**
- 不改变既有 ✅ 特性语义。
- 不追求各特性生产级性能,以正确性 + 端到端可用为先。
- 不新增大范式(用户允许新特性,但本变更聚焦「把已有部分实现补齐」)。

## Decisions

### D1: 全链路接线统一模式

每个特性按六层接线,缺哪层补哪层:

| 层 | 机制 | 落点 |
|----|------|------|
| lexer/reader | 识别关键字/语法形式 | `lexer.rs`/`reader.rs` |
| desugar | 生成 `CoreExprNode`/`CoreDef` | `desugar.rs` |
| type_infer | 类型构造/推断臂 | `type_infer.rs` |
| effect/grade | 效应行/等级检查臂 | `effect_infer.rs`/`grade_check.rs` |
| interpreter | 求值分支 | `interpreter.rs` |
| codegen | LLVM 降级(可选) | `codegen.rs` |

**理由**:统一接线模式使每项补齐可独立推进、可独立验证。
**备选**:逐项特化实现(被否——无统一模式,难审计「是否全链路」)。

### D2: ⚠️ 特性精确落点

| 特性 | 关键接线点 | 完成判据 |
|------|-----------|----------|
| 六维注解 `->[ε,ρ,@r,m,d]` | desugar 解析 → `FunAnnotation` | `--desugar`/`--typecheck` 保留六维 |
| 私有定义 `defn-` | desugar 写 `visibility` + ns 过滤 | 跨文件私有不可见 |
| deriving Ord | desugar 生成 `ord-*` | `:deriving Ord` 产出排序函数 |
| □_r/◇_ε 推理 | type_infer 补 Modal 引入/消去 | `(□_n a)` 分级信息进入类型 |
| Cost 全推导 | desugar `@Cost` + grade_check 渐近复合 | `@Cost` 超上界报错 |
| 完整立方填充 | hott.rs HComp 扩 N 维 | 多维面组合一致/不一致报错 |
| Cohesive 同伦 | hott.rs ♭/♯/ʃ adjoint 语义 | 模态组合符合 adjoint-triple |
| □_t 语义保证 | type_infer 稳定类型 + temporal 生产率 | 非稳定类型跨时刻报错 |
| 依赖等级 r+s | grade_check 有限等级传播 | 有限 r/s 组合求解(非 ω 恒过) |
| 区域逃逸 | region_infer 作用域判定 | with-region 退出后指针不可用 |
| inkwell 闭包 | codegen define/call + 环境打包 | llc 编译通过 |
| 范式全链路 | reader/desugar/type/interpreter 经 `ParadigmRegistry` | 范式源码 `--run` 正确 |

### D3: 范式全链路接线(复用 ParadigmFacility)

24 个范式经既有的 `ParadigmFacility`/`ParadigmRegistry`(`tisp-runtime/src/facility.rs`)接入:
reader 识别 `keyword` → desugar 生成范式 `CoreExprNode` → type_infer 用 `type_con` → effect_infer 用 `effects` → interpreter 经 `eval` 分发。
**理由**:复用上一轮建好的可接入接口,补齐 reader/desugar/type/interpreter 四层即可全链路。
**备选**:每范式硬编码分支(被否——破坏可接入性)。

### D4: ⚠️→✅ 升级判据

一项特性从 ⚠️ 升级 ✅ 必须同时满足:①有源码级用例经 `--typecheck` 通过;②`--run` 结果正确;③有单元测试;④`standard_doc` 状态与 `file:line` 证据更新。
**理由**:避免「接了一层就标 ✅」,确保「全链路可用」名副其实。

## Risks / Trade-offs

- [24 范式 + 10+ 项特性全接线工作量大] → 按 D2 表逐项推进,每项独立合入,不追求一次性完成。
- [inkwell 闭包真代码生成复杂] → 先函数 define/call 真生成,闭包环境用 display 间接层逐步补,以 llc 通过为准。
- [依赖等级 r+s 对符号等级不可判定] → 可判定情形求解、不可判定明确警告放行(与 spec 一致)。
- [逐项升级可能触碰既有 ✅ 逻辑] → 每项改动最小化 + 全量回归测试把关。

## Migration Plan

逐项增量接线,每项完成即更新状态与测试;全程 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。无破坏性迁移。

## Open Questions

- inkwell 闭包环境是否用堆分配 display 层还是内联结构:可延后,先以 llc 通过 + 语义一致为准。
- Cohesive 完整同伦模型是否需要引入无穷范畴(∞-groupoid)表示:可延后,初版以 adjoint-triple 的模态组合语义为准。
