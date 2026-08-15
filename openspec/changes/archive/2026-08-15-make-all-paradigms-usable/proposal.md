## Why

0.1.0 文档把 32 章全部标为 ✅「全链路可用」，但实测发现多个范式在发布路径上不可用或静默给出错误结果（`--verify` 跑硬编码 demo、`frp-counter` 无法通过 `--typecheck`、FFI 对 `sin`/`strlen` 错用 ABI 导致错误值与段错误、`--run` 跳过类型检查、OOP 特化器丢失 `around` 组合等）。本变更要求：**所有范式必须在「静态类型 + 纯声明 + 统一内存管理」三重约束下达到可验证的可用水平**，在此之前不关闭本变更。

## What Changes

- 新增统一「范式可用性契约」，作为所有范式能力规范的公共验收标准：每个范式必须具备统一声明表面、通过 `--typecheck`、有端到端可运行示例、错误显式（不静默返回错误值/占位值）、内存经统一区域模型跟踪。
- 修复发布路径阻断问题：`--run` 执行前必须做静态检查；`--verify` 真正验证用户程序属性；`--eval` 真正求值；`--compile` 的行为与文档一致；FFI 按签名正确分派 i64/f64/字符串/指针且签名不匹配时显式报错；lambda 支持 `->` 返回类型注解；`ns :require ... :as` 别名生效；反射返回推断后的真实静态信息；泛型特化不得破坏 `around` 方法组合；会话 `send`/`recv` 不丢 payload；`spawn`/`join` 实现结构化并发。
- 将 12 类逻辑范式、8 类编程范式、进程演算、会话类型、HoTT/Cohesive 从「宿主演示函数」提升为「类型可查、效应可推断、内存可跟踪、失败显式」的可用级实现。
- 统一内存管理按完整四支柱执行：范式句柄/状态全部作为分级值进入 QTT（0/1/ω）、依赖线性类型、分级线性类型（□_r/`@Cost`）检查；裸访问仅经 `Unsafe` 效应门控；区域栈作为底层分配/回收载体，`--run` 统计可审计。
- 按上述实现结果回写文档状态（README/PLAN/standard_doc/CHANGELOG/spec 状态符号），状态与实测一致。

## Capabilities

### New Capabilities

- `paradigm-usability-contract`: 定义「范式可用」的统一契约——统一声明表面、静态类型检查、端到端示例、显式错误、统一内存跟踪五条验收标准，并约束所有范式能力规范引用它。

### Modified Capabilities

- `logic-and-verification`: `--verify` 必须检查用户程序中的 `defprop`/模型属性并输出对应反例 trace；会话类型运行时语义真实（payload 不丢失、协议按通道检查）。
- `logic-programming-paradigms`: 12 类逻辑范式必须经统一声明表面接入，具备类型/效应注解与内存跟踪；禁止静默错误值。
- `programming-paradigms`: 8 类编程范式与 AOP 必须满足可用性契约，端到端示例全绿。
- `paradigm-integration`: ParadigmRegistry/Facility 的每个设施必须携带并校验六维注解（类型/效应/区域/等级/模式/确定性），执行不得绕过统一内存跟踪。
- `temporal-types`: FRP/时序示例必须通过 `--typecheck` 并可运行；lambda 返回类型注解语法成立。
- `toolchain-and-macros`: `--eval` 求值、`--run` 先静态检查、`--compile` 真实 JIT（或明确降级并文档一致）、FFI 按 ABI 签名安全分派、反射返回推断后的静态信息。
- `unified-memory-management`: 解释器路径的范式值与通道/流/逻辑变量生命周期纳入统一区域管理模型；悬垂与越界访问显式报错。
- `module-visibility`: `ns :require [lib :as alias]` 的限定引用 `alias/name` 生效且尊重私有定义。
- `type-system-extensions`: 推断结果回填反射签名表；依赖类型/Pi/Sigma 运行时语义显式且无 panic 占位。
- `hott-and-deriving`: HoTT 运行时不 panic、语义与文档一致（squash/equiv/hcomp 边界行为显式）；演算互编码结果可端到端验证。

## Impact

- 代码：`tisp-cli/src/main.rs`（管线顺序、--eval/--verify/--compile）、`tisp-frontend/src/desugar.rs`（lambda 注解、ns 别名）、`tisp-middle`（specialize/reflect/constraint 回填）、`tisp-backend/src/interpreter.rs` 与 `codegen.rs`（会话/并发/FFI/反射/区域）、`tisp-runtime`（12+8 范式、进程、HoTT）。
- 测试：新增每范式端到端 `.tisp` 示例与回归测试；保持 `cargo test --workspace` 全绿、普通构建零警告。
- 文档：`docs/spec.md`、`standard_doc/04-implementation-status.md`、README、CHANGELOG 与 OpenSpec 主规范同步。
- 不新增运行时依赖；LLVM/Z3/FFI 继续按 feature 门控，默认构建不可退化。
