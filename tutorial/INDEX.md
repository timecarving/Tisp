# Tisp 教程

欢迎学习 **Tisp 0.1.0** —— 一种静态类型、纯声明式、系统级定位的 Lisp 方言。

本教程面向初学者与有 Lisp/函数式语言经验的开发者，按「基础 → 类型 → 效果 → 元编程 → 领域特性 → 系统级」递进编排，覆盖语言全部特性。每章包含概念讲解、可运行示例与练习题；示例代码均以 `tisp` 命令验证。

## 快速开始

```bash
# 构建编译器
cargo build --release

# 运行第一个程序
./target/release/tisp --run examples/hello.tisp

# 类型检查
./target/release/tisp --typecheck examples/hello.tisp
```

## 学习路线

| 路线 | 推荐顺序 | 适合人群 |
|------|---------|---------|
| **新手路线** | 01 → 02 → 04 → 10 → 13 → A3 | 第一次接触 Tisp / Lisp 的读者 |
| **类型系统深入** | 01 → 02 → 03 → 15 | 想掌握 QTT、依赖类型、HoTT 的读者 |
| **系统编程** | 01 → 02 → 10 → 11 → 16 | 关注 FFI、内存、LLVM 的读者 |
| **效果与元编程** | 01 → 02 → 04 → 05 → 14 | 关注代数效应与宏的读者 |
| **逻辑编程** | 01 → 02 → 07 → 12 | 关注 Prolog 风格逻辑/验证的读者 |
| **并发与进程** | 01 → 02 → 08 → 09 | 关注 FRP 与进程演算的读者 |

## 章节列表

### 基础篇
- [第 01 章 开始使用](01-getting-started.md) —— 安装 · Hello World · REPL · 基本语法
- [第 02 章 类型与模式匹配](02-types-and-patterns.md) —— ADT · GADT · 模式匹配

### 类型系统篇
- [第 03 章 深入类型系统](03-type-system-deep.md) —— QTT 等级 · 区域 · 依赖类型 · 液态类型
- [第 15 章 HoTT 与 deriving](15-hott-and-derived.md) —— Path/Interval · HIT · deriving

### 效果与元编程篇
- [第 04 章 效果系统](04-effect-system.md) —— effect · handle/perform · 续延 · monadic 优化
- [第 05 章 宏与元编程](05-macros-and-metaprogramming.md) —— defmacro · syntax-quote · 卫生 · gensym · comptime

### OOP 与逻辑编程篇
- [第 06 章 OOP 与类型类](06-oop-and-typeclasses.md) —— defgeneric/defmethod · 方法组合 · 特化 · typeclass
- [第 07 章 逻辑编程](07-logic-programming.md) —— defpred · 回溯 · CLP · 溯因

### 并发、进程与系统篇
- [第 08 章 并发与 FRP](08-concurrency-and-frp.md) —— Signal · Stream · 通道
- [第 09 章 进程演算](09-process-calculi.md) —— SKI · π · ρ · κ · spi · ambient
- [第 10 章 模块与命名空间](10-modules-and-namespaces.md) —— ns · require · refer
- [第 11 章 FFI 与系统级编程](11-ffi-and-system.md) —— defextern · 裸指针 · 区域 · Unsafe
- [第 16 章 编译与工具链](16-compilation-and-toolchain.md) —— CLI flags · --ir · --compile · comptime

### 验证、范式与 AOP 篇
- [第 12 章 验证](12-verification.md) —— defprop · verify · 模型检查
- [第 13 章 八类编程范式](13-programming-paradigms.md) —— 数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流
- [第 14 章 AOP 面向切面编程](14-aop.md) —— 切面 · pointcut · 编译期编织

### 附录
- [A1 语法参考](A1-syntax-reference.md) —— 完整语法 BNF · 保留字 · 优先级
- [A2 内置函数速查](A2-builtins-catalog.md) —— 算术/比较/集合/IO/类型操作
- [A3 常用模式速查卡](A3-quick-reference.md) —— 类型/效应/等级/宏等常用写法
- [A4 常见错误诊断](A4-error-messages.md) —— 错误信息与修复建议

## 状态符号说明

本教程所有代码示例都带有实现状态标记，与 `standard_doc/04-implementation-status.md`（唯一事实源）保持一致：

| 符号 | 含义 | 验证标准 |
|------|------|---------|
| ✅ | 完全实现 | `--typecheck` 与 `--run` 均通过且输出符合预期 |
| ⚠️ | 部分实现 | 语法/类型检查可用，运行时语义未完全实现 |
| ⬜ | 设计阶段 | 仅展示规划中的语法形态，当前实现不可用 |

## 目录约定

- 教程正文：`tutorial/*.md`
- 可运行示例：`tutorial/examples/chNN-*.tisp`（与各章代码块一一对应）
- 深入阅读：各章末尾给出 `docs/spec.md` 对应章节引用

## 贡献

发现教程与实现不一致？请对照 `standard_doc/04-implementation-status.md` 与 `cargo test --workspace` 验证后，按 OpenSpec 流程提出变更。
