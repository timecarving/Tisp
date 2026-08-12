# 递归与闭包类型检查修复 — 任务清单

规范依据:`specs/recursion-closure-fixes/spec.md`;方案依据:`design.md`。

## 1. 最小复现与诊断

- [x] 1.1 诊断结论:bug14 原用例为无限类型(T = Unit -> T),HM 拒绝是正确行为;有限类型递归返回闭包(i64 -> (i64 -> i64))已正常工作;递归/闭包类 bug 实为 3 个:定义顺序敏感、相互递归、let 内递归
- [x] 1.2 复现用例固化为测试骨架(bug5/11/13 三个形态 + 有限递归闭包通过用例 + 一致性用例)

## 2. 两遍推断(定义顺序无关)

- [x] 2.1 `infer_program` 重构:第一遍为全部 defs 插入 fresh 占位类型到 env;第二遍逐 def 推断(unify 占位)
- [x] 2.2 前向引用通过:`(defn main [] (foo 1))` 在前 + `foo` 在后 → typecheck 通过
- [x] 2.3 相互递归通过:is-even/is-odd 互调 → typecheck 通过、run 正确
- [x] 2.4 既有递归测试回归(fibonacci/递归 defs)+ let 多态测试回归

## 3. let 内递归

- [x] 3.1 `Let` 分支:绑定名先入 env(fresh 变量)再推断 value,unify 后推断 body
- [x] 3.2 let 内递归通过:`(let [fact (fn ... (fact ...))] (fact 5))` → typecheck 通过、run 输出 120
- [x] 3.3 let 多态回归(let-polymorphism 不破坏)

## 4. 递归返回闭包确认(修正后)

- [x] 4.1 有限类型递归返回闭包用例固化测试(make-adder-n:i64 -> (i64 -> i64),typecheck + run 输出正确)
- [x] 4.2 无限类型负例确认:自引用返回(T = Unit -> T)被拒绝(occurs check 正确行为)
- [x] 4.3 负例确认:真实类型错误(如 i64 当函数调用)仍报错(修复不引入误放行)

## 5. 测试固化与一致性

- [x] 5.1 4 个复现形态 + 一致性用例全部转正式测试(通过态)
- [x] 5.2 `--typecheck`/`--run` 一致性用例(定义顺序两阶段均接受)
- [x] 5.3 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告

## 6. 文档

- [x] 6.1 `CHANGELOG.md` 记录修复(4 形态 + 一致性)
- [x] 6.2 `standard_doc/04-implementation-status.md` 更新(如类型系统相关状态变化);深递归栈溢出记录为已知局限
- [x] 6.3 最终验证:示例抽查(递归/闭包类)+ 全量测试
