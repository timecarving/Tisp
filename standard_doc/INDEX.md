# Tisp 语言标准文档

**版本**: 0.1.0 | **状态**: 持续更新 | **最后修改**: 2026-07

---

## 文档导航

| 文件 | 内容 | 适合 |
|------|------|------|
| [01-language-core.md](./01-language-core.md) | 词法、语法、类型、表达式、定义、ADT、模式匹配 | 语言用户、初学者 |
| [02-advanced-features.md](./02-advanced-features.md) | QTT、效果系统、模式/确定性/区域、液态类型、HoTT、FRP、逻辑编程、进程演算、验证 | 进阶用户、研究者 |
| [03-reference.md](./03-reference.md) | 内置函数表、CLI 参考、类型系统附录、Core AST 附录、实现状态矩阵 | 所有用户、贡献者 |

---

## 快速定位

| 你需要了解... | 去这里 |
|--------------|--------|
| 如何在 Tisp 中定义函数？ | [01 第4章](./01-language-core.md#4-定义定义) |
| 泛型类型怎么用？ | [01 第6.3节](./01-language-core.md#63-多态与泛化类型) |
| 如何定义代数数据类型？ | [01 第5章](./01-language-core.md#5-代数数据类型) |
| 模式匹配怎么写？ | [01 第5.2节](./01-language-core.md#52-模式匹配) |
| 效果系统是什么？ | [02 第2章](./02-advanced-features.md#2-效果系统) |
| `println` 的效果签名？ | [03 第1章](./03-reference.md#1-内置函数表) |
| CLI 有哪些 flags？ | [03 第2章](./03-reference.md#2-cli-参考) |
| 哪些特性已实现？ | [03 第5章](./03-reference.md#5-实现状态矩阵) |
| 类型系统完整定义？ | [03 第3章](./03-reference.md#3-类型系统附录) |
| Core AST 有哪些节点？ | [03 第4章](./03-reference.md#4-core-ast-附录) |

---

## 实现状态符号

| 符号 | 含义 |
|------|------|
| ✅ | 完全实现：lexer → parser → typecheck → runtime 全链路可用 |
| ⚠️ | 部分实现：AST 节点/类型定义存在，部分链路工作 |
| ⬜ | 设计阶段：仅存在于 spec 设计文档中 |

---

## 项目仓库

```
/tmp/tisp/
├── standard_doc/         ← 你在看这里
├── crates/
│   ├── tisp-core/        # 类型、AST、效果、等级
│   ├── tisp-frontend/    # 词法分析、语法分析、脱糖
│   ├── tisp-middle/      # 类型/效果/区域推断、优化器
│   ├── tisp-backend/     # 解释器、模型检查器、时序运行时
│   ├── tisp-runtime/     # 逻辑、约束、FRP、HoTT、进程、定理
│   └── tisp-cli/         # CLI + REPL
├── examples/             # 示例程序 (.tisp)
├── tests/                # 测试目录
└── docs/spec.md          # 原始设计 spec（1528 行）
```
