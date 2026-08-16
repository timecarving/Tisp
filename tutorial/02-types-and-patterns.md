# 第 02 章 类型与模式匹配

## 目标

- 使用 `defdata` 定义代数数据类型（ADT）和 GADT
- 理解 record 语法和 `deriving`
- 掌握 `match` 的多种模式（字面量/变量/构造函数/通配符/守卫/or 模式）
- 运用穷尽性检查消除遗漏情况

---

## 2.1 代数数据类型（ADT）

### 基本 ADT

```tisp
;; ✅ 可类型检查
(defdata (Maybe a)
  (Nothing)
  (Just a))

(defdata Color
  (Red)
  (Green)
  (Blue))

(defdata (Pair a b)
  (MkPair a b))
```

`defdata` 定义一个**带类型变量的新类型**，随后可用匹配区分构造。

- `Maybe a`：含类型变量 `a` 的多态类型
- `Color`：枚举式 ADT（Red/Green/Blue 不带值）
- `Pair a b`：含两个类型变量的产品类型

### 构造与匹配

```tisp
;; ✅ 可运行
(defdata (Maybe a) (Nothing) (Just a))

(defn safe-div [n d]
  (if (= d 0) (Nothing) (Just (/ n d))))

(defn describe [m]
  (match m
    (Nothing) "nothing"
    (Just x) (str-concat "got " (str x))))

(defn main []
  (println (describe (safe-div 10 2)))
  (println (describe (safe-div 10 0))))
```

说明：
- `Nothing` / `Just 5` 等是**构造子调用**，返回 ADT 值
- `match` 按模式分派——（`Just x`）绑定 `x` 到构造子的内部值

### 自定义列表（List ADT）

`defdata` 可以定义链式列表：

```tisp
;; ✅ 可类型检查
(defdata (List a)
  (Nil)
  (Cons a (List a)))
```

用法：

```tisp
;; ✅ 可运行
(defn len [xs]
  (match xs
    (Nil) 0
    (Cons _h rest) (+ 1 (len rest))))

(len (Cons 1 (Cons 2 (Cons 3 (Nil)))))  ;; → 3
```

---

## 2.2 Record 语法

含多个字段的构造子可使用 record 语法：

```tisp
;; ✅ 可类型检查
(defdata Person
  (MkPerson name age))
```

```tisp
;; ✅ 可运行
(defn greet [p]
  (match p
    (MkPerson n a)
    (str-concat "Hello, " n)))

(greet (MkPerson "Alice" 30))  ;; → "Hello, Alice"
```

---

## 2.3 GADT（广义代数数据类型）

GADT 允许**构造子的返回类型**随参数变化，实现类型级求值：

```tisp
;; ✅ 可类型检查
(defdata (Expr a)
  (IntLit i64 -> (Expr i64))
  (BoolLit bool -> (Expr bool))
  (Add (Expr i64) (Expr i64) -> (Expr i64))
  (If (Expr bool) (Expr a) (Expr a) -> (Expr a)))
```

**声明格式**：`(Constructor param1 param2 … -> ReturnType)`

类型安全求值器：

```tisp
;; ✅ 可运行
(defn eval-expr [e]
  (match e
    (IntLit n) n
    (BoolLit b) b
    (Add x y) (+ (eval-expr x) (eval-expr y))
    (If c t f) (if (eval-expr c) (eval-expr t) (eval-expr f))))
```

- `IntLit 42` 求值为 i64、`BoolLit true` 为 bool——GADT 保证类型安全
- `(Add (IntLit 40) (IntLit 2))` 经 `eval-expr` 得 `42`

---

## 2.4 模式匹配完全手册

### 基本模式

```tisp
(match value
  42        "literal"               ;; 字面量匹配
  x         "variable"              ;; 变量绑定（可匹配任何值）
  (Just y)  "constructor"           ;; 带绑定变量的构造子模式
  _         "wildcard")             ;; 通配符（丢弃值）
```

### 守卫模式

```tisp
;; ✅ 可运行
(defn describe-n [n]
  (match n
    0 "zero"
    (when x (= x 42)) "the answer"
    x "other"))
```

`(when x (= x 42))`：绑定到 `x` 并施加条件 `(= x 42)`。

### or 模式（多选一）

```tisp
;; ✅ 可运行
(match x
  (or 1 2 3) "small"
  (or 4 5 6) "medium"
  _ "large")
```

一个 `or` 分支内所有子模式必须有相同数量的变量绑定。

### 穷尽性检查

匹配未覆盖全部构造子 → 编译错误：

```tisp
;; ❌ 编译期报错
(defdata Color (Red) (Green) (Blue))

(defn name [c]
  (match c
    (Red) "red"
    (Green) "green"))
;; Error: match is non-exhaustive — missing constructors: [Blue]
```

**实践规则**：始终包含通配符 `_` 或完整列举全部构造子。

---

## 2.5 `if-let` / `when-let`

简化单分支匹配的语法糖：

```tisp
;; ✅ 可运行
;; if-let：模式匹配到 x 时执行 then，否则执行 else
(if-let [x (Just 42)]
  x
  0)  ;; → 42

;; when-let：成功则执行体，失败返回 false
(when-let [x true]
  x)  ;; → true
```

---

## 2.6 `deriving` 自动派生

```tisp
;; ✅ 可类型检查
(defdata Color :deriving (Eq Ord Show)
  (Red)
  (Green)
  (Blue))
```

- `Eq`：自动生成 `=` / `!=` 比较
- `Ord`：自动生成 `<` / `>` / `<=` / `>=`
- `Show`：自动生成 `->string` / `println` 格式化

---

## 示例：完整 ADT + 模式匹配

```tisp
;; tutorial/examples/ch02-adt-match.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch02-adt-match.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch02-adt-match.tisp

;; 多态 Maybe
(defdata (Maybe a) (Nothing) (Just a))

;; GADT 表达式
(defdata (Expr a)
  (IntLit i64 -> (Expr i64))
  (BoolLit bool -> (Expr bool))
  (Add (Expr i64) (Expr i64) -> (Expr i64))
  (If (Expr bool) (Expr a) (Expr a) -> (Expr a)))

;; 求值器
(defn eval-expr [e]
  (match e
    (IntLit n) n
    (BoolLit b) b
    (Add x y) (+ (eval-expr x) (eval-expr y))
    (If c t f) (if (eval-expr c) (eval-expr t) (eval-expr f))))

(defn safe-div [n d]
  (if (= d 0) (Nothing) (Just (/ n d))))

(defn describe-graded [score]
   (cond (>= score 90) "A"
         (>= score 80) "B"
         (>= score 70) "C"
         "F"))

(defn guess-color [c]
  (match c
    (or Red Green Blue) "known"
    _ "unknown"))

(defn main []
  ;; GADT 求值
  (println (eval-expr (Add (IntLit 40) (IntLit 2))))
  ;; Maybe 模式匹配
  (println (match (safe-div 10 2)
             (Nothing) "error"
             (Just v) (str-concat "ok: " (str v))))
  ;; cond 成绩
  (println (describe-graded 85))
  ;; or-pattern
  (println (guess-color Green)))
```

预期输出：
```
42
ok: 5
B
known
```

---

## 练习

1. 用 `defdata` 定义一个 `(Tree a)`，包含 `(Leaf a)` 和 `(Node (Tree a) (Tree a))`，编写 `sum` 函数（递归）或 `depth`。
2. 基于 `Expr` GADT，增加 `Mul` 乘法构造子，扩展 `eval-expr` 支持乘法。
3. 定义 `(Maybe a)` 的基础上，写一个函数 `map-maybe [f m]` 对 `Just v` 应用 `f`，对 `Nothing` 返回 `Nothing`。
4. 有意写出一个漏掉 `Blue` 的 `match`，观察编译期穷尽性报错，然后修正。

---

## 本章小结

- `(defdata (T a) (Con1 ...) (Con2 ...))` —— 定义 ADT
- GADT：`(Constructor params -> ReturnType)` 控制构造子返回类型
- `match` 模式：literal / variable / constructor / wildcard / `(when pat guard)` / `(or pat…)`
- 穷尽性检查：编译期强制覆盖全部构造子
- `deriving` 自动派生 `Eq/Ord/Show`

---

> 上一章: [第 01 章 开始使用](01-getting-started.md) | 下一章: [第 03 章 深入类型系统](03-type-system-deep.md) | [返回目录](INDEX.md)