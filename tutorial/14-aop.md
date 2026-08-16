# 第 14 章 AOP 面向切面编程

## 目标

- 声明切面 `defaspect` 与切入点 `pointcut`
- 使用 `around` / `before` / `after` 三种建议
- 理解 `call-next-method` 方法链
- 掌握 comptime 编译期编织与 AOP 效应行合成

---

## 14.1 基础：泛型函数 + 方法

AOP 建立在 OOP 基础上（详见[第 06 章](06-oop-and-typeclasses.md)）。先来看基础：

```tisp
;; ✅ 可运行
(defgeneric area [x])            ;; 泛型声明
(defmethod area [5] 50)          ;; 对 5 的方法体 → 50
(area 5)                         ;; 调用 → 50
```

---

## 14.2 声明切面

```tisp
;; ✅ 可运行  $ tisp --run
(defgeneric area [x])
(defmethod area [5] 50)

;; defaspect：对 area 的 [x] 签名命中的点，执行 :around 建议
(defaspect double-area (pointcut area [x]) :around (* 2 (call-next-method)))

(area 5)  ;; → 100（around 翻倍了 primary 的 50）
```

语法：`(defaspect name (pointcut generic-fn params) :around body)`

**三种建议**：

| 建议类型 | 行为 |
|----------|------|
| `:around` | 包裹原方法，`call-next-method` 调用内层 |
| `:before` | 在主方法前执行，不改变返回值 |
| `:after` | 在主方法后执行，不改变返回值 |

---

## 14.3 方法链执行顺序

多个切面作用于同一方法时，执行顺序为：

```
around(注册序) → before → primary → after → ...返回 around
```

```tisp
;; ✅ 可运行
(defgeneric greet [x])
(defmethod greet [5] "hi")

(defaspect loud (pointcut greet [x]) :before (println "before!"))
(defaspect quiet (pointcut greet [x]) :after  (println "after!"))

(greet 5)  ;; 输出 before! → primary("hi") → after! → 返回 "hi"
```

---

## 14.4 `call-next-method` 方法链

`call-next-method` 在 `:around` 中调用内层链（可能是另一个 around 或 primary）：

```tisp
;; ✅ 可运行
(defgeneric compute [x])
(defmethod compute [x] (+ x 1))                            ;; primary: +1
(defaspect add10 (pointcut compute [x]) :around (+ (call-next-method) 10))  ;; +10

(compute 1)  ;; primary: 1+1=2 → around: 2+10=12
```

---

## 14.5 comptime 编译期编织

AOP 编织发生在**编译期**（desugar 阶段），编织后的方法链写入 Core AST，`--desugar` 可见：

```tisp
;; ✅ 可运行——
(defgeneric area [x])
(defmethod area [5] 50)
(defaspect double-area (pointcut area [x]) :around (* 2 (call-next-method)))

;; (area 5) 在 desugar 阶段被编织为直接调用 woven_area_1
;; 用 --desugar 查看：可见 __woven_area_1 定义
```

编译期编织意味着：
- `--desugar` 输出包含编织后的 `__woven_*` 方法
- 运行时无动态反射，调用性能与普通函数一致
- 编织失败报编译错误

---

## 14.6 AOP 效应行合成

切面的效应行影响编织后的方法链：

```tisp
;; ✅ 可类型检查
;; Pure 切面包裹 Pure 方法 → 编织后仍为 Pure
;; State 切面包裹 Pure 方法 → 编织后方法链含 State
(defgeneric read-data [x])
(defmethod read-data [x] x)  ;; Pure primary
(defaspect log-read (pointcut read-data [x]) :around
  (do (put (get))                    ;; 使用 State
      (call-next-method)))           ;; → 编织后含 State 效应
```

- 切面声明了 `State` 效应的方法链，入口必须声明相应效应行
- 纯切面保持 Pure

---

## 14.7 编译期 KB 与 AOP 协作

切面可与 comptime 编译期 KB 协作：

```tisp
;; ✅ 可运行
(defgeneric area [x])
(defmethod area [5] 50)

;; comptime 写入编译期 KB
(defn main [] -> [[State], rho1, @omega, in, det] Unit
  (do
    (comptime (set-kb [1 2]))   ;; 编译期执行一次
    (println (area 5))          ;; 100（编织生效）
    (println (get-kb))))        ;; []（运行时 KB 分离）
```

---

## 示例

```tisp
;; tutorial/examples/ch14-aop.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch14-aop.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch14-aop.tisp

(defgeneric area [x])
(defmethod area [5] 50)
(defaspect double-area (pointcut area [x]) :around (* 2 (call-next-method)))

(defn main [] -> [[State], rho1, @omega, in, det] Unit
  (do
    (comptime (set-kb [1 2]))
    (println (area 5))
    (println (get-kb))))
```

预期输出：
```
100
[]
```

---

## 练习

1. 定义一个 `greet` 泛型，为 `3` 写 primary 方法，声明 around 切面在前后添加 "Hello " 前缀。
2. 创建 before + after 切面打印日志，验证它们不改变 primary 返回值。
3. 使用 `--desugar` 查看 `ch14-aop.tisp` 的编织后输出，找到 `__woven_` 开头的定义。
4. 修改切面使用 `State` 效应操作，观察类型检查对入口效应行的要求。

---

## 本章小结

- `(defaspect name (pointcut fn params) :advise body)` 声明切面
- 执行顺序：around(注册序) → before → primary → after
- `call-next-method` 调用内层方法链
- 编织在编译期完成，`--desugar` 可见；运行时无反射
- 切面效应行影响编织后方法链——遵从 OOP 语义保持

---

> 上一章: [第 13 章 八类编程范式](13-programming-paradigms.md) | 下一章: [第 15 章 HoTT 与 deriving](15-hott-and-derived.md) | [返回目录](INDEX.md)