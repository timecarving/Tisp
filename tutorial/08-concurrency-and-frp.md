# 第 08 章 并发与 FRP

## 目标

- 理解 Tisp 的时序类型 `⃝`（next）与 `Stream` 定义
- 掌握 `stream`/`stream-take`/`stream-sink` 内置流操作
- 使用 Signal 节点：`signal`/`signal-map`/`signal-filter`/`signal-fold`
- 理解 `delay`/`advance` 的时序语义
- 掌握 `chan`/`send`/`recv`/`spawn`/`join` 的基础并发

---

## 8.1 时序类型与流

Tisp 用**时序模态类型**描述随时间变化的值（`docs/spec.md` §18）。核心算子：

| 算子 | 含义 |
|------|------|
| `⃝ A` | 下一时刻可用的 `A` 值（也写作 `(next A)`） |
| `delay : a → ⃝ a` | 把当前值推迟到下一时刻 |
| `advance : ⃝ a → a` | 解开 `⃝`，推进到当前时刻 |

### 纯函数式流（ADT）

流是「当前值 + 下一个流」的惰性结构（§18.2）：

```tisp
;; ✅ 可类型检查（与 examples/frp-counter.tisp 一致）
(defdata (Stream a)
  (::: [a, (⃝ (Stream a))]))
;; Stream a ≅ a × ⃝(Stream a)
```

`(⃝ (Stream a))` 读作「下一时刻的流」——这就是递归惰性尾的类型表达。基于它可定义常量流与滚动求和：

```tisp
;; ✅ 可类型检查
(defn const-stream [a : Type, x : a] -> (Stream a)
  (::: x (delay (const-stream x))))

(defn running-sum [s : (Stream Int)] -> (Stream Int)
  (let [go (fn [acc : Int, s : (Stream Int)] -> (Stream Int)
             (match s
               (::: x tail)
               (let [new-acc (+ acc x)]
                 (::: new-acc (delay (go new-acc (advance tail)))))))]
    (go 0 s)))
```

要点：
- `delay` 把 `(const-stream x)` 的求值推迟为 `⃝ (Stream a)`——`delay : a → ⃝ a`
- `advance tail` 解开 `⃝`——`advance : ⃝ a → a`
- 纯函数式的 `Stream` 是**类型级建模**：`const-stream`/`running-sum` 通过 `--typecheck`，但不产生 `main` 的运行时输出（惰性尾以 `delay` 装箱）

### 内置流（惰性整数流）

Tisp 另有一组**运行时内置流**（默认从整数 `n` 生成 `n, n+1, n+2, …`）：

```tisp
;; ✅ 可运行
(stream 1)                    ;; 惰性流 1, 2, 3, …
(stream-take (stream 1) 5)    ;; → [1 2 3 4 5]
(stream-sink (stream 1) 5)    ;; → [1 2 3 4 5]（汇聚到向量）
(advance (stream 7))          ;; → (Stream 8 0)（推进到下一时刻）
```

> 注意：内置 `stream` 返回运行时句柄 `(Stream 头值 id)`；`advance` 接受这种句柄。
> 而纯 ADT 的 `::: x (delay …)` 值才是构造子数据。二者名字相同但运行时表示不同，
> 不能混用——这正是 Tisp「类型级时序 vs 运行时流」的分野。

`always`/`eventually`（LTL-as-types）对**内置流**做有限窗口判定：

```tisp
;; ✅ 可类型检查
(always (stream 1) (fn [x] (> x 0)) 5)   ;; 窗口 5 内全部 >0 → true
(eventually (stream 1) (fn [x] (= x 5)) 5) ;; 窗口 5 内出现 5 → true
```

---

## 8.2 Signal 节点

`Signal` 是 FRP 的抽象信号。Tisp 中 Signal 节点求值为**值管道**语义（立即求值，
而非响应式订阅——见 `standard_doc/02-advanced-features.md` §8.3）。

| 操作 | 形式 | 语义 |
|------|------|------|
| 创建 | `(signal init)` | 以初始值创建信号 |
| 映射 | `(signal-map f sig)` | 当前值经 `f` 映射为新信号 |
| 过滤 | `(signal-filter pred sig)` | 当前值是否通过谓词 |
| 折叠 | `(signal-fold f init sig)` | 折叠为累计值 |

```tisp
;; ✅ 可运行（需 Signal 效应）
(let [s (signal 10)]
  (println (signal-fold + 0 s))              ;; → 10
  (println (signal-map (fn [x] (* x 2)) s))  ;; → (Signal 1)
  (println (signal-filter (fn [x] (> x 5)) s))) ;; → true
```

### 效应行声明

使用 Signal 节点的 `main`（或任何函数）必须在签名中声明效应，否则约束求解报错：

```tisp
;; ✅ 可运行  —— 效应行 [[Signal] ...] 声明
(defn main [] -> [[Signal], rho1, @omega, in, det] Unit
  ...)

;; ❌ 缺效应行 → "Signal 效应缺失:定义 main 调用状态/信号类范式操作,但效应行未声明"
(defn main [] Unit
  (signal 0))
```

上面 `[[Signal], rho1, @omega, in, det]` 是 Tisp 统一函数的六维注解：
`[效应行, 区域, 等级, 模式, 确定性]`（名称后为该维默认值时可省略对应位置）。

---

## 8.3 π-calculus 通道（基础并发）

通道是 Tisp 进程演算的家常入口（`docs/spec.md` §27.2）：

```tisp
;; ✅ 可运行
(defn main [] Unit
  (let [c (chan)]       ;; (chan) 0 参——创建 FIFO 通道
    (send c 42)         ;; 发送：Chan a → a → Unit（缓冲，非阻塞）
    (println (recv c)))) ;; 接收：Chan a → a（空通道报错）
```

类型签名（`crates/tisp-middle/src/type_infer.rs`，已验证）：

```tisp
;; (chan)   : Unit -> Chan i64
;; (send c) : Chan a -> a -> Unit（多态）
;; (recv c) : Chan a -> a
```

`send`/`recv` 的效应由 `effect_infer` 自动推导为 `Channel`，无需手工声明——
`main` 推断出 `Channel(i64) + Session + IO` 即通过类型检查。

### spawn / join（结构化并发）

```tisp
;; ✅ 可运行
(defn main [] Unit
  (let [h (spawn (+ 40 2))]   ;; spawn 在新线程求值 body，返回句柄
    (println (join h))))       ;; join 等待并取回结果 → 42
```

- `spawn` 产 `Spawn` 效应；`join` 等待子任务结果并传播错误
- 子解释器共享 `ProcessRuntime`（`Arc<Mutex>`）——spawn 的线程可与父线程共享通道

单解释器内 `send`→`recv` 走 session 协议顺序（send 后状态翻转到 recv，再 recv 成功）。
跨线程通信当前受 session 协议状态机约束 —— 子线程各自持协议状态，与父线程经缓冲通道
传输时顺序校验可能报 `session protocol error`。故以下仅保证**类型检查通过**，运行受该
限制（⚠️）：

```tisp
;; ⚠️ 可类型检查；运行时可能因 session 协议顺序校验失败（子线程/父线程各自状态机）
(defn main [] Unit
  (let [c (chan)]
    (spawn (send c 99))
    (println (recv c))))
```

---

## 完整示例

本章全部特性的可运行示例见 `tutorial/examples/ch08-frp.tisp`：

```tisp
;; tutorial/examples/ch08-frp.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch08-frp.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch08-frp.tisp
;; （内容见示例文件）
```

预期输出：

```
[0 1 2 3 4]
[5 6 7]
10
(Signal 1)
true
(Stream 43 2)
```

---

## 练习

1. 定义 `(defdata (Stream a))`，编写 `take-n [s n]`：对纯 ADT 流取前 `n` 个值。
2. 用 `stream`/`stream-take` 打印从 `10` 开始的 8 个数，并计算其和（提示：`reduce + 0`）。
3. 用 `chan`/`send`/`recv` 写一个双线程 ping-pong：线程 A `send` 一个值，线程 B `recv` 后回送。
4. 给不声明效应行的 `main` 里调用 `(signal 0)`，观察约束求解报错，再补上 `[[Signal] …]`。

---

## 本章小结

- 时序类型：`⃝ A`/`(next A)`，`delay : a → ⃝ a`，`advance : ⃝ a → a`
- 纯 ADT 流：`(defdata (Stream a) (::: [a, (⃝ (Stream a))]))`——类型级建模
- 内置流：`stream`/`stream-take`/`stream-sink`（惰性整数流）
- Signal 节点：`signal`/`signal-map`/`signal-filter`/`signal-fold`（值管道，需 Signal 效应）
- 通道：`chan`（0 参）/`send`/`recv`（通道效应自动推导）
- 并发：`spawn`/`join`（Spawn 效应，结构化等待）

---

> 上一章: [第 07 章 逻辑编程](07-logic-programming.md) | 下一章: [第 09 章 进程演算](09-process-calculi.md) | [返回目录](INDEX.md)
