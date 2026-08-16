## Context

当前问题（见 proposal.md - Why）：
1. `tutorial/07-logic-programming.md` 的 12 范式表引用 `pf-*` 名，但 `pf-*` 简化投影有 panic/签名错误，真实可用内置为 `subsume`/`tabling`/`higher-order-call` 等
2. `pf-settle` 在 `crates/tisp-runtime/src/facility.rs:225` 直接 panic（数组越界）
3. `pf-higher-order`/`pf-prob`/`pf-subsume` 在 `type_infer.rs` 的注册签名与投影实现不一致
4. 12 个真实逻辑范式内置没有任何教程示例文件验证可运行性
5. 验收链未要求 `cargo build --release`，导致 release 二进制停留在旧代码
6. REPL 提示符下 State/Signal 效应操作的可用性限制未在教程中系统记录

## Goals / Non-Goals

**Goals:**
- 教程 ch07 表格指向真实可用内置名，而非有 bug 的 `pf-*` 投影
- `pf-settle` 越界 panic 改为 `Error: eval error: defeasible-settle:rules 长度须为 3 的倍数` 形式的显式报错
- `pf-higher-order`/`pf-prob`/`pf-subsume` 签名与其实现类型对齐（通过检查或修正使 typecheck 通过）
- 新增 `tutorial/examples/ch07-logic12.tisp` 示例文件，调用全部 12 个真实内置，通过 `--typecheck` 和 `--run`
- 所有验收任务末尾补充 `cargo build --release` + release 二进制冒烟测试（调用几个范式内置确认不 panic）

**Non-Goals:**
- 不删除 `pf-*` 简化投影（保持向后兼容），但不再在教程中推荐使用
- 不修改 REPL 的表达式行效应处理方式（该问题需单独规范提案）
- 不重新设计逻辑范式架构

## Decisions

### D1: 教程表直接替换为真实内置名，pf-* 加脚注

ch07 表格改为：

| # | 范式 | 内置入口（推荐） | 遗留投影 | REPL 可用 | 状态 |
|---|------|-----------------|---------|----------|------|
| 1 | 高阶逻辑 | `higher-order-call` | — | ✅ | ⚠️ |
| 2 | 归纳逻辑 | `ilp-induce` | — | ✅ | ⚠️ |
| ... | ... | ... | ... | ... | ... |

`pf-*` 名从主表移除，加脚注说明「遗留投影 `pf-*` 仍可调用但建议使用新名；`pf-settle`/`pf-higher-order`/`pf-prob`/`pf-subsume` 有已知问题」。

**理由**：真实内置是 commit `c212f11` 添加的正式接口，`pf-*` 是过渡投影。教程应指向正式接口。

### D2: pf-settle panic 修复方案

`crates/tisp-runtime/src/facility.rs` 第 225 行（`pf-settle` 实现）在 `defeasible-settle` 被调用时数组越界。修复：
- 在逻辑运算前增加前置检查：`rules` 长度 `% 3 == 0`，否则返回 `Err("defeasible-settle:rules 长度须为 3 的倍数")`
- 该验证已在真实内置 `defeasible-settle` 中实现（验证输出可见），`pf-settle` 投影缺失了同样的验证

**理由**：真实内置已有正确验证，`pf-settle` 投影实现缺少同等级别的输入合法性检查。

### D3: pf-higher-order/pf-prob/pf-subsume 签名修正

检查 `type_infer.rs` 中这些 `pf-*` 的签名类型与实际投影实现返回类型，确保对齐：
- `pf-higher-order`: 当前签名 `i64 → i64 → bool` 但实际投影期望 `i64 → (i64 → bool)` ？需要检查实现确定正确类型
- `pf-prob`: 当前签名 `i64 → list → f64` 但输入第一个参数是 i64（事实 id），第二个是 list（概率列表）— 实际实现需 `i64 → list → f64` 但检查 list 元素是否为 f64？需调试
- `pf-subsume`: 类似问题

### D4: 新增 ch07-logic12.tisp 示例

直接写入 12 个真实内置的调用，使用合法的哑数据，预期全部 `--typecheck` 通过、`--run` 输出合理结果。`reactive-eval` 因需 Signal 效应，在示例中通过带效应行的 `main` 包裹后验证。

### D5: 验收补 release 重建

在 tasks.md 的验收任务中添加：
```bash
cargo build --release
./target/release/tisp --eval '(subsume [1 2] 1 2)'  # 确认范式内置可用
./target/release/tisp --eval '(tabling [1 2] [3 4] 1)'  # 确认无 panic
```

## Risks / Trade-offs

- **[风险] pf-* 签名修正可能引发向后兼容性问题**：如果下游代码依赖了 `pf-*` 的当前（错误的）签名。→ **缓解**：这些 `pf-*` 当前签名不正确导致 typecheck 失败，实际上无法被下游使用；修正后 typecheck 反而能通过，不会破坏现有代码。
- **[风险] 教程表格只显示推荐名后，用户可能尝试 `pf-higher-order` 等旧名并遇到错误**：→ **缓解**：加脚注明确说明哪些 `pf-*` 有已知问题，并建议使用新名。
- **[风险] 新增示例文件增加维护成本**：→ **缓解**：示例文件结构简单，每个范式一行调用，易于随真实内置更新。