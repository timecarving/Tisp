## 1. 入口与计划文档

- [x] 1.1 `README.md`:章节数 30→32、⚠️ 部分实现清单收敛(仅 §11/§19)、示例数 18→19
- [x] 1.2 `PLAN.md`:行数/测试数 17,878/177→26,527/351、状态分布 8/21/1→30/2/0、剩余缺口刷新、tisp-core 行数 1,193→1,519

## 2. 语言标准文档

- [x] 2.1 `standard_doc/INDEX.md`:章节数 30→32、docs/spec.md 行数 1569→1680
- [x] 2.2 `standard_doc/03-reference.md`:测试数 177→351、示例表补 finish-design/finish-partial 两条(17→19)
- [x] 2.3 `standard_doc/02-advanced-features.md`:节级状态符号 ⚠️/⬜→✅(依赖等级/HoTT/溯因/其他演算/类型类/LLVM)并删已实现子句

## 3. 设计规范与项目档案

- [x] 3.1 `docs/spec.md`:17 个章节标题内联符号 ⚠️→✅,仅 §11/§19 保留 ⚠️
- [x] 3.2 `openspec/project.md`:章节数 30→32、示例 13→19、能力规范「空」→19、文档地图(PHASE{N}_SUMMARY 移除)、已知局限收敛
- [x] 3.3 `CHANGELOG.md`:顶部补「当前状态(0.1.0)」摘要条目

## 4. 验证与收尾

- [x] 4.1 `cargo check --workspace` 零警告 + `cargo test --workspace` 351 全绿(文档改动不影响编译,回归确认)
- [x] 4.2 全文档 grep 复查:无 177/17,878/30 章/8-21-1 等残留过时事实
