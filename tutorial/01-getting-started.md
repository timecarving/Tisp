# 第 01 章 开始使用

## 目标

- 构建 Tisp 编译器并运行第一个程序
- 掌握 REPL 交互模式的基本操作
- 理解字面量、变量定义与函数定义
- 使用 `let`、`if`、`cond` 控制流
- 了解类型标注的基本写法

---

## 1.1 安装与构建

**前置条件**：Rust（2021 edition）、Cargo。可选：LLVM 17（Debian `llvm-17` 包，用于 `--ir`/`--compile`）、Z3（`libz3-dev`，用于液态类型）。

```bash
git clone <tisp-repo> && cd tisp
cargo build --release
```

编译产物：`target/release/tisp`。

验证安装：

```bash
./target/release/tisp --version
```

---

## 1.2 Hello, World!

创建 `hello.tisp`：

```tisp
;; ✅ 可运行  $ tisp --run hello.tisp
;; ✅ 可类型检查  $ tisp --typecheck hello.tisp
(println "Hello, Tisp!")
```

```bash
$ ./target/release/tisp --run hello.tisp
Hello, Tisp!
=> ()
; region stats: 1 allocs, 1 deallocs, 0 bytes (peak: 0)
```

Tisp 中 `println` 是内置函数，返回 `Unit`（显示为 `()`）。注释以 `;;` 开头。

---

## 1.3 REPL 交互模式

不带参数启动 REPL：

```bash
$ ./target/release/tisp
Tisp>
```

**定义行**：`defn`、`defdata`、`defpred` 等定义存入累积环境，不立即执行。

```tisp
Tisp> (defn square [x] (* x x))
```

**表达式行**：其他表达式先类型检查，通过后求值输出。

```tisp
Tisp> (square 5)
=> 25
```

**`:type` 命令**：查询表达式类型，不求值。

```tisp
Tisp> :type (square 3)
i64
```

**退出**：`(exit)` 或 `Ctrl-D`。

> **约定**：REPL 中定义行持久存在（跨表达式行累积），关闭后消失。

---

## 1.4 基本数据类型

### 数值与字符串

```tisp
;; ✅ 可类型检查  $ tisp --typecheck
42          ;; 整数（i64 有符号 64 位）
3.14        ;; 浮点数（f64）
"hello"     ;; 字符串
true false  ;; 布尔（Bool）
()          ;; 单位（Unit）
```

### 列表

```tisp
;; ✅ 可类型检查
(range 0 5)                  ;; 区间构造列表 → [0 1 2 3 4]
(range 1 4)                  ;; → [1 2 3]
(count (range 0 5))          ;; 长度 → 5
(nth (range 0 5) 0)          ;; 索引 → 0（先列表后下标）
(take 2 (range 0 5))         ;; 截取 → [0 1]
(concat (range 0 3) (range 3 5))  ;; 拼接 → [0 1 2 3 4]
(cons 1 2)                   ;; 二元有序对（同型二元构造）
```

Tisp 列表是**不可变**持久化数据结构（Rust `im` crate）。`range` 是最常用的列表构造器；自定义列表类型请见[第 02 章](02-types-and-patterns.md)。

### 布尔运算

```tisp
;; ✅ 可类型检查
(= 1 1)     ;; 相等比较 → true
(!= 1 2)    ;; 不等 → true
(< 1 5)     ;; 小于 → true
(and true false)  ;; → false
(or true false)   ;; → true
(not true)        ;; → false
```

### 示例 1：基本数据类型

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch01-datatypes.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch01-datatypes.tisp
(defn main []
  (println 42)
  (println "hello")
  (println (range 1 4))
  (println (cons 1 2))
  (println (and true false))
  (println (or true false)))
```

预期输出：
```
42
hello
[1 2 3]
(1 . 2)
false
true
```
（列表输出格式可能因 Tisp 版本略有不同）

---

## 1.5 变量与函数定义

### 用 `defn` 定义

```tisp
;; ✅ 可运行
(defn greet [] "Hello!")          ;; 0 参数函数
(defn add [x y] (+ x y))          ;; 普通函数
(defn sq [x] (* x x))             ;; 单参数函数
```

### 匿名函数（lambda）

```tisp
;; ✅ 可类型检查
(fn [x] (+ x 1))                  ;; lambda
((fn [x y] (+ x y)) 3 4)          ;; 立即调用 → 7
```

### `let` 局部绑定

```tisp
;; ✅ 可运行
(let [x 10
      y 20]
  (+ x y))  ;; → 30
```

`let` 绑定是**不可变的**——不能给 `x` 重新赋值（Tisp 无赋值语义）。

### 示例 2：函数与绑定

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch01-functions.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch01-functions.tisp
(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

(defn main []
  (let [x 5
        y 10]
    (println (factorial x))
    (println (+ x y))
    (println ((fn [a b] (- a b)) 100 7))))
```

预期输出：
```
120
15
93
```

---

## 1.6 控制流

### `if` 表达式

```tisp
;; ✅ 可运行
(if (> x 0)
  "positive"
  "non-positive")
```

`if` 是**表达式**（返回值），不是语句。两个分支必须有相同类型。

### `cond` 多路分支

```tisp
;; ✅ 语法格式
(cond (< n 0) "negative"    ;; test1 body1
      (= n 0) "zero"        ;; test2 body2
      (> n 0) "positive"    ;; test3 body3
      "unknown")            ;; 默认分支（必需，否则分支类型须为 Unit）
```

规则：
- 所有分支的类型必须统一
- 最后一项始终是默认分支（与 Rust 的 `else` 语义相同）
- 如果默认分支省略，默认为 `()`（Unit），此时其他分支类型也必须为 `Unit`

### `if-let` 与 `when-let`

```tisp
;; ✅ 可类型检查
(if-let [x (if true 42 0)]     ;; 将匹配结果绑定到 x
  x                             ;; 成功分支
  0)                            ;; 失败分支

(when-let [x true]              ;; 仅成功分支，失败返回 false
  x)
```

### 示例 3：控制流

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch01-control-flow.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch01-control-flow.tisp
(defn grade [score]
  (cond (>= score 90) "A"
        (>= score 80) "B"
        (>= score 70) "C"
        (>= score 60) "D"
        "F"))

(defn main []
  (println (grade 95))
  (println (grade 82))
  (println (grade 55)))
```

预期输出：
```
A
B
F
```

---

## 1.7 类型标注

Tisp 中类型部分由 HM（Hindley-Milner）算法推断，但也支持显式标注。

### 参数类型标注

```tisp
;; ✅ 可类型检查
(defn add [x : i64, y : i64] -> i64 (+ x y))
```

`[x : i64]` 标注 `x` 的类型为 `i64`（64 位有符号整数）。

### 返回类型标注

```tisp
;; ✅ 可类型检查
(defn add [x y] -> i64 (+ x y))      ;; 返回 i64
(defn id [x] -> a x)                  ;; 返回多态类型 a
```

### 随文标注 `(ann expr Type)`

```tisp
;; ✅ 可类型检查
(ann 42 i64)         ;; 标注表达式类型，运行时无操作——但提供静态类型信息
```

### 多态函数

```tisp
;; ✅ 可类型检查
(defn id [x] x)             ;; 自动推导为 a → a
(defn const [x y] x)        ;; 自动推导为 a → b → a
(defn compose [f g]
  (fn [x] (f (g x))))       ;; 自动推导为 (b → c) → (a → b) → a → c
```

### 示例 4：类型标注

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch01-typeann.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch01-typeann.tisp
(defn add [x : i64, y : i64] -> i64 (+ x y))
(defn id [x] x)
(defn compose [f g]
  (fn [x] (f (g x))))

;; 使用时可以查类型
(defn main []
  (println (add 40 2))
  (println (id 42))
  (println ((compose (fn [x] (+ x 1)) (fn [x] (* x 2))) 3)))
```

预期输出：
```
42
42
7
```

---

## 1.8 更多实用函数

### 字符串操作

```tisp
;; ✅ 可类型检查
(str-concat "hello" " world")     ;; 字符串拼接 → "hello world"
(str-len "hello")                 ;; 长度 → 5
(str-sub "hello" 1)               ;; 子串（从 1 起）→ "ello"
(str-split "a,b,c" ",")           ;; 按分隔符切分 → ["a" "b" "c"]
(str-join ["a" "b" "c"] ",")      ;; 连接 → "a,b,c"
```

### 算术函数

```tisp
;; ✅ 可运行
(+ 1 2 3 4)   ;; 求和 → 10
(- 10 3 2)    ;; 减法 → 5
(* 2 3 4)     ;; 乘法 → 24
(/ 10 3)      ;; 整数除法 → 3（i64 语义）
```

### 高阶函数

```tisp
;; ✅ 可类型检查
(map (fn [x] (* x 2)) (range 1 4))       ;; 映射 → [2 4 6]
(reduce + 0 (range 1 5))                 ;; 折叠 → 10
(filter (fn [x] (> x 2)) (range 1 5))    ;; 过滤 → [3 4]
```

---

## 练习

1. 编写函数 `greet [name]`，返回 `(str-concat "Hello, " (str-concat name "!"))`，并运行验证。
2. 使用 `cond` 编写 `sign [n]`，返回 `"positive"` / `"negative"` / `"zero"`。
3. 用 `map` 和 `reduce` 计算列表元素的平均值（提示：`(reduce + 0 xs)`，`xs` 可用 `(range 1 5)`）。
4. 写一个 `compose` 调用链：`(compose inc sq)` 先平方再加一，对 `3` 求值应得 `10`。

---

## 本章小结

- Tisp 构建：`cargo build --release` → `target/release/tisp`
- 基本执行：`--run`（运行） / `--typecheck`（类型检查）
- REPL：`:type` 查类型，`(exit)` 退出
- 字面量：整数 `42` / 浮点 `3.14` / 字符串 `"hello"` / 布尔 `true false` / 单位 `()`
- 定义：`(defn name [params] body)`，支持类型标注 `[x : i64]` 和返回标注 `-> i64`
- 绑定：`(let [x 1 y 2] expr)`
- 控制流：`if` / `cond` / `if-let` / `when-let`
- 集合：`list` / `cons` / `count` / `map` / `reduce` / `filter`
- 类型：自动 HM 推断 + 显式标注 `(ann expr Type)`

---

> 上一章: 无 | 下一章: [第 02 章 类型与模式匹配](02-types-and-patterns.md) | [返回目录](INDEX.md)