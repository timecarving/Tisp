## Why

两轮探索(2026-08)确认了项目的双重债务:① 导航文档腐化——`standard_doc/04-implementation-status.md` 章节号错位、两张未实现清单大面积过时(至少 14 项已实现仍标缺失),`docs/spec.md` 附录 A/B/F 与实现脱节(示例索引 6/11 指向不存在文件);② 剩余功能缺口约 20 项,横跨类型系统、逻辑/验证、工具链、HoTT,且真缺口被假缺口淹没导致优先级失真。本变更一次性补齐文档债与全部剩余功能缺口。

## What Changes

**文档修复(实施期间同步完成)**
- 重建 `standard_doc/04-implementation-status.md`:修正总表章节号(23 章起偏移 -1)、按 CHANGELOG+代码抽查重建两张清单、剔除过时条目
- 对齐 `docs/spec.md` 附录:附录 A BNF 补块注释 `#| |#`/构造器名 `:::`/时序算子 `⃝`;附录 B 保留字补 `defsession`/`verify!`/`solve-all`/`find-all` 等;附录 F 示例索引重建
- 状态内联:`docs/spec.md` 各章标注 ✅/⚠️/⬜(与 04 同符号),spec 成为唯一事实源

**功能实现(按 4 组 20 项)**

*类型系统深化(A 组)*
- QTT 运行时擦除/移动(§10.1):0 级参数运行时擦除、1 级移动语义
- Mercury 多模式谓词(§13):multi/semidet 等模式推断与调用检查
- 类型族/关联类型(§9):type family 声明、简写与归约
- 类型一等值(§9):`Value::Type` 变体,运行时类型操作
- 依赖等级传播(§19.1):Π/Σ 的等级维 r+s 传播(类型变体已有)
- defresource-algebra(§11.1):资源代数声明与 Cost 半环
- committed-choice 运行时语义(§14.3):CcMulti/CcNonDet 的 commit 行为

*逻辑与验证(B 组)*
- CLP constrain 域间传播(§21.5):不等式约束的域收缩(替换恒真 propagator)
- ALP 溯因真实化(§21.6):abduce 验证假设集而非占位字符串
- find-attack/dolev-yao(§28):攻击搜索与协议等价检查
- MPST 语法集成(§20.2/20.3):多参与方会话类型

*宏、工具链与系统(C 组)*
- 宏 hygiene/gensym(§24):卫生展开与唯一符号生成
- 编译期特化(§22.4):GenericDef 的 middle 层 monomorphization
- 真实 dlopen FFI(§26):libloading 动态库加载替代模拟函数表
- 反射函数真实化(§29):MetaQuery 返回真实类型/环境信息
- Monad 优化路径接线(§12.6):EffectCompiler 从检测到单处理器降级编译

*HoTT 与小件(D 组)*
- Cohesive HoTT 语义层(§17):ʃ(Shape)节点、crisp 上下文(⚠️ 最大件)
- HIT :boundary 语义(§7.4/16.3):路径构造器边界检查
- deriving 派生生成(§7.5):Eq/Ord/Show 自动生成
- 演算互编码(§27.10):pi-to-ski 等演算间转换

## Capabilities

### New Capabilities

- `type-system-extensions`:QTT 擦除/移动、Mercury 多模式、类型族、类型一等值、依赖等级传播、资源代数、committed-choice 的行为规范
- `logic-and-verification`:CLP 域间传播、ALP 溯因、find-attack/dolev-yao、MPST 的行为规范
- `toolchain-and-macros`:宏 hygiene、编译期特化、dlopen FFI、反射函数、Monad 优化路径的行为规范
- `hott-and-deriving`:Cohesive HoTT、HIT boundary、deriving 生成、演算互编码的行为规范

### Modified Capabilities

(无 — 文档修复无行为变化,不建 spec)

## Impact

- `crates/tisp-core`:`types.rs`(类型族/一等值/资源代数)、`core_ast.rs`(新节点或字段)
- `crates/tisp-frontend`:`desugar.rs`(defresource-algebra/type family/deriving 生成解析)、`lexer.rs`
- `crates/tisp-middle`:`type_infer.rs`(多模式/类型族/等级传播)、`effect_compile.rs`(Monad 降级)、新模块(特化)
- `crates/tisp-backend`:`interpreter.rs`(QTT 擦除/committed-choice/反射/Cohesive/abduce/CLP)、新模块(dlopen FFI、find-attack)
- `crates/tisp-runtime`:`logic.rs`/`constraint.rs`(CLP/ALP)、`abduction.rs`、`process.rs`(MPST/攻击搜索)、`hott.rs`、`metaprogram.rs`(gensym)
- `crates/tisp-cli`:`main.rs`(--verify 扩展)
- `Cargo.toml`:`libloading`(dlopen FFI)等新依赖
- 文档:`standard_doc/04-implementation-status.md`、`docs/spec.md`(附录+状态内联)、`CHANGELOG.md`、`README.md`、示例
