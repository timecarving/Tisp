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
cargo test --workspace           # 105 个测试(0.1.0)
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

语言特性:ADT/GADT、模式匹配(同名变量一致性)、效果系统(handle/perform)、逻辑编程(defpred/CLP/回溯)、宏(defmacro)、OOP 泛型函数(defgeneric/defmethod)、FRP 流、进程演算通道与加密、QTT 等级、HoTT。

### 文档

| 文档 | 说明 |
|------|------|
| [standard_doc/INDEX.md](./standard_doc/INDEX.md) | 语言标准文档(词法/核心语言/高级特性/参考) |
| [CHANGELOG.md](./CHANGELOG.md) | 变更记录(0.1.0 变更声明 + 已知局限) |
| [docs/spec.md](./docs/spec.md) | 原始设计规范(设计目标与语法定义,部分特性仍处设计阶段) |

### 实现状态概览

✅ 全链路可用:核心语言、效果系统、宏、OOP 泛型分发、逻辑编程(子句/CLP)、通道与加密、FRP 流、LLVM IR 文本生成
⚠️ 部分实现:Search effect 续延搜索、液态类型(Z3)、HoTT、类型类实例查找、LLVM 真编译
⬜ 设计阶段:详见 docs/spec.md 与 CHANGELOG 已知局限

### 示例程序

`examples/` 下 13 个示例;8 个可运行输出正确(hello/fibonacci/adt/advanced/run/type-infer/state-effect/logic-test),4 个为定义型(无入口,用 `--typecheck` 检查),1 个部分支持(logic-search)。
