# 递归与闭包类型检查修复 — 设计

## Context

动机与复现形态见 `proposal.md`;行为要求见 `specs/recursion-closure-fixes/spec.md`。关键现状(2026-08 实测):

- `TypeInfer::infer_program`(type_infer.rs:53)逐 def 推断:先 `initial_env`,再遍历 defs 调 `infer_def`(226 行)。`infer_def` 已有递归处理:「for recursive definitions, add a fresh type variable first」(fresh_ty → env → infer body → unify)
- 缺陷 1(定义顺序):前向引用 `(defn main [] (foo 1))` 在前时,`foo` 不在 env → `unbound variable`(Var 分支对未绑定符号直接报错);相互递归同样(后定义者未入 env)
- 缺陷 2(let 内递归):`infer_expr` 的 `Let` 分支(约 570 行)先推断 value 再 bind——value 中引用自身 → unbound
- 缺陷 3(递归返回闭包):`(defn make-countdown [n] (if (= n 0) (fn [] 0) (fn [] (make-countdown (- n 1)))))` 报 `cannot unify i64 with Fun`——if 分支合并时 `(make-countdown ...)` 的占位类型与闭包类型 unify 失败,推测是递归占位 fresh 变量在 if 分支中被替换为错误类型或 generalize 时机问题(实施时以诊断为准)
- 运行时:解释器自递归求值,深递归(如 sum-to 100)栈溢出——**非本变更范围**(记录为已知局限)
- 约束:既有 177 测试保持全绿;`cargo check --workspace` 零警告

## Goals / Non-Goals

**Goals**
- 定义顺序无关:前向引用与相互递归通过类型检查
- let 内递归通过类型检查
- 递归返回闭包通过类型推导
- `--typecheck` 与 `--run` 对同一程序接受行为一致

**Non-Goals**
- 解释器 TCO/显式栈(深递归栈溢出修复)——独立后续,本变更仅记录
- 运行时语义改动(仅类型检查层)

## Decisions

### D1:两遍推断 — infer_program 先行收集全部 def 占位

重构 `infer_program`:
1. 第一遍:遍历全部 defs,对每个 def 名插入 `fresh 类型变量`(mono scheme)到 env——**全部 defs 先入环境**(前向引用与相互递归自然解决)
2. 第二遍:逐 def 调 `infer_def`;`infer_def` 现有「fresh var → infer → unify」逻辑保留,但**不再重复插入**同名绑定(占位已存在,直接 unify 占位与推断结果)
3. 推断完成后照常 generalize(env 中占位类型此时已被替换为具体类型)
**备选**:惰性解析(引用时若未定义则注册占位)。否决:两遍方案更简单、与现有递归处理机制同构。

### D2:let 内递归 — value 推断前先绑定 fresh 变量

`infer_expr` 的 `Let` 分支改为:绑定名先插入 env(fresh 变量)→ 推断 value → unify 绑定类型与 value 类型 → 推断 body。与 def 的递归处理机制一致。
**风险**:let-polymorphism(generalize at let)——绑定 fresh 变量为 mono,value 推断后 generalize 时机不变;既有 let 多态测试回归验证。

### D3:递归返回闭包 — 诊断结论:非 bug,修正范围

2026-08 实施诊断:原复现用例 `make-countdown` 是无限类型(`T = Unit -> T`,递归返回自引用),HM 类型推断拒绝(`cannot unify i64 with Fun`)是 **occurs check 的正确行为**;有限类型递归返回闭包(`make-adder-n : i64 -> (i64 -> i64)`)已正常工作。
**修正**:本能力不修复「递归返回闭包」(无 bug),改为固化两类用例——有限类型通过 + 无限类型拒绝(防回归)。

### D4:回归固化 — 复现形态转测试

4 个复现用例(bug5/11/13/14)+ 一致性用例转 `type_infer`/interpreter 测试;真实类型错误用例(负例)确认仍被拒绝(防误放行)。既有 177 测试全绿为回归门槛。

### D5:栈溢出 — 记录不修复

`(sum-to 100)` 栈溢出源于解释器自递归求值(每层解释多帧),TCO 或显式求值栈是独立工程;本变更在 04 文档记录为已知局限。

## Risks / Trade-offs

- [两遍推断破坏 let 多态 generalize 时机] → 既有 let 多态测试回归;generalize 逻辑保持逐 def 执行
- [占位变量污染使类型错误误放行] → 负例测试(真实类型错误仍报错)兜底
- [bug14 根因诊断耗时] → 实施首步为最小复现 + 诊断打印;修复方向已列
- [env 占位被后续 def 覆盖] → 第一遍插入后第二遍只 unify 不重插

## Migration Plan

无部署概念。实施顺序:最小复现与诊断(bug14)→ 两遍推断(D1)→ let 递归(D2)→ 闭包返回修复(D3)→ 测试固化(D4)→ 全量验证。回滚:git revert 对应提交。

## Open Questions

- 无。bug14 的具体失败点以实施时诊断为准,修复方向已明确,不改变 spec/方案/任务拆分。
