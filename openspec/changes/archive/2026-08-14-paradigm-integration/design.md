## Context

前两轮把 12 逻辑范式(`tisp-runtime/src/paradigms.rs`)、8 编程范式(`programming.rs`)、AOP(`aop.rs`)与 ⚠️ 补齐助手(`full_chain.rs`)实现为自包含运行时模块 + 内联测试。这些是**语义演示**:与类型/效应/值系统、与其他特性之间无接入接口。本变更把这些模块升级为**一等设施**,补齐接入接口并贯通全链路。动机与「组合 = 可接入(非语义自举)」的澄清见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 每个范式(12 逻辑 + 8 编程 + AOP)升级为一等 Rust 设施,暴露统一集成接口。
- 定义统一接入抽象,使范式经值/类型/效应/等级与其他特性插接。
- 贯通全链路:源码 → desugar → 类型/效应/等级 → 求值,`--run` 端到端可用。

**Non-Goals:**
- 不重写范式语义(稳定模型求解、DFA 识别、数组归约等语义保留,仅升级接口与接线)。
- 不追求各范式的生产级性能;以正确性 + 可接入为先。
- 不改动既有已 ✅ 特性语义。

## Decisions

### D1: 统一接入抽象

以既有 `Value`(求值)、`Type`(类型)、`EffectRow`(效应)、`Grade`(等级)作为唯一接入抽象。所有范式值/类型/效应/等级都落到这四个抽象上,不另立平行体系。
**理由**:四个抽象已贯穿 compiler 全链路,范式接入它们即天然获得类型检查、效应追踪、等级约束。
**备选**:每范式独立值/类型体系(被否——无法跨范式组合,违背可接入目标)。

### D2: 范式设施 trait(可接入接口)

在 tisp-runtime 定义统一 trait,每个范式实现之:

```rust
pub trait ParadigmFacility {
    fn keyword(&self) -> &'static str;      // 语法形式关键字(接入 reader/desugar)
    fn type_con(&self) -> TypeCon;          // 类型构造器(接入 Type::Con 与 HM 推断)
    fn effects(&self) -> Vec<EffectLabel>;  // 效应操作(接入效应行)
    fn eval(&self, args: &[Value]) -> Result<Value, String>; // 求值(接入 interpreter)
}
```

21 个范式各提供一个实现;desugar/type_infer/effect_infer/interpreter 经该 trait 分发,而非硬编码分支。
**理由**:trait 即「可接入接口」——新特性只需实现 trait 即可插接;替换「语义演示」的散落 free fn。
**备选**:巨型 match 分发(被否——不可扩展,无法体现可组合)。

### D3: 全链路接线映射

| 层 | 接入方式 |
|----|----------|
| reader/lexer | 识别范式关键字(如 `dfa`/`state-machine`/`sym`/`array`)为特殊形式 |
| desugar | 生成新增 `CoreExprNode` 变体(如 `AutomatonDef`/`ArrayExpr`/`SymExpr`) |
| type_infer | 经 `ParadigmFacility::type_con` 接入 `Type::Con`,参与 HM 推断 |
| effect_infer | 经 `effects()` 接入效应行;副作用声明为 State/Search/Signal |
| interpreter | 经 `eval()` 分发求值,范式值以 `Value` 表示 |

**理由**:各范式贯通完整管线,兑现「全链路可用」;分发表由 trait 统一驱动。
**备选**:仅 interpreter 接线(被否——无法类型/效应检查,非端到端)。

### D4: 副作用统一接入代数效应/单子

范式副作用映射到既有效应:`State`(栈顶/状态机态/数组缓冲)、`Search`(自动机回溯/逻辑搜索)、`Signal`(流/FRP)。单处理器路径复用 §12.6 直接状态线程降级。
**理由**:兑现「效应是万能胶」,范式副作用与既有效应共享同一套类型/效应检查。

### D5: 与既有模块的关系(升级而非丢弃)

把 `paradigms.rs`/`programming.rs`/`aop.rs`/`full_chain.rs` 的纯函数语义保留为范式求值器内核,外层包裹 `ParadigmFacility` 实现 + 接线;不重写已通过的语义与测试。
**理由**:复用已验证的语义,仅补接口与接线,降低回归风险。

## Risks / Trade-offs

- [21 个范式全接线工作量大] → tasks 按「trait + 接线框架 → 逐范式接入 → 端到端测试」分层,每层独立可测可合入。
- [范式类型接入 HM 推断复杂] → 范式类型用具体 `Type::Con`(非多态),先静态标注、后逐步加推断。
- [效应行爆炸] → 每个范式声明最小效应集合,未用效应不进入行。
- [升级可能破坏既有模块测试] → 保留原 free fn 语义,仅新增 trait 实现与接线,既有测试继续绿。

## Migration Plan

增量升级:先建 `ParadigmFacility` trait 与接线框架,再逐个范式包裹实现并接线,最后补端到端测试。每步保持 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。无破坏性迁移。

## Open Questions

- `ParadigmFacility::eval` 是否返回 `Value` 还是引入 `ParadigmValue` 枚举:可延后,初版返回 `Value` 复用既有表示即可。
- 各范式是否都需要 LLVM codegen 降级:可延后,spec 仅要求 `--run` 端到端,LLVM 降级后续按需补。
