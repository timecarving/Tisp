## Why

ch07 教程中 12 种逻辑编程范式的表格指向了 `pf-*` 简化投影名称，但这些 `pf-*` 内置存在多个运行时 bug（`pf-settle` panic、类型签名不匹配、非法输入静默异常而非显式报错），而真实可用内置（`subsume`/`tabling`/`higher-order-call` 等）未被教程覆盖。同时，8 类编程范式与 12 逻辑范式的 REPL 可用性未在教程中准确记录。旧 release 二进制因验收链未要求重建而长期缺少这些内置，导致用户 `./tisp` 中全部 unbound variable。

## What Changes

1. **教程 ch07 逻辑范式表更新**：将表格从 `pf-*` 简化投影名更新为真实内置名（`subsume`、`tabling`、`higher-order-call`、`ilp-induce`、`plp-marginal`、`temporal-eventually`、`defeasible-settle`、`fuzzy-eval`、`typed-pred`、`reactive-eval`、`context-query`、`modal-possible`），补充 REPL 可用性列，标注各范式实现状态
2. **教程新增 ch07 验证示例**：添加一个 `ch07-logic12.tisp` 示例文件，调用全部 12 个真实内置并验证可 `--typecheck` 和 `--run`
3. **修复 `pf-settle` panic**：`crates/tisp-runtime/src/facility.rs:225` 数组越界 panic 改为显式错误返回（违反 paradigm-usability-contract 第④条）
4. **修复 `pf-higher-order`/`pf-prob`/`pf-subsume` 类型签名**：`type_infer.rs` 中这些 `pf-*` 的注册签名与实际投影实现不一致，需对齐
5. **教程补充 REPL 效应限制说明**：在 ch01、ch07、ch13 及 A4 中补充 State/Signal 效应操作无法在 REPL 提示符直接求值的说明（已完成初步修改，需确认一致性）
6. **教程 ch11 FFI 示例标注修正**：将 `✅ 可运行` 改为 `⚠️ 运行需 --features ffi`（默认构建无 ffi feature，`--run` 报错）
7. **验收任务补 release 重建**：所有验收任务末尾增加 `cargo build --release` 及 release 二进制冒烟测试（确保 future 变更不再次出现新旧二进制不一致）

## Capabilities

### New Capabilities
（无新能力引入——变更集中于修复现有 bug 和更新文档）

### Modified Capabilities
- `paradigm-usability-contract`: 现有 Requirement 第④条「非法输入 SHALL 显式报错」当前被 `pf-settle` panic 违反，需确认修复后符合。无需修改 spec 文本，仅修复实现使其符合已有要求。
- `logic-programming-paradigms`: 教程 ch07 的范式表名与实现不一致，需更新文档。不修改 spec 要求，只修正教程内容。

## Impact

- `tutorial/07-logic-programming.md`：表格更新 + 新增 12 范式验证示例引用 + REPL 可用性列
- `tutorial/examples/ch07-logic12.tisp`：新增示例文件
- `crates/tisp-runtime/src/facility.rs`：修复 `pf-settle` panic（第225行数组越界）
- `crates/tisp-middle/src/type_infer.rs`：修复 `pf-higher-order`/`pf-prob`/`pf-subsume` 签名
- `tutorial/01-getting-started.md`、`tutorial/13-programming-paradigms.md`、`tutorial/A4-error-messages.md`：REPL 效应限制补充（已部分完成）
- `tutorial/11-ffi-and-system.md`：FFI feature 标注修正（已部分完成）