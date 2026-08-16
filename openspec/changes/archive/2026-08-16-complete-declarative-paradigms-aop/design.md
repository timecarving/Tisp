## Context

8 类范式已有宿主 Rust 实现（programming.rs/aop.rs/facility.rs），但解释器公开路径仍主要是 `pf-*` 简化投影；`comptime` 只有 `CoreExprNode::Comptime` 语法节点；MOP 有 GetKB/SetKB 运行时效应与编译期占位；AOP 无真实切面声明与编译期编织。前一变更已建立统一静态检查管线、范式设施六维元数据与 `RegionBox` 内存体系，本变更只在其上继续（动机见 proposal.md - Why）。

## Goals / Non-Goals

**Goals:**

- 8 范式全部经源码表面 → Core AST → type/effect/grade 检查 → 解释执行；副作用只经代数效应/单子表达。
- `pf-*` 与完整内置语义一致（同一实现或显式别名），旧简化投影不再作为公开正确性来源。
- comptime 在 desugar 之后、静态检查之前编译期求值，并把结果写回 CoreProgram；`--desugar`/`--typecheck`/`--run` 看到同一内联结果。
- AOP 切面以 comptime 纯声明式 MOP 在编译期编织 OOP 方法链，`call-next-method` 语义保持，特化路径等价。

**Non-Goals:**

- 不把 8 范式重写为生产级数组/自动机库（保持教学级但语义自洽、错误显式）。
- 不给 codegen 增加 8 范式/切面的 LLVM 形态（仍走解释器路径）。
- 不改变 comptime 之外运行时的 MOP 语义（GetKB/SetKB 运行期继续可用）。

## Decisions

### D1 源码表面 = CoreExpr 节点 + 类型化内置

每个范式提供专用 CoreExprNode 变体或受控内置集，由 desugar 统一映射；type_infer/effect_infer/grade_check 用与设施元数据一致的签名表接线（沿用 `ParadigmFacility::signature()`）。状态类范式值使用分级句柄类型（`Stack a`、`SM`、`Table a`），复用上一变更的句柄检查。数组/符号/自动机保持纯数据值。
*备选*：全部做成字符串内置——被否：无法进入效应行与等级检查。

### D2 副作用只经效应/单子

Stack/SM/DataDriven 操作注册为 `State` 效应操作（stack-new/push/pop/peek/dup/swap/rotate、sm-new/sm-drive、table-new/table-dispatch）；Stream 节点注册 `Signal`；Array/Sym/DFA 为 Pure。interpreter 对这些操作走 `perform_effect`/handler 栈；`pf-*` 改为调用同一实现的别名。纯代码未经 handler 调用状态操作被 effect_infer 拒绝（复用前一变更的 Unsafe 门控模式，扩展到 State/Signal）。
*备选*：把状态藏在解释器全局——被否：违反纯声明式约束与效应行审计。

### D3 单子降级真实状态线程

`effect_compile` 增加对单处理器 State handler + `mlet/get-m/put-m/pure` 的降级重写：把 `mlet` 链改写为显式状态参数传递（直接状态线程），interpreter 的 `direct_state` 槽承接；多处理器保持 handler 语义。`--run` 输出降级数量与结果等价性断言测试。
*备选*：只检测不改写——被否：spec 明确要求真实路径，前次只做到计数。

### D4 comptime 作为 backend 编译期 pass

`tisp-backend` 新增 `ComptimePass`：遍历 CoreProgram 中的 `Comptime` 节点，用受限 Interpreter 求值并把结果替换进 AST（字面量/Data/闭包可内联；无法内联的值保留为运行时构造并记录警告/错误）。CLI 在 desugar 后立即运行 pass，`--desugar` 与 `--typecheck` 共用。pass 持有编译期 KB（`tisp_core::evolp::Program`），comptime 内 `get-kb/set-kb` 直接读写该 KB。
*备选*：frontend 内嵌解释器——被否：frontend 不依赖 backend，会形成职责倒置；backend 做 pass 与现有依赖方向一致。

### D5 编译期 MOP 知识库与运行时分离

ComptimePass 的 KB 独立于 Interpreter 运行时 KB；编译期写入通过 `CoreProgram` 的 pragma/元数据回传（本变更先支持「同单元后续编译可见」：pass 内部 KB 驱动后续 aspect 编织与宏式查询；跨文件编译期 KB 留作后续）。运行时不读编译期 KB。
*备选*：共享同一 KB——被否：违反编译期/运行时分离 spec。

### D6 AOP 编织 = desugar 声明 + comptime pass 重写

`(defaspect name (pointcut Gen) [:around|:before|:after] body)` 脱糖为 `AspectDef` 节点。ComptimePass 在完成 comptime 求值后执行编织：对每个 pointcut 命中的 GenericDef，按其 MethodDef 集合生成新的方法链（around 注册序包裹 + call-next-method 绑定到内层组合），写回 CoreProgram；`--desugar` 因此可见。specialize 对已编织的 primary-only 泛型继续特化；含组合链的泛型保持运行时分发（沿用保守策略）。
*备选*：运行时切面表——被否：spec 要求编译期完成、运行时无动态反射。

## Risks / Trade-offs

- [comptime 求值可能引用尚未定义的函数] → 求值失败即编译错误；允许引用已处理的先行定义（按 defs 顺序分两遍：先声明再求值）。
- [effect_compile 降级重写破坏 handler 语义] → 只对单处理器 + monadic 风格重写；每类先加等价性测试。
- [pf-* 别名语义变化影响旧示例] → 完整内置接管后保留别名并跑全量示例回归；对无法保留的投影形式显式报错（**BREAKING** 记录 CHANGELOG）。
- [AOP 编织与特化交互] → 编织先于 specialize；特化器复用「含非 primary 不特化」规则，等价性回归锁死。
- [compile 期 KB 范围] → 明确本变更仅同单元可见，跨文件留后续，避免无限范围蔓延。

## Migration Plan

1. 按 tasks.md 顺序实施；每个阶段提交 `feat: ...`。
2. 验收：`cargo test --workspace` 全绿零警告、8 范式 + AOP/MOP 示例矩阵、`--desugar` 可见 comptime 内联与切面编织。
3. 回滚：分阶段 git revert；不引入外部依赖与迁移文件。

## Open Questions

无——上述取舍已收敛，不影响 spec 与任务分解。
