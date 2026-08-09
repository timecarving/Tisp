# 变更记录 (Changelog)

本文件记录 Tisp 的可见变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
状态符号与 [standard_doc/INDEX.md](./standard_doc/INDEX.md) 一致。

---

## [0.1.0] - 2026-08

### 新增

**效果系统(§12)**
- `handle`/`perform` 运行时:handler 作用域栈、按操作名分发、续延闭包 `k`(`(k result new_state)` 状态回写;`(k v)` 搜索续延)
- 内置效果操作注册:`get`/`put`/`ask`/`tell`/`throw`/`choose`,无 handler 时明确报错
- `state-effect.tisp` 示例跑通(输出 `3`)

**宏系统(§24)**
- `defmacro` 注册与调用点展开:参数替换、递归 desugar、多表达式模板自动包 `do`
- 宏与关键字/函数调用优先级正确(宏优先)

**OOP 泛型函数(§22/§23)**
- `defgeneric`/`defmethod` 去 stub:方法模式 `(name Type)` 绑定整个值,分发器运行时查 `generic_table` 按模式匹配
- `defclass`/`definstance` 解析与实例登记(`instance_dict`)
- 声明类节点(defgeneric/defmethod/defclass/definstance)在程序加载时立即求值

**进程演算与通信(§27)**
- `chan`/`send`/`recv` 接线 `ProcessRuntime`:真实缓冲通道收发;`spawn` 子解释器共享通道运行时
- 加密全链路:`secret!` 密钥声明、`encrypt`/`decrypt`/`sign`/`verify`/`hash` 接线 `CryptoEngine`(XOR/简单哈希占位,生产应换 AES/ChaCha/SHA-256)

**FRP(§18)**
- `stream`/`stream-take`/`advance` 接线 `temporal::Stream`(惰性流);`SignalNew/Map/Filter/Fold` 节点接线 `frp::Signal`(值管道语义)

**逻辑编程(§21)**
- `defpred` 子句形式(三种模式列表写法)+ `:free`/`:ground` 模式注解 + `:det`/`:nondet` 注解
- CLP:`domain`/`label` 真实求解并回绑变量;`Domain` 改用有序集合(label 升序枚举解)
- `match_pattern` 同名变量一致性(模式中重复变量要求绑定值一致,§8)
- `and_parallel` 分批处理全部目标(不静默截断)+ 结果合并回共享 store(`merge_from` 共享变量直接 unify、新变量重编号)

**词法与语法**
- lexer 支持 `,`(分隔符)、`:::`(构造器名)、`⃝`(时态算子,§18.1)
- 顶层表达式收集为隐式入口 `__top__` 并执行(§6.3)
- 零参调用 `(f)` 生成 `App(f, Unit)`;`fresh` 多变量形式;`search` 零参形式;GADT 字段列表 `[T1, T2]`(§7.3);`Unit` 值上下文字面量

**LLVM IR 生成(§30)**
- 函数头语法、`ret`→赋值转换(多行)、if-phi 寄存器一致性修复;`--ir` 输出合法文本 IR

**测试**
- 新增约 35 个单元测试,总计 105 个(效果处理器/通道/流/加密/CLP/泛型分发/IR/回溯/合并)

### 修复

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

### 文档

- 重写 `standard_doc/`(INDEX + 01 语言核心 + 02 高级特性 + 03 参考),全部示例实测验证
- 本变更记录

### 已知局限

- `logic-search.tisp`:Mercury 自由变量 + Search `choose` 续延搜索需完整 Prolog 式回溯引擎(部分支持)
- `type_infer` 的 `Perform` 节点类型规则仍返回 fresh var(需 effect row 上下文)
- CLP `constrain` 对 CLP 变量的算术约束编译未实现(仅接受已求值约束)
- 加密算法为 XOR/简单哈希占位;`--compile`/LLVM 真编译需 llvm 工具链
- 宏 hygiene/syntax-quote、类型类实例查找、HIT 完整语义未实现

---

## [0.0.x] - 2026-07(开发阶段摘要)

- **Phase 1-2**:workspace 骨架、core AST、词法/解析/脱糖基础
- **Phase 3**:类型推断(多态、ADT 构造器类型)
- **Phase 4-5**:效果推断、洞(hole)、合约、确定性分析
- **Phase 6-7**:等级检查(QTT)、模式分析(Mercury 风格)
- **Phase 8-9**:区域推断与运行时、解释器
- **Phase 10**:HoTT/会话等高级节点贯通、优化器 pass 骨架
- **Phase 11-13**:运行时模块(逻辑/约束/FRP/进程/定理)、示例、文档初稿

> 各阶段细节见仓库根目录 `PHASE*_SUMMARY.md` 与 `PLAN.md`(历史记录,部分内容已过时,以 standard_doc 为准)。
