# 文档重写与 git 卫生 — 任务清单

方案依据:`design.md`;事实源:`standard_doc/04-implementation-status.md`。

## 1. 核心文档重写

- [x] 1.1 `PLAN.md` 重写:项目现状 + 30 章状态表(引用 04)+ 剩余缺口 + 后续方向;删除 Phase 框架
- [x] 1.2 `README.md` 核对更新:实现状态概览(液态/依赖线性 ✅)、测试数 177、示例数 17、文档地图、CLI flags
- [x] 1.3 `CHANGELOG.md` 整理:0.1.0 五大块合并重排(按特性域分组),条目全保留;重排后逐条比对

## 2. standard_doc 核对

- [x] 2.1 `03-reference.md`:CLI 参考更新(`--typecheck` 输出含液态统计/特化数/monadic 标注);内置函数表补 gensym/find-attack/check-equivalence/reflect-type/solve-all 等;示例表核对 17 个
- [x] 2.2 `01-language-core.md`/`02-advanced-features.md`:核对已实现语法与文档一致(等级表达式、类型族、MPST 等已增补的部分确认)
- [x] 2.3 `INDEX.md`:导航更新(测试数、示例数、章节链接)

## 3. spec 正文对齐

- [x] 3.1 `docs/spec.md` ✅ 章正文核对:§10(QTT 依赖等级)、§12(效果)、§14(cc)、§15(液态)、§23(类型类)、§25(模块)、§27(进程)——正文与现状冲突处更新
- [x] 3.2 `docs/spec.md` ⚠️ 章核对:近似实现标注(Cohesive 最小语义、反射、特化等)——补充「现状近似」说明
- [x] 3.3 30 章逐章过一遍,确认无「愿景语言冒充现状」的段落

## 4. PHASE 历史归档

- [x] 4.1 合并 PHASE2-12 为 `docs/PHASE-HISTORY.md`(按 phase 排序,标注历史性质)
- [x] 4.2 删除根目录 11 份 PHASE 散文件

## 5. git 卫生

- [ ] 5.1 `.gitignore` 追加 `target.tar`/`target.tar.zst`
- [ ] 5.2 `git add -A`:三轮变更文件 + openspec/ + 新示例入库
- [ ] 5.3 `git status` 验证:仅剩有意保留项(.agents/、.zcode/ 按现状);`git ls-files` 抽查关键新文件已追踪
- [ ] 5.4 不自动 commit(提交由用户决定)

## 6. 最终验证

- [ ] 6.1 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告(文档变更不破坏构建)
- [ ] 6.2 文档交叉核对:README/04/INDEX 的数字与链接一致;CHANGELOG 条目数与重排前一致
- [ ] 6.3 最终 `git status` 快照确认
