# 04 — Spec 实现状态与未实现特性清单

> 对照 `docs/spec.md`(32 章 + 6 附录)与 `crates/` 实际实现的逐章审计结果(0.1.0,2026-08)。
> 符号:✅ 完全实现 | ⚠️ 部分实现 | ⬜ 仅设计。证据为 `file:line`。

---

## 1. 逐章状态总表

| 章 | 特性 | 状态 | 摘要 |
|----|------|------|------|
| 1 | Introduction | ✅ | Lisp-1/无 GC 区域栈已落地;LLVM 经 inkwell 生成真实 IR(llvm feature,llc 验证) |
| 2 | Design Philosophy | ✅ | 统一 def+六维注解、语法糖脱糖最实;统一六维约束求解(constraint.rs 共享约束图 + solve.rs fixpoint 迭代聚合六 pass 冲突 + 去重 + 跨维度报告)已接线;演算统一(check_trace_equivalence/check_observational_equivalence 跨 π/ρ/ambient/SKI 迹等价)已实现;维度间完整 fixpoint 反馈(跨维度约束传播)为后续增强 |
| 3 | Lexical Structure | ✅ | 标识符/字面量/行注释/块注释 `#| |#`/字符串转义齐 |
| 4 | Data Structures | ✅ | List/Vector/Map/Set 持久化(im HAMT 结构共享)+ quote 产生可操作数据;conj/assoc/contains?/dissoc/disj 齐 |
| 5 | Expressions | ✅ | 9 项中 9 项实现(含 `ann` §5.9) |
| 6 | Definitions | ✅ | def/defn/defpred/defmacro/defgeneric 等全有;私有定义(defn-/def-)写 visibility + ns 引用过滤;六维注解 `->[ε,ρ,@r,m,d]` 解析并贯通 FunAnnotation |
| 7 | ADT | ✅ | defdata/记录/字段访问/GADT 可用;deriving (Eq/Ord/Show) 生成 eq-/ord-/show- 函数(结构递归) |
| 8 | Pattern Matching | ✅ | match/cons/通配符/穷尽性检查/or-pattern/guard 双形式/refined 模式齐 |
| 9 | Type System Overview | ✅ | HM 推断+泛化+rank-n+行多态;类型族多模式/类型一等值(Value::Type)/五维子类型/subtype.rs;tlambda(类型 λ)+ defpoly/where + conj/disj 字面量 + trait 语法糖(deftrait/polytrait/with) |
| 10 | QTT | ✅ | Grade 0/1/ω + 依赖等级(数字/符号/复合表达式,上界检查,等级变量绑定);运行时 0 级擦除 + 1 级移动检查;符号等级不等式经 Z3 验证(verify_grade_inequalities,无 z3 降级) |
| 11 | Graded Modal Types | ⚠️ | `defresource-algebra` 真实解析(名称/单位元/运算/阶 + `:asymptotic`,`--desugar` 可见);□_r/◇_ε Modal 类型在 type_infer reduce_families/unify 已接线;引入/消去等级推导 `resolve_modal_grade`(等级变量默认 ω + 递归解析)已接线;完整可推断情形推导(按使用次数推导 r/ε)缺 |
| 12 | Effect System | ✅ | defeffect/handle/perform/续延 k 语义(多 body+状态写回)齐;§12.5 行消减;§12.6 单处理器 handle 走直接状态传递(计数输出) |
| 13 | Mode System | ✅ | `:mode` 签名解析 + 调用点匹配 + 未声明谓词按调用形态自动推断模式 |
| 14 | Determinism | ✅ | 8 类范畴+注解写入 CoreDef;cc_multi/cc_nondet 运行时提交(首子句+cut) |
| 15 | Liquid Types | ✅ | 精化类型 `{x : T | pred}` 解析、`:requires`/`:ensures` 契约、Z3 求解验证(调用点/返回精化/契约,违反带反例)、无 z3 降级常量折叠;未知谓词警告放行(证据:liquid_verify.rs / z3_bridge.rs / desugar.rs) |
| 16 | HoTT | ⚠️ | Interval/Path/Glue 齐;HComp(KanFill 边界)/Transp(端点传输)真实求值;`:boundary` 端点方程可满足性检查;完整立方填充:HComp 边界不一致报错 + 2 维 Kan `hcomp-2d`(四边四角一致性)+ hott.rs `kan_fill_2d`;N(≥3)维立方组合接线缺 |
| 17 | Cohesive HoTT | ⚠️ | ♭(Flat 容器)/♯(Sharp 容器)可区分语义;ʃ(shape)路径连通 + shape-graph 连通图;crisp 上下文检查;adjoint-triple(ʃ ⊣ ♭ ⊣ ♯)全语义:counit(♭∘♯ = id、ʃ∘♭ = id)+ unit(♯∘♭、♭∘ʃ 单元嵌入)已接线;自然性(完整范畴同伦模型)缺 |
| 18 | Temporal Types | ⚠️ | 惰性 Stream/Signal/Clock 可运行;`(next T)` 等时序模态类型语法与推断;□_t 稳定类型 `is_stable_type` + 生产率检查 `check_productivity` 已接线(§18.3/18.4);因果性经 LTL-as-types(delay/advance 时序方向)+ 生产率 + 稳定类型覆盖;空间回收(⃝ 值两时刻后回收)缺 |
| 19 | Dependent Graded Types | ⚠️ | **Type 有 Pi/Sigma 变体**(§19.1 语法/显示/推断统一,`->` 注解生效);依赖等级 r+s 传播已实现(grade_check.rs 类型级使用计数 + 有限等级求解),符号等级不可判定时警告放行 |
| 20 | Session Types | ✅ | defsession 协议解析为 SessionType;MPST `:role` 分段投影;类型级协议顺序检查(期望违反报错) |
| 21 | Logic Programming | ✅ | defpred/回溯/CLP label/constrain 线性+算术(乘除模)/all-different 传播/solve-all/find-all/abduce 多解枚举+不可满足原因;任意谓词 Prolog 式全多解(结构化值统一+分支隔离+空解过滤) |
| 22 | Generic Functions | ✅ | defgeneric/defmethod 模式分发+方法组合(:around/:before/:after/call-next-method);构造器类型驱动编译期特化(monomorphize)+ 多参数特化接入 --run |
| 23 | Typeclasses | ✅ | defclass 方法分发器(隐式字典按参数类型查 instance_dict)+ definstance 方法参数绑定 + 构造器→ADT 映射 |
| 24 | Macros | ✅ | defmacro 展开+syntax-quote/unquote/unquote-splice;fn/lambda/if-let/when-let/match 卫生重命名 + gensym + `~x` unquote 替换 |
| 25 | Module System | ✅ | ns (:require [lib])/(:require [lib :as a])/(:refer [f]) 解析 + 跨文件加载(基准目录+防循环) |
| 26 | FFI & System-Level | ⚠️ | defextern 经 `ffi` feature 真实 dlopen(字符串/指针/i64/f64);ptr-read/ptr-write/region-alloc/with-region 模拟内存 + Unsafe 门控警告;悬垂指针运行时检测(region-free/with-region 退出后 PtrRead 报「悬垂指针」错误);编译期区域逃逸检查(region_infer:返回值逃逸 + let 绑定地址数据流逃逸);完整别名分析缺 |
| 27 | Process Calculi | ✅ | π 通道(FIFO)/Async/加密/spi commit-check/SKI 组合子/ambient/ρ/κ 全部接线 |
| 28 | Verification | ✅ | defprop/verify/ModelChecker 可达性 + find-attack + check-equivalence + dolev-yao 攻击者知识合成(窃听/拼接/重放) |
| 29 | Built-in Functions | ✅ | 90+ 注册、集合/字符串/IO/构造器齐;反射(type-of/effects-of/grade-of/mode-of/determinism-of/reflect)返回真实静态信息(名称/参数/类型/效果/等级/模式/确定性) |
| 30 | Compiler Pragmas | ⚠️ | inline!/specialize!/opt-level/suppress-warning 解析;opt-level 调内联阈值 + inline! 强制内联已接线;--ir 文本回退带参签名+call;闭包代码生成(嵌套 lambda 参数绑定 + 捕获自由变量标注 `; closure captures`);inkwell 闭包环境打包(堆分配 display 层)feature 门控(llvm) |
| 31 | Everything-as-ADT 逻辑编程扩展 | ✅ | Rule/Program/EvolInstr 一等 ADT(tisp-core/evolp.rs);EVOLP 稳定模型+foldl+不动点、DLP 动态稳定模型(tisp-runtime/evolp.rs);GetKB/SetKB+handler 元解释器+State 引用(tisp-runtime/mop.rs);12 类 LP 范式(tisp-runtime/paradigms.rs);interpreter 已接线全部核心语义:表格化 `tabling`、描述逻辑 `subsume`、稳定模型 `evolp-stable`、动态稳定模型 `dlp-stable`、演化指令 `evolp-evolve`、KB 读写 `get-kb`/`set-kb` |
| 32 | Programming Paradigms & AOP | ✅ | 8 类范式(数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流,tisp-runtime/programming.rs);AOP 编织(tisp-runtime/aop.rs);□_r/◇_ε 推理、Cost 渐近、稳定类型、区域逃逸、完整立方填充语义助手(tisp-runtime/full_chain.rs);可接入接口 `ParadigmFacility`+`ParadigmRegistry`(tisp-runtime/facility.rs);interpreter 经 `pf-*` 内置接入 + type_infer 补 24 个 `pf-*` 单态签名;真实求解器已接线:自动机 `dfa-accept`、状态机 `sm-drive`、描述逻辑 `subsume`、表格化 `tabling`、符号 `sym-eval`、稳定模型 `evolp-stable`、动态稳定模型 `dlp-stable`、演化 `evolp-evolve`、KB 读写 `get-kb`/`set-kb` |

---

## 2. 未实现特性清单

### ⬜ 仅设计(无有效实现)

> 2026-08 补齐轮(`finish-design-stage-features`):10 项全部升级 ✅/⚠️;已无 ⬜ 项。仍 ⚠️ 的为语义深度缺口。

| # | 特性 | spec 章节 | 现状与缺口 |
|---|------|-----------|------------|
| 1 | fun-ext / 幺半等价 / HIT 端点方程 | §16.3-16.5 | ✅ fun-ext/幺半等价内置 + HIT 符号端点求解(i 只可钉 i0/i1) |
| 2 | 时序模态完整语义 / LTL-as-types / 多时钟 | §18.1/18.5/18.6 | ✅ clock/always/eventually/resample + LTL-as-types(delay/advance 时序类型) |
| 3 | Cohesive 完整同伦语义 | §17 | ✅ shape-graph 连通图;⚠️ 完整模态层/同伦语义缺 |
| 4 | 类型族简化规则(rewrite) | §9 | ✅ 多模式 + rewrite 形式(上一轮 finish-partial-features) |
| 5 | 类型一等值(Value::Type) | §9 | ✅ 显示/匹配/六维反射(上一轮 finish-partial-features) |
| 6 | 依赖会话类型 / MPST 类型级协议检查 | §20.2/20.3 | ✅ 依赖负载类型可解析;顺序检查保持 |
| 7 | 类型类完整实例解析 | §23 | ✅ `:fun-deps`/超类/kind 解析 + 冲突/超类检测 |
| 8 | 真实 dlopen FFI 全签名 | §26 | ✅ 字符串(CString)+ 指针(i64 透传)+ i64/f64 |
| 9 | 验证 find-attack/dolev-yao 完整 | §28 | ✅ dolev-yao 知识合成(窃听/拼接/重放) |
| 10 | 编译指示全处理 | §30 | ⚠️ 解析 + suppress-warning 过滤;opt-level/inline! 优化器接线为最小 |

### ⚠️ 部分实现(有骨架、语义残缺)

> 2026-08 补齐轮(`finish-partial-features`):17 条大部分已升 ✅;仍 ⚠️ 的为语义深度缺口。

| # | 特性 | spec 章节 | 现状与缺口 |
|---|------|-----------|------------|
| 1 | def 六维注解语法 `->[ε,ρ,@r,m,d]` | §6.6 | ✅ 解析 + `FunAnnotation` 贯通;region 字段落 CoreDef |
| 2 | 私有定义语义(defn-) | §6.5 | ✅ visibility 字段 + ns `:refer` 过滤 + 跨文件私有不可见 |
| 3 | HIT 完整边界语义 | §7.4 | ✅ 结构化子句 + 端点唯一性 + 符号端点求解 + 端点值构造 + hott.rs Interval 接线 |
| 4 | deriving Ord | §7.5 | ✅ `ord-*` 结构化排序(Eq/Show/Ord 齐) |
| 5 | 类型族多模式/rewrite | §9 | ✅ 单声明多模式 + `rewrite` 形式 + 未声明族报错 |
| 6 | 类型一等值 Value::Type | §9 | ✅ 类型值显示/匹配 + 六维反射(effects/grade/mode/determinism 真实化) |
| 7 | QTT 隐式绑定默认 0 | §10.2 | ✅ `{n : T}` → `Grade::Zero` + 擦除检查 |
| 8 | 资源代数 Cost 检查 | §11.1 | ⚠️ `:semiring` 关键字形式 + `check_cost_bound`;Cost 注解语法/全推导缺 |
| 9 | Mercury 多模式推断 | §13 | ✅ 内联 `:in/:out` + `infer_modes` 接线 + 同名多模式合并 |
| 10 | 宏 fn 参数卫生 | §24 | ✅ fn/lambda/if-let/when-let/match 卫生 + `~x` unquote 替换 |
| 11 | 泛型完整特化 | §22.4 | ✅ 构造器类型驱动 + 多参数 + 接入 `--run` |
| 12 | 反射函数补全 | §29 | ✅ `type-of` 静态类型 + `effects/grade/mode/determinism-of` 查 def_sigs |
| 13 | Monad 优化完整编译 | §12.6 | ✅ `mlet/get-m/put-m/pure` + 真检测 + 直接状态线程(direct_state 槽) |
| 14 | 任意谓词 Prolog 式多解 | §21 | ✅ 结构化值统一(Cons)+ 逻辑变量统一 + 分支隔离 + 递归多解(空解过滤) |
| 15 | CLP 非线性约束 | §21.5 | ✅ 乘/除/模收 z + 精确除法 + 线性 `+`/`-` 传播 |
| 16 | ALP 多解解释 | §21.6 | ✅ domain 感知生成 + `assign` 域相交 + 多解枚举 |
| 17 | 演算互编码完整 | §27.10 | ✅ 5 编码齐 + SKI K 负载修复 + 全演算迹等价(channel_trace/check_trace_equivalence) |

## 3. 已知运行时局限

- **深递归栈溢出(已修)**:TCO 尾调用消除已接线(`sum-to 100000` 不溢出);多顶层表达式 + 递归卡死已修(Do 包装 `__top__` 修复)

## 4. 状态说明

- 本清单随实现进度更新(见 [CHANGELOG.md](../CHANGELOG.md))
- 优先级建议:§12 续延 k、§21 多解搜索、§22 方法组合是「演算 > 代数效应」核心思想的直接载体,建议优先
- 类型系统相关(§13/14/19/20)与「强静态类型」主线直接相关,次优先
