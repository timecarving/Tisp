# 文档重写与 git 卫生 — 设计

## Context

动机与范围见 `proposal.md`。关键现状(2026-08 探索实测):

- `PLAN.md` 声称「4/13 phases complete,Next: Phase 4 Liquid Types」——液态类型、类型族、依赖线性、进程演算、验证等均已实现;Phase 框架已无意义
- `PHASE2-12_SUMMARY.md` 共 11 份早期阶段总结,内容被 04 实现状态文档取代
- `README.md`:实现状态概览「⚠️ 液态类型(Z3)」过时(已 ✅)、测试数 105 过时(177)、示例数已更新但需全面核对
- `docs/spec.md`:30 章标题已内联状态符号(✅/⚠️/⬜),但**正文内容**仍为愿景语言(如 §15 液态类型正文写「编译错误」但实现是验证器;§10 正文无依赖等级)
- `standard_doc/03-reference.md`:内置函数表、示例表、CLI 参考需与实现核对
- `CHANGELOG.md`:多轮追加,0.1.0 下堆积 5 个大块,结构可整理但条目须全保留
- git:三轮变更(25+ 修改文件、5 新文件、openspec/ 全目录)未追踪;`target.tar`/`target.tar.zst` 未忽略;`.gitignore` 现有 `/target`、`.reasonix/`、`reasonix.toml`
- 约束:本变更 `skip_specs: true`(无行为变化);文档以 `standard_doc/04-implementation-status.md`(2026-08 重建)为唯一事实源

## Goals / Non-Goals

**Goals**
- 所有用户可见文档与 177 测试/30 章状态的实现现状一致
- 三轮变更成果 + openspec 规划产物入库,构建产物忽略,`git status` 仅剩有意保留项
- 不丢失任何 CHANGELOG 条目与 spec 设计内容

**Non-Goals**
- 不改 crates/ 源码(纯文档与 git 元操作)
- 不扩展 docs/spec.md 愿景章节(保留设计,仅标注状态)
- 不重写 standard_doc 04 的实现状态(2026-08 已重建)

## Decisions

### D1:PLAN.md — 放弃 Phase 框架,重写为「现状 + 状态 + 缺口 + 方向」

Phase 0-12 框架完成使命(全部 phase 已实施或转缺口)。新结构:项目概览 → 30 章实现状态表(引用 04)→ 剩余缺口清单(引用 04 未实现清单)→ 后续方向(符号等级 Z3 验证、统一约束求解、LLVM 真编译链、core 测试补强)。
**备选**:保留 Phase 框架只更新进度。否决:框架本身已失真(Phase 4-12 的「待实施」与现状矛盾),保留会持续误导。

### D2:PHASE 总结 — 合并归档为 `docs/PHASE-HISTORY.md`

11 份 `PHASE{N}_SUMMARY.md` 合并为单份历史文档(按 phase 排序,标注「历史记录,现状以 standard_doc/04 为准」),删除散文件。
**备选**:保留散文件。否决:11 份过时文件占根目录,与「文档对齐现状」目标冲突。

### D3:docs/spec.md — 已实现章正文对齐现状,愿景章保留标注

两分法:
- **✅/⚠️ 已实现章节**:正文更新为现状描述(实现语义、已知近似);⚠️ 章保留「缺什么」说明
- **⬜ 设计阶段章节**:正文保留愿景,强化「设计阶段」标注(现状无 ⬜ 章,故实际是 ⚠️ 章的近似说明)
实现方式:按 04 的 30 章状态逐章核对正文,只改与现状冲突的段落,不重写设计意图。

### D4:CHANGELOG — 结构整理,条目全保留

0.1.0 下 5 个大块(LLVM/效果/宏/OOP/进程/FRP/逻辑/词法/IR/测试/液态/类型系统/逻辑验证/工具链/HoTT/文档/依赖线性)合并重排为「新增(按特性域分组)/修复」,每条目保留原文。无条目删除。
**风险**:条目丢失 → 重排后逐条比对(脚本统计前后条目数)。

### D5:standard_doc 核对 — 以 03 参考表为主

03-reference 的 CLI 参考(`--typecheck` 输出含液态验证统计等)、内置函数表(新增 gensym/find-attack/check-equivalence/reflect-type 等)、示例表(17 个)逐项与 main.rs/interpreter.rs 核对;01/02 增补已实现的语法(依赖等级已在 01 §6.0);INDEX 导航更新。

### D6:git 卫生顺序 — 先 ignore 后 add,验证收尾

1. `.gitignore` 追加 `target.tar`/`target.tar.zst`
2. `git add -A`(遵守新 ignore;`.agents/`/`.zcode/` 若未被 ignore 则按现状保留未追踪——需确认是否 ignore)
3. `git status` 验证:仅剩有意保留项
4. 不提交(提交由用户决定;本变更只保证追踪状态)
**风险**:误加构建产物 → ignore 先行;`.agents/`/`.zcode/` 工具目录是否追踪由用户现状决定(不动)。

## Risks / Trade-offs

- [spec 正文改写误伤设计意图] → 只改与现状冲突的段落,保留设计语言;每章改动后与 04 对照
- [CHANGELOG 条目丢失] → 重排后脚本比对条目数
- [PHASE 合并丢历史] → 单份历史文档保留全部内容,仅去重
- [git add 误纳] → ignore 先行 + status 验证;不自动 commit
- [文档量大(30 章 + 5 文档 + 11 历史)] → 分层提交:README/PLAN/CHANGELOG → standard_doc → spec → PHASE → git

## Migration Plan

无部署概念。实施顺序:PLAN/README/CHANGELOG → standard_doc → spec 正文 → PHASE 归档 → git 卫生 → 全量验证。回滚:git checkout 对应文件(git 卫生步骤本身提供快照)。

## Open Questions

- 无。`.agents/`/`.zcode/` 工具目录保持现状(不追踪也不 ignore),若用户后续想追踪可单独处理。
