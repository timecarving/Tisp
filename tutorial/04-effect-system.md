# 第 04 章 效应系统

## 目标

- 用 `defeffect` 声明自定义效应与操作
- 用 `handle` 编写效应处理器，理解续延 `k` 的传递机制
- 在函数签名中声明效应行（`-> [State, IO] i64`）
- 掌握内置效应：State / Except / Search / Reader / Writer
- 使用 monadic 风格（`mlet` / `get-m` / `put-m` / `pure`）获得零开销状态传递
- 理解「效应缺失」编译错误及其修复方法

---

## 4.1 效应声明：`defeffect`

Tisp 通过**代数效应**实现纯声明式副作用管理——函数调用有副作用的操作（如读写状态）时，必须在签名里声明，并由 handler 在上层接管语义。

### 基本格式

```tisp
;; ✅ 可类型检查
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))
```

- `State` 是效应名，`s` 是类型参数（状态类型）
- 每个操作声明为 `(op-name [params] -> ReturnType)`
- `get` 无参数，返回当前状态
- `put` 接收新状态，返回 `Unit`

内置效应（如 `State`、`Except`、`Search`）已预注册，即使不写 `defeffect` 也可直接使用 `get`/`put`/`throw` 等操作。

---

## 4.2 效应处理器：`handle`

`handle` 接管效应操作，提供自定义语义：

```tisp
;; ✅ 可类型检查
(defn run-state [init f] -> i64
  (handle (let [_ (put init)]
            (f))
    (State s)
    (get [] [k s] (k s s))
    (put [v] [k _s] (k Unit v))))
```

### 语法结构

```
(handle body
  (EffectName type-args...)
  (op-name [op-params] [k state-var] handler-body)
  ...)
```

### 关键概念

| 部分 | 说明 |
|------|------|
| `body` | 被监听的表达式（效应在此作用域内被拦截） |
| `(State s)` | 效应名及类型参数 |
| `(get [] [k s] (k s s))` | 操作子句：`[k s]` 第一项是续延 `k`、第二项（可选）是当前状态 |
| 续延 `k` | handler 结束后的「剩余计算」，调用 `(k result state)` 传递结果与新状态 |
| `(k s s)` | 读取当前状态，保持状态不变并把值传给后续计算 |

**执行流程**（以 `get` 为例）：
1. `body` 中执行 `(get)` → handler 拦截
2. 当前状态 `s` 绑定，续延 `k` 绑定
3. handler body `(k s s)`：将当前状态作为 get 返回值，状态不变

### 内置 Except 效应示例

```tisp
;; ✅ 可类型检查
(defn catcher [f] -> i64
  (handle (f)
    (Except e)
    (throw [v] [k] 0)))  ;; throw 被捕获，返回 0
```

`throw` 的续延 `k` 可用于「恢复」计算，或直接返回替代值跳过续延（本 handler 选择了后者）。

---

## 4.3 效应行：在函数签名中声明

调用效应操作的函数**必须在签名中声明效应行**，否则编译器报「State 效应缺失」：

```tisp
;; ❌ 编译错误：效应缺失
(defn counter-bad [] -> i64
  (let [x (get)]
    (put (+ x 1))
    x))
;; Error: State 效应缺失 — 调用 get/put 但效应行未声明
```

```tisp
;; ✅ 正确：在签名声明 [State]
(defn counter [] -> [State] i64
  (let [x (get)]
    (put (+ x 1))
    (get)))
```

### 效应行格式

```tisp
(defn f [] -> [IO] Unit ...)             ;; 单效应
(defn g [] -> [State, IO] i64 ...)       ;; 多效应
(defn h [] -> [Pure] i64 ...)            ;; 纯函数（可省略为 -> i64）
```

效应行在 `->` 之后、返回类型之前：`-> [effect1, effect2] ReturnType`。

---

## 4.4 内置效应一览

| 效应 | 操作 | 说明 |
|------|------|------|
| `State s` | `get`, `put` | 可变状态 |
| `Reader r` | `ask` | 只读环境 |
| `Writer w` | `tell` | 累加输出 |
| `Except e` | `throw` | 错误处理 |
| `IO` | `println`, `read-line` | 输入/输出 |
| `Search` | `choose` | 回溯搜索 |

内置效应无须 `defeffect`，直接使用即可（但调用方仍需声明效应行）。

### 使用示例

```tisp
;; ✅ 可类型检查
(defn asker [] -> [Reader] i64   (ask))
(defn writer [] -> [Writer] i64  (tell 42) 0)
(defn chooser [] -> [Search] i64 (choose 10))
```

> **注意**：当前实现中 `Reader`/`Writer`/`Except` 的效应行检查较为宽松，但 `State` 严格强制声
> 明——所有带 `get`/`put` 的函数均须标注 `[State]`。

---

## 4.5 Monadic 风格：零开销状态传递

对于单一 handler、无嵌套的状态用法，编译器可自动降级为直接状态传递（零 overhead）。使用 monadic
操作符显式标注这一意图：

| 操作符 | 说明 |
|--------|------|
| `(get-m)` | 获取当前状态 |
| `(put-m v)` | 写入状态 |
| `(pure v)` | 将值提升为 monadic 结果 |
| `(mlet [bindings] body)` | monadic let（同 `let` 语法） |

```tisp
;; ✅ 可运行
(defn monadic-demo [] -> [State] i64
  (mlet [x (get-m)
         _ (put-m (+ x 1))]
    (pure x)))
```

`get-m` / `put-m` 在内部等价于 `(perform get)` / `(perform put v)`，`pure` 为恒等包装，
`mlet` 与 `let` 同构。仍需在签名声明 `[State]` 效应行。

---

## 4.6 完整示例

```tisp
;; tutorial/examples/ch04-effect-handler.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch04-effect-handler.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch04-effect-handler.tisp

;; (完整代码见 tutorial/examples/ch04-effect-handler.tisp)
```

运行输出：
```
3
0
0
0
```

关键点：
- `counter` 三次调用 → 状态从 0 累加到 3
- `monadic-demo` 初始状态 0，`get-m` 返回 0 后 `put-m 1`，`pure x` 得 0
- `throw` 被 handler 捕获，返回 0 替代抛出的 42
- `choose` 被 handler 接管，传入 0 继续

---

## 4.7 `do` 顺序执行

`(do expr1 expr2 ... exprN)` 按顺序求值所有表达式，返回最后一个表达式的值：

```tisp
;; ✅ 可运行
(defn main [] -> i64
  (do
    (println "A")
    (println "B")
    42))
```

适用于在 handler body 或效应函数中执行多个有副作用的步骤。

---

## 4.8 `perform` 显式效应调用

除了直接在函数体写 `(get)`、`(put v)` 外，也可使用显式 `perform`：

```tisp
;; ✅ 可类型检查
(perform get)
(perform put 42)
(perform throw "error")
```

两种写法等价；`perform` 主要用于强调「此处触发效应」的场合。

---

## 练习

1. 定义自己的 `(defeffect Counter i64 (inc [] -> i64) (reset [] -> Unit))`，
   编写 handler，使 `inc` 自增计数器、`reset` 归零。
2. 为 `State s` 编写一个「只读」handler：`get` 正常返回，`put` 被忽略（状态不变）。
3. 用 `mlet` 链编写函数，读取状态 → 翻倍 → 写回 → 再次读取返回。
4. 写一个同时使用 `State` 和 `Except` 的函数（签名 `-> [State, Except] i64`），
   用嵌套 `handle` 分别处理两个效应。观察编译器输出中的「monadic optimization」提示。

---

## 本章小结

- `(defeffect Name params (op [args] -> Ret) ...)` —— 声明效应
- `(handle body (EffectName t) (op [args] [k s] body) ...)` —— 效应处理器
- 续延 `k` 是 handler 的「剩余计算」，手工调用 `(k result state)` 继续执行
- 效应行：`(defn f [] -> [State, IO] i64 ...)`，缺失时编译器报「效应缺失」
- 内置效应：State / Except / Search / Reader / Writer / IO
- Monadic 风格：`mlet` / `get-m` / `put-m` / `pure`，触发零开销状态传递优化
- `(do ...)` 顺序执行多个表达式

---

> 上一章: [第 03 章 深入类型系统](03-type-system-deep.md) | 下一章: [第 05 章 宏与元编程](05-macros-and-metaprogramming.md) | [返回目录](INDEX.md)