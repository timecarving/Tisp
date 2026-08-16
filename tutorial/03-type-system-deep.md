# 第 03 章 深入类型系统

## 目标

- 掌握 QTT 定量类型理论：等级 0/1/ω 的语义与标注
- 使用区域系统与等级检查管理资源
- 理解依赖类型（Π/Σ）与类型族
- 使用液态类型（精化类型）与契约（requires/ensures）

---

## 3.1 QTT 等级：资源被使用几次

Tisp 是 **Quantitative Type Theory（QTT）** 语言：每个参数带一个**等级（grade/multiplicity）**，表示该值被使用的次数：

| 等级 | 含义 | 典型用途 |
|------|------|---------|
| `0` | 不可使用（编译期擦除） | 隐式参数、类型参数 |
| `1` | 恰好使用一次（线性） | 文件句柄、裸指针、资源 |
| `ω`（默认） | 任意次 | 普通数据 |
| `n`（自然数） | 最多 n 次 | 有界资源 |

### 基本标注语法

```tisp
;; ✅ 可类型检查
(defn use-once [{1 x : i64}] -> i64 x)      ;; 恰好一次
(defn use-any [x] x)                        ;; 默认 ω
(defn use-zero [{0 n : i64}] -> i64 0)      ;; 擦除参数（隐式）
(defn use-max3 [{3 x : i64}] -> i64         ;; 最多 3 次
  (do x x x))
```

- `{1 x : i64}` 声明 x 等级为 1（线性）
- 不带 `{}` 的参数默认 ω
- 等级也可为表达式：`{(+ n 1) y : i64}`

### 违反等级 → 编译错误

```tisp
;; ❌ 4 次使用 > 等级 3 → grade violation
(defn bad [{3 x : i64}] -> i64 (do x x x x))
```

### 依赖等级（等级由类型参数决定）

```tisp
;; ✅ 可类型检查
(defn use-n1 [xs : (Vec i64 n) ((+ n 1) y : i64)] -> i64 y)
```

---

## 3.2 等级检查示例

**线性参数使用后不可复用**：

```tisp
;; ✅ 通过：一次使用
(defn f [{1 p : i64}] -> i64 p)

;; ❌ 两次使用 → grade violation
;; (defn g [{1 p : i64}] -> i64 (+ p p))
```

**分支上界**：if 两分支使用次数取最大值。

```tisp
;; ✅ 通过：then 2 次、else 1 次，等级 3 满足上界 2
(defn branch [(3 x : i64)] -> i64
  (if true (do x x) x))
```

---

## 3.3 区域系统

Tisp 用 **区域（region）** 追踪内存分配位置，并在类型检查时执行逃逸检查。区域变量写作 `rhoN`，可显式标注：

```tisp
;; ✅ 可类型检查
(defn in-root [x : i64] -> [IO, rho1, @1, out, det] i64 x)
```

- 签名 `-> [effects, region, grade, mode, determinism] ReturnType`
- `rho1` 是预置的顶层区域
- `--typecheck` 输出会报告每个定义的 `regions` 列表

---

## 3.4 依赖类型（Π/Σ）

Tisp 支持依赖积（Π）与依赖和（Σ）：

```tisp
;; ✅ 可类型检查
;; Π 类型：返回类型依赖参数 n
(defn repeat-vec [n : i64] -> (Vec i64 n) ...)

;; Σ 类型：值连同其长度
;; (defn with-len [xs] -> (Sigma n (Vec i64 n)) ...)
```

`Vec i64 n` 表示长度为 n 的 i64 向量；`n` 由类型参数绑定。

---

## 3.5 液态类型（精化类型）

**液态类型**用谓词约束类型的取值范围（需要 z3 feature）：

```tisp
;; ✅ 可类型检查  $ tisp --typecheck（需 z3）
(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 x)
```

`{n : i64 | (>= n 0)}` 表示「满足 `(>= n 0)` 的 i64」。

**返回类型精化**：函数体所有路径必须满足返回谓词。

```tisp
(defn abs [x] -> {n : i64 | (>= n 0)}
  (if (>= x 0) x (- 0 x)))
```

**契约**：`requires`/`ensures` 声明前置/后置条件。

```tisp
(defn divide [n d]
  :requires (!= d 0)
  :requires (> d 0)
  (+ n d))

(defn add-positive [x y]
  :requires (> x 0)
  :requires (> y 0)
  :ensures (> result 0)
  (+ x y))
```

**违反示例**（预期 typecheck 失败）：

```tisp
;; ❌ (sqrt -1) 违反 (>= n 0)
(defn bad-call [] (sqrt -1))
```

---

## 3.6 类型族与类型一等值

**类型族**：

```tisp
;; ✅ 可类型检查
(typefamily Elem (List a) a (Pair b c) b)
(rewrite Elem (Map k v) k)
```

**类型一等值（Reader Principle）**：类型/效果/等级可在运行时反射。

```tisp
;; ✅ 可运行
(defn f [x : i64] -> i64 x)
(println (reflect-type f))       ;; 返回 f 的静态类型
(println (type-of 42))           ;; 返回表达式静态类型
(println (effects-of f))         ;; 效果行
(println (grade-of "use5"))      ;; 等级
(println (mode-of f))            ;; 模式
(println (determinism-of f))     ;; 确定性
```

---

## 示例

`tutorial/examples/ch03-type-system.tisp`：

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch03-type-system.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch03-type-system.tisp
;; QTT 等级：使用最多 3 次
(defn use3 [{3 x : i64}] -> i64
  (do x x x))

;; 分支上界 2 ≤ 等级 3
(defn branch [(3 x : i64)] -> i64
  (if true (do x x) x))

;; 六维注解
(defn annotated [x : i64] -> [IO, rho1, @1, out, det] i64 x)

;; 反射
(defn f [x : i64] -> i64 x)

(defn main []
  (println (use3 7))
  (println (branch 1))
  (println (annotated 42))
  (println (reflect-type f))
  (println (grade-of "use3")))
```

---

## 练习

1. 写 `swap-pair [{1 x : i64} {1 y : i64}]` 返回 `(+ x y)`，一次使用；尝试写两次使用版本观察报错。
2. 给 `fib` 加等级标注 `{2 n : i64}` 并解释为什么 2 足够（if 条件 1 次 + 递归实参 2 次，参考 `examples/fibonacci.tisp`）。
3. 声明一个资源代数 `(defresource-algebra Cost 0 + <=)` 并思考 `□_r` 的使用。
4. 给函数 `sqrt` 调用负实参，观察液态类型检查的报错（需要 z3 feature）。

---

## 本章小结

- 等级标注 `{r x : T}`；默认 ω；1 = 线性；0 = 擦除
- 区域：`rho1` 等区域变量参与逃逸检查
- 六维注解：`-> [effects, region, grade, mode, determinism] Ret`
- 液态类型 `{x : T | pred}` + `:requires`/`:ensures`（z3）
- 类型族 `typefamily`/`rewrite`；反射 `reflect-type`/`type-of`/`grade-of`

---

> 上一章: [第 02 章 类型与模式匹配](02-types-and-patterns.md) | 下一章: [第 04 章 效果系统](04-effect-system.md) | [返回目录](INDEX.md)