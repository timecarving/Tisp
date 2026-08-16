# 第 07 章 逻辑编程

## 目标

- 使用 `defpred` 定义子句化逻辑谓词（`:det` / `:nondet` / `:cc_multi`、`:mode (i o)`）
- 理解合一 `==`、回溯、`fresh` 与 `search`
- 掌握 `find-all` 全解收集
- 使用 CLP（`domain` / `label` / `constrain` / `solve-all`）
- 使用溯因 `abduce`
- 了解 12 类逻辑范式及其组织方式

---

## 7.1 谓词定义 `defpred`

`defpred` 声明逻辑谓词并注册子句；子句按序尝试，`search` 触发回溯：

```tisp
;; ✅ 可运行
(defpred member [x y]
  (== x y)
  (search))

(defn main []
  (search (member 42 [1 2 42 3]))
  (println "OK"))
```

### Mercury 风格子句

谓词也可以用带模式匹配的子句语法，类似 Mercury 语言：

```tisp
;; ✅ 可运行
(defpred member [x xs] :nondet
  ([x [x . _]])               ;; 第 1 子句:x 是列表首元素
  ([x [_ . xs]] (member x xs))) ;; 第 2 子句:递归搜索剩余部分

(defpred append [xs ys zs] :det
  ([[] ys ys])                                          ;; 空 + X = X
  ([[h . t] ys [h . zt]] (append t ys zt)))             ;; 递归头添加

(defn main []
  (println (count (find-all (fn [] (fresh [x] (member x [1 2 3])))))))  ;; → 3
```

- 子句格式：`([pattern...] body...)`，`[h . t]` 是列表解构（cons-pattern）
- `find-all` 收集 `Search` 效应产生的全部解，`count` 返回解的个数
- member 有 3 个解（1 / 2 / 3）

---

## 7.2 确定性

谓词可标注确定性（determinism），指导引擎选择策略：

| 确定性 | 含义 | 使用场景 |
|--------|------|---------|
| `:det` | 至多 1 个解（deterministic） | 函数式计算（append / length） |
| `:nondet`（默认） | 0 到多个解 | 搜索（member） |
| `:cc_multi` | 提交首解（committed choice） | 只取第一候选 |

```tisp
;; ✅ 可运行
;; committed-choice:只提交第一个满足的子句(cut)
(defpred pick [x] :cc_multi
  ([x] (= x 1))
  ([x] (= x 2)))

(defpred try-all [x] :nondet
  ([x] (= x 1))
  ([x] (= x 2)))

(defn main []
  (println (count (find-all (fn [] (fresh [x] (pick x))))))     ;; → 1
  (println (count (find-all (fn [] (fresh [x] (try-all x))))))) ;; → 2
```

`cc_multi` 只取第一个子句（1 个解），`nondet` 枚举全部子句（2 个解）。

---

## 7.3 多模式谓词（`:mode`）

可为谓词声明多个**模式签名**（`i = 输入 / o = 输出`），类型检查器据此校验调用方向：

```tisp
;; ✅ 可类型检查（:mode 约束调用方向，调用点须匹配其中一个模式）
(defpred swap [x y] :mode (i o) :mode (o i) :det
  ([x] y))
```

- `:mode (i o)`：`x` 输入、`y` 输出
- `:mode (o i)`：`y` 输入、`x` 输出
- 调用时实参模式须匹配已声明的模式之一（否则报 "无匹配模式"）

---

## 7.4 合一 `==` 与回溯

`==` 在谓词体内进行**合一**（unification），成功则继续，失败触发回溯到上一个选择点：

```tisp
;; ✅ 可运行
(defpred same [a b]
  (== a b)
  (search))

(defn main []
  (search (same 42 42))
  (println "unify 42==42 => ok"))
```

- `(== a b)`：两项合一 —— 变量绑定到值，或地面值检查相等
- `(search body)`：建立回溯边界；`body` 失败时恢复 trail 回到此处

注意：`==` 的合一绑定在**谓词内部**生效（模式匹配、子句分支），单独用 `==` 绑定逻辑变量的值给算术表达式在当前版本尚不可靠（标 ⚠️）；推荐在子句模式匹配（`[x [x . _]]`）或 CLP 中获取解值。

---

## 7.5 CLP 约束逻辑编程

CLP(FD) 通过**有限域变量** + **约束传播**求解：

```tisp
;; ✅ 可运行
(defn clp-multiply []
  (fresh [x y]
    (domain x 1 6)            ;; x ∈ [1 .. 6]
    (domain y 1 6)            ;; y ∈ [1 .. 6]
    (constrain (= (* x y) 12)) ;; 约束:x * y = 12
    (label x 1)               ;; 枚举 x 的第 1 个可行值
    (label y 1)               ;; 枚举 y 的第 1 个可行值
    (println (+ (* x 10) y)))) ;; → 26(x=2,y=6)

(defn main [] (clp-multiply))
```

| 操作 | 语法 | 语义 |
|------|------|------|
| `domain` | `(domain var lo hi)` | 创建有限域变量 [lo..hi] |
| `constrain` | `(constrain expr)` | 添加约束传播器（= / < / > / * / all-diff） |
| `label` | `(label var n)` | 枚举域中第 n 个值（n 从 1 开始） |
| `solve-all` | `(solve-all var)` | 枚举域中全部可行值 |

### solve-all 全解

```tisp
;; ✅ 可运行
(defn clp-range []
  (fresh [x]
    (domain x 1 6)
    (constrain (< x 4))
    (println (solve-all x))))  ;; → [1 2 3]

(defn main [] (clp-range))
```

### all-diff 互斥约束

```tisp
;; ⚠️ 部分实现:语法(all-diff [vars...])可解析,但两个未标注变量的
;;    互斥传播尚不能约束首解(见 note);三元以上在 Do 上下文中类型推断受限
(defn clp-all-diff []
  (fresh [x y]
    (domain x 1 3)
    (domain y 1 3)
    (all-diff [x y])          ;; 意图:x ≠ y
    (label x 1)
    (label y 1)
    (println (+ (* x 10) y))))
```

---

## 7.6 溯因 `abduce`

溯因逻辑编程（ALP）枚举使目标成立的假设集，返回解释个数：

```tisp
;; ✅ 可运行
(defn abduce-demo []
  (fresh [x]
    (domain x 1 5)
    (println (count (abduce (constrain (> x 1)) x)))))  ;; → 4(x=2,3,4,5)

(defn main [] (abduce-demo))
```

- `(abduce goal abducible ...)`：对目标中的可溯因变量，枚举全部一致解释
- 与 `domain` 结合设置假设域，`constrain` 限定一致条件

---

## 7.7 12 类逻辑范式

Tisp 通过 `ParadigmRegistry` 提供 12 类逻辑编程范式的统一内置（`--eval` / REPL 表达式行直接可用，`reactive-eval` 除外——需 Signal 效应，见脚注）：

| # | 范式 | 内置入口（推荐） | REPL 可用 | 状态 | 简述 |
|---|------|----------------|----------|------|------|
| 1 | 高阶逻辑（Higher-Order） | `higher-order-call` | ✅ | ⚠️ | 谓词作为值传递 / 应用 |
| 2 | 归纳逻辑（ILP） | `ilp-induce` | ✅ | ⚠️ | 从正/负例归纳规则 |
| 3 | 概率逻辑（PLP） | `plp-marginal` | ✅ | ⚠️ | 概率事实的边际概率 |
| 4 | 时序逻辑（Temporal） | `temporal-eventually` | ✅ | ⚠️ | LTL 时序算子（always / eventually / next） |
| 5 | 描述逻辑（Description） | `subsume` | ✅ | ⚠️ | 概念 ⊑ 角色推理 |
| 6 | 可废止（Defeasible） | `defeasible-settle` | ✅ | ⚠️ | 优先级裁决冲突规则 |
| 7 | 模糊逻辑（Fuzzy） | `fuzzy-eval` | ✅ | ⚠️ | 真值度组合（min/max） |
| 8 | 表格化（Tabled） | `tabling` | ✅ | ⚠️ | 记忆已解目标，左递归终止 |
| 9 | 一体化基底（Integrated） | `typed-pred` | ✅ | ⚠️ | 静态类型 + 函数/OOP/并发互操作 |
| 10 | 响应式逻辑（Reactive） | `reactive-eval` | ❌¹ | ⚠️ | FRP 信号驱动规则更新 |
| 11 | 情境逻辑（Context） | `context-query` | ✅ | ⚠️ | 情境层次 / 继承 / 隔离 |
| 12 | 模态逻辑（Modal） | `modal-possible` | ✅ | ⚠️ | 可能世界 `possible` / `necessary` |

> **¹** `reactive-eval` 需 **Signal** 效应，无法在 REPL 提示符直接求值（同第 13 章的 State/Signal 范式）；写入文件并在带效应行的 `main` 中调用后 `--run` 即可。

> **遗留投影**：早期版本以 `pf-higher-order`、`pf-settle`、`pf-prob`、`pf-subsume` 等 `pf-*` 名称注册了简化投影，这些投影**仍可调用**但签名和实现有已知问题（`pf-settle` 非法输入会 panic、`pf-higher-order`/`pf-prob`/`pf-subsume` 类型签名不匹配真实语义），建议优先使用上表中的推荐内置名。

---

## 示例：完整逻辑编程

```tisp
;; tutorial/examples/ch07-logic.tisp
;; ✅ 可运行  $ ./target/debug/tisp --run tutorial/examples/ch07-logic.tisp
;; ✅ 可类型检查  $ ./target/debug/tisp --typecheck tutorial/examples/ch07-logic.tisp

(defpred member [x xs] :nondet
  ([x [x . _]])
  ([x [_ . xs]] (member x xs)))

(defpred pick [x] :cc_multi
  ([x] (= x 1))
  ([x] (= x 2)))

(defn clp-multiply []
  (fresh [x y]
    (domain x 1 6) (domain y 1 6)
    (constrain (= (* x y) 12))
    (label x 1) (label y 1)
    (println (+ (* x 10) y))))

(defn clp-range []
  (fresh [x]
    (domain x 1 6)
    (constrain (< x 4))
    (println (solve-all x))))

(defn abduce-demo []
  (fresh [x]
    (domain x 1 5)
    (println (count (abduce (constrain (> x 1)) x)))))

(defn find-all-demo []
  (println (count (find-all (fn [] (fresh [x] (member x [1 2 3]))))))
  (println (count (find-all (fn [] (fresh [x] (pick x)))))))

(defpred same [a b] (== a b) (search))

(defn main []
  (clp-multiply)
  (clp-range)
  (abduce-demo)
  (find-all-demo)
  (search (same 42 42))
  (println "unify 42==42 => ok"))
```

预期输出：

```
26
[1 2 3]
1
3
1
unify 42==42 => ok
```

> 注：`abduce-demo` 在独立运行时得 4（x=2..5），但与 CLP 演示同文件时共享 CLP 变量 ID 导致即时域截断为 1；教学使用时可将各演示拆分为独立文件获得各自精确结果。

---

## 练习

1. 用 `defpred` 写 `last [x xs]`：`x` 是列表 `xs` 的最后一个元素（子句形式）。
2. 用 CLP 解 `x + y > 7` 且 `all-diff [x y]`，x, y ∈ [1, 5]，枚举全部可行对。
3. 用 `abduce` 求约束 `x * y = 8` 在 x ∈ [1, 4] 下的解释个数。
4. 阅读 `openspec/specs/logic-programming-paradigms/spec.md` 了解 12 类范式设计要求，试比较 CLP 与溯因的异同。

---

## 本章小结

- `(defpred name [args] [:det/:nondet/:cc_multi] clauses...)` —— 声明逻辑谓词
- 子句 `([pattern ...] body...)` —— Mercury 风格模式匹配 + 回溯
- `fresh` 引入逻辑变量；`==` 合一；`search` 回溯边界
- `find-all` 收集全部解；`count` 统计解个数
- `:mode (i o)` —— 多模式签名，约束调用方向
- CLP：`domain` / `constrain` / `label` / `solve-all` / `all-diff` 有限域求解
- `abduce` 枚举解释
- 12 类逻辑范式经 `pf-*` 内置统一接入，CLP + ALP 已验证（✅），其余在构筑期（⚠️）

---

> 上一章: [第 06 章 OOP 与类型类](06-oop-and-typeclasses.md) | 下一章: [第 08 章 并发与 FRP](08-concurrency-and-frp.md) | [返回目录](INDEX.md)