# Tisp 语言标准文档

**版本**: 0.1.0 | **状态**: 对齐当前实现 | **最后修改**: 2026-08

---

## 文档导航

| 文件 | 内容 | 适合 |
|------|------|------|
| [01-language-core.md](./01-language-core.md) | 词法、语法、数据类型、表达式、定义、ADT/GADT、模式匹配、类型基础 | 语言用户、初学者 |
| [02-advanced-features.md](./02-advanced-features.md) | 效果系统、模式/确定性、区域、HoTT、FRP、逻辑编程(CLP/ALP)、进程演算、宏、OOP、验证 | 进阶用户、研究者 |
| [03-reference.md](./03-reference.md) | 内置函数表、CLI 参考、类型系统附录、Core AST 附录、实现状态矩阵、示例程序清单 | 所有用户、贡献者 |
| [04-implementation-status.md](./04-implementation-status.md) | spec 30 章逐章实现状态与未实现特性清单(含 file:line 证据) | 贡献者、研究者 |

---

## 快速定位

| 你需要了解... | 去这里 |
|--------------|--------|
| 如何定义函数？ | [01 第4章](./01-language-core.md#4-定义) |
| 泛型类型怎么用？ | [01 第6章](./01-language-core.md#6-类型系统基础) |
| 如何定义代数数据类型？ | [01 第5章](./01-language-core.md#5-代数数据类型) |
| 模式匹配怎么写？ | [01 第5.3节](./01-language-core.md#53-模式匹配) |
| 效果系统(handle/perform)是什么？ | [02 第2章](./02-advanced-features.md#2-效果系统) |
| 逻辑编程(defpred/CLP)怎么写？ | [02 第9章](./02-advanced-features.md#9-逻辑编程) |
| 宏系统怎么用？ | [02 第12章](./02-advanced-features.md#12-宏系统) |
| 内置函数完整清单？ | [03 第1章](./03-reference.md#1-内置函数表) |
| CLI 有哪些 flags？ | [03 第2章](./03-reference.md#2-cli-参考) |
| 哪些特性已实现？ | [03 第4章](./03-reference.md#4-实现状态矩阵)、[04 全量清单](./04-implementation-status.md) |
| 有哪些示例程序？ | [03 第5章](./03-reference.md#5-示例程序) |

---

## 核心设计思想

Tisp 的设计围绕两条主线展开(详见 docs/spec.md §2):

1. **演算 > 代数效应(Calculi over Algebraic Effects)**
   演算(进程演算 π/ρ/ambient/κ/spi/applied π/SKI、逻辑搜索、时序流、状态转换)是语言的**抽象核心**——用户用演算思考与表达程序;代数效应(handle/perform)是演算关系的**编码与验证载体**——handler 编码演算之间的转换关系,验证器是探索所有路径的 effect handler。一切高级抽象最终都可由「演算 + effect handler」组合而成。

2. **强静态类型(Strong Static Typing)**
   Tisp 是强静态类型语言:类型在**编译期**检查(类型推断 + 多态,`--typecheck`/REPL `:type`),程序通过检查即保证**无运行时类型错误**。类型、效果、等级、模式、确定性、区域六维注解由统一约束系统求解,且全部是可在运行时操作的一等公民(Reader Principle)。

配套原则:**效应是万能胶**(State/errors/search/IO/unsafe/signals 都是 effect,Monad 只是优化路径)、**统一方法**(所有定义都是 `def` + 六维注解,defn/defpred/defgeneric 只是语法糖)、**全程声明式**(无命令式逃逸口,系统级编程 = 资源约束声明 + Unsafe effect 门控)。

---

## 实现状态符号

| 符号 | 含义 |
|------|------|
| ✅ | 完全实现：lexer → parser → desugar → runtime 全链路可用(含测试) |
| ⚠️ | 部分实现：主要链路工作，部分语义/边界未完成 |
| ⬜ | 设计阶段：仅存在于 docs/spec.md 设计文档中 |

状态以 `docs/spec.md`(设计规范)与 `crates/` 源码为准，随 [CHANGELOG.md](../CHANGELOG.md) 同步更新。

---

## 项目仓库结构

```
Tisp/
├── standard_doc/         ← 你在看这里(语言标准文档)
├── docs/spec.md          # 原始设计规范(1569 行,设计目标与语法定义)
├── CHANGELOG.md          # 变更记录
├── crates/
│   ├── tisp-core/        # 类型、AST、效果、等级、模式、区域、数据声明
│   ├── tisp-frontend/    # 词法分析(lexer)、语法分析(parser)、脱糖(desugar)
│   ├── tisp-middle/      # 类型/效果/模式/确定性/等级/区域推断、液态类型、优化器
│   ├── tisp-backend/     # 解释器(interpreter)、LLVM IR 生成(codegen)、进程运行时、时序运行时
│   ├── tisp-runtime/     # 逻辑、约束(CLP)、回溯、并发、FRP、进程、HoTT、依赖等级、定理
│   └── tisp-cli/         # CLI + REPL(入口)
├── examples/             # 示例程序(.tisp)
└── tests/                # 测试目录
```

## 快速开始

```bash
# 构建
cargo build --release

# 运行示例
cargo run --release -- --run examples/hello.tisp
# 或直接运行二进制
./target/release/tisp --run examples/hello.tisp

# 运行全部测试
cargo test --workspace

# 交互式 REPL
./target/release/tisp
```
