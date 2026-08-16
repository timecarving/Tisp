# 附录 A3 常用模式速查卡

## 定义速查

```tisp
;; 函数
(defn add [x : i64, y : i64] -> i64 (+ x y))

;; 私有函数
(defn- helper [x] (+ x 1))

;; 多态函数（自动推断）
(defn id [x] x)

;; lambda
(fn [x] (* x 2))

;; ADT
(defdata (Maybe a) (Nothing) (Just a))

;; GADT
(defdata (Expr a)
  (IntLit i64 -> (Expr i64))
  (Add (Expr i64) (Expr i64) -> (Expr i64)))

;; deriving
(defdata Color :deriving (Eq Ord Show) (Red) (RGB i64 i64 i64))
```

## 类型 / 效应 / 等级

```tisp
;; 参数类型
[x : i64]

;; 返回类型
-> i64

;; 等级（线性 / 上界 / 擦除）
{1 x : (Ptr a)}
{3 x : i64}
{0 n : i64}

;; 精化类型（z3）
{x : i64 | (> x 0)}

;; 六维注解
-> [IO, rho1, @1, out, det] i64

;; 液态契约
:requires (!= d 0)
:ensures (> result 0)
```

## 效应声明与处理

```tisp
;; 声明
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))

;; 处理
(handle body
  (State s)
  (get [] [k s] (k s s))
  (put [v] [k _s] (k Unit v)))

;; 执行
(perform (get))

;; monadic 风格
(mlet [x (get-m) _ (put-m (+ x 1))] (pure x))

;; 效应行声明（使用 State/Signal 时必填）
(defn main [] -> [[State Signal], rho1, @omega, in, det] Unit ...)
```

## 控制流

```tisp
(if test then else)

(cond t1 b1 t2 b2 default)

(match v
  pat1 e1
  pat2 e2
  _ default)

;; 守卫
(match n
  0 "zero"
  (when x (= x 42)) "answer"
  x "other")

;; or 模式
(match c
  (or Red Green Blue) "known"
  _ "unknown")
```

## 逻辑编程

```tisp
;; 谓词
(defpred member [x y]
  (== x y)
  (search))

;; 模式与确定性
(defpred length [xs n] :det ...)
(defpred pick [x] :cc_multi ...)

;; 搜索
(search (member 42 xs))
(find-all (fn [] (fresh [x] (p x))))

;; CLP
(fresh [x y]
  (domain x 1 6)
  (constrain (= (* x y) 12))
  (label x 1))

;; 溯因
(abduce (constrain (> x 1)) x)
```

## OOP / 类型类

```tisp
(defgeneric area [x])
(defmethod area [5] 50)
(defmethod area :around [x] (* 2 (call-next-method)))

(defclass Coll [c e] :fun-deps [(c -> e)]
  (elem [c] -> e))
(definstance (Coll i64 i64) (elem [x] x))
```

## 宏 / 元编程

```tisp
(defmacro add1 [x] (+ x 1))
(comptime (+ 1 2))
(comptime (set-kb [1 2]))
(get-kb)
(gensym)
```

## AOP

```tisp
(defgeneric area [x])
(defmethod area [5] 50)
(defaspect double-area (pointcut area [x])
  :around (* 2 (call-next-method)))
```

## 验证

```tisp
(defprop p
  (model-check 0
    (fn [n] (= n 5))
    (fn [n] [(+ n 1) (+ n 2)])
    20))
```

## FFI

```tisp
(defextern c-abs "abs" "libc.so.6")
(defextern c-sin "sin" "libm.so.6" :abi "f64->f64")
(defextern c-strlen "strlen" "libc.so.6" :abi "str->i64")
```

## 模块（见第 10 章）

```tisp
(ns my-app.core)
(ns my-app.main
  (:require [my-app.core :refer [f] :as core]))
```

---

> 返回 [目录](INDEX.md)