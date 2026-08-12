# 变更记录 (Changelog)

本文件记录 Tisp 的可见变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
状态符号与 [standard_doc/INDEX.md](./standard_doc/INDEX.md) 一致。

---

## [0.1.0] - 2026-08

### 新增

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
