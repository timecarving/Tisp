# Tisp编程语言

Tisp设计上是目标是基于静态类型纯声明式的系统级Lisp方言; *Tisp*的灵感是我**做梦**做出来的

Tisp早期设计由Qwen 3.7-Max主持;但是仓库主是学生党,所以后续的工作交给的是***Deepseek V4 pro***来干这件事情,我使用的LLM编程工具是Opencode;

我一直期望Lisp有着能系统级的编程能力和其他学术函数式编程语言的特性

由于是大部分工作是Agent在做,而我又不太会维护或者审查仓库; 所以我发布了0.1版本可能就真的...只是0.1版本

当然啦,因为是纯vibe出来的bug满天飞是正常现象,我连 BNF语法都没给,反正是Lisp,我想LLM做实验类编译器把Lisp基于S-表达式的前端写的很熟练了吧,但如果你不是Lisp方言,或者后缀表达式一类很容易被写的前端那还是BNF吧

## 一些事情

虽然如此我本人其实不太喜欢vibe一点掌控感也没有,但是如果真让我按目标写得写到猴年马月

想Fork的话随便Fork; 只要满足文档里那些设计目标就好;(因为纯vibe的原因作者自己都不确定有些设计目标有没有完成,而且还是在让LLM断断续续发现自己没有实现设计目标的那些特性)

数据结构很大程度上受到Cloujre的影响,因为Cloujre很多类似不可变数据很类似与现代的函数式语言. 而我梦到的恰好是个带许多现代学术特性的Lisp方言;

我非常希望能用这样的Lisp方言编程,但是当我学到编译原理觉得我的目标太远不可及,而且我还有学业上的事情没法专门学习编译原理;所以拿LLM快速实现原型
 
 虽然内存管理是基于QTT的,但如果你能加个编译时自动释放内存的特性,那我也服了你,不过目前vlang都没法确定100%回收完毕呢
 
 以后,如果我真有空闲时间的话有可能学习然后看看这个项目吧
 
 我非常喜欢CLP ALP等逻辑编程; 所以逻辑编程组件一定程度上受Mercury影响
---

## 技术概况

**Tisp 0.1.0** — 基于静态类型、纯声明式、系统级定位的 Lisp 方言。Rust workspace 实现,6 个 crate:

```
crates/
├── tisp-core/       # 类型、AST、效果、等级、模式、区域、数据声明
├── tisp-frontend/   # 词法(lexer)、语法(parser)、脱糖(desugar,含宏展开)
├── tisp-middle/     # 类型/效果/模式/确定性/等级/区域推断、液态类型、优化器
├── tisp-backend/    # 解释器(interpreter)、LLVM IR 生成(codegen)、进程/时序运行时
├── tisp-runtime/    # 逻辑、约束(CLP)、回溯、并发、FRP、进程、HoTT、定理
└── tisp-cli/        # CLI + REPL
```

### 构建与测试

```bash
cargo build --release            # 构建
cargo test --workspace           # 358 个测试
```

### 运行

```bash
./target/release/tisp --run examples/hello.tisp   # 运行示例
./target/release/tisp --desugar examples/adt-test.tisp  # 查看脱糖 Core AST
./target/release/tisp --typecheck examples/adt-test.tisp # 类型检查
./target/release/tisp --ir examples/run-test.tisp # 生成 LLVM IR 文本
./target/release/tisp            # 交互式 REPL
```

CLI flags:`--eval` `--print-ast` `--print-tokens` `--desugar` `--typecheck` `--run` `--verify` `--ir` `--compile`

### 快速上手

```clojure
(defn fib [n]
  (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))

(println (fib 10))      ; => 55
```

### 语言特性

| 类别 | 已实现特性 |
|------|-----------|
| **核心语言** | ADT/GADT(defdata/记录/字段)、模式匹配(match/cons/or-pattern/guard/穷尽性检查)、deriving(Eq/Ord/Show)、持久化数据结构(List/Vector/Map/Set,im HAMT 结构共享)、统一 `def` + 六维注解 `->[ε,ρ,@r,m,d]`、私有定义(defn-/def-) |
| **类型系统** | HM 推断 + 多态 + rank-n + 行多态、QTT 等级(0/1/ω + 数字/符号/复合依赖等级)、液态类型 `{x:T\|pred}`(Z3 验证 + 反例)、Π/Σ 依赖类型、类型族(多模式/rewrite)、类型一等值(Value::Type)、类型类(defclass/definstance + :fun-deps)、tlambda/defpoly/conj/disj/trait 语法糖、五维子类型 |
| **效果系统** | defeffect/handle/perform、续延 `k`(多 body + 状态写回)、效果行消减与子类型、单处理器直接状态线程(monadic 优化)、内置操作(get/put/ask/tell/throw/choose) |
| **逻辑编程** | defpred 子句、CLP(域/标签/约束/all-different/精确除法)、回溯 + trail 恢复、溯因(ALP 多解)、committed-choice、任意谓词 Prolog 式多解、12 类逻辑范式(高阶/归纳 ILP/概率 PLP/时序/描述逻辑/可废止/模糊/表格化/一体化基底/响应式/情境/模态)、EVOLP 稳定模型/DLP 动态稳定模型/MOP |
| **进程演算** | π 通道(FIFO)+ Async + 结构化并发、加密(spi/applied π)、SKI/ambient/ρ/κ 组合子、跨演算迹等价、MPST 会话类型(投影 + 顺序检查) |
| **宏系统** | defmacro、syntax-quote/unquote/unquote-splice、卫生宏(fn/lambda/if-let/when-let/match 重命名)、gensym |
| **OOP** | defgeneric/defmethod(模式分发)、方法组合(around/before/after + call-next-method)、构造器类型驱动编译期特化、类型类 |
| **时序 / FRP** | Stream/Signal/Clock、LTL-as-types(delay/advance/⃝)、生产率检查 + 稳定类型、多时钟重采样、空间回收 |
| **HoTT / Cohesive** | Interval/Path/HIT、HComp(KanFill)/Transp(端点传输)、`:boundary` 可满足性、2 维/N 维立方填充、Cohesive ♭(flat)/♯(sharp)/ʃ(shape)+ adjoint-triple 全语义 |
| **系统级** | defextern + dlopen(字符串/指针/i64/f64)、无 GC 栈式区域内存、ptr-read/write + Unsafe 效应门控、编译期区域逃逸检查、统一内存管理(线性类型 + 分级线性 + 手动 Unsafe 经单一 Grade + EffectRow) |
| **验证** | defprop/verify、模型检查(可达性 + 反例 trace)、find-attack、dolev-yao 攻击者知识合成、check-equivalence |
| **编译器 / 工具链** | 解释器 + LLVM IR 生成(inkwell 真实 IR + 文本回退 + 闭包环境)、编译指示(inline!/specialize!/opt-level/suppress-warning)、反射(type-of/effects-of/grade-of/mode-of/determinism-of)、90+ 内置函数、REPL(:type 查询) |

> 全部 32 章 ✅ 全链路可用(无 ⚠️/⬜ 项)。逐章证据见 [standard_doc/04-implementation-status.md](./standard_doc/04-implementation-status.md)。

### 文档

| 文档 | 说明 |
|------|------|
| [standard_doc/INDEX.md](./standard_doc/INDEX.md) | 语言标准文档(词法/核心语言/高级特性/参考/实现状态) |
| [standard_doc/04-implementation-status.md](./standard_doc/04-implementation-status.md) | spec 32 章实现状态与未实现特性清单 |
| [CHANGELOG.md](./CHANGELOG.md) | 变更记录(0.1.0 变更声明 + 已知局限) |
| [docs/spec.md](./docs/spec.md) | 语言设计规范(32 章 + 6 附录,状态符号 ✅/⚠️/⬜ 内联) |
| [PLAN.md](./PLAN.md) | 项目现状与后续方向(替代历史分阶段计划) |

### 实现状态概览

✅ 全链路可用(32/32 章):核心语言、效果系统、宏(卫生)、OOP 泛型分发(+特化)、逻辑编程(子句/CLP/溯因/多解)、通道与加密、FRP 流、液态类型(Z3 验证)、类型族、多模式谓词、committed-choice、MPST 会话、验证(find-attack)、依赖线性类型、Graded Modal Types(按使用次数推导 r/ε)、Dependent Graded Types(符号等级 Z3 判定)、HoTT/Cohesive、时序类型、进程演算、Everything-as-ADT(12 类逻辑范式)、8 类编程范式 + AOP、LLVM IR 生成(inkwell 真实 IR + 文本回退)
⚠️ 部分实现(0/32 章):无
⬜ 设计阶段:无(详见 standard_doc/04-implementation-status.md 唯一事实源)

### 示例程序

`examples/` 下 19 个示例;14 个可运行输出正确(hello/fibonacci/adt/advanced/run/type-infer/state-effect/logic-test/liquid-types-test/remaining-gaps-demo/dependent-linear-test/partial-completion-demo/finish-design-demo/finish-partial-demo),3 个为定义型(无入口,用 `--typecheck` 检查:frp-counter/phase5-test/_qtt-test),1 个部分支持(logic-search),1 个预期报错(液态类型负面用例,`--typecheck` 演示违反与反例)。
