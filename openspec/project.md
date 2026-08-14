# Tisp 项目档案

> 本文件是 OpenSpec 工作流的项目上下文总览,供 AI 与贡献者参考。
> OpenSpec 实际读取的简短上下文见 `openspec/config.yaml` 的 `context` 字段。

## 1. 项目概述

**Tisp 0.1.0** — 基于静态类型、纯声明式、系统级定位的 Lisp 方言。
Rust workspace 实现,6 个 crate,当前 358 个单元测试,零编译警告。

核心设计思想(详见 `docs/spec.md` §2 与 `standard_doc/INDEX.md`):

1. **演算 > 代数效应** — 进程演算(π/ρ/ambient/κ/spi/applied π/SKI)、逻辑搜索、时序流是抽象核心;`handle`/`perform` 是编码与验证载体;验证器 = 探索所有路径的 effect handler。
2. **强静态类型** — 类型在编译期检查,通过检查即保证无运行时类型错误。
3. **统一方法(Everything is an Annotated Relation)** — 所有定义都是 `def` + 六维注解(type/effect/region/grade/mode/determinism),由统一约束系统求解;`defn`/`defpred`/`defgeneric` 只是语法糖。
4. **效应是万能胶** — State/errors/search/IO/unsafe/signals 都是 effect,Monad 只是优化路径(§12.6)。
5. **Reader Principle** — 类型、效果、等级、模式都是一等公民,可在运行时操作。
6. **全程声明式** — 无命令式逃逸口;系统级编程 = 资源约束声明 + Unsafe effect 门控。

## 2. 技术栈

| 领域 | 选型 | 说明 |
|------|------|------|
| 实现语言 | Rust(edition 2021, resolver 2) | 6-crate workspace |
| 词法 | `logos 0.14` | tisp-frontend lexer |
| 持久化数据结构 | `im 15` | 不可变 List/Map/Set,Clojure 风格 |
| 错误报告 | `miette 7`(fancy) + `ariadne 0.4` | 诊断输出;CLI 主错误类型为 `miette::Result` |
| 错误派生 | `thiserror 2` | 各 crate 错误枚举 |
| CLI | `clap 4`(derive) | tisp-cli |
| REPL | `rustyline 14` | 跨行保持宏表与定义累积 |
| 图算法 | `petgraph 0.6` | 依赖图/调用图分析 |
| 标识符 | `unicode-xid 0.2` | Unicode 标识符判定 |
| LLVM IR | `inkwell 0.5`(feature `llvm17-0`) | 真实 IR 生成(需 `llvm` feature) |
| LLVM 动态链接 | `llvm-sys 170`(feature `force-dynamic`) | Debian llvm-17 只有共享库;默认 prefer-static 会因缺 `libPolly.a` 构建失败 |
| SMT 求解 | `z3 0.12` | 液态类型/精化类型(需 `z3` feature 与 `libclang-17-dev`) |
| 序列化 | `serde 1` + `serde_json 1` | 备用 |

**LLVM 工具链接线**:`LLVM_SYS_170_PREFIX=/usr/lib/llvm-17`(零安装);`--ir` 输出经 `llc-17` 编译验证。

## 3. Workspace 结构

```
crates/
├── tisp-core/       # 无依赖核心:span、symbol(Arc<str> 驻留)、ast(S 表达式)、
│                    #   core_ast(Core AST)、types(完整类型系统)、effects(效果行)、
│                    #   grades(QTT 等级半环)、modes(Mercury 模式)、determinism、regions、data(ADT)
├── tisp-frontend/   # lexer(logos)→ reader(S 表达式)→ parser → desugar(S 表达式→Core,含宏展开)
├── tisp-middle/     # type_infer(Algorithm W/HM)、effect_infer、grade_check(QTT)、
│                    #   mode_analysis、determinism_analysis、region_infer、effect_compile、
│                    #   liquid_types(Z3)、optimize/optimizer、holes
├── tisp-backend/    # interpreter(含内置函数)、codegen(LLVM IR 生成)、
│                    #   process(模型检查器)、temporal(时序流)、z3_bridge
├── tisp-runtime/    # logic(逻辑编程)、constraint(CLP)、abduction、concurrent、frp、
│                    #   process(通道运行时)、hott、theorem、persistent、region、effect、
│                    #   depgraded、metaprogram、stdlib
└── tisp-cli/        # main.rs:clap CLI + rustyline REPL
```

依赖方向:`cli → backend → middle → frontend → core`,`cli → runtime`。

## 4. 语言特性速览

- **核心**:ADT/GADT、模式匹配(同名变量一致性、or-pattern、guard、refined 模式)、Π/Σ 依赖类型、`(ann expr Type)` 标注
- **类型系统**:HM + 多态、QTT 等级(0/1/ω)、效果行、区域、液态类型(`{x : T | pred}`)、时序模态(`⃝`/`□_t`/`◇_t`)、HoTT(Path/Interval)
- **效果系统**:`handle`/`perform`、续延 `k`、内置效果操作(get/put/ask/tell/throw/choose)、效果行消减、monadic 优化检测
- **宏**:`defmacro`、语法引号(`\`x`/`,`/`,@`)
- **OOP**:`defgeneric`/`defmethod`(模式匹配分发、`:around/:before/:after/:primary` 组合、`call-next-method`)、`defclass`/`definstance` 类型类
- **逻辑编程**:`defpred`(子句)、CLP(`domain`/`label`/`constrain`/`solve-all`)、回溯、`find-all`、`abduce`
- **进程演算**:`chan`/`send`/`recv`/`spawn`、Async/ambient/ρ/κ/spi 通道、SKI 组合子、加密(encrypt/decrypt/sign/verify/hash,当前为占位实现)
- **FRP**:`stream`/`stream-take`/`advance`、Signal 节点(Map/Filter/Fold)
- **模块与 FFI**:`ns`(`:require`/`:refer`/`:as`)、跨文件加载、`defextern`
- **验证**:`defprop` + `verify`、模型检查器(reachability 搜索)
- **内置函数**:`append`/`slurp`/`spit`、list/vector/hash-map/hash-set 构造器、`range`/`zip`/`concat`/`reduce`/`count`/`length` 等

## 5. 代码约定

- **注释**:简体中文(与文档一致);重要修复/回归附文件与行号级证据链
- **测试**:内联 `#[cfg(test)]` 单元测试分布在各 crate 源文件中,构造 `CoreExpr` 直接驱动 interpreter(如 `tisp-backend/src/interpreter.rs` 的 tests 模块);`cargo test --workspace` 应全绿
- **警告**:保持 `cargo check --workspace` 零警告
- **feature 门控**:LLVM/Z3 相关代码用 feature 门控(非 llvm 回退文本生成器),不要破坏默认构建
- **git 提交**:`feat: 中文描述`(conventional commits 前缀 + 中文摘要);`.gitignore` 排除 `/target`、`.reasonix/`、`reasonix.toml`

## 6. 构建 / 测试 / 运行

```bash
cargo build --release            # 构建
cargo test --workspace           # 358 个测试
cargo check --workspace          # 应 0 警告

./target/release/tisp --run examples/hello.tisp      # 解释运行
./target/release/tisp --desugar examples/adt-test.tisp  # 查看脱糖 Core AST
./target/release/tisp --typecheck examples/adt-test.tisp # 类型+效果+等级+模式+确定性+区域+优化
./target/release/tisp --ir examples/run-test.tisp    # LLVM IR 文本(需 llvm feature)
./target/release/tisp                                # REPL(:type EXPR 只查类型)
```

CLI flags:`--eval -e` `--print-ast` `--print-tokens` `--desugar` `--typecheck` `--run` `--verify` `--ir` `--compile`
REPL 约定:定义行(defn/defdata/defpred/defmacro/...)并入累积;表达式行先类型检查、错误不求值;`(exit)` 退出。

## 7. 文档地图

| 文档 | 用途 |
|------|------|
| `docs/spec.md` | 原始设计规范(32 章 + 6 附录,状态符号内联) |
| `standard_doc/INDEX.md` | 语言标准文档导航(01 核心 / 02 高级特性 / 03 参考 / 04 实现状态) |
| `standard_doc/04-implementation-status.md` | 32 章逐章实现状态(含 file:line 证据,唯一事实源) |
| `CHANGELOG.md` | Keep a Changelog 格式变更记录 |
| `PLAN.md` | 项目现状与后续方向 |
| `docs/PHASE-HISTORY.md` | 历史阶段总结(归档) |
| `examples/` | 19 个示例;14 个可运行、3 个定义型(仅 `--typecheck`)、1 个部分支持、1 个预期报错 |

**状态符号**(文档与代码注释通用):✅ 完全实现 / ⚠️ 部分实现 / ⬜ 设计阶段

## 8. OpenSpec 工作流约定

- schema:`spec-driven`(`openspec/config.yaml`)
- 变更文件放 `openspec/changes/<change-name>/`,归档到 `openspec/changes/archive/`
- 能力规范放 `openspec/specs/<capability>/spec.md`(当前 19 份)
- 每个变更遵循:proposal → design → tasks → specs(主 spec 可先放 `docs/spec.md`/`standard_doc/` 参照)
- 变更实现后更新 `CHANGELOG.md` 与 `standard_doc/04-implementation-status.md`

## 9. 已知局限(修改代码前必读)

- 加密原语为 XOR/简单哈希占位,生产需换 AES/ChaCha/SHA-256(crypto feature 下接 RustCrypto)
- LLVM 真编译链:`--ir` 生成 llc 可编译 IR;编译/链接/运行闭环未做
- 以 `standard_doc/04-implementation-status.md` 为唯一事实源
