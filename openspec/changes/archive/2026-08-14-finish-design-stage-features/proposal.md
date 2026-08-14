## Why

`standard_doc/04-implementation-status.md` 仍有「⬜ 仅设计」10 项无有效实现,加上上一轮 `finish-partial-features` 之后仍标 ⚠️ 的 5 处语义深度缺口(HIT hott.rs 接线、Cost 注解、Monad 状态线程、Prolog 续延回溯、演算互模拟),外加 deriving 未移到 desugar。本变更吸收进行中变更 `implement-design-stage-features`(0/29,目标 ⬜ 10 项),把剩余全部「未实现/部分实现」补齐到全链路可用。

## What Changes

**设计阶段(⬜ 10 项,吸收 implement-design-stage-features)**
- fun-ext / 幺半等价(有限域枚举验证)
- 时序模态完整语义(替换 ClockNew 占位)+ LTL-as-types + 多时钟
- Cohesive 连通图代数(ʃ 从端点连通推进到连通图)
- 依赖会话类型(§20.2/20.3 值依赖)+ MPST 保持
- 类型类完整实例解析(`:fun-deps`/超类/kind)
- 真实 dlopen FFI 全签名(指针/字符串/可变参)
- dolev-yao 攻击者模型(替换场景搜索)
- 编译指示全处理(`opt-level`/`inline!`/`specialize!`/`suppress-warning`)

**状态同步(已实现,仅 04/spec 标注)**
- 类型族 rewrite(多模式 + rewrite 形式,上一轮已做)
- 类型一等值 Value::Type(显示/匹配/六维反射,上一轮已做)

**深度缺口(⚠️ 语义层,本变更补)**
- HIT:hott.rs 泛型模块(Interval/PathTerm/Circle)接线解释器,替换内联占位
- 资源代数:Cost 注解语法 + 代价/复杂度推导
- Monad:直接状态线程编译(替换计数占位)
- Prolog:完整续延式回溯重入(替换首解/0 解)
- 演算:全演算互模拟/barbed 观察等价(替换 π→SKI 特例)
- deriving:从运行时内置移到 desugar 代码生成(`--desugar` 可见 + 不可派生字段报错)

## Capabilities

### New Capabilities

- `temporal-types`: 时序模态、LTL-as-types 与多时钟的行为规范

### Modified Capabilities

- `type-system-extensions`: 依赖会话类型、类型类 fun-deps/超类/kind、Cost 注解与推导、类型族 rewrite + 类型一等值状态同步
- `hott-and-deriving`: fun-ext/幺半等价、HIT 端点方程、Cohesive 连通图、hott.rs 接线、deriving 移 desugar、演算互模拟
- `toolchain-and-macros`: dlopen 全签名、编译指示全处理、Monad 直接状态线程
- `logic-and-verification`: dolev-yao 攻击者模型、Prolog 完整续延回溯

## Impact

- `crates/tisp-frontend`:desugar(时序/时钟/编译指示语义、deriving 代码生成、Cost 注解)、lexer/parser
- `crates/tisp-core`:types.rs(依赖会话、时序语义)、grades.rs(Cost 半环推导)
- `crates/tisp-middle`:type_infer(依赖会话/类型类约束/时序类型)、specialize/effect_compile(状态线程)、优化器(编译指示)
- `crates/tisp-backend`:interpreter(HoTM 接线、续延回溯、dolev-yao、dlopen 全签名、deriving)、process.rs(互模拟)
- `crates/tisp-runtime`:hott.rs(接线)、logic.rs(续延引擎)、process.rs(互模拟)、constraint.rs(Cost)
- 文档:`standard_doc/04-implementation-status.md`(⬜ 10 项升级 + ⚠️ 5 项升 ✅ + 过时同步)、`CHANGELOG.md`、示例

> 注:本变更吸收 `implement-design-stage-features`;实现后该变更应归档/废弃,避免两份计划并存。
