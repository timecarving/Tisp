# 第 05 章 宏与元编程

## 目标

- 用 `defmacro` 定义宏，理解模板替代机制
- 理解卫生展开（hygiene）：宏内部绑定不会被调用方捕获
- 认识 `gensym` 与语法引号（syntax-quote）的当前实现限制
- 用 `comptime` 在编译期求值表达式并内联结果
- 操作编译期知识库（`get-kb` / `set-kb`）
- 识别并处理编译期错误

---

## 5.1 宏定义：`defmacro`

```tisp
;; ✅ 可运行
(defmacro add1 [x] (let [y (+ x 1)] y))

(defn main [] -> i64 (add1 10))
;; (add1 10) → 11
```

`defmacro` 在**脱糖期**展开：宏体作为模板，形参绑定到实参后进行替换，再交给编译器继续脱糖。

- 宏是纯语法变换，不产生运行时调用
- 模板中可使用任意表达式（`let` / `if` / 算术……）
- 宏可以调用其他宏

### 带 `if` 的宏

```tisp
;; ✅ 可运行
(defmacro max2 [a b] (if (> a b) a b))

(max2 3 7)  ;; → 7
```

### 嵌套宏展开

```tisp
;; ✅ 可运行
(defmacro square [x] (* x x))
(defmacro sum-sq [a b] (+ (square a) (square b)))

(sum-sq 3 4)  ;; → 25
```

---

## 5.2 卫生展开（Hygiene）

Tisp 宏**默认卫生**：模板中由宏引入的绑定名会被自动重命名（加唯一后缀 `_gN`），
避免与调用方的同名变量冲突。

```tisp
;; ✅ 可运行
(defmacro double-x [y]
  (let [x (* 2 y)]    ;; 宏内部引入 x
    x))

(let [x 100]
  (double-x 5))       ;; → 10，而非捕获外层的 x=100
```

脱糖后宏内部的 `x` 被重命名为 `x_g1`，与调用方作用域中的 `x` 相互隔离：

```
;; --desugar 可见：
(let [x 100]
  (let [x_g1 (* 2 5)] x_g1))
```

卫生覆盖以下绑定形式：

| 形式 | 卫生处理 |
|------|---------|
| `(fn [params] ...)` / `lambda` | 参数名加后缀 |
| `(let [name value] ...)` | 绑定名加后缀 |
| `(if-let [name v] ...)` / `when-let` | 绑定名加后缀 |
| `(match ...)` 模式变量 | 模式变量加后缀 ⚠️ |

> **⚠️ 已知限制**：当前实现中 `match` 出现在宏模板内时，构造子名（如 `Just`/`Nothing`）也
> 会被误加后缀（`Just_g1`），导致运行时 match 失败。因此**编辑宏模板时请避免使用 `match`**，
> 改用 `if` / `let` / `cond`。

---

## 5.3 `gensym` 与语法引号（syntax-quote）

### `gensym`

```tisp
;; ⚠️ 保留字，但当前 typecheck 未登记
(gensym "t")  ;; 解释器内置：返回唯一字符串 "g0"、"g1"……
```

`gensym` 是解释器内置函数，但在类型检查环境中**未注册**，因此在 `--typecheck` 下使用会报
`unbound variable: gensym`。由于 Tisp 的卫生展开是自动的，绝大多数宏并不需要手动 `gensym`。

### 语法引号 `` ` `` 与 `~x` / `~@x`

```tisp
;; ⚠️ 语法占位，当前实现不可类型检查
`(+ 1 ~x)      ; syntax-quote + unquote
`(list ~@xs)   ; unquote-splice
```

语法引号在 Tisp 中脱糖为 `(list ...)` / `(concat ...)` 的**运行时数据构造**，但 `list` 尚未登记
到类型检查环境，因此 `` `(...) ``、`~x`、`~@x` 目前**无法通过 `--typecheck`**。

**实践建议**：当前编写宏时使用普通模板表达式（如上文 `defmacro` 各例），不使用语法引号。
这是与经典 Lisp `defmacro` 的一个关键区别。

---

## 5.4 `comptime` 编译期求值

`comptime` 包裹的表达式在**编译期**（脱糖后的 ComptimePass）求值，结果作为字面量内联回代码，
运行时不重复执行：

```tisp
;; ✅ 可运行
(defn compiled-in [] -> i64
  (comptime (+ 100 200)))

(compiled-in)  ;; → 300（编译期折叠）
```

- 适用于常量预计算：算术、列表操作、纯函数调用等
- 编译期求值使用独立的解释器，支持 `set-kb` / `get-kb` 覆盖到编译期 KB

---

## 5.5 编译期知识库（`get-kb` / `set-kb`）

Tisp 维护一个**编译期知识库**（MOP），与运行时 KB 分离：

```tisp
;; ✅ 可运行
(defn main [] -> [State] i64
  (comptime (set-kb [7 8 9]))   ;; 编译期写入 KB
  (println (get-kb))            ;; 运行时读取 → []（两者分离）
  42)
```

要点：

- `(comptime (set-kb [1 2]))` 在编译期把 `[1 2]` 写入编译期 KB，内联为 `Unit`
- `(get-kb)` 返回 `(KB Unit)` 类型，直接 `println` 可见其内容
- 运行时 `get-kb` 返回运行时 KB（默认空 `[]`）；编译期写入的值不会泄漏到运行时
- 读写 KB 属于 `State` 类效应操作，函数签名需声明 `[State]`

---

## 5.6 编译期错误

`comptime` 中出现的错误在编译期即被报告，并带 `comptime` 上下文：

```tisp
;; ❌ 编译期报错
(defn main [] -> i64
  (comptime (no-such-fn 1)))
;; Error: comptime 求值失败: eval error: unbound variable: no-such-fn
```

由于 comptime 在编译期求值，任何未定义变量在编译阶段即失败，而非推迟到运行期。

---

## 5.7 完整示例

```tisp
;; tutorial/examples/ch05-macros.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch05-macros.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch05-macros.tisp

;; (完整代码见 tutorial/examples/ch05-macros.tisp)
```

运行输出：
```
11
300
[]
10
```

关键点：
- `(add1 10)` 宏展开为 `(let [y (+ 10 1)] y)` → 11
- `comptime (+ 100 200)` 编译期折叠 → 300
- `set-kb` 写入编译期 KB，运行时 `get-kb` 仍为 `[]`（隔离）
- `double-x 5` 的宏内部 `x` 被卫生重命名，不捕获调用方的 `x=100` → 10

---

## 练习

1. 定义宏 `square-macro [x]`，展开为 `(* x x)`，对比运行时函数 `square` 的行为。
2. 定义宏 `twice [body]`，把 `body` 执行两次（提示：`(do body body)`）。
3. 用 `comptime` 预计算 `(+ 1 2 3 4 ... 100)` 并内联，观察 `--run` 输出结果相同。
4. 写一个宏模板包含 `(let [x 1] ...)`，在调用方也绑定 `x`，通过 `--desugar` 观察 `x_g1` 重命名。
5. （探索）尝试在宏模板中使用 `match`，观察构造子被重命名导致的运行时 `match failure`，
   然后用 `if`/`cond` 改写修复。

---

## 本章小结

- `(defmacro name [params] template)` —— 脱糖期模板替换，纯语法变换
- 卫生展开：宏内部 `fn`/`let`/`match` 绑定自动加唯一后缀，避免变量捕获
- `gensym` / 语法引号 `` ` `` `~` `~@` 为保留特性，但当前 `--typecheck` 下不可用（⚠️）
- `(comptime expr)` —— 编译期求值并内联结果，错误在编译期报出
- `(get-kb)` / `(comptime (set-kb [..]))` —— 编译期 KB，与运行时 KB 分离
- 宏模板请用 `if` / `let` / `cond`，避免 `match`（构造子重命名限制）

---

> 上一章: [第 04 章 效应系统](04-effect-system.md) | 下一章: [第 06 章 OOP 与类型类](06-oop-and-typeclasses.md) | [返回目录](INDEX.md)