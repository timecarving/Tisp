# Tisp 项目现状与方向

> 更新:2026-08(对齐 0.1.0 实现;实现状态以 [standard_doc/04-implementation-status.md](./standard_doc/04-implementation-status.md) 为准)

## 1. 项目概览

**Tisp** 是静态类型、纯声明式、系统级定位的 Lisp 方言(Rust workspace,6 crate,17,878 行,177 测试,零编译警告)。设计思想与语言特性见 [README.md](./README.md) 与 [docs/spec.md](./docs/spec.md)。

早期「Phase 0-12 分阶段实施」框架已完成使命(各 phase 内容均已实施或转入剩余缺口),本文档替代 PLAN.md 的历史角色,记录现状与方向。

## 2. 实现状态(30 章)

| 状态 | 章节 | 说明 |
|------|------|------|
| ✅ 完全实现(8 章) | 1 Introduction、10 QTT(含依赖等级)、12 Effect、14 Determinism、15 Liquid Types、23 Typeclasses、25 Module、27 Process Calculi | 全链路可用(lexer → parser → desugar → 检查 → 解释) |
| ⚠️ 部分实现(21 章) | 其余章节 | 骨架可用,深度缺(详见 04 文档逐章说明) |
| ⬜ 仅设计(1 章) | — | 无(Cohesive 已升为 ⚠️) |

逐章状态与未实现清单:`standard_doc/04-implementation-status.md`(2026-08 重建,为唯一事实源)。

## 3. 剩余缺口(2026-08 清单)

### 类型系统

- 符号等级(Z3 不等式)严格验证(依赖线性类型以「常量判定 + 警告放行」落地)
- 类型一等值 `Value::Type` 变体(反射已真实化)
- 类型族多模式/rewrite 规则(单模式归约已实现)
- 类型类完整实例解析(:fun-deps/超类/kind)
- 隐式绑定默认 0(§10.2)
- 依赖会话类型(MPST 投影与顺序检查已实现)

### 运行时与工具链

- LLVM 真编译链(`--ir` 产出 llc 可编译 IR;编译/链接/运行闭环未做)
- 真实 dlopen FFI 全签名(目前 i64/f64 C ABI)
- 宏 fn 参数卫生(let 绑定卫生已实现)
- Monad 优化完整编译(单处理器状态传递路径已接线)
- Cohesive 完整同伦语义(最小可区分语义已落地)

### 验证与逻辑

- find-attack/dolev-yao 完整攻击者模型(场景搜索已实现)
- CLP 非线性约束、ALP 多解解释
- 演算互编码完整(π→SKI、ambient→消息已实现)

## 4. 后续方向(按价值排序)

1. **core 层测试补强**:`tisp-core`(1,193 行)零直接测试——`Type`/`Grade` 半环/`Predicate` 是全局地基,应补枚举覆盖与属性测试(grade_add/grade_le 的折叠缺陷曾两次靠上层测试暴露)
2. **统一约束求解**:设计核心「六维注解由统一约束系统求解」(spec §2 Principle 3)尚未兑现——当前六维是独立 pass
3. **LLVM 真编译链**:兑现「系统级、高性能、无 GC」定位的关键一步
4. **解释器错误处理**:133 个生产 unwrap 逐步替换为 `EvalError` 传播
5. **符号等级 Z3 验证**:复用液态验证的 `verify_implication` 整数比较能力

## 5. 文档地图

| 文档 | 用途 |
|------|------|
| `README.md` | 项目入口(构建/运行/示例/状态概览) |
| `docs/spec.md` | 语言设计规范(30 章,状态符号内联) |
| `standard_doc/INDEX.md` | 语言标准导航 |
| `standard_doc/04-implementation-status.md` | 实现状态与未实现清单(唯一事实源) |
| `docs/PHASE-HISTORY.md` | 历史阶段总结(归档) |
| `CHANGELOG.md` | 变更记录(Keep a Changelog) |
| `openspec/` | OpenSpec 规划产物(变更提案/能力规范) |
