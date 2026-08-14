## Context

Tisp 已有:代数效应(§12 handle/perform + State/Reader/Writer/Search 等)、单子优化路径(§12.6)、FRP/时序流(§18)、进程演算、OOP(defgeneric/defmethod/defclass)、MOP(metaprogram.rs 的 Meta/Protocol/Aspect/AOP 骨架)、持久化集合(im)、HoTT/Cohesive/时序类型。剩余 ⚠️ 缺口见 `standard_doc/04-implementation-status.md`。本变更补齐缺口并新增 8 类范式 + AOP,动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 把 ⚠️/⬜ 特性补齐到「全链路可用」(语言表面 → 类型/效应/等级 → 求值/代码生成全部接通)。
- 以纯声明式副作用管理(代数效应 + 单子)落地 8 类编程范式,端到端可运行。
- 在编译器纯声明式 MOP 上实现 AOP,辅助 OOP 方法组合,端到端可用。

**Non-Goals:**
- 不引入命令式逃逸:所有范式副作用经效应/单子门控,保持引用透明。
- 不做生产级数值库(数组/符号只需正确语义,不追求 BLAS/CAS 级性能)。
- 不改动既有已 ✅ 特性的语义——只在既有骨架上补齐深度,不重写。

## Decisions

### D1: 补齐 ⚠️ 特性(逐项,不重复 spec)

| 缺口 | 补齐方式 | 关键落点 |
|------|----------|----------|
| □_r/◇_ε 无推理 | Modal 引入/消去接入 type_infer/unify | `type_infer.rs` 补 Modal 臂 |
| Cost 注解全推导 | `@Cost` 语法 + 渐近代价复合(复用 `grade_le_asymptotic`) | `desugar.rs`/`grade_check.rs` |
| 完整立方填充 | HComp 扩到 N 维 Kan 填充 | `hott.rs` |
| Cohesive 完整同伦 | ♭/♯/ʃ adjoint-triple 全语义 | `hott.rs` |
| □_t 时序保证 | 稳定类型跨时刻检查 + 生产率 | `temporal.rs`/`type_infer.rs` |
| 区域逃逸检查 | with-region 作用域 + 逃逸判定 | `region_infer.rs` |
| inkwell 闭包 | 环境打包/解包 + define/call | `codegen.rs` |
| 密码学真原语 | chacha20/sha2(已 optional)+ aes | `process.rs` crypto feature |
| EVOLP/DLP/MOP 表面 | 语法 → desugar → 类型 → interpreter 接线 | `frontend/middle/backend` |

**理由**:每项在既有骨架上做深度补齐,复用已就位的类型/效应/等级机制,不另起炉灶。
**备选**:重写对应子系统(被否——破坏已 ✅ 特性,风险高)。

### D2: 8 类范式组合优先映射

| 范式 | 组合/新增 | 关键既有特性 |
|------|-----------|--------------|
| 数组编程 | 新增多维数组类型 + 归约组合子 | im 集合 + 高阶函数 |
| 栈编程 | 组合(State effect 持有栈) | `State s` get/put + 纯函数 |
| 连接式编程 | 组合(点自由 = 函数复合) | 函数一等值 + `compose` |
| 符号编程 | 组合(quote 惰性 ADT + 代换) | quote/模式匹配/`map` |
| 自动机编程 | 组合(表驱动 + Search 回溯) | 表 + Search effect |
| 状态机编程 | 组合(State + 转移表) | State effect + 数据驱动 |
| 数据驱动编程 | 组合(查表/策略/解释器) | 一等表/闭包 + 模式匹配 |
| 基于流编程 | 组合(数据流网络 = 节点图) | FRP Signal + 时序流 |

**理由**:遵循「可用既有特性组合则组合、否则少量新增」;唯一实质新增为「多维数组类型」与「符号表达式 ADT」,其余均为组合。
**备选**:每范式独立运行时子系统(被否——违反组合优先,重复造轮子)。

### D3: 副作用统一为代数效应/单子

所有范式的副作用(栈顶、状态机当前态、数组缓冲、流缓冲、自动机搜索)SHALL 建模为效应操作
(`State`/`Search`/`Signal`),单处理器路径经 §12.6 直接状态线程降级;纯代码未经 handler SHALL 无法触发副作用。
**理由**:统一副作用管理兑现「效应是万能胶」,使各范式共享同一套类型/效应检查。

### D4: AOP = 编译器 MOP 编织

复用 `metaprogram.rs` 的 `Pointcut`/`Advice`/`AspectWeaver` 骨架:
- `aspect`/`pointcut`/`advice` 语法脱糖为切面声明。
- 编译期经 MOP 反射(方法名/注解)匹配切入点,把 before/after/around 建议编织为 `:before/:after/:around` 方法组合(§22.3)。
- 编织是纯函数变换(`CoreProgram → CoreProgram`),运行时无反射。
**理由**:满足「编译期纯声明式 MOP」约束,AOP 作为 OOP 方法组合的语法糖,不引入运行时反射。
**备选**:运行时动态代理(被否——违背纯声明式 + 编译期约束)。

## Risks / Trade-offs

- [多维数组类型接入 HM 推断复杂] → 数组作为 `Type::App(Array n, elem)` 具体类型 + 专用归约内置,限制为具体维度。
- [inkwell 闭包真代码生成(环境打包)工作量大] → 先函数 define/call 真生成,闭包环境用 display 间接层,逐步补。
- [密码学需新依赖 aes(未在 workspace)] → 复用已 optional 的 chacha20/sha2,对称加密用 ChaCha20-Poly1305 风格;若无 aes 依赖则回退 XOR 并警告(保持默认构建)。
- [8 范式 + 补齐一次性落地风险高] → tasks 按「补齐 → 范式 → AOP」三段拆分,每段独立可测可合入。
- [符号/数组与既有类型检查交互] → 符号表达式用 `quote` 型 ADT 隔离于 HM 推断,避免 rank-n 回归。

## Migration Plan

全部为增量新增 + 深度补齐,不改动既有 ✅ 特性语义,无破坏性迁移。按段提交:①补齐 ⚠️ → ②8 范式 → ③AOP,每段保持 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- 密码学是否引入 `aes` crate(或仅 ChaCha20/SHA-256):可延后,默认构建不受影响,spec 仅要求「真实原语替换 XOR 占位」。
- 数组编程是否支持秩泛型(rank polymorphism):可延后,spec 仅要求「多维数组 + 归约」,初版固定维度即可。
