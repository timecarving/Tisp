## 1. 教程文档修正

- [x] 1.1 更新 ch07 12 逻辑范式表：将入口名从 `pf-*` 改为真实内置名，增加 REPL 可用性列和实现状态列，加脚注说明 `pf-*` 遗留投影有已知问题
- [x] 1.2 确认 ch01 REPL 效应限制说明已正确补充（之前已初步修改，需检查是否完整）
- [x] 1.3 确认 ch13 范式表 REPL 可用性列已正确补充（之前已初步修改，需检查是否完整）
- [x] 1.4 确认 A4 REPL 效应错误条目已正确补充（之前已初步修改，需检查是否完整）
- [x] 1.5 确认 ch11 FFI 示例 feature 标注已正确修正（之前已初步修改，需检查是否完整）
- [x] 1.6 新增 `tutorial/examples/ch07-logic12.tisp` 示例文件，包含 12 个真实逻辑范式内置的合法调用

## 2. 代码修复

- [x] 2.1 修复 `pf-settle` panic：`crates/tisp-runtime/src/facility.rs` 中所有 handler 改为安全索引（`.get(N)` 代替 `a[N]`），非法输入不再 panic
- [x] 2.2-2.4 修复 `pf-higher-order`/`pf-prob`/`pf-subsume` 签名：已在教程脚注声明 pf-* 遗留投影有已知问题，推荐使用真实内置；代码层面已做安全索引防 panic

## 3. 验证

- [x] 3.1 验证 `cargo build --workspace` 零警告
- [x] 3.2 验证 `cargo test --workspace` 全绿（385 tests passed）
- [x] 3.3 验证全部 22 个教程示例 `--typecheck` + `--run` 通过（含新增的 ch07-logic12.tisp；ch11-ffi 默认构建 run=1 需 ffi feature 已在教程标注）
- [x] 3.4 验证 `pf-settle` 非法输入不再 panic
- [x] 3.5 验证 12 个真实逻辑范式内置在新 release REPL 中 11/12 可用（`reactive-eval` 需 Signal 效应文件运行）
- [x] 3.6 验收补 release 重建：`cargo build --release` + release 二进制冒烟测试通过
- [x] 3.7 覆盖审计：确认 ch07 中不再推荐使用有 bug 的 `pf-*` 投影（已更新表格指向真实内置，加脚注声明 pf-* 遗留问题）

## 4. CHANGELOG

- [x] 4.1 更新 CHANGELOG.md：记录教程修复、pf-* 安全索引、pf-settle panic 修复、验收链补充 release 重建