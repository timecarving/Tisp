## Why

2026-08 实现状态审计(standard_doc/04)显示 30 章中 21 章为 ⚠️ 部分实现。用户点名四项核心特性(依赖等级、HoTT、溯因 ALP、CLP 算术约束编译)继续实现到**全链路可用**(04 文档 ✅ 级,含测试与示例),并涵盖同方向的类型系统类缺口。这些能力已有主 spec(type-system-extensions / logic-and-verification / hott-and-deriving),本变更以 MODIFIED delta 推进——是本项目首次的既有能力增量。

## What Changes

**type-system-extensions(MODIFIED 4 需求 + ADDED 1)**
- 依赖类型等级传播:符号等级使用不等式(count ≤ n)经 Z3 严格验证(替换「不可判定警告放行」)
- 类型一等值:`Value::Type` 运行时类型值变体(反射已实现,类型值可绑定/传递/比较)
- 类型族与关联类型:多模式实例与 rewrite 简化规则(单模式归约已实现)
- Mercury 多模式谓词:多模式自动推断(显式 `:mode` 签名已实现)
- **ADDED** 隐式绑定默认 0:§10.2 隐式绑定默认 0 级(擦除)

**logic-and-verification(MODIFIED 2)**
- CLP 域间约束传播:算术约束编译——非线性约束(乘除/模)与全局约束(全不同等)的域传播(线性已实现)
- ALP 溯因真实化:多解解释枚举与目标不可满足原因报告(单解一致性已验证)

**hott-and-deriving(MODIFIED 2 + ADDED 1)**
- Cohesive HoTT 语义层:ʃ 形状代数(路径连通计算,替换最小可区分语义)
- HIT 边界语义:端点方程求解(替换符号一致性检查)
- **ADDED** HComp/Transp 真实求值(§16.3:同伦合成与传输的非平凡语义,替换直通求值)

**验收标准**:上述能力相关章节 04 状态升级为 ✅(全链路可用,含测试与示例);其余 ⚠️ 章不受影响。

## Capabilities

### New Capabilities

(无)

### Modified Capabilities

- `type-system-extensions`:依赖等级 Z3 验证、类型一等值 Value::Type、类型族 rewrite、多模式推断、隐式绑定默认 0 的需求变化
- `logic-and-verification`:CLP 算术约束编译、ALP 多解枚举与原因报告的需求变化
- `hott-and-deriving`:Cohesive 形状代数、HIT 端点方程、HComp/Transp 真实求值的需求变化

## Impact

- `crates/tisp-core`:`types.rs`(Value::Type 相关、TypeFamilyInstance 多模式)、`core_ast.rs`(如需要)
- `crates/tisp-frontend`:`desugar.rs`(类型族多模式语法、隐式绑定等级)
- `crates/tisp-middle`:`grade_check.rs`(符号等级不等式产出)、`liquid_types.rs`(Z3 不等式验证接口)、`type_infer.rs`(多模式推断、类型一等值)、`specialize.rs`/类型族归约(rewrite)
- `crates/tisp-backend`:`interpreter.rs`(Value::Type 求值、HComp/Transp、ʃ 形状)、`liquid_verify.rs`(等级不等式验证接入)
- `crates/tisp-runtime`:`constraint.rs`(非线性/全局约束)、`abduction.rs`(多解枚举)、`hott.rs`(HComp/Transp/形状)
- 文档:`standard_doc/04-implementation-status.md`(相关章 ⚠️→✅)、`standard_doc/01/02`、`CHANGELOG.md`、示例
