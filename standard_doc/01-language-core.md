# 01 — Tisp 核心语言参考

> 覆盖：词法结构 · 数据类型 · 表达式 · 定义 · ADT · 模式匹配 · 类型系统基础

---

## 1. 词法结构

### 1.1 字符集与编码

源文件使用 **UTF-8** 编码。

### 1.2 注释

```clojure
;; 行注释 — 到行尾为止
#| 块注释
   可以跨多行 |#
```

### 1.3 空白符

空格、制表符、换行符、换页符用作 token 分隔符，无其他语义。

### 1.4 标识符

```
标识符 ::= (字母 | 符号-char)+

符号-char ::= 字母 | 数字 | - | _ | ? | ! | = | < | > | * | / | + | .
```

✅ 合法标识符示例：`x`, `add-one`, `list-length`, `+`, `->`, `foo?`, `set!`

### 1.5 关键字

以 `:` 开头的标识符用作命名参数/标签：

```clojure
:requires  :ensures  :in  :out  :det  :around
```

### 1.6 字面量

| 类型 | 语法 | 示例 | ✅ 实现 |
|------|------|------|---------|
| 整数 (i64) | `-?[0-9]+` | `42`, `-7`, `0` | ✅ |
| 浮点数 (f64) | `-?[0-9]+\.[0-9]+` | `3.14`, `-0.5` | ✅ |
| 布尔 | `true` / `false` | `true` | ✅ |
| 字符串 | `"[^"]*"` | `"hello"` | ✅ |
| 字符 | `\c` | `\a`, `\n` | ⚠️ (部分) |
| Nil / 空列表 | `nil` | `nil` | ✅ |

### 1.7 特殊字符

| 字符 | 作用 |
|------|------|
| `(` `)` | 列表/表达式定界符 |
| `[` `]` | 向量定界符 |
| `{` `}` | Map/Set 定界符 |
| `'`  | 引号 (quote) |
| `#`  | 指令前缀 |

---

## 2. 数据类型

### 2.1 原始类型

| Tisp 类型 | Rust 对应 | 字面量示例 |
|-----------|----------|-----------|
| `i8` `i16` `i32` `i64` | `i8`..`i64` | `42` (i64), `-7` |
| `u8` `u16` `u32` `u64` | `u8`..`u64` | `255u8` ⚠️ (语法未实现) |
| `f32` `f64` | `f32` `f64` | `3.14` (f64) |
| `bool` | `bool` | `true`, `false` |
| `String` | `String` | `"hello"` |
| `Unit` | `()` | 函数无返回时 |

### 2.2 复合类型 ✅

| 类型 | 构造语法 | 示例 |
|------|---------|------|
| 列表 | `(1 2 3)` | `(Cons 1 (Cons 2 (Nil)))` |
| 向量 | `[1 2 3]` | `[x y z]` |
| Map | `{:a 1 :b 2}` | ⚠️ (语法已解析，未完全脱糖) |
| Set | `#{1 2 3}` | ⚠️ (语法已解析，未完全脱糖) |
| 元组 | `(1 "a" true)` | ⚠️ (语法设计阶段) |

### 2.3 函数类型 ✅

```
(param-type ->[effects, region, @grade, mode, det] return-type)
```

简化形式（省略标注 = 使用默认值）：

```clojure
i64 -> i64            ; 纯函数，ω 使用，det 确定性
i64 ->[IO] Unit       ; 带 IO 效果的函数
```

---

## 3. 表达式

### 3.1 字面量 ✅

```clojure
42        ; i64
3.14      ; f64
true      ; bool
"hello"   ; String
nil       ; nil/空列表
```

### 3.2 变量引用 ✅

```clojure
x         ; 在当前作用域查找 x
```

### 3.3 函数调用 ✅

```clojure
(f arg1 arg2 ...)
```

Tisp 采用 **左结合函数调用**：`(+ 1 2)` 等价于把 `+` 作用于 `1` 和 `2`。

```clojure
(+ 1 2 3)       ; → 6
(* (+ 3 4) 2)   ; → 14
(println "hi")  ; 输出 "hi"，返回 ()
```

### 3.4 Lambda 表达式 ✅

```clojure
(fn [x] (+ x 1))
(fn [x y] (* x y))
```

### 3.5 Let 绑定 ✅

```clojure
(let [x 42
      y (+ x 8)]
  (println y))          ; 输出 50
```

Let 绑定按顺序求值，后续绑定可引用前面的变量。

### 3.6 If 表达式 ✅

```clojure
(if condition then-expr else-expr)
```

```clojure
(if (<= n 1)
  1
  (* n (factorial (- n 1))))
```

### 3.7 Cond 表达式 ⬜

多分支条件（设计阶段）：

```clojure
(cond
  (< x 0)  "negative"
  (= x 0)  "zero"
  :else    "positive")
```

### 3.8 Do 表达式 ✅

顺序执行多个表达式，返回最后一个的值：

```clojure
(do
  (println "processing...")
  (compute-result)
  (return-value))
```

### 3.9 类型标注 ✅

```clojure
x : i64
name : String
f : (i64 -> i64)
```

---

## 4. 定义

### 4.1 值定义 `(def name body)` ✅

```clojure
(def answer 42)
(def greeting (str "Hello, " name))
```

### 4.2 函数定义 `(defn name [params] body)` ✅

```clojure
(defn add [x y]
  (+ x y))

(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))
```

### 4.3 带类型标注的函数定义 ✅

```clojure
(defn add-one [x : i64] -> i64
  (+ x 1))

(defn greet [name : String] ->[IO] Unit
  (println (str "Hello, " name)))
```

### 4.4 带合约的函数定义 ⚠️

```clojure
(defn divide [n : i64, d : {x : i64 | (!= x 0)}] -> i64
  :requires true
  :ensures (= result (quot n d))
  (quot n d))
```

> ⚠️ `:requires` / `:ensures` 语法已解析但液态类型验证未完成。

### 4.5 递归函数 ✅

```clojure
(defn list-length [lst]
  (match lst
    (Nil) 0
    (Cons _ rest) (+ 1 (list-length rest))))
```

### 4.6 统一 `def` 形式

所有定义都是 `def` 的特化形式，带有六维标注（缺省使用默认值）：

```clojure
(def name                   ; 名称
  [p1 : T1, ...]            ; 参数
  ->[ε, ρ, @r, m, d] Ret    ; 六维标注（可选）
  body)                     ; 主体
```

| 维度 | 缩写 | 默认值 | 含义 |
|------|------|--------|------|
| 效果 | ε | `Pure` | 副作用集合 |
| 区域 | ρ | (推断) | 内存分配区域 |
| 等级 | @r | `ω` | QTT 使用次数 |
| 模式 | m | `In` | Mercury 模式 |
| 确定性 | d | `Det` | 解数+失败可能 |

---

## 5. 代数数据类型 (ADT)

### 5.1 定义数据类型 `(defdata ...)` ✅

```clojure
(defdata (Maybe a)          ; 单类型参数
  (Nothing)                 ; 无字段构造器
  (Just a))                 ; 带字段构造器

(defdata Color              ; 无参数
  (Red) (Green) (Blue))

(defdata (Pair a b)         ; 多类型参数
  (MkPair a b))
```

### 5.2 构造器应用 ✅

```clojure
(Just 42)                   ; → (Just 42) : Maybe i64
(Nothing)                   ; → Nothing : Maybe ?a
(MkPair 1 "hello")          ; → MkPair 1 "hello" : Pair i64 String
```

构造器自动拥有正确的多态类型方案，在应用时进行即时化（instantiation）。

### 5.3 模式匹配 `(match ...)` ✅

```clojure
(defn maybe-to-string [m]
  (match m
    (Nothing) "nothing"
    (Just x) (str "just: " x)))
```

支持的 match arm 形式：

| 形式 | 语义 |
|------|------|
| `(Pattern body)` | 无 guard 的分支 |
| `(Pattern :when guard body)` | 带 guard 的分支 ⬜ |

支持的 Pattern 形式：

| Pattern | 示例 | 语义 |
|---------|------|------|
| 变量 | `x` | 绑定任意值 |
| 通配符 | `_` | 匹配任意，不绑定 |
| 构造器 | `(Just x)` | 匹配指定构造器 |
| 构造器嵌套 | `(Cons x (Cons y (Nil)))` | 深层匹配 |
| 字面量 | `42`, `true` | 精确匹配 ⚠️ |

---

## 6. 类型系统

### 6.1 HM 类型推断 (Algorithm W) ✅

Tisp 使用 **Hindley-Milner 类型推断**，支持：

- **类型变量**：`?1`, `?2`, ... 自动生成
- **合一 (Unification)**：通过 occurs check 解决 `t1 = t2`
- **即时化 (Instantiation)**：每次使用多态类型时用新鲜变量替换
- **泛化 (Generalization)**：let 绑定中泛化不在环境中的自由类型变量

```clojure
(defn id [x] x)
;; 推断类型: id : ∀a. a → a
;; 实际输出: id : ?1 -> ?1

; 每次调用 id 使用不同的即时化:
(defn main []
  (let [a (id 42)      ; id : i64 → i64
        b (id "hello")] ; id : String → String
    a))
```

### 6.2 函数类型构造

```clojure
i64 -> i64                      ; defn add [x : i64] -> i64 (+ x 1)
i64 -> i64 -> i64               ; defn add [x y] (+ x y)
i64 ->[IO] Unit                 ; defn print-val [x] (println x)
```

### 6.3 多态与泛化

`let` 绑定的值如果其类型中的变量不在当前环境中，则泛化为 `Forall` 类型方案：

```clojure
(defn const [x y] x)
;; const : ?3 -> ?4 -> ?3   (类型保留为变量，因为 y 自由)

; 但一旦实例化到 let 中：
(let [f (fn [x] x)]
  ;; f : ∀a. a → a (泛化后)
  ...)
```

### 6.4 类型环境 (TypeEnv)

内置类型环境注册了所有原始运算的类型方案：

```clojure
+ : i64 -> i64 -> i64
= : ∀a. a -> a -> bool
println : ∀a. a ->[IO] Unit
```

### 6.5 Kind 系统 ⚠️

```
Kind ::= Star | Arrow(Kind, Kind) | Effect | Grade | Region | Mode | Determinism | Session
```

Kind 类型已在 Core 中完整定义，但 kind checking 在类型推断中只有 basic 支持。

### 6.6 液态/Refinement 类型 ⚠️

```clojure
{x : T | predicate}
```

> ⚠️ 类型变体 `Type::Refined` 已定义，`requires`/`ensures` 语法已解析，但 Z3 验证仅部分集成。

---
