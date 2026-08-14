## Why

`standard_doc/04-implementation-status.md` 的「⚠️ 部分实现」清单列有 17 条有骨架但语义残缺的特性。逐条代码审计(2026-08)显示:它们的「已实现」半边多是上一轮 `complete-partial-features` 刚接上线的骨架,而「缺」半边是更深的语义层——且这 17 条的缺失半边**既不在进行中变更 `implement-design-stage-features`(目标是 10 条 ⬜ 仅设计项),也不在 `PLAN.md` 的后续方向里**,是路线图真空。本变更补上这一层,把 17 条全部推到 ✅ 全链路可用。

## What Changes

分五个领域,把每条「部分实现」推到完整(✅):

**类型 / 等级 / 模式系统**
- def 六维注解 `->[ε,ρ,@r,m,d]` 语法解析 + 贯通 `FunAnnotation`(现为硬编码常量,`@` 被 lexer 标记但 parser 拒绝)
- 类型族:单声明多模式语法 + `rewrite` 规则 + 未声明族正确报错 + 测试覆盖
- 类型一等值:类型值显示(println 现输出 `...`)、类型值模式匹配(现恒 false)、EffectRow/Grade/Mode/Determinism 一等值、codegen 贯通
- QTT 隐式绑定默认 0(§10.2):`{n : T}` 语法(现解析报错)+ 默认等级逻辑
- 资源代数:spec `:semiring` 关键字语法(现仅位置式)+ Cost/复杂度检查 + `□_r`/`@[n]` 语法与推断
- Mercury 多模式:内联 `:in/:out` 参数模式(现静默丢弃)+ 函数体自动模式推断(接线死代码 `infer_modes`)+ 同名多模式重载

**模块与定义**
- 私有定义 `defn-`/`def-`:定义可见性字段 + 导出表 + 跨文件导入过滤(现无任何导出机制)

**宏 / 泛型 / 反射 / 效果优化**
- 宏卫生:fn/lambda/if-let/match 绑定卫生(现仅 let)+ `~x` unquote 参与参数替换
- 泛型特化:类型(构造器)驱动特化,替换字面值特化;接入 `--run` 执行路径(现仅 `--typecheck` 展示)
- 反射:`mode-of`/`effects-of`/`determinism-of` 查询真实签名,替换硬编码常量;`type-of` 返回静态类型
- Monad 优化:真实单处理器/无嵌套分析 + 直接状态传递编译 + `mlet`/`get-m`/`put-m`/`pure` monadic 编码

**逻辑与验证**
- Prolog 多解:续延式回溯 + 逐分支解隔离 + 结构化值统一(替换首解/0 解/垃圾解现状)
- CLP:非线性约束精确收窄(z 也收窄)+ 精确除法 + 线性表达式编译
- ALP:domain 感知的假设生成 + 正确域相交 + 逻辑变量溯因

**HoTT 与演算**
- HIT 端点方程求解:结构化边界子句 + 端点代入 + 唯一一致性(现为字符串 + 死代码 hott.rs)
- deriving Ord:`ord-*` 生成(现 `_ => {}` 静默忽略)
- 演算互编码:补 async→sync / applied→π / ρ→π 三种编码 + 观察等价(互模拟)检查 + 修复 SKI reduce 丢负载

## Capabilities

### New Capabilities

- `module-visibility`: 定义可见性(私有/导出)与跨文件导入过滤语义

### Modified Capabilities

- `type-system-extensions`: 六维注解、类型族多模式+rewrite、类型一等值完整、QTT 默认 0、资源代数 Cost+□_r、Mercury 自动模式推断
- `toolchain-and-macros`: 宏 fn 参数卫生、类型驱动特化、反射补全、Monad 优化完整编译
- `hott-and-deriving`: HIT 端点方程求解、deriving Ord、演算互编码+观察等价
- `logic-and-verification`: Prolog 多解、CLP 非线性、ALP 多解

## Impact

- `crates/tisp-frontend`: lexer/parser(六维注解、`@`、`□`、`{n:T}`、`:semiring` 语法)、desugar(HIT 边界结构化、deriving Ord、宏卫生)
- `crates/tisp-core`: types.rs(`FunAnnotation` 贯通、类型族多模式)、core_ast.rs(可见性字段)、grades.rs(Semiring/Order 接线)
- `crates/tisp-middle`: type_infer(类型族 rewrite、□_r 引入)、grade_check(默认 0)、mode_analysis(自动推断接线)、specialize(类型驱动)、effect_compile(状态传递/monadic)
- `crates/tisp-backend`: interpreter(类型值显示/匹配、反射、Prolog 回溯、CLP/ALP、HIT 端点、SKI/演算)、codegen(类型值/reflect)
- `crates/tisp-runtime`: logic.rs(续延回溯接线)、constraint.rs(非线性/域相交)、hott.rs(端点)、process.rs(编码/互模拟)
- 文档: `standard_doc/04-implementation-status.md`(17 条 ⚠️→✅ 同步)、`CHANGELOG.md`

> 注:本变更覆盖的「类型族 rewrite」与「HIT 端点方程」与进行中变更 `implement-design-stage-features` 的任务 1.1 / 2.3 存在重叠;实现时需协调归属,避免重复实现。
