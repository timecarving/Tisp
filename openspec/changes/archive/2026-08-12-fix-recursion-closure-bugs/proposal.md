## Why

用户报告的三类恶性 bug 经 2026-08 复现验证,**在新版本(0.1.0,177 测试全绿)仍然存在**:

1. **递归类型推导错误**(4 个具体形态):相互递归(defpred 式互调)报 `unbound variable`;let 内递归(`(let [fact (fn ... (fact ...))])`)报 `unbound variable: fact`;递归返回闭包(尾递归产出闭包)报 `cannot unify i64`——类型推断对「定义时引用自身或尚未定义的函数」处理不完整
2. **闭包生命周期/类型推导不稳定**:闭包类型在部分上下文可推断、部分失败(如上递归返回闭包)
3. **闭包时可用时不可用**:`--typecheck` 对「使用在前、定义在后」的函数/闭包报 `unbound variable`,而 `--run` 正常(运行时全量注册)——两阶段行为不一致,闭包场景最易触发
4. **复现修正**:原「递归返回闭包」用例经诊断为无限类型(`T = Unit -> T`),HM 拒绝是正确行为;有限类型递归返回闭包已正常工作——该形态从修复范围移除,固化为通过/拒绝双用例
5. **附带发现**:深递归 `(sum-to 100)` 在 `--run` 栈溢出(解释器无尾调用优化,100 层解释递归即耗尽默认栈)

## What Changes

- **type_infer 定义顺序无关**:两遍推断——先收集全部 def 签名(占位类型变量)再推断各 body;前向引用与相互递归通过类型检查
- **let 内递归**:let 绑定值推断前先把绑定名(带 fresh 类型变量)插入环境,支持 `(let [f (fn ... (f ...))] ...)` 局部递归
- **递归返回闭包确认**(修正):有限类型递归返回闭包已工作(测试固化);无限类型拒绝确认为正确行为
- **行为一致性**:`--typecheck` 与 `--run` 对同一程序的接受/拒绝一致(定义顺序不再影响结果)
- **回归保障**:三类 bug 的 4 个复现形态固化为测试用例;既有 177 测试保持全绿
- **运行时栈溢出**:记录为已知局限(解释器递归深度受调用栈限制;TCO/显式栈列为后续,不在本变更修复范围)

## Capabilities

### New Capabilities

- `recursion-closure-fixes`:前向引用/相互递归/let 内递归/递归返回闭包的类型检查行为,以及 `--typecheck` 与 `--run` 一致性的行为规范

### Modified Capabilities

(无)

## Impact

- `crates/tisp-middle/src/type_infer.rs`:infer_program 两遍推断、let 递归绑定、递归闭包统一
- `crates/tisp-backend/src/interpreter.rs`:(仅测试用例,无行为改动)
- `examples/`:递归/闭包回归示例(可选)
- 文档:`CHANGELOG.md`、`standard_doc/04-implementation-status.md`(如有状态变化)
