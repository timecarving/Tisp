# 04 — Spec 实现状态与未实现特性清单

> 对照 `docs/spec.md`(30 章 + 6 附录)与 `crates/` 实际实现的逐章审计结果(0.1.0,2026-08)。
> 符号:✅ 完全实现 | ⚠️ 部分实现 | ⬜ 仅设计。证据为 `file:line`。

---

## 1. 逐章状态总表

| 章 | 特性 | 状态 | 摘要 |
|----|------|------|------|
| 1 | Introduction | ✅ | Lisp-1/无 GC 区域栈已落地;LLVM 经 inkwell 生成真实 IR(llvm feature,llc 验证) |
| 2 | Design Philosophy | ⚠️ | 统一 def+六维注解、语法糖脱糖最实;统一约束求解、演算统一未达成 |
| 3 | Lexical Structure | ⚠️ | 标识符/字面量/行注释/块注释 `#| |#`/字符串转义齐; |
| 4 | Data Structures | ⚠️ | List/Unit/构造器(list/vector/hash-map/hash-set)可用;Vec/Map/Set 无持久化语义;quote 不产生数据 |
| 5 | Expressions | ⚠️ | 9 项中 9 项实现(含 `ann` §5.9); |
| 6 | Definitions | ⚠️ | def/defn/defpred/defmacro/defgeneric 等全有;私有语义、六维注解语法未解析 |
| 7 | ADT | ⚠️ | defdata/记录/字段访问/GADT 可用;deriving (Eq/Show) 生成 eq-/show- 函数(结构递归);Ord 生成缺 |
| 8 | Pattern Matching | ⚠️ | match/cons/通配符/穷尽性检查/or-pattern/guard 双形式/refined 模式齐; |
| 9 | Type System Overview | ⚠️ | HM 推断+泛化+rank-n+行多态有;前向引用/相互递归/let 递归/零参 lambda(Unit -> T)类型正确;类型族(§9 声明/归约/悬挂报错)、类型反射(reflect-type)已实现;Value::Type 一等值、五维子类型缺 |
| 10 | QTT | ✅ | Grade 0/1/ω + 依赖等级(数字/符号/复合表达式,上界检查,等级变量绑定);运行时 0 级擦除 + 1 级移动检查;符号等级 Z3 验证未做 |
| 11 | Graded Modal Types | ⚠️ | `defresource-algebra` 真实解析(名称/单位元/运算/阶,`--desugar` 可见);□_r/◇_ε 无推理 |
| 12 | Effect System | ✅ | defeffect/handle/perform/续延 k 语义(多 body+状态写回)齐;§12.5 行消减;§12.6 单处理器 handle 走直接状态传递(计数输出) |
| 13 | Mode System | ⚠️ | `:mode (i o)` 多模式签名解析 + 调用点按实参 free/ground 匹配(无匹配报错);多模式自动推断不支持 |
| 14 | Determinism | ✅ | 8 类范畴+注解写入 CoreDef;cc_multi/cc_nondet 运行时提交(首子句+cut) |
| 15 | Liquid Types | ✅ | 精化类型 `{x : T | pred}` 解析、`:requires`/`:ensures` 契约、Z3 求解验证(调用点/返回精化/契约,违反带反例)、无 z3 降级常量折叠;未知谓词警告放行(证据:liquid_verify.rs / z3_bridge.rs / desugar.rs) |
| 16 | HoTT | ⚠️ | Interval/Path/Glue 齐;HComp/Transp 平凡求值;defdata-hit `:boundary` 解析与一致性检查(未知符号报错) |
| 17 | Cohesive HoTT | ⚠️ | ♭/♯ 直通;ʃ(shape)返回 Shape 容器(路径端点,最小可区分语义);crisp 上下文检查(非 crisp 解包 ♭ 报错) |
| 18 | Temporal Types | ⚠️ | 惰性 Stream/Signal/Clock 可运行;`(next T)` 等时序模态类型语法与推断;□_t 语义保证缺 |
| 19 | Dependent Graded Types | ⚠️ | **Type 有 Pi/Sigma 变体**(§19.1 语法/显示/推断统一,`->` 注解生效);依赖等级检查机制就位(r+s 对 ω 绑定恒过) |
| 20 | Session Types | ⚠️ | defsession 协议解析为 SessionType;MPST `:role` 分段投影;类型级协议顺序检查(期望违反报错) |
| 21 | Logic Programming | ⚠️ | defpred 子句/回溯/CLP label/constrain 域间传播/solve-all/find-all/abduce 一致性验证可用;任意谓词 Prolog 式多解仍限第一解 |
| 22 | Generic Functions | ⚠️ | defgeneric/defmethod 模式分发+方法组合(:around/:before/:after/call-next-method)可用;字面量调用编译期特化(monomorphize) |
| 23 | Typeclasses | ✅ | defclass 方法分发器(隐式字典按参数类型查 instance_dict)+ definstance 方法参数绑定 + 构造器→ADT 映射 |
| 24 | Macros | ⚠️ | defmacro 展开+syntax-quote/unquote/unquote-splice 可用;模板 let 绑定卫生重命名 + gensym 内置;hygiene 未覆盖 fn 参数 |
| 25 | Module System | ✅ | ns (:require [lib])/(:require [lib :as a])/(:refer [f]) 解析 + 跨文件加载(基准目录+防循环) |
| 26 | FFI & System-Level | ⚠️ | defextern 经 `ffi` feature 真实 dlopen(libloading,i64/f64 C ABI,符号缺失报错);默认构建回退模拟 C 函数表 |
| 27 | Process Calculi | ✅ | π 通道(FIFO)/Async/加密/spi commit-check/SKI 组合子/ambient/ρ/κ 全部接线 |
| 28 | Verification | ⚠️ | defprop/verify/ModelChecker 可达性 + find-attack(攻击场景搜索)+ check-equivalence(可达集比较);dolev-yao 缺 |
| 29 | Built-in Functions | ⚠️ | 90+ 注册、集合/字符串/IO/构造器齐;反射(reflect-type)真实化(签名/效果);其余反射函数近似 |
| 30 | Compiler Pragmas | ⚠️ | inline!/specialize!/opt-level/suppress-warning 语法接受;--ir 经 inkwell 生成真实 IR(llvm feature),非 llvm 回退文本 |

---

## 2. 未实现特性清单

### ⬜ 仅设计(无有效实现)

| # | 特性 | spec 章节 | 说明 |
|---|------|-----------|------|
| 1 | fun-ext / 幺半等价 / HIT 端点方程 | §16.3-16.5 | PathLam 直通求值;:boundary 已解析检查,端点方程求解缺 |
| 2 | 时序模态类型完整语义 / LTL-as-types / 多时钟 | §18.1/18.5/18.6 | ClockNew 返回字面量占位;`(next T)` 语法与推断有 |
| 3 | Cohesive 完整同伦语义 | §17 | ʃ 最小语义(Shape 容器)与 crisp 检查已实现;ʃ 形状代数(路径连通计算)缺 |
| 4 | 类型族简化规则(rewrite) | §9 | 单模式匹配归约已实现;多模式与 rewrite 规则缺 |
| 5 | 类型一等值(Value::Type) | §9 | reflect-type 反射已实现;运行时类型值变体缺 |
| 6 | 依赖会话类型 / MPST 类型级协议检查 | §20.2/20.3 | :role 投影与顺序检查已实现;依赖会话(值依赖类型)缺 |
| 7 | 类型类完整实例解析 | §23 | 分发器有消费者;:fun-deps/超类/kind 未解析 |
| 8 | 真实 dlopen FFI 全签名 | §26 | ffi feature 下 i64/f64 C ABI 已实现;指针/字符串/可变参签名缺 |
| 9 | 验证 find-attack/dolev-yao 完整 | §28 | find-attack 场景与 check-equivalence 已实现;dolev-yao 攻击者模型与协议等价证明缺 |
| 10 | 编译指示全处理 | §30 | 语法接受;--ir 真实 IR(llvm feature)已有;opt-level 等无处理 |

### ⚠️ 部分实现(有骨架、语义残缺)

| # | 特性 | spec 章节 | 现状与缺口 |
|---|------|-----------|------------|
| 1 | def 六维注解语法 `->[ε,ρ,@r,m,d]` | §6.6 | 仅支持 `: Ty -> Ret` |
| 2 | 私有定义语义(defn-) | §6.5 | 与公开同路径,无导出限制 |
| 3 | HIT 完整边界语义 | §7.4 | :boundary 解析+符号一致性检查已实现;端点方程求解缺 |
| 4 | deriving Ord | §7.5 | Eq/Show 生成已实现;Ord 生成缺 |
| 5 | 类型族多模式/rewrite | §9 | 单模式归约已实现;rewrite 规则缺 |
| 6 | 类型一等值 Value::Type | §9 | reflect-type 反射已实现;运行时类型值缺 |
| 7 | QTT 隐式绑定默认 0 | §10.2 | 擦除/移动已实现;隐式绑定默认等级未做 |
| 8 | 资源代数 Cost 检查 | §11.1 | 声明解析已实现;Cost 类型检查与推导缺 |
| 9 | Mercury 多模式推断 | §13 | `:mode` 签名匹配已实现;自动推断缺 |
| 10 | 宏 fn 参数卫生 | §24 | let 绑定卫生+gensym 已实现;fn 参数卫生缺 |
| 11 | 泛型完整特化 | §22.4 | 字面量调用特化已实现;类型驱动特化缺 |
| 12 | 反射函数补全 | §29 | reflect-type 真实化;其余反射近似 |
| 13 | Monad 优化完整编译 | §12.6 | 单处理器检测+状态传递路径已实现;monadic 编码降级编译缺 |
| 14 | 任意谓词 Prolog 式多解 | §21 | 子句多解/find-all 已实现;任意谓词全多解仍限第一解 |
| 15 | CLP 非线性约束 | §21.5 | 线性域间传播已实现;非线性/全局约束缺 |
| 16 | ALP 多解解释 | §21.6 | 一致性验证已实现;多解解释枚举缺 |
| 17 | 演算互编码完整 | §27.10 | π→SKI、ambient→消息已实现;观察等价验证缺 |

## 3. 已知运行时局限

- **多顶层表达式 + 递归卡死**:含多个顶层表达式(如两个 `(println ...)`)的程序中,若顶层表达式调用递归函数(如 fib),`--run` 可能栈溢出;单顶层表达式正常。根因待查(Do 包装的 `__top__` 执行路径),非本次类型检查修复范围
- **深递归栈溢出**:`(sum-to 100)` 等深度递归在默认栈下溢出(解释器自递归求值,无 TCO)

## 4. 状态说明

- 本清单随实现进度更新(见 [CHANGELOG.md](../CHANGELOG.md))
- 优先级建议:§12 续延 k、§21 多解搜索、§22 方法组合是「演算 > 代数效应」核心思想的直接载体,建议优先
- 类型系统相关(§13/14/19/20)与「强静态类型」主线直接相关,次优先
