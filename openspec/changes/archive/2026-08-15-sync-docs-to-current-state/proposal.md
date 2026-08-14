## Why

项目经过 2026-08 多轮全链路补齐后,实际状态已变为 6 crate / 64 源文件 / 26,527 行 / 351 测试 / 32 章(30 ✅ + 2 ⚠️)/ 19 示例。但 `README.md`、`PLAN.md`、`standard_doc/INDEX.md`、`standard_doc/03-reference.md`、`docs/spec.md`、`openspec/project.md` 仍残留早期数字(177 测试、17,878 行、30 章、8/21/1 章状态分布),且 `docs/spec.md` 与 `02-advanced-features.md` 的章节内联状态符号(⚠️/⬜)与唯一事实源 `standard_doc/04-implementation-status.md` 不一致。这些过时描述相互矛盾,无法反映现状。

## What Changes

- 重写全部用户可见文档中的过时量化事实,与现状对齐:
  - 测试数 177 → **351**;代码行数 17,878 → **26,527**(64 源文件);章节数 30 → **32 章 + 6 附录**;示例 13/18 → **19**
  - 章节状态分布 8/21/1 → **30 ✅ / 2 ⚠️ / 0 ⬜**(仅 §11 Graded Modal、§19 Dependent Graded 仍 ⚠️)
- 修正 `docs/spec.md` 17 个章节内联状态符号(⚠️ → ✅),仅保留 §11/§19 为 ⚠️
- 修正 `standard_doc/02-advanced-features.md` 各节状态符号(依赖等级/HoTT/溯因/其他演算/类型类/LLVM 等 ⚠️→✅),与 04 一致
- 刷新过时的「剩余缺口」「已知局限」清单与示例程序表
- 对齐 `openspec/project.md` 的文档地图(能力规范空 → 19 份;PHASE{N}_SUMMARY 已不存在)

## Capabilities

(无新增/修改能力——本变更为纯文档事实同步,不改变任何 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。)

## Impact

- **文档**(无代码改动):`README.md`、`PLAN.md`、`standard_doc/INDEX.md`、`standard_doc/03-reference.md`、`standard_doc/02-advanced-features.md`、`docs/spec.md`、`openspec/project.md`、`CHANGELOG.md`
- **不变**:`standard_doc/04-implementation-status.md` 为唯一事实源,保持为准;`docs/PHASE-HISTORY.md` 为历史归档,不重写
- 验收:`cargo test --workspace` 全绿(351)、`cargo check --workspace` 零警告(文档改动不影响编译)
