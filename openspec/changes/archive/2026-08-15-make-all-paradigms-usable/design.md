## Context

Tisp 0.1.0 的六 crate 管线（core/frontend/middle/backend/runtime/cli）已具备声明式基底（统一 def+六维注解、效果系统、QTT/液态类型）与 21+ 范式设施，但检测发现发布路径存在「声明完成、实际不可用」的缺口：`--run` 不做静态检查、`--verify` 与输入无关、`--eval` 不求值、lambda 无返回注解、FFI 按错误 ABI 试探导致段错误、会话语法丢 payload、specializer 丢 around、`ns :as` 无效、反射返回运行时标签、HoTT 占位 panic。设计目标是把每个范式拉过「静态类型 + 纯声明 + 统一内存」的可用门槛（动机与范围见 proposal.md，验收见 specs/）。

## Goals / Non-Goals

**Goals:**

- 建立唯一管线入口：`读取 → 脱糖 → 静态检查 → （可选）执行/IR`，`--typecheck`/`--run`/`--eval`/`--verify`/`--compile` 全部经过它。
- 让每个范式有「类型可查、效应可推断、内存受 Unsafe+依赖线性+QTT+分级线性四支柱约束、失败显式、示例全绿」的端到端证据。
- 保持 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告、默认构建无 LLVM/Z3/FFI 硬依赖。

**Non-Goals:**

- 不把解释器 `Value` 树整体改造成 arena/region 存储（超出本变更，且破坏不可变语义）。
- 不实现完整 C ABI 解析器、不接入 inkwell JIT ExecutionEngine（`--compile` 走 llc/clang 编译运行闭环）。
- 不把 12 逻辑范式/8 编程范式的宿主求解器重写为生产级 Prolog/概率推理器；只要求语义自洽、错误显式、可端到端验证。
- 不新增第三方 crate；继续使用既有依赖。

## Decisions

### D1 统一管线（cli 层）

`compile_file` 抽取 `run_static_checks(&CoreProgram) -> CheckReport`，内部依次执行现有 type/effect/grade/mode/det/region/liquid 检查并聚合冲突（复用 `ConstraintSolver` 的共享图报告）。`--run`、`--eval`、`--verify`、`--compile` 在解释/生成 IR 前必须通过检查；`--typecheck` 只打印报告。
*备选*：在 `Interpreter::run_program` 里做检查——被否：backend 不应反向承担 middle 管线编排，且 `--eval` 也要复用。

### D2 lambda 返回注解（frontend）

`desugar_lambda` 在 params 后扫描可选的 `->` / `->[ε,ρ,@r,m,d]` 子句：存在则解析返回类型与六维注解并写入 `Lambda.ret_type`（现为 `None`），body 从注解之后收集。解析逻辑复用 `desugar_defn_form` 的现有箭头解析（`desugar_six_dim_annotation`）。type_infer 的 `Lam` 分支在 `ret_type.is_some()` 时把函数结果与注解统一。
*备选*：把带注解 lambda 脱糖为 `defn`——被否：会产生顶层定义副作用，改变作用域语义。

### D3 推断签名回填（middle → backend 边界）

类型/效果/等级/模式/确定性推断完成后，CLI 用 `apply_checked_signatures(&mut CoreProgram)` 把推断结果写回各 `CoreDef` 的 `ty/effects/mode/determinism/param grades`，再进入执行路径。解释器现有 `def_sigs` 读取机制即可让 `type-of "add"` 返回真实推断类型。显式标注以推断后最终类型为准。
*备选*：给 `Interpreter::run_program` 加签名参数——被否：改 API 面更大，回填 CoreDef 对 IR/优化器也有益。

### D4 FFI ABI 安全分派（frontend + backend）

`defextern` 增加可选 `:abi` 关键字（`i64->i64`、`f64->f64`、`str->i64`、`str->str`、`ptr->i64`），无 `:abi` 时默认 `i64->i64` 且运行时校验实参为 `Int`。loader 按**唯一声明签名** `lib.get::<fn(...)->...>` 解析调用，取消「先试 i64」的盲试。默认构建（无 `ffi` feature）对带库路径的调用报「未启用 ffi feature」；模拟函数表仅保留显式已知符号（abs/strlen/sqrt）并输出一次性警告，未知符号报错，不再恒等回退。
*备选*：从 `defextern` 的类型参数推导 ABI——被否：desugar 现有类型参数是空 vec，补全签名语法成本更高；`:abi` 最小且显式。

### D5 会话与结构化并发（backend）

`Session(Send/Recv/Close, e)` 不再维护全局字符串状态：把 e 求值为通道名，`Send` 写入真实通道（携带 payload），`Recv` 读取真实通道，协议状态改存 `HashMap<channel_id, Expectation>`，每通道独立；type_infer 同步把 `session_state` 改为按通道 id 键控并检查首操作。`ProcessRuntime::recv` 改为阻塞等待（Mutex+Condvar），`close` 唤醒并置终态。`Spawn` 保存 `JoinHandle` 到解释器，`Join` 真正 join 并返回子结果/传播错误。`recv!`/`async-recv` 复用同一实现（后者仍非阻塞）。
*备选*：保留非阻塞 recv 并给示例加轮询——被否：竞态不可接受（spec 明确要求）。

### D6 特化保持方法组合语义（middle）

`specialize.rs` 在目标泛型函数存在任何非 Primary 方法（around/before/after）或方法体引用 `call-next-method` 时，该泛型调用保持运行时分发（不生成可能丢组合链的特化副本）；纯 primary 的调用继续 monomorphize。先加「特化前后结果等价」回归测试锁定 100 vs 50 的回归。
*备选*：特化器生成完整组合链副本——被否：实现复杂且收益小，保守回退已满足 spec 的行为等价要求。

### D7 用户程序验证（cli + backend）

`--verify` 改为：对 `CoreProgram` 中的每个 `TheoremDef` 属性执行求值（复用 `verify` 内置），输出每个属性的成立/不成立与证据；新增 `model-check` 内置（init、goal 谓词、next 函数、max-depth 四参），用现有 `ModelChecker` 对 `Value` 状态做可达性搜索并返回 trace。用户用 `defprop` + `model-check` 声明任意协议/状态模型；无属性时报错。`find-attack` 改为接收用户提供的协议参数（消息/机密/动作列表），不再内置唯一场景。
*备选*：引入新验证 DSL——被否：`defprop` 已存在，扩展内置即可覆盖可达性与 dolev-yao 演示级验证。

### D8 --compile 编译运行闭环（cli）

`llvm` feature 下：`--ir` 输出 IR；`--compile` 生成 IR 后调用 `llc-17 -filetype=obj` 与 `clang-17` 链接临时可执行文件并运行，输出结果；工具链缺失或程序含 codegen 不支持的构造（println/ADT/字符串等）时报告明确「codegen 不支持」错误。默认构建下 `--compile` 报 feature 缺失。`--compile` 的 help 文本同步。
*备选*：接入 inkwell ExecutionEngine——被否：需要额外 LLVM 组件与运行时桥接，超出本变更。

### D9 完整统一内存体系（Unsafe + 依赖线性 + QTT + 分级线性 + RegionStack）

统一内存管理不是「区域统计」，而是既有 `unified-memory-management` 定义的四支柱加区域载体：**QTT 等级（0 擦除/1 线性/ω 共享）管所有权、依赖线性类型管值依赖结构（长度 n、容量、时钟）、分级线性类型（□_r/`@Cost`）管资源上界、Unsafe 效应管裸指针逃逸，RegionStack 是底层分配/回收载体**。范式句柄（流、通道、逻辑存储、知识库）必须作为分级值进入该体系：
- 类型层：为范式句柄引入 `Handle<τ>` 类类型构造（或复用 `Ref`/`Type::App` 表示），携带等级与效应行；type_infer 给 `stream`/`chan`/`fresh` 等内置返回分级句柄类型，grade_check 对 1 级句柄执行移动检查。
- 依赖线性：范式结构携带值依赖（`(Vec i64 n)` 负载、`(Stream Int)` 与 `(Clock k)`）时，grade_check 用依赖等级表达式参与判定，复用现有 `GradeInequality` + Z3 路径。
- 分级线性：`□_r`/`@Cost` 作用于范式操作（`search`、`stream-take`、CLP `label`），在 grade_check/Cost 检查中记录资源使用并判上界。
- Unsafe：`ptr-read`/`ptr-write` 指向范式内部存储必须声明 `Unsafe` 效应并经 handler，纯代码被 effect_infer 拒绝。
- RegionStack：实现 `RegionBox<T>`（真实分配、Drop 回收），单线程范式状态（CLP 域表、逻辑变量表、Tabler、ContextKb/ModalKb、流缓存）迁移到 `RegionBox`；跨线程通道缓冲保留 `Arc<Mutex<...>>` 但在 RegionStack 登记 handle（创建/关闭 track/free）。`--run` 区域统计反映范式状态规模。

*备选*：只做区域计数不做等级/代价检查——被否：与「统一内存管理」既有语义和本变更目标不符；四支柱检查必须与区域载体一起落地。

### D10 模块别名（frontend）

desugar 加载 `(:require [lib :as alias])` 时，把导入定义的符号名重写为 `alias/name`（含 `/` 的 Symbol），并对加载文件内部的引用同步重写；未给别名时保持直名。`(ns name ...)` 只产出 `Namespace` 元数据，不再注册名为 `name` 的 CoreDef。私有过滤在重命名前完成。
*备选*：解释器环境支持两级命名空间表——被否：Symbol 已可表达 `/`，重写路径最小且与 `--desugar` 可见。

### D11 HoTT 无 panic（runtime）

`Squash::elim` 改为返回 `Result`/`Option` 并在不可提取时报可读错误；`Equiv::new` 要求调用方提供 section/retraction 见证，并在给定见证值上校验往返方程，不一致则构造失败；所有 HComp/立方填充路径把边界错误作为 `EvalError` 返回。解释器不再有可达的 `panic!("squash elim...")`。
*备选*：实现完整 cubical type theory——非本变更目标，已列入 Non-Goals。

## Risks / Trade-offs

- [范围大、回归多] → 每项按「先加失败测试、再改实现」推进；每个任务结束跑 `cargo test --workspace` 与示例矩阵。
- [阻塞 recv 可能使旧程序死锁] → `close` 明确唤醒；对无 `close` 的空通道接收报可读错误并设置测试超时；文档写明语义。
- [RegionBox 迁移触及多模块] → 分批迁移（先 CLP/logic，再 FRP/KB），每批独立测试，不一次性大改。
- [四支柱检查扩展面大（句柄分级/依赖等级/代价上界）] → 复用既有 grade_check/GradeInequality/Cost 与 Z3 判定路径，每个范式先落 1 级线性句柄与效应门控，再逐步接入依赖与 `@Cost`；不可判定场景按既有约定显式警告放行，不静默通过。
- [llc/clang 依赖环境] → `--compile` 在工具缺失时返回明确错误；默认构建零依赖。
- [FFI 行为变严可能破坏旧测试/用户] → 旧「未映射符号恒等」改为报错属于 **BREAKING** 修复，在 CHANGELOG 注明；模拟表保留已知符号并警告。
- [特化保守跳过降低优化收益] → 语义优先；纯 primary 泛型仍特化，统计中区分「运行时回退（方法组合）」数量。

## Migration Plan

1. 按 tasks.md 顺序实施；每个阶段提交一次 `feat: ...`。
2. 全部完成后运行验收矩阵：358+ 测试全绿、`cargo build --workspace` 零警告、`cargo build -p tisp-cli --features llvm,ffi` 可构建、19 示例矩阵（含 frp-counter `--typecheck`、液态负面用例、`--verify` 用户属性、`--eval`、FFI sin/strlen、OOP around）。
3. 回滚策略：每阶段独立提交，可 `git revert` 单阶段；不引入数据库/迁移文件。

## Open Questions

无——本变更的取舍已在上述决策中收敛，剩余不确定项不影响 spec 与任务分解。
