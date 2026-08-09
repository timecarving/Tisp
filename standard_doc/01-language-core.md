# 01 — Tisp 核心语言参考

> 覆盖：词法结构 · 数据类型 · 表达式 · 定义 · ADT/GADT · 模式匹配 · 类型系统基础
> 本文档对齐当前实现(0.1.0)，所有语法均可由 `cargo run -- --run <file>` 实际运行验证。

---

## 1. 词法结构

### 1.1 字符集与注释

源文件使用 **UTF-8** 编码。

```clojure
;; 行注释 — 到行尾为止
```

### 1.2 标识符与关键字

标识符由字母、数字与符号字符组成：

```
标识符 ::= (字母 | 符号-char)+
符号-char ::= 字母 | 数字 | - | _ | ? | ! | = | < | > | * | / | + | . | :
```

- 普通标识符：`x`、`add-one`、`list-length`、`foo?`、`set!`、`+`、`->`
- **以 `:` 开头的标识符**（如 `:else`、`:free`）是关键字/标签（keyword），用作命名参数、模式注解与特殊标记
- **以 `:` 开头的构造器名**（如 `::`、`:::`）合法——`:::` 用于 FRP 流的构造（§8）

### 1.3 特殊字符与分隔符

| 字符 | 含义 |
|------|------|
| `,` | 分隔符（元组/向量/参数列表中的分隔，可忽略） |
| `[` `]` | 向量字面量与模式列表 |
| `(` `)` | 列表（调用/分组）与模式列表 |
| `.` | cons 模式尾标记（`[X . Xs]`） |
| `⃝` | 时态算子：`⃝ A` 表示「下一时刻的 A」，等价于 `(delay A)` |
| `nil` | 空值（`Unit` 字面量） |

### 1.4 字面量

```clojure
42            ; 整数 (i64)
3.14          ; 浮点数 (f64)
true false    ; 布尔
"hello"       ; 字符串
\a            ; 字符
nil           ; Unit(也写作 `Unit`)
:tag          ; 关键字
```

---

## 2. 数据类型

| 类型 | 语法 | 说明 |
|------|------|------|
| List | `(cons 1 (cons 2 (Nil)))` | 不可变链表；`Nil`/`Cons` 为 ADT 构造器 |
| Vector | `[1 2 3]` | 向量字面量，desugar 为 `Vec` 构造 |
| Map | 设计阶段 | 见 docs/spec.md §4.3 |
| Set | `#{1 2 3}` | 设计阶段 |
| Unit | `nil` / `Unit` | 空值 |

向量字面量在**模式匹配**中与 Cons 链兼容：模式 `[X . _]` 可以匹配向量 `[1 2 3]`（§21 逻辑编程中常用）。

---

## 3. 表达式

### 3.1 函数应用(多参数)

```clojure
(+ 1 2)        ; => 3
(+ 1 2 3)      ; => 6(多参数折叠)
(println "hi") ; 输出 hi
```

应用为左结合，解释器把应用链的参数合并后一次性分发；内置函数按 arity 支持部分应用：

```clojure
((+ 1) 2)      ; => 3(部分应用/柯里化)
```

### 3.2 Lambda

```clojure
(fn [x] (* x 2))           ; 单表达式
(fn [x]
  (println x)
  (* x 2))                 ; 多表达式(依次求值,返回最后一个)
```

### 3.3 Let 绑定

```clojure
(let [x 1
      y 2]
  (+ x y))                 ; => 3(多 body 依次求值)
```

### 3.4 条件

```clojure
(if (> x 0) "pos" "neg")
(cond (< x 0) "neg"
      (= x 0) "zero"
      :else   "pos")       ; :else 分支;无 :else 时最后一项为默认值
```

### 3.5 Do

```clojure
(do expr1 expr2 expr3)     ; 依次求值,返回最后一个
```

### 3.6 模式匹配

```clojure
(match lst
  (Nil) 0
  (Cons x _) x)
```

### 3.7 宏展开(§24)

```clojure
(defmacro double [x]
  (* 2 x))

(double 21)                  ; => 42
```

---

## 4. 定义

### 4.1 函数

```clojure
(defn add [a b] (+ a b))
(defn counter []           ; 零参函数:(counter) 调用
  (println "tick")
  1)
```

### 4.2 顶层表达式

顶层非定义表达式会被收集为隐式入口 `__top__` 并执行：

```clojure
(defn main [] 42)
(println (main))           ; 顶层表达式,输出 42
```

### 4.3 代数数据类型(§7)

```clojure
(defdata (List a)
  (Nil)
  (Cons a (List a)))

(defdata (Maybe a)
  (Nothing)
  (Just a))
```

构造器自动注册：零参构造经 `(Nil)` 调用，带参构造直接调用 `(Just 42)`。

### 4.4 GADT 字段列表(§7.3)

```clojure
(defdata (Expr a)
  (IntLit  [Int]        -> (Expr Int))
  (BoolLit [Bool]       -> (Expr Bool))
  (Add     [(Expr Int), (Expr Int)] -> (Expr Int)))
```

### 4.5 谓词(§21,逻辑编程)

```clojure
(defpred member [X Xs]
  ([X [X . _]])
  ([X [_ . T]] (member X T)))

(member 3 [1 2 3])       ; 成功
(member 9 [1 2 3])       ; => false(失败不报错)
```

### 4.6 效果声明与处理器(§12)

```clojure
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))

(handle (let [_ (put 0)] (f))
  (State s)
  (get [] [k s] (k s s))
  (put [v] [k _s] (k Unit v)))
```

---

## 5. 代数数据类型与模式匹配

构造器字段可匿名或命名(§7.2)：

```clojure
(defdata Person
  (MkPerson {name : String, age : Int}))

(match p
  (MkPerson n a) (println n a))
```

模式种类：变量、通配符 `_`、字面量、构造器、cons 模式 `[X . Xs]`、向量模式 `[a b c]`(编译为 Cons 链)。

---

## 6. 类型系统基础

Tisp 是**强静态类型**语言:所有类型在编译期检查(类型推断 + 多态),通过检查的程序保证**无运行时类型错误**;类型本身是运行时一等公民(Reader Principle)。

内置类型：`i8/i16/i32/i64/u8/u16/u32/u64/f32/f64/bool/String/Unit`。

- 函数类型：`(a -> b)`、效果行 `(a ->[{IO}] b)`(§12.4)
- 多态：类型参数经 defdata 声明，如 `(List a)`；推断自动泛化(rank-n 保留 `forall`)
- 类型推断：`cargo run -- --typecheck <file>` 检查整个文件；**REPL 中每行表达式自动显示推断类型**,`(:type EXPR)` 只查类型不求值
- 等级(§QTT)：`Grade::Zero/One/Omega`，`grade_check` 校验线性资源
- 六维注解：类型/效果/等级/模式/确定性/区域由统一约束系统求解(见 docs/spec.md §9)

> 注：类型推断(`type_infer`)覆盖核心语言；部分高级节点(如 effect perform)的类型规则仍在完善中。
