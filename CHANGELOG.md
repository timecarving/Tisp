# 变更记录 (Changelog)

本文件记录 Tisp 的可见变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
状态符号与 [standard_doc/INDEX.md](./standard_doc/INDEX.md) 一致。

---

## [0.1.0] - 2026-08

### 新增

**12 逻辑范式全链路(logic-paradigms-full-chain)**
- 12 类 LP 范式全部接线真实求解器(tisp-runtime/paradigms.rs),替换 `pf-*` 简化投影:高阶 `higher-order-call`(谓词一等值 + `call`)、归纳 `ilp-induce`(`induce`)、概率 `plp-marginal`(`marginal` 精确枚举)、时序 `temporal-eventually`(`TemporalKb`)、描述逻辑 `subsume`(`Ontology::is_instance`)、可废止 `defeasible-settle`(`settle` 优先级裁决)、模糊 `fuzzy-eval`(`fuzzy_and` min)、表格化 `tabling`(`Tabler` 左递归终止)、一体化基底 `typed-pred`(`filter_by` 静态谓词过滤)、响应式 `reactive-eval`(`ReactiveRule` 信号派生)、情境 `context-query`(`ContextKb::query` 继承链)、模态 `modal-possible`(`ModalKb::possible` 可达世界)
- type_infer 补 12 范式单态签名(`subsume`/`tabling`/`plp-marginal`/`ilp-induce`/`fuzzy-eval`/`defeasible-settle`/`temporal-eventually`/`context-query`/`modal-possible`/`higher-order-call`/`typed-pred`/`reactive-eval`)
- effect_infer 效应门控:Search 注册 `ilp-induce`、Signal 注册 `reactive-eval`
- `list_to_vec` 支持源码 `[..]` Vec 字面量,12 范式源码端到端(源码 → typecheck → run)全链路可用
- 新增端到端测试 `test_logic_paradigms_numeric` + `test_logic_paradigms_full_chain_source`,总计 351 个单元测试

**最终残余收尾(complete-final-residuals)**
- §17 态射级自然性:一阶态射 `Morphism<A,B>` + counit/unit 自然变换方块(♭(f)∘η = η∘f)→ ✅
- §26 跨区域/全局别名:region_infer 数据结构嵌入(Data)/match 分支别名逃逸 → ✅
- §30 inkwell 闭包堆分配:`@closure_env` 全局槽 + 捕获计数(llvm feature 门控,默认回退文本 IR)→ ✅

**深语义收尾(finish-deep-semantics)**
- N(≥2)维立方:`hcomp-nd`(2^N 角一致性,泛化 `hcomp-2d`)+ §16 → ✅
- adjoint-triple 自然性:点级三角形恒等式 + 态射级自然性为剩余深度(§17)
- 空间回收:`Next`(⃝ 值两次 advance 后回收,无泄漏)+ §18 → ✅
- 完整别名分析:region_infer 闭包捕获/实参流入别名逃逸(§26)
- 效果操作门控修复:effect_infer 补注册 Unsafe/Search/Channel/Reader/Writer/State(ref/deref/set!),副作用操作不再漏判为 Pure

**统一内存管理(unified-memory-management)**
- 归档 4 个已完成变更,同步 7 个新能力到主规范(18 specs 通过)
- `Type::Ref(Box<Type>)` 变体 + Display + reduce_families + unify 接线
- `ref`/`deref`/`set!` 内置建模为 State 效应操作(区别于 Unsafe 的 ptr-read/write),带悬垂检测
- `ref : i64 -> Ref i64`、`deref : Ref i64 -> i64`、`set! : Ref i64 -> i64 -> Unit` 类型签名接线,`(ref 42)` 源码端到端 typecheck 通过

**全链路补齐(complete-full-chain)**
- 范式解释器接线:24 范式经 `ParadigmRegistry` 接入 interpreter,注册为 `pf-*` 内置(如 `(pf-array-sum [1 2 3])` → 6),`Value ↔ ParadigmValue` 转换,从源码 `--run` 端到端可用
- 范式类型接线:type_infer 补 24 个 `pf-*` 单态签名,`--typecheck` 端到端通过
- HoTT 完整立方填充:HComp 边界不一致报错(替换静默返回一端)+ hott.rs `kan_fill_2d` 2 维 Kan
- 真实自动机:`dfa-accept` 内置接线 `programming::Dfa`(替换 sum%2 占位),源码端到端识别
- 状态修正:六维注解/私有定义/deriving Ord/□_r◇_ε Modal unify/@Cost 渐近/r+s 依赖等级/□_t 稳定类型/统一约束求解/演算迹等价 核验已实现,status ⚠️→✅

**范式集成(paradigm-integration)**
- 可接入接口:`ParadigmFacility`(keyword/type_con/effects/eval)+ `ParadigmRegistry` 统一插接入口(tisp-runtime/src/facility.rs)
- 21+ 范式一等化:12 逻辑范式 + EVOLP/DLP/MOP + 8 编程范式 + AOP 注册为带类型/效应元数据的可接入设施
- 跨范式组合:经统一 `ParadigmValue` 抽象传递(如 stream-take → array-sum),副作用接入效应行(State/Search/Signal)

**全链路范式与 AOP(full-chain-paradigms-aop)**
- 8 类编程范式(组合优先,纯声明式副作用管理):数组(多维 + 归约)、栈(纯函数栈操作)、连接式(点自由组合)、符号(代换/化简)、自动机(DFA)、状态机(事件驱动)、数据驱动(分发表)、基于流(惰性数据流)(tisp-runtime/src/programming.rs)
- AOP:aspect/pointcut/advice 编织(before/after/around),基于编译器纯声明式 MOP,无运行时反射(tisp-runtime/src/aop.rs)
- 全链路补齐语义助手:□_r/◇_ε 分级模态推理、Cost 渐近代价(Big-O)、时序稳定类型、区域逃逸检查、HoTT 完整立方填充(tisp-runtime/src/full_chain.rs)

**Everything-as-ADT 逻辑编程扩展(evolp-dlp-mop)**
- Everything-as-ADT:`Rule`/`Program`/`EvolInstr` 一等不可变数据模型(tisp-core/src/evolp.rs),规则/程序可绑定/传递/匹配/增删查
- EVOLP:演化指令(assert/retract)+ `evolve` 纯函数 + `foldl` 折叠 + 稳定模型求解器(Gelfond-Lifschitz 约化 + 最小模型)+ `evolve_fixpoint` 不动点
- DLP:状态序列 + `dynamic_stable_models`(拒绝被后续状态否定的规则 + 约化 + 最小模型)
- MOP:`GetKB`/`SetKB` 效应操作 + `MetaInterpreter` handler 元解释器 + `compile_time_resolve` 编译期元编程
- State Effect:`Ref<T>` + `ref_`/`deref_`/`set_` 引用管理(线性写消费句柄)
- 12 类 LP 范式(组合优先):高阶/ILP/PLP/时序/描述/可废止/模糊/Tabled/一体化/响应式/情境/模态(tisp-runtime/src/paradigms.rs)

**草案方向落地(implement-draft-directions)**
- 语法 DSL:`::>` 声明式语法定义 + meta-tag(`<tag>`/`\x`/`[]`/`{}`/`[{}]`/`|`)+ 内置字符类 + 扫描器(grammar.rs)
- 类型 λ:`Type::TLambda` 变体 + `A => B`/`=> B` 类型字面量
- 多态类型:`(defpoly Name [params where 约束] body)` + 类型别名 + `(Pair i32 f32)` 类型实参替换
- 和/积类型字面量:`(conj A B)` → Tuple、`(disj A B)` → defdata ADT、`()` → Unit
- trait 语法糖:`deftrait`/`polytrait` → `defclass`、`defabsmember`/`defmember` → 抽象方法

**全链路缺口补齐(close-full-chain-gaps)**
- 持久化集合:Vector/Map/Set 改用 im HAMT(结构共享),`Value` 实现结构 Hash/Eq;`conj/assoc/contains?/dissoc/disj` 内置;quote 产生可操作数据
- TCO:解释器 `apply` 蹦床 + `eval_tail` 尾位置循环(`sum-to 100000` 1MB 栈不溢出);`--run` 改 256MB 栈线程(修深递归 debug 溢出)
- 五维子类型:`subtype.rs`(det/grade/mode/region 子类型格)+ 确定性子类型接入 determinism_analysis
- Cost 渐近代价:`grade_le_asymptotic`(Big-O 忽略常数因子)
- Cohesive:♭/♯ 从直通改为 Flat/Sharp 容器(可区分语义)
- 时序:稳定类型谓词 `is_stable_type` + always/eventually 类型方案
- 系统级:ptr-read/ptr-write/region-alloc/with-region 模拟内存 + Unsafe 门控警告
- 优化器:`opt-level`(内联阈值)+`inline!`(强制内联)+`noinline!` 接线;pragma 数值参数解析修复
- 反射:`reflect` 内置返回名称/参数/类型/效果/等级/模式/确定性全真实;密码学 XOR 占位输出警告
- 文本 IR:带参函数签名 + 用户函数 call 指令(替换 `ret i64 0` 占位)

**设计阶段 + 深度缺口补齐(finish-design-stage-features)**
- 类型类:defclass 支持 `[tvars]`/`:fun-deps`/`:super`;definstance 支持 `(Class T1 T2)` 形式 + fun-deps 冲突/超类检测
- 依赖会话:defsession 协议体可引用依赖类型(Vec/Pi);Cost 注解 `@Cost` 经 `@` 前缀
- HoTT:fun-ext/幺半等价内置(有限域枚举)、shape-graph 连通图、dolev-yao 知识合成、hott.rs Interval 接线、HIT 符号端点求解
- 时序:clock/always/eventually/resample + LTL-as-types(delay : a→(next a)、advance : (next a)→a)
- deriving 移 desugar:生成 eq-/ord-/show- 函数定义(`--desugar` 可见;未知 trait 报错)
- dlopen 字符串签名(CString);编译指示解析 + suppress-warning 过滤
- Prolog:逻辑变量统一 + 递归多解(空解过滤)——递归 `member` 现返回正确解集
- Monad:直接状态线程(direct_state 状态槽);演算:全演算迹等价

**部分实现补齐(finish-partial-features)**
- 六维注解:`->[ε,ρ,@r,m,d]` 解析 + `FunAnnotation` 贯通;`CoreDef` 增 region 字段
- 模块可见性:`defn-`/`def-` 私有 + `CoreDef.visibility` + ns `:refer` 过滤 + 跨文件私有不可见
- 类型族:单声明多模式 + `rewrite` 形式 + 未声明族报错;类型一等值(显示/模式匹配/六维反射真实化)
- QTT 隐式绑定默认 0(`{n : T}` → 擦除);资源代数 `:semiring` 关键字形式 + `check_cost_bound`
- Mercury:内联 `:in/:out` + `infer_modes` 接线 + 同名多模式合并
- 宏卫生:fn/lambda/if-let/when-let/match 绑定 + `~x` unquote 参与替换
- 泛型特化:构造器类型驱动 + 多参数 + 接入 `--run`;反射 `type-of/effects-of/grade-of/mode-of/determinism-of` 真实化
- Monad:`mlet/get-m/put-m/pure` 语法 + 真单处理器/无嵌套检测
- 逻辑:结构化值统一(Cons)+ 分支 trail 隔离;CLP 乘/除/模收 z + 精确除法 + 线性 `+`/`-`;ALP domain 感知 + `assign` 域相交
- HIT:结构化边界子句 + 端点唯一一致性 + spec `(i = i0)` 语法;deriving `ord-*`;演算 5 编码 + SKI K 负载修复

**类型系统**
- 词法与语法:
  - lexer 支持 `,`(分隔符)、`:::`(构造器名)、`⃝`(时态算子,§18.1)
  - 顶层表达式收集为隐式入口 `__top__` 并执行(§6.3)
  - 零参调用 `(f)` 生成 `App(f, Unit)`;`fresh` 多变量形式;`search` 零参形式;GADT 字段列表 `[T1, T2]`(§7.3);`Unit` 值上下文字面量
- 液态类型与契约(§15):
  - 精化类型 `{x : T | pred}` 解析进 `Type::Refined`(desugar);`:requires`/`:ensures` 契约解析进 CoreDef,多个 `:requires` 合取为 And(修复 keyword 匹配 bug——契约此前被混入函数体)
  - LiquidVerifier 接入 `--typecheck`:调用点实参精化、返回精化(if→ite 路径敏感)、契约 requires⇒ensures;Z3(SMT-LIB2 外部进程)求解,违反输出反例(如 `x = -1`),退出码非零
  - 无 z3 时优雅降级为常量折叠(`apt install z3` 提示);未知谓词/不可翻译表达式警告放行,不误报
  - Z3Bridge 扩展:`verify_implication` 蕴含验证、反例模型提取(修复多行与 `(- n)` 负数解析)、SMT 标识符收集;修复 get-model 死锁
  - 移除从未使用的 `z3` crate 依赖与 `z3` feature(保留外部进程桥);`--desugar` 输出契约字段
  - 新增 34 个单元测试(翻译器/桥/验证驱动),总计 139 个
- 类型系统深化(A 组,剩余缺口第一轮):
  - QTT:确认 0 级擦除/1 级移动语义并补 6 个测试(擦除:实参不求值、不绑定;移动:二次引用/未用报错)
  - §13 多模式谓词:`:mode (i o)` 签名解析进 CoreDef.mode_sigs;mode_analysis 调用点按实参 free/ground 匹配,无匹配报错
  - §9 类型族:`(typefamily 名称 模式 结果)` 声明、小写符号=类型变量、type_infer 应用归约(悬挂/模式不匹配报错);`--typecheck` 验证
  - §9 类型反射:`reflect-type` 特殊形式 + MetaQuery 真实化(返回定义签名/未定义提示)
  - §19.1 依赖等级检查机制:Pi/Sigma 绑定在结果中的使用计数(r+s 就位,当前绑定 ω 恒过)
  - §11.1 `defresource-algebra` 真实解析(替换 stub;名称/单位元/运算/阶,`--desugar` 输出)
  - §14.3 committed-choice:cc_multi/cc_nondet 谓词只试首子句 + Commit(cut)提交,find-all 只收 1 解
  - 新增 16 个单元测试(总计 155)
- 依赖线性类型系统(§10 推广):
  - 等级表达式语法:`(5 x : a)` → Nat(5)、`(n x : a)` → Var(n)、`((+ n 1) x : a)` → Add;0/1/ω 兼容
  - 等级检查:`grade_check` 使用计数 ≤ 等级(上界);grades.rs 半环补 Nat+One 折叠;分支合并取上界;等级变量须绑定自类型参数(小写 Con 按类型变量,与类型族一致),未绑定报错
  - 运行时:`grade-of` 查询参数等级列表;Nat 等级不擦除(Zero 擦除不变)
  - 示例 `examples/dependent-linear-test.tisp`;新增 6 个单元测试

**效果与宏**
- 效果系统(§12):
  - `handle`/`perform` 运行时:handler 作用域栈、按操作名分发、续延闭包 `k`(`(k result new_state)` 状态回写;`(k v)` 搜索续延)
  - 内置效果操作注册:`get`/`put`/`ask`/`tell`/`throw`/`choose`,无 handler 时明确报错
  - `state-effect.tisp` 示例跑通(输出 `3`)
- 宏系统(§24):
  - `defmacro` 注册与调用点展开:参数替换、递归 desugar、多表达式模板自动包 `do`
  - 宏与关键字/函数调用优先级正确(宏优先)
- OOP 泛型函数(§22/§23):
  - `defgeneric`/`defmethod` 去 stub:方法模式 `(name Type)` 绑定整个值,分发器运行时查 `generic_table` 按模式匹配
  - `defclass`/`definstance` 解析与实例登记(`instance_dict`)
  - 声明类节点(defgeneric/defmethod/defclass/definstance)在程序加载时立即求值
- 工具链与宏(C 组):
  - §24 宏卫生:模板 let 绑定重命名(hygienic substitution,防捕获调用点变量)+ gensym 内置(解释器原子计数)
  - §22.4 泛型编译期特化:middle specialize 模块,字面量调用匹配方法模式生成特化 def,`--typecheck` 报告特化数
  - §26 真实 dlopen FFI:`ffi` feature + libloading(i64→i64 / f64→f64 C ABI),defextern 支持库路径,符号缺失报错;默认构建回退模拟表
  - §29 反射增强:MetaQuery 返回签名含效果行与参数数
  - §12.6 Monad 优化接线:单处理器 handle 计数并标注直接状态传递路径(`--run` 输出)
  - 新增 6 个单元测试

**逻辑与验证**
- 逻辑编程(§21):
  - `defpred` 子句形式(三种模式列表写法)+ `:free`/`:ground` 模式注解 + `:det`/`:nondet` 注解
  - CLP:`domain`/`label` 真实求解并回绑变量;`Domain` 改用有序集合(label 升序枚举解)
  - `match_pattern` 同名变量一致性(模式中重复变量要求绑定值一致,§8)
  - `and_parallel` 分批处理全部目标(不静默截断)+ 结果合并回共享 store(`merge_from` 共享变量直接 unify、新变量重编号)
- 逻辑与验证(B 组):
  - §21.5 CLP 域间传播确认并补测试:两变量 (constrain (< x y)) 传播、冲突约束无解(域清空)
  - §21.6 ALP 溯因真实化:abduce 假设绑定后验证目标可满足性,只返回一致假设(替换占位)
  - §28 find-attack/check-equivalence 接线:ModelChecker 泛型 find_attack + 可达集比较;内置 find-attack(攻击者窃听场景)、check-equivalence(状态集等价)
  - §20.2 MPST:defsession `:role` 角色分段与投影;send/recv/close 会话特殊形式;type_infer 协议顺序检查(期望违反报错)
  - 新增 8 个单元测试

**进程与 FRP**
- 进程演算与通信(§27):
  - `chan`/`send`/`recv` 接线 `ProcessRuntime`:真实缓冲通道收发;`spawn` 子解释器共享通道运行时
  - 加密全链路:`secret!` 密钥声明、`encrypt`/`decrypt`/`sign`/`verify`/`hash` 接线 `CryptoEngine`(XOR/简单哈希占位,生产应换 AES/ChaCha/SHA-256)
- FRP(§18):
  - `stream`/`stream-take`/`advance` 接线 `temporal::Stream`(惰性流);`SignalNew/Map/Filter/Fold` 节点接线 `frp::Signal`(值管道语义)

**工具链**
- LLVM 工具链与真实 IR 生成(§30):
  - LLVM 工具链接线:`LLVM_SYS_170_PREFIX=/usr/lib/llvm-17`(零安装,Debian llvm-17 共享库);workspace 直接依赖 `llvm-sys 170` + `force-dynamic` feature 修复静态库缺失
  - `IrGenerator` 重构:启用 `llvm` feature 时用 inkwell 生成真实 LLVM IR(函数参数绑定、多参调用收集、比较结果 zext i64、if/else phi 汇合),非 llvm 回退文本生成器
  - 修复文本生成器三个缺陷:函数参数被忽略、调用最后参数丢失、icmp 操作数类型不匹配
  - `--ir` 输出经 `llc-17` 编译验证合法(fibonacci/advanced-test)
  - z3 feature 需安装 `libclang-17-dev`(bindgen 依赖 libclang C API)
- LLVM IR 生成(§30):
  - 函数头语法、`ret`→赋值转换(多行)、if-phi 寄存器一致性修复;`--ir` 输出合法文本 IR
- HoTT 与小件(D 组):
  - §7.4/16.3 HIT `:boundary`:解析与一致性检查(引用须为构造器/端点,运算符豁免);构造器解析跳过 boundary
  - §7.5 deriving 生成:`eq-Name`/`show-Name` 函数(结构递归相等/显示),`:deriving` 支持向量与圆括号
  - §27.10 演算互编码:π 通道操作 → SKI 组合子(ChannelOp 编码,K 提取常量)、ambient 能力 → 通道消息
  - §17 Cohesive 最小语义:ʃ(shape)返回 Shape 容器(路径端点,可区分);crisp 上下文检查(非 crisp 解包 ♭ 报错)
  - 新增 6 个单元测试

**文档**
- 04 实现状态重建:总表章节号修正(23 章起偏移 -1)、两张清单按当前实现重建(剔除已实现条目)
- spec 附录对齐:BNF 补块注释/`:::`/`⃝`/精化类型/类型族;保留字补 defsession/verify!/solve-all/find-all/gensym 等;示例索引重建(14 个实际示例)
- spec 30 章状态内联(✅/⚠️/⬜ 与 04 同符号),spec 成为唯一事实源
- 新增 `examples/remaining-gaps-demo.tisp` 综合示例(类型族/多模式/cc/反射/特化/宏卫生)

### 修复

**测试**
- 新增约 35 个单元测试,总计 105 个(效果处理器/通道/流/加密/CLP/泛型分发/IR/回溯/合并)

**后续补齐批次(同一 0.1.0)**
- §12.2 效果续延 k:clause 多 body 保留(此前只取第一个表达式)
- §21.5 CLP 真实传播(`constrain` 编译为 add_lt/add_eq,常数提升 singleton)+ `solve-all` 多解枚举;`search` 内置真实化(1 参 thunk 回溯边界)
- §21 find-all 多解收集(Search/Match 收集模式 + 逻辑变量快照,val_to_logic 识别变量 id)+ §21.6 abduce 返回假设列表
- §22.3 泛型方法组合:`:around/:before/:after/:primary` 修饰符 + `call-next-method`
- §13/§14 defpred 确定性注解(:det/:nondet/:cc_multi/:cc_nondet 等)写入 CoreDef
- §19.1 Π/Σ 依赖类型:`(pi (x : T) R)`/`(sigma ...)` 语法、Pi/Sigma 显示与 unify/替换支持、`->` 返回类型注解修复(Sym 形式)、旧类型名别名(Int→i64 等)
- §20.1 defsession 会话协议解析为 SessionType(Send/Recv/Choice/Offer/End)
- §29 内置补全:append/slurp/spit + §4 list/vector/hash-map/hash-set 构造器
- §8.2 or-pattern(Pattern::Or 全链路)、guard 双形式(`(when pat g)` + `pat :when g`,Match 求值检查 guard)、refined 模式 `{x : T | pred}`(parser Map 的 `:`/`|` 语法)
- §3 词法:块注释 `#| ... |#`、字符串转义解码(\n \t \r \0 \\ \")
- §5.9 `(ann expr Type)` 类型标注节点;§7.2 记录字段访问 `(:field obj)`(字段名表)与 `{name : T}` 记录字段语法
- §24.1 语法引号:quote/syntax-quote 函数形式 + 'x/`x,~x 求值插入、~@xs concat 拼接(符号→字符串);concat/append 可变参
- §27 进程演算:Async 通道(FIFO 修复)、ambient-new/enter/exit/open、ρ quote/drop/lift、κ bind/unbind/react、spi commit!/check!(加密摘要验证)、SKI 组合子(S/K/I 部分应用链)
- §25 模块系统:ns (:require [lib])/(:require [lib :as a])/(:refer [f]) 解析、跨文件加载(base_dir + 防循环)
- §26 FFI:defextern 注册外部函数(模拟 C 表 abs/strlen/sqrt);§30 编译指示 inline!/specialize!/opt-level 接受
- §28 验证:defprop 属性声明 + verify 内置((verify name)/(verify thunk));CryptoVerify 改 verify!
- §23 类型类:defclass 方法分发器(隐式字典按参数类型查 instance_dict)、definstance 方法参数绑定、构造器→ADT 映射
- §12.5 效果行消减(Handle 移除被处理效果);§12.6 monadic 优化检测接入 cli
- §18 时序模态类型:(next T)/(always T)/(eventually T) 语法与推断
- `range`/`zip`/`concat` 双重反转导致的倒序输出
- `desugar_cond` 无 `:else` 时最后一项重复求值;`desugar_let`/`desugar_lambda` 丢弃多余 body
- `some->` 的 nil 短路失效(`values_eq` 缺 `Unit == Unit`);列表形式步骤缺短路
- ADT 构造函数未注册(`Just`/`Nil` unbound);`send`/`recv` 被错误映射为 session 操作
- 多参数调用崩溃(应用链参数合并收集)
- `Search` 节点 choice point 泄漏(失败/成功均清理)
- `and_parallel` 静默截断与忽略共享 store
- `nth` 仅支持 Cons;`abs`/`pow`/`str-sub` 边界 panic;`(f)` 零参函数不执行
- 编译警告清零(0 warnings);删除 8 个纯声明空壳文件(optimize/ 下 5 个 + closure/runtime_ffi/builtin)
- `target/` 构建产物移出 git 索引,新增 `.gitignore`
- 重写 `standard_doc/`(INDEX + 01 语言核心 + 02 高级特性 + 03 参考),全部示例实测验证
- 新增 `standard_doc/04-implementation-status.md`:spec 30 章逐章实现状态与未实现特性清单(12 项仅设计 + 31 项部分实现,含 file:line 证据)
- 显式化核心设计思想:演算 > 代数效应(演算是抽象核心,effect handler 是编码/验证载体)、强静态类型(编译期检查/推断/多态/无运行时类型错误)——standard_doc 新增「核心设计思想」章节,spec.md 补充 Principle 7 扩展与 Principle 8
- 本变更记录
- `logic-search.tisp`:Mercury 自由变量 + Search `choose` 续延搜索需完整 Prolog 式回溯引擎(部分支持)
- `type_infer` 的 `Perform` 节点类型规则仍返回 fresh var(需 effect row 上下文)
- CLP `constrain` 对 CLP 变量的算术约束编译未实现(仅接受已求值约束)
- 加密算法为 XOR/简单哈希占位;`--compile`/LLVM 真编译需 llvm 工具链
- 宏 hygiene/syntax-quote、类型类实例查找、HIT 完整语义未实现

**递归与闭包类型检查修复**
- 两遍推断:type_infer 先行收集全部 def 占位类型——前向引用与相互递归通过类型检查(此前报 unbound variable)
- let 内递归:`(let [f (fn ... (f ...))] ...)` 先绑定占位再推断值(此前报 unbound variable: f)
- 零参 lambda 类型修正:`(fn [] body)` 推断为 `Unit -> body`(此前误判为 body 本身,导致递归返回闭包类程序类型错误或误放行)
- 有限类型递归返回闭包确认可用;无限类型(T = Unit -> T)被拒绝(occurs check 正确行为)
- 新增 6 个单元测试(前向引用/相互递归/let 递归/有限递归闭包/无限拒绝/真实错误拒绝)

**递归与闭包类型检查修复(续)**
- 复现修正:原「递归返回闭包」用例经诊断为无限类型(T = Unit -> T),HM 拒绝为正确行为;有限类型递归返回闭包确认可用
- 零参 lambda 类型修正:`(fn [] body)` → `Unit -> body`(此前误判为 body 本身,导致递归返回闭包类程序类型错误或误放行)
- 已知局限(预存在,非本变更引入):多顶层表达式 + 递归的 `--run` 卡死;深递归栈溢出(记录于 04 文档)

**部分实现特性全链路化**
- §10 符号等级诊断:自由等级变量记录诊断警告(不误报);实施修正:Z3 严格验证对自由变量不成立(任何 count 有 n=0 反例),spec 已同步
- §16.3 HIT 端点方程可满足性检查(端点常量 i0/i1 等式判定);HComp(KanFill 边界值)/Transp(目标端点传输)真实求值(替换直通)
- §17 ʃ 路径连通计算(端点相等性判定,替换最小容器)
- §21.5 CLP 算术约束编译:乘法/除法/模传播器 + all-different 全局约束;修复 add_eq/add_mul 空域冲突处理与 label 传播状态保留
- §21.6 abduce 多解枚举(全部一致解释)+ 不可满足原因(no-consistent-explanation);修复假设绑定(assign 替代 add_eq 的 id 冲突)
- §9 类型一等值:Value::Type 变体,reflect-type 返回类型值(绑定/传递/比较)
- §9 类型族多模式实例归约(遍历同名实例);修复 collect_type_vars 把小写内置类型名(i64 等)误当类型变量
- §13 多模式自动推断(未声明 :mode 谓词按调用形态收集);修复 walk_calls 重复收集
- 修复既有缺陷:reflect-type 测试适配、desugar_hott_unary 未用方法清理
- 新增 9 个单元测试

### 测试

- 新增约 35 个单元测试,总计 105 个(效果处理器/通道/流/加密/CLP/泛型分发/IR/回溯/合并)


## [0.0.x] - 2026-07(开发阶段摘要)

- **Phase 1-2**:workspace 骨架、core AST、词法/解析/脱糖基础
- **Phase 3**:类型推断(多态、ADT 构造器类型)
- **Phase 4-5**:效果推断、洞(hole)、合约、确定性分析
- **Phase 6-7**:等级检查(QTT)、模式分析(Mercury 风格)
- **Phase 8-9**:区域推断与运行时、解释器
- **Phase 10**:HoTT/会话等高级节点贯通、优化器 pass 骨架
- **Phase 11-13**:运行时模块(逻辑/约束/FRP/进程/定理)、示例、文档初稿

> 各阶段细节见仓库根目录 `PHASE*_SUMMARY.md` 与 `PLAN.md`(历史记录,部分内容已过时,以 standard_doc 为准)。
