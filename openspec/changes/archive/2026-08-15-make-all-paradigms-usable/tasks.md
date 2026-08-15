## 1. 统一静态检查管线

- [x] 1.1 在 `tisp-cli` 抽取 `run_static_checks(&CoreProgram) -> CheckReport`（复用 type/effect/grade/mode/det/region/liquid 各 pass 与 `ConstraintSolver` 报告），`--typecheck` 改为调用它
- [x] 1.2 `--run` 执行前运行静态检查，任一维度失败即拒绝执行并输出诊断（新增 `examples/` 临时用例 `(+ 1 true)` 验证拒绝）
- [x] 1.3 `--eval EXPR` 实现「读取 → 脱糖 → 静态检查 → 求值 → 打印结果」；非法表达式报错退出（`--eval '(+ 1 2)'` 输出 3）
- [x] 1.4 REPL 与 `--run` 共用检查入口，删除重复的逐 pass 调用

## 2. lambda 注解与命名空间

- [x] 2.1 `desugar_lambda` 支持 `(fn [params] -> Ret body...)` 与 `->[ε,ρ,@r,m,d] Ret`，写入 `Lambda.ret_type`（复用 `desugar_six_dim_annotation`）；补 frontend 测试
- [x] 2.2 `type_infer` 的 `Lam` 分支在 `ret_type` 存在时把函数结果与注解统一，类型不匹配报错；补 middle 测试
- [x] 2.3 `ns (:require [lib :as alias])` 导入时把公开定义符号重写为 `alias/name`（私有过滤在前），限定引用可解析；`(ns name ...)` 不再注册同名函数
- [x] 2.4 修复 `value_to_string`/`show_value` 对 Vector/Cons/Stream 的可读打印；`examples/frp-counter.tisp --typecheck` 通过，`(stream-take (stream 1) 5)` 输出 `[1 2 3 4 5]`

## 3. 反射与泛型特化

- [x] 3.1 CLI 在检查后把推断签名回填 `CoreDef`（ty/effects/mode/determinism/参数 grades），`(type-of "add")` 返回推断类型 `i64 -> i64 -> i64`；补端到端测试
- [x] 3.2 值表达式 `type-of` 返回静态类型（如 `(type-of 42)` → `i64`）；Π/Σ 运行时语义显式（支持则求值，不支持报明确错误）
- [x] 3.3 `specialize.rs` 对含 around/before/after 或 `call-next-method` 的泛型调用保持运行时分发；新增回归测试锁定「around 翻倍 = 100」（特化前后等价）

## 4. FFI 安全分派

- [x] 4.1 `defextern` 支持 `:abi`（i64->i64 / f64->f64 / str->i64 / str->str / ptr->i64），desugar 写入 FFIDecl；无 `:abi` 默认 i64->i64
- [x] 4.2 loader 按唯一声明签名 `lib.get` 调用，删除「先试 i64」盲试；实参不匹配签名时报错
- [x] 4.3 默认构建（无 ffi feature）对带库路径的调用报「未启用 ffi feature」；模拟表仅保留已知符号并输出一次性警告，未知符号报错（取消恒等回退）
- [x] 4.4 `--features ffi` 下端到端测试：`abs(-42)=42`、`sin(0.5)≈0.479`、`strlen("hello")=5`、签名不匹配报错

## 5. 会话与结构化并发

- [x] 5.1 `Session(Send/Recv/Close, e)` 求值为通道名并读写真实通道负载，协议状态改为 `HashMap<channel_id, Expectation>`（每通道独立）
- [x] 5.2 `type_infer` 会话状态按通道 id 键控并检查首操作；首操作违规报错（含 `send!`/`recv!` 与 `defsession` 同一顺序检查）
- [x] 5.3 `ProcessRuntime::recv` 阻塞等待（Mutex+Condvar），`close` 唤醒；`recv!` 空通道语义从随机失败改为等待或显式关闭错误
- [x] 5.4 `spawn` 保存 JoinHandle，`join` 真正等待并返回子结果/传播错误；补 `spawn+send!/recv!` 与 join 端到端测试

## 6. 用户程序验证

- [x] 6.1 `--verify` 遍历程序的 `TheoremDef` 属性并逐个求值输出结论；无属性时明确报错（删除硬编码 0→5 演示）
- [x] 6.2 新增 `model-check` 内置（init、goal 谓词、next 函数、max-depth），用 `ModelChecker` 做 Value 状态可达性搜索并返回 trace
- [x] 6.3 `find-attack` 改为接受用户协议模型参数（消息/机密/动作）并执行 dolev-yao 知识合成，输出攻击证据或安全结论
- [x] 6.4 新增 `examples/verify-user.tisp`（defprop + model-check 可达目标）与 `--verify` 回归测试

## 7. 完整统一内存体系（Unsafe + 依赖线性 + QTT + 分级线性）

- [x] 7.1 为范式句柄引入分级类型表示（流/通道/逻辑存储/知识库句柄携带 Grade + EffectRow），`stream`/`chan`/`fresh`/KB 内置返回句柄类型并接入 type_infer/effect_infer
- [x] 7.2 QTT 句柄检查：`{1 c : (Chan Int)}` 移交后复用、`{ω s : Stream}` 多读、`{0 ...}` 擦除，grade_check 报错/放行符合语义；补 middle 测试
- [x] 7.3 依赖线性：值依赖范式结构（`(Vec i64 n)` 负载、按 n/时钟消费）经依赖等级表达式判定，复用 `GradeInequality` + Z3；补通过/违反两侧测试
- [x] 7.4 分级线性：`□_r`/`@Cost` 作用于 `search`/`stream-take`/CLP `label` 等范式操作并判上界，超界报错或显式警告；补 Cost 测试
- [x] 7.5 Unsafe 门控：`ptr-read`/`ptr-write` 访问范式内部存储必须声明 `Unsafe` 效应，纯代码被 effect_infer 拒绝；补测试
- [x] 7.6 `tisp-runtime::region` 提供 `RegionBox<T>`（RegionStack 真实分配、Drop 回收）与 stats 接口，补单元测试
- [x] 7.7 单线程范式状态迁移到 `RegionBox`：CLP 域表、逻辑变量表/trail、Tabler、ContextKb/ModalKb、流缓存；通道缓冲保留 `Arc<Mutex<...>>` 但登记 region handle（创建/关闭 track/free）
- [x] 7.8 已回收状态访问报悬垂错误；同一范式程序连跑两次区域统计一致；跑四支柱整体回归（线性句柄/依赖等级/代价上界/Unsafe 门控全绿）

## 8. HoTT 与演算编码

- [x] 8.1 `Squash::elim` 改为返回 `Result`，不可提取时报可读错误（删除 panic 占位）
- [x] 8.2 `Equiv::new` 要求提供 section/retraction 见证并在见证值上校验，不一致构造失败；HComp/立方填充边界错误作为 `EvalError` 返回
- [x] 8.3 演算互编码（π→SKI、async→π、applied→π、ρ→π、ambient→通道）暴露为源码可调用内置并返回可执行结果；不可编码构造报错
- [x] 8.4 补 HoTT/编码端到端测试：非法消去不 panic、边界错误可读、编码结果可执行

## 9. 范式可用性契约落地

- [x] 9.1 `ParadigmFacility` 元数据改为必填六维（类型/效应/区域/等级/模式/确定性，等级须区分 QTT、依赖线性、分级 `□_r`/`@Cost`，效应须声明 `Unsafe`/`State` 门控），缺失拒绝注册；type_infer 范式签名从元数据生成
- [x] 9.2 12 逻辑范式补错误语义：负数/越界概率报错、模态全集判定、ILP 假设可执行验证、组合概率正确；补对应测试
- [x] 9.3 8 编程范式补非法输入报错（数组越界、DFA 未知符号、状态机非法转移）与 AOP around 端到端测试
- [x] 9.4 为每个范式新增/修复 `.tisp` 示例并通过 `--typecheck` + `--run`；建立 `scripts/check-paradigm-matrix.sh` 验收矩阵

## 10. --compile 与文档同步

- [x] 10.1 `llvm` feature 下 `--compile` 生成 IR → `llc-17` → `clang-17` 链接运行并输出结果；工具链缺失或 codegen 不支持时报明确错误
- [x] 10.2 默认构建 `--compile` 报「未启用 llvm feature」；修正 `--compile`/`--eval` 的 help 文本
- [x] 10.3 跑全量验收：`cargo test --workspace` 全绿、`cargo build --workspace` 零警告、`cargo build -p tisp-cli --features llvm,ffi` 可构建、19 示例矩阵全通过
- [x] 10.4 按实测回写 README/PLAN/docs/spec.md/standard_doc/CHANGELOG 状态符号，并同步 openspec 主规范（含 FFI 行为收紧的 BREAKING 说明）
