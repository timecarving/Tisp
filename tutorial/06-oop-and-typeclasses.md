# 第 06 章 OOP 与类型类

## 目标

- 使用 `defgeneric` / `defmethod` 定义泛型函数与模式匹配分发
- 掌握方法组合 `:around` / `:before` / `:after` / `:primary` 与 `call-next-method`
- 理解编译期特化与构造子类型匹配
- 使用 `defclass` / `definstance` 声明类型类与 `:fun-deps` 函数依赖

---

## 6.1 泛型函数：defgeneric / defmethod

`defgeneric` 声明一个按模式分派的泛型函数，`defmethod` 为它注册具体方法：

```tisp
;; ✅ 可运行
(defgeneric describe [x])          ;; 泛型声明
(defmethod describe [5] "five")    ;; 对 5 的方法
(defmethod describe [9] "nine")    ;; 对 9 的方法

(defn main []
  (println (describe 5))   ;; → "five"
  (println (describe 9)))  ;; → "nine"
```

- `defgeneric` 的参数向量即分派维度（支持多分派）
- `defmethod` 的模式向量写**方法模式**，调用时按模式匹配选择方法
- ground 类型调用在编译期特化为直连方法，未知类型退化为运行时分发

### 字面量与通配模式

```tisp
;; ✅ 可运行
(defgeneric classify [n])
(defmethod classify [0] "zero")
(defmethod classify [42] "the answer")
(defmethod classify [_] "other")

(defn main []
  (println (classify 0))    ;; → "zero"
  (println (classify 42))   ;; → "the answer"
  (println (classify 7)))   ;; → "other"
```

`_` 是通配方法模式，作为兜底分支。

---

## 6.2 构造子类型匹配（编译期特化 §22.4）

方法模式 `(c Type)` 按**构造子类型**匹配——把整值绑定到 `c`，再在方法体内以 `match` 解构字段：

```tisp
;; ✅ 可运行
(defdata Shape
  (Circle radius)
  (Square side))

(defgeneric area [s])

(defmethod area [(c Circle)]
  (match c
    (Circle r) (* r r 3)))

(defmethod area [(c Square)]
  (match c
    (Square n) (* n n)))

(defn main []
  (println (area (Circle 2)))   ;; → 12
  (println (area (Square 3))))  ;; → 9
```

- `(c Circle)`：实参是 `Circle` 值（构造子类型）时匹配，绑定整值到 `c`
- 编译期特化（§22.4）：调用点类型已知（literal / 构造子类型）时直接特化为对应方法，零查表开销；语义与运行时分发一致

---

## 6.3 方法组合（§22.3）

一个泛型可同时挂载四类方法，按固定顺序执行：

| 类别 | 执行时机 | 返回值 |
|------|----------|--------|
| `:around` | 最外圈，包裹内层 | 决定整体结果 |
| `:before` | primary 之前（副作用） | 丢弃 |
| `:primary`（默认） | 主方法 | 主体结果 |
| `:after` | primary 之后（副作用） | 丢弃 |

### around + call-next-method

```tisp
;; ✅ 可运行
(defgeneric price [p])
(defmethod price :around [p] (* 2 (call-next-method)))
(defmethod price :primary [p] 50)

(defn main [] (println (price 5)))  ;; → 100
```

`call-next-method` 把控制权交给内层组合（这里即 `:primary`）：primary 得 50，around 乘 2 → 100。

### before / after 副作用

```tisp
;; ✅ 可运行
(defgeneric audit [a])
(defmethod audit :before [a] (println "before: entering"))
(defmethod audit :primary [a] 42)
(defmethod audit :after [a] (println "after: leaving"))

(defn main [] (println (audit 0)))
```

输出顺序：

```
before: entering
after: leaving
42
```

`before` / `after` 的结果被丢弃，`primary` 的值成为整体结果。

---

## 6.4 类型类：defclass / definstance（§23）

`defclass` 声明带类型变量、抽象方法与函数依赖的类型类；`definstance` 提供具体实现：

```tisp
;; ✅ 可类型检查
(defclass Coll [c e] :fun-deps [(c -> e)]
  (elem [c] -> e))

(definstance (Coll i64 i64)
  (elem [x] x))
```

- `[c e]`：类型变量；`:fun-deps [(c -> e)]` 表示集合类型 `c` 唯一确定元素类型 `e`
- `(elem [c] -> e)`：抽象方法签名；`definstance` 里给出方法体
- `(Coll i64 i64)`：为 `c = i64, e = i64` 提供实例

### 函数依赖（:fun-deps）

`:fun-deps [(c -> e)]` 约束输入类型唯一确定输出类型，同输入不同输出的实例被拒绝：

```tisp
;; ⚠️ 运行时报 fun-deps 冲突
(defclass Coll [c e] :fun-deps [(c -> e)] (elem [c] -> e))
(definstance (Coll i64 i64)    (elem [x] x))
(definstance (Coll i64 String) (elem [x] "s"))
;; Error: fun-deps 冲突:i64 的同输入已有不同输出
```

> 说明：`defclass` / `definstance` 声明本身可通过 `--typecheck`；实例方法按实参运行时类型的分派逐步接入类型检查器，当前直接在表达式里调用 `elem` 会报 `unbound variable`（标 ⚠️），故实例方法的分派演示见运行时注释而非可运行代码块。

---

## 示例：完整 OOP + 类型类

```tisp
;; tutorial/examples/ch06-oop.tisp
;; ✅ 可运行  $ ./target/debug/tisp --run tutorial/examples/ch06-oop.tisp
;; ✅ 可类型检查  $ ./target/debug/tisp --typecheck tutorial/examples/ch06-oop.tisp

(defdata Shape (Circle radius) (Square side))

(defgeneric area [s])
(defmethod area [(c Circle)]
  (match c (Circle r) (* r r 3)))
(defmethod area [(c Square)]
  (match c (Square n) (* n n)))

(defgeneric price [p])
(defmethod price :around [p] (* 2 (call-next-method)))
(defmethod price :primary [p] 50)

(defgeneric audit [a])
(defmethod audit :before [a] (println "before: entering"))
(defmethod audit :primary [a] 42)
(defmethod audit :after [a] (println "after: leaving"))

(defgeneric describe [d])
(defmethod describe [(c Circle)] "a circle")
(defmethod describe [(c Square)] "a square")

(defclass Coll [c e] :fun-deps [(c -> e)] (elem [c] -> e))
(definstance (Coll i64 i64) (elem [x] x))

(defn main []
  (println (area (Circle 2)))
  (println (area (Square 3)))
  (println (price 5))
  (println (audit 0))
  (println (describe (Circle 1)))
  (println (describe (Square 1))))
```

预期输出：

```
12
9
100
before: entering
after: leaving
42
a circle
a square
```

---

## 练习

1. 给 `Shape` 增加 `(Triangle base height)` 构造子，新增 `area` 方法（面积 = base × height / 2）。
2. 定义 `defgeneric describe` 并挂载 `:around` 方法，把 primary 返回值用括号包裹再返回。
3. 定义 `defclass Ord`（含 `compare` 抽象方法），为 `i64` 提供 `definstance`。
4. 为一个泛型同时写 `:before`、`:primary`、`:after`，观察执行顺序与返回值。

---

## 本章小结

- `(defgeneric name [params])` —— 声明泛型分派函数
- `(defmethod name [:cat] [patterns] body)` —— 注册方法，`:cat` ∈ {`:around` `:before` `:primary` `:after`}
- `call-next-method` —— 方法组合中进入内层
- 方法模式 `(c Type)` —— 按构造子类型匹配（编译期特化 §22.4）
- `(defclass Name [tvars] :fun-deps [(a -> b)] (m [args] -> ret))` —— 类型类
- `(definstance (Name T1 T2) (m [args] body))` —— 实例

---

> 上一章: [第 05 章 宏与元编程](05-macros-and-metaprogramming.md) | 下一章: [第 07 章 逻辑编程](07-logic-programming.md) | [返回目录](INDEX.md)
