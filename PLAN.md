# Tisp 项目现状与方向

> 更新:2026-08(对齐 0.1.0 实现;实现状态以 [standard_doc/04-implementation-status.md](./standard_doc/04-implementation-status.md) 为准)

## 1. 项目概览

**Tisp** 是静态类型、纯声明式、系统级定位的 Lisp 方言(Rust workspace,6 crate,64 源文件,26,527 行,358 测试,零编译警告)。设计思想与语言特性见 [README.md](./README.md) 与 [docs/spec.md](./docs/spec.md)。

早期「Phase 0-12 分阶段实施」框架已完成使命(各 phase 内容均已实施或转入剩余缺口),本文档替代 PLAN.md 的历史角色,记录现状与方向。

## 2. 实现状态(32 章)

| 状态 | 章节 | 说明 |
|------|------|------|
| ✅ 完全实现(32 章) | 1–32 | 全链路可用(lexer → parser → desugar → 检查 → 解释) |
| ⚠️ 部分实现(0 章) | — | 无 |
| ⬜ 仅设计(0 章) | — | 无 |

逐章状态与未实现清单:`standard_doc/04-implementation-status.md`(2026-08 重建,为唯一事实源)。

## 3. 剩余缺口(2026-08 清单)

### 类型系统:已无 ⚠️

§11 Graded Modal Types(按使用次数推导 r/ε)与 §19 Dependent Graded Types(符号等级 Z3 判定)已在收尾轮(`complete-graded-type-inference`)升 ✅。

### 运行时与工具链(后续方向,非 spec ⚠️)

- LLVM 真编译链:inkwell 已生成真实 IR + 闭包环境堆分配(§30 ✅);`llc` 编译/链接/运行闭环未做
- 真实 dlopen FFI 全签名:字符串(CString)/指针(i64 透传)/i64/f64 已接;完整 C ABI 签名缺

## 4. 后续方向(按价值排序)

1. **core 层测试补强**:`tisp-core`(1,519 行)直接测试少——`Type`/`Grade` 半环/`Predicate` 是全局地基,应补枚举覆盖与属性测试(grade_add/grade_le 的折叠缺陷曾两次靠上层测试暴露)
2. **统一约束求解**:设计核心「六维注解由统一约束系统求解」(spec §2 Principle 3)已以 constraint.rs 共享约束图 + solve.rs fixpoint 聚合六 pass 冲突落地;维度间完整 fixpoint 反馈为增强方向
3. **LLVM 真编译链**:兑现「系统级、高性能、无 GC」定位的关键一步(编译/链接/运行闭环)

## 5. 文档地图

| 文档 | 用途 |
|------|------|
| `README.md` | 项目入口(构建/运行/示例/状态概览) |
| `docs/spec.md` | 语言设计规范(32 章 + 6 附录,状态符号内联) |
| `standard_doc/INDEX.md` | 语言标准导航 |
| `standard_doc/04-implementation-status.md` | 实现状态与未实现清单(唯一事实源) |
| `docs/PHASE-HISTORY.md` | 历史阶段总结(归档) |
| `CHANGELOG.md` | 变更记录(Keep a Changelog) |
| `openspec/` | OpenSpec 规划产物(变更提案/能力规范) |
