# 第 15 章 HoTT 与 deriving

## 目标

- 了解 Tisp 对同伦类型论（HoTT）的支持：Path、Interval、端点传输
- 掌握 `deriving` 自动派生（Eq/Ord/Show）
- 使用代数结构验证工具（fun-ext、monoid-check）

---

## 15.1 HoTT 路径与区间

Tisp 在类型层提供同伦概念的核心原语（实现状态：⚠️ 部分，相关内置可用）：

### 区间端点传输 `transp`

`transp` 沿路径把一个端点值传输到另一端：

```tisp
;; ✅ 可运行
(transp (fn [i] 9) true)   ;; 路径恒为 9，true 端点 → 9
```

### 路径连通 `shape`

```tisp
;; ✅ 可运行
(shape (fn [i] 42))        ;; → (Shape true 42 42)（路径两端连通）
```

### 区间逻辑

```tisp
;; ✅ 可运行
(interval-neg true)        ;; 区间取反 → false
(interval-and true false)  ;; 区间合取 → false
(interval-or true false)   ;; 区间析取 → true
```

> 深入阅读：`docs/spec.md` §16-17 关于 HoTT、依赖路径与区间类型的规范。

---

## 15.2 HIT（高阶归纳类型）

HIT 在 `defdata` 基础上允许构造子带路径边界条件（设计阶段 ⬜，语法保留）：

```tisp
;; ⬜ 设计阶段语法展示（当前实现未覆盖）
(defdata-hit S1
  (base)
  (loop [i : I]
    :boundary [(i = i0) -> base
               (i = i1) -> base]))
```

---

## 15.3 `deriving` 自动派生

```tisp
;; ✅ 可运行
(defdata Color :deriving (Eq Ord Show)
  (Red)
  (RGB i64 i64 i64))
```

- `Eq`：派生 `=` / `!=`
- `Ord`：派生 `ord-<类型名>` 比较函数，返回 -1/0/1
- `Show`：派生打印

```tisp
;; ✅ 可运行
(ord-Color (RGB 1 2 3) (RGB 1 2 4))  ;; → -1（前者小）
(= (RGB 1 2 3) (RGB 1 2 3))          ;; → true
```

---

## 15.4 代数结构验证工具

Tisp 提供代数性质检查内置（对应 `docs/spec.md` §16.3-16.4）：

### 函数点态等价 `fun-ext`

```tisp
;; ✅ 可运行
(defn id [x] x)
(defn not-id [x] (+ x 1))

(fun-ext id id (range 1 4))      ;; → true（在样本点上一致）
(fun-ext id not-id (range 1 4))  ;; → false
```

`fun-ext f g samples`：在 `samples` 每个点上比较 `f` 与 `g`。

### 幺半群检查 `monoid-check`

```tisp
;; ✅ 可运行
(defn plus [a b] (+ a b))

(monoid-check plus 0 (range 1 4))  ;; → true（加法 + 0 满足结合律与单位元）
```

---

## 15.5 时序模态（与 HoTT 相邻的特性）

时序类型使用 `⃝`（next）、`□_t`（always）、`◇_t`（eventually）：

```tisp
;; ✅ 可类型检查（参考 examples/frp-counter.tisp）
(defdata (Stream a)
  (::: [a, (⃝ (Stream a))]))
```

- `delay expr`：延迟求值到下一时刻
- `advance tail`：推进时序流
- `clock "c" 5`：时钟值

详细用法见[第 08 章 并发与 FRP](08-concurrency-and-frp.md)。

---

## 示例

`tutorial/examples/ch15-hott-derived.tisp`：

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch15-hott-derived.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch15-hott-derived.tisp

;; deriving
(defdata Color :deriving (Eq Ord Show)
  (Red)
  (RGB i64 i64 i64))

;; 代数性质验证
(defn id [x] x)
(defn not-id [x] (+ x 1))
(defn plus [a b] (+ a b))

(defn main []
  ;; HoTT 原语
  (println (transp (fn [i] 9) true))
  (println (shape (fn [i] 42)))
  (println (interval-and true false))
  ;; 函数点态等价
  (println (fun-ext id id (range 1 4)))
  (println (fun-ext id not-id (range 1 4)))
  ;; 幺半群
  (println (monoid-check plus 0 (range 1 4)))
  ;; deriving
  (println (ord-Color (RGB 1 2 3) (RGB 1 2 4)))
  ;; 时钟
  (println (clock "c" 5)))
```

预期输出：
```
9
(Shape true 42 42)
false
true
false
true
-1
(Clock 5 c)
```

---

## 练习

1. 用 `deriving` 定义一个 `Suit`（Spades/Hearts/Diamonds/Clubs）并比较两个不同值。
2. 构造两个在样本点 `(range 0 10)` 上相等但定义不同的函数，验证 `fun-ext` 的样本语义。
3. 尝试 `(monoid-check * 1 (range 1 5))`（乘法幺半群），观察输出。
4. 阅读 `docs/spec.md` §17 理解 Path 与 `transp` 的规范，写一条注释解释 `transp` 的第一个参数。

---

## 本章小结

- HoTT 内置：`transp`（端点传输）、`shape`（连通）、`interval-neg/and/or`（区间逻辑）
- HIT：`defdata-hit`（⬜ 设计阶段）
- `deriving (Eq Ord Show)` 自动派生比较与打印
- `fun-ext`（点态等价）、`monoid-check`（幺半群验证）用于代数结构检查

---

> 上一章: [第 03 章 深入类型系统](03-type-system-deep.md) | 下一章: [第 16 章 编译与工具链](16-compilation-and-toolchain.md) | [返回目录](INDEX.md)