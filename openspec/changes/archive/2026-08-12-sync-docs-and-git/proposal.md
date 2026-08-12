## Why

2026-08 全项目探索确认:三轮功能变更(liquid-types-z3 / implement-remaining-gaps / dependent-linear-types)落地了大量实现,但**文档体系停留在变更前的状态**,且**仓库卫生未维护**——具体脱节:PLAN.md 停在「Phase 4(液态类型)待实施」(早已完成)、PHASE2-12 总结未反映现状、README 实现状态概览过时、docs/spec.md 正文仍是愿景语言(状态符号已标 ✅ 但内容未对齐)、CHANGELOG 为多轮追加结构;git 侧三轮变更全部未提交、新源码/示例/openspec 规划产物未追踪、`target.tar`/`target.tar.zst` 构建产物未忽略。

## What Changes

**文档重写(全部对齐现状)**
- PLAN.md:从「分阶段实施计划(停在 Phase 4)」重写为「项目现状 + 30 章实现状态 + 剩余缺口 + 后续方向」
- PHASE2-12_SUMMARY.md:归档为历史附录(合并为一份 `docs/PHASE-HISTORY.md` 或保留并标注历史状态)
- README.md:实现状态概览、测试数(177)、示例数(17)、文档地图、CLI 说明全面核对更新
- CHANGELOG.md:整理为 Keep a Changelog 结构(0.1.0 变更合并整理,不丢失任何条目)
- docs/spec.md:已实现章节(✅/⚠️)正文与状态对齐(愿景语言改为现状描述,标注近似实现);设计阶段章节保留愿景并明确标注
- standard_doc 01/02/03/04/INDEX:与实现逐项核对(内置函数表、示例表、CLI 参考、实现状态)

**git 卫生**
- `git add`:openspec/(规划产物)、新源码(liquid_verify.rs、specialize.rs 等)、新示例(dependent-linear-test 等 3 个)、三轮变更修改的文件
- `.gitignore` 追加 `target.tar`/`target.tar.zst`(构建产物)
- 验证 `git status` 干净(仅剩有意保留的未跟踪项:.agents/、.zcode/、reasonix.toml 按现状)

## Capabilities

### New Capabilities

(无 — 纯文档重写与版本控制元操作,无行为变化,变更声明 `skip_specs: true`)

### Modified Capabilities

(无)

## Impact

- 文档:`PLAN.md`、`PHASE2-12_SUMMARY.md`、`README.md`、`CHANGELOG.md`、`docs/spec.md`、`standard_doc/01-04 + INDEX.md`
- `.gitignore`:追加构建产物忽略
- git index:三轮变更的全部文件 + openspec 规划产物入库
- 不涉及 crates/ 源码行为(仅 git 追踪状态变化)
