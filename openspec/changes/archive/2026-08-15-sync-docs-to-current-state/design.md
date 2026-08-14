## Context

`standard_doc/04-implementation-status.md` 是唯一事实源,已在上一轮(`logic-paradigms-full-chain`)更新为 32 章 / 30 ✅ + 2 ⚠️。其余文档仍散落早期数字与状态符号。本变更只做事实对齐,不改代码、不改任何 spec 能力需求。动机见 proposal.md - Why。

现状量化基准(2026-08,实测):
- 6 crate / 64 Rust 源文件 / 26,527 行(core 1519 / frontend 5505 / middle 5463 / backend 8128 / runtime 5480 / cli 432)
- 351 单元测试(backend 117 + core 7 + frontend 71 + middle 42 + runtime 114)
- `docs/spec.md` 1680 行,32 章 + 6 附录
- 19 示例文件;19 份 `openspec/specs/<capability>/spec.md`
- 章节状态:✅ 30 章,⚠️ 2 章(§11 Graded Modal Types、§19 Dependent Graded Types),⬜ 0 章

## Goals / Non-Goals

**Goals:**
- 单一事实源一致:所有文档的数字与状态符号对齐 `04-implementation-status.md` 与源码
- 消除「177 测试 / 17,878 行 / 30 章 / 8-21-1 分布」等过时事实
- `docs/spec.md` 与 `02-advanced-features.md` 的内联 ⚠️/⬜ 与 04 一致

**Non-Goals:**
- 不修改任何 Rust 代码、spec 能力需求或行为
- 不重写 `docs/PHASE-HISTORY.md`(历史归档)与 `04-implementation-status.md`(已是事实源)
- 不重写 `docs/spec.md` 的设计内容(只改章节标题内联状态符号与明确过时的「未实现」注记)

## Decisions

1. **以 `04-implementation-status.md` 为唯一事实源**,其余文档的数字/符号单向对齐之,不回写 04。
2. **章节状态分布定为 30 ✅ / 2 ⚠️ / 0 ⬜**;⚠️ 仅保留 §11(缺完整可推断等级推导)与 §19(Pi/Sigma 有、符号等级不可判定警告放行)。
3. **`docs/spec.md` 章节标题内联符号**:17 个 ⚠️ 章节(2/3/4/5/6/7/8/9/16/17/18/20/26/29/30/31/32)改为 ✅;仅 §11、§19 保留 ⚠️。
4. **`02-advanced-features.md` 节级符号**:依赖等级(1.2)、HoTT(7)、溯因(9.4)、其他演算(10.3)、类型类(12.2)、LLVM(14)的 ⚠️/⬜ 升级为 ✅,并删去已实现的「尚未实现」子句。
5. **示例清单**:19 个示例,补齐 `finish-design-demo.tisp`、`finish-partial-demo.tisp` 两条;统一 README/03-reference/project 三处的示例数。
6. **`PLAN.md` 剩余缺口**:按 04 收敛为仅 §11/§19 两处语义深度缺口 + LLVM 真编译链(link 闭环)/真实 dlopen 全签名等少数保留项。

## Risks / Trade-offs

- [文档与代码再次漂移] → 本次把数字集中为一处事实源并给出可复算命令(test/行数),降低复发
- [spec.md 标题符号改动量大] → 仅改标题行与明确「未实现」注记,不动设计正文,回归由 `git diff` 审查
- [`02-advanced-features.md` 局部仍可能与 04 有细微出入] → 以 04 为准,不一致处一律收敛到 04
