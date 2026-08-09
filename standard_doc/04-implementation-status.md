# 04 — Spec 实现状态与未实现特性清单

> 对照 `docs/spec.md`(30 章 + 6 附录)与 `crates/` 实际实现的逐章审计结果(0.1.0,2026-08)。
> 符号:✅ 完全实现 | ⚠️ 部分实现 | ⬜ 仅设计。证据为 `file:line`。

---

## 1. 逐章状态总表

| 章 | 特性 | 状态 | 摘要 |
|----|------|------|------|
| 1 | Introduction | ⚠️ | Lisp-1/无 GC 区域栈已落地;LLVM 仅文本 IR 骨架 |
| 2 | Design Philosophy | ⚠️ | 统一 def+六维注解、语法糖脱糖最实;统一约束求解、演算统一未达成 |
| 3 | Lexical Structure | ⚠️ | 标识符/字面量/行注释/块注释 `#| |#`/字符串转义齐; |
| 4 | Data Structures | ⚠️ | List/Unit/构造器(list/vector/hash-map/hash-set)可用;Vec/Map/Set 无持久化语义;quote 不产生数据 |
| 5 | Expressions | ⚠️ | 9 项中 9 项实现(含 `ann` §5.9); |
| 6 | Definitions | ⚠️ | def/defn/defpred/defmacro/defgeneric 等全有;私有语义、六维注解语法未解析 |
| 7 | ADT | ⚠️ | defdata/记录/记录字段访问 `(:field obj)`/GADT 返回类型可用;HIT 边界、deriving 生成缺 |
| 8 | Pattern Matching | ⚠️ | match/cons/通配符/穷尽性检查/or-pattern/guard 双形式/refined 模式齐; |
| 9 | Type System Overview | ⚠️ | HM 推断+泛化+rank-n+行多态有;类型族、类型一等值、五维子类型缺 |
| 10 | QTT | ⚠️ | Grade 0/1/ω 静态检查器有;运行时擦除/移动语义、隐式绑定默认 0 缺 |
| 11 | Graded Modal Types | ⚠️ | 资源代数结构有;`defresource-algebra` stub、□_r/◇_ε 无推理 |
| 12 | Effect System | ✅ | defeffect/handle/perform/续延 k 语义(多 body+状态写回)齐;§12.5 行消减、§12.6 monadic 优化检测接入 |
| 13 | Mode System | ⚠️ | Mode 枚举 + :free/:ground/:det/:nondet/:cc_* 注解生效;多模式谓词推断不支持 |
| 14 | Determinism | ⚠️ | 8 类范畴+注解写入 CoreDef;运行时 committed-choice 语义 |
| 15 | Liquid Types | ⚠️ | 精化类型结构+Z3 桥存在;**未知谓词一律放行**;检查器未接入 cli |
| 16 | HoTT | ⚠️ | Interval/Path/Glue 运行时类型齐;HComp/Transp 平凡求值、:boundary 忽略 |
| 17 | Cohesive HoTT | ⬜ | ♭/♯ 直通求值;ʃ(Shape)无节点、crisp/cohesive 上下文缺 |
| 18 | Temporal Types | ⚠️ | 惰性 Stream/Signal/Clock 可运行;`(next T)` 等时序模态类型语法与推断;□_t 语义保证缺 |
| 19 | Dependent Graded Types | ⚠️ | **Type 有 Pi/Sigma 变体**(§19.1 语法/显示/推断统一,`->` 注解生效);r+s 传播未实现 |
| 20 | Session Types | ⚠️ | defsession 协议解析为 SessionType(Send/Recv/Choice/Offer);类型级协议检查、MPST 语法集成缺 |
| 21 | Logic Programming | ⚠️ | defpred 子句/回溯/CLP label/constrain 传播/solve-all 多解/find-all 多解收集/abduce 假设列表可用;任意谓词 Prolog 式多解仍限第一解 |
| 22 | Generic Functions | ⚠️ | defgeneric/defmethod 模式分发+方法组合(:around/:before/:after/call-next-method)可用;编译期特化缺 |
| 23 | Typeclasses | ✅ | defclass 方法分发器(隐式字典按参数类型查 instance_dict)+ definstance 方法参数绑定 + 构造器→ADT 映射 |
| 24 | Macros | ⚠️ | defmacro 展开+syntax-quote/unquote/unquote-splice(符号→字符串)可用;hygiene/gensym 缺 |
| 25 | Module System | ✅ | ns (:require [lib])/(:require [lib :as a])/(:refer [f]) 解析 + 跨文件加载(基准目录+防循环) |
| 26 | FFI & System-Level | ⚠️ | defextern 注册外部函数(模拟 C 函数表 abs/strlen/sqrt);真实 dlopen 链接缺 |
| 27 | Process Calculi | ✅ | π 通道(FIFO)/Async/加密/spi commit-check/SKI 组合子/ambient/ρ/κ 全部接线 |
| 28 | Verification | ⚠️ | defprop 属性声明+verify 内置+ModelChecker 可达性;find-attack/dolev-yao 缺 |
| 29 | Built-in Functions | ⚠️ | 90+ 注册、集合/字符串/IO/构造器齐;反射函数硬编码 |
| 30 | Compiler Pragmas | ⚠️ | inline!/specialize!/opt-level/suppress-warning 语法接受;--ir 文本生成 |

---

## 2. 未实现特性清单

### ⬜ 仅设计(无有效实现)

| # | 特性 | spec 章节 | 说明 |
|---|------|-----------|------|
| 1 | defresource-algebra / Cost 代数 | §11.1 | `desugar_stub_defn` 占位,body 为 Unit |
| 2 | liquid 推断与 QTT 交互 | §15.2/15.4 | 未知谓词一律 assume true |
| 3 | fun-ext / 幺半等价 / HIT 边界 | §16.3-16.5 | PathLam 直通求值;defdata-hit 的 :boundary 被忽略 |
| 4 | Cohesive 语义层 | §17 | ʃ(Shape)无节点;♭/♯ 直通;无 crisp/cohesive 上下文 |
| 5 | 时序模态类型 / LTL-as-types / 多时钟 | §18.1/18.5/18.6 | ClockNew 返回字面量占位 |
| 6 | Π_r / Σ_r 类型语法 | §19.1/19.2 | Type 枚举无 Pi/Sigma 变体 |
| 7 | 依赖会话类型 / MPST 语法集成 | §20.2/20.3 | defsession 丢弃协议结构 |
| 8 | 类型类实例解析 | §23 | instance_dict 无消费者;:fun-deps/超类/kind 未解析 |
| 9 | 模块系统 | §25 | ns 的 requires/refers 被丢弃;NSDef 空操作;stdlib Namespace 未接入 |
| 10 | FFI 真实语义 | §26 | 无链接/调用;PtrRead/Write 透传;Unsafe 门控未强制 |
| 11 | 验证(属性/等价/攻击) | §28 | defprop/verify/check-equivalence/find-attack/dolev-yao 全缺 |
| 12 | 编译指示 | §30 | inline!/specialize!/opt-level/suppress-warning 无处理 |

### ⚠️ 部分实现(有骨架、语义残缺)

| # | 特性 | spec 章节 | 现状与缺口 |
|---|------|-----------|------------|
| 1 | 块注释 `#| |#`、字符串转义 | §3.2/3.4 | lexer 无块注释;`"\n"` 保留为反斜杠序列 |
| 2 | quote 数据语义 | §4.1 | `'(1 2 3)` 不产生列表数据(desugar.rs:1397 TODO) |
| 3 | `(list)`/`(vector)`/`(hash-map)`/`(hash-set)` 构造器 | §4 | 未注册 |
| 4 | 持久化 Vec/Map/Set | §4.2-4.4 | 仅 Data 包装,无 get/assoc/contains |
| 5 | `(ann expr Type)` | §5.9 | 特殊形式分发表无分支 |
| 6 | def 六维注解语法 `->[ε,ρ,@r,m,d]` | §6.6 | 仅支持 `: Ty -> Ret` |
| 7 | 私有定义语义(defn-) | §6.5 | 与公开同路径,无导出限制 |
| 8 | or-pattern | §8.2 | 只取第一个子模式 |
| 9 | guard 形式 | §8.2 | 实现为 `:when`,spec 为 `(when pat guard)` |
| 10 | refined 模式 `{x : Int | p}` | §8.2 | desugar_pattern 无分支 |
| 11 | HIT 边界语义 | §7.4 | 仅 is_hit 标志 |
| 12 | deriving 派生 | §7.5 | 仅收集名字,无 Eq/Ord 生成 |
| 13 | 记录字段访问器 | §7.2 | 无 `(:field obj)` |
| 14 | 类型族/关联类型 | §9 | type_infer 无实现 |
| 15 | 类型一等值 | §9 | Value 无 Type 值变体 |
| 16 | QTT 运行时擦除/移动 | §10.1 | 0 级参数不擦除、1 级无移动 |
| 17 | 效果续延 k | §12.2 | k 闭包 body 是 Lit(Unit) 占位,continuation 不真正续接 |
| 18 | 效果子类型/行消减 | §12.5 | effect_infer.rs:106 TODO |
| 19 | Monad 优化路径 | §12.6 | EffectCompiler 占位且 cli 未调用 |
| 20 | Mercury 多模式谓词 | §13 | 仅 :free/:ground;:det/:nondet 注解被跳过 |
| 21 | committed-choice 类别 | §14.3 | to_det 不产出 CcMulti/CcNonDet |
| 22 | Z3 桥接入 | §15 | Z3Bridge 存在但 cli 走 ModelChecker |
| 23 | 逻辑搜索多解/策略 | §21 | Search 节点只回第一解;DFS/BFS/find_all 未接入 |
| 24 | CLP constrain 传播 | §21.5 | 只挂恒真 propagator,域间约束未接通 |
| 25 | ALP 溯因 | §21.6 | abduce 返回占位字符串,不验证目标 |
| 26 | 泛型方法组合 | §22.3 | :around/:before/:after、call-next-method 全缺 |
| 27 | 泛型编译期特化 | §22.4 | middle 层不认识 GenericDef |
| 28 | 宏 syntax-quote/unquote | §24 | `~`/`~@` 只是递归 desugar;hygiene/gensym 无 |
| 29 | Async 通道/ambient/ρ/κ/spi-commit | §27 | 解释器节点透传或恒 true;SKI 字符串 stub |
| 30 | 演算互编码 | §27.10 | pi-to-ski 等转换未实现 |
| 31 | 内置函数补全 | §29 | append/slurp/spit 缺;反射函数硬编码返回 |

---

## 3. 状态说明

- 本清单随实现进度更新(见 [CHANGELOG.md](../CHANGELOG.md))
- 优先级建议:§12 续延 k、§21 多解搜索、§22 方法组合是「演算 > 代数效应」核心思想的直接载体,建议优先
- 类型系统相关(§13/14/19/20)与「强静态类型」主线直接相关,次优先
