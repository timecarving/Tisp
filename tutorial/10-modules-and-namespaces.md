# 第 10 章 模块与命名空间

## 目标

- 使用 `ns` 声明命名空间
- 掌握 `:require`/`:refer`/`:as` 导入导出的三种组合
- 跨文件项目组织（`base_dir` 自动解析 `{mod}.tisp`）
- 理解 `defn-`/`def-` 私有定义的可见性规则
- 理解模块加载的防循环与 `:refer` 过滤机制

---

## 10.1 命名空间声明

`ns` 是 Tisp 的模块边界（`docs/spec.md` §25）：

```tisp
(ns my-app.core
  (:require [tisp.core :as core])
  (:require [my-app.utils :refer [helper]]))
```

语法规则：
- `ns` 后跟命名空间名（一个符号）
- 余下形态为 `(:keyword [vector ...])` 列表，按顺序解析
- 支持的 keyword：`:require`（导入模块）、`:refer`（过滤导入符号）
- `ns` 声明本身**不生成同名函数**——它只声明模块边界，不影响运行时变量

---

## 10.2 导入导出

### 10.2.1 `:require`（模块导入）

最简形式：直接导入模块，全部公开定义可用（需以模块名限定）：

```tisp
;; lib.tisp
(defn pub [x] (* x 2))
(defn- priv [x] (+ x 1))      ;; 私有——外模块不可见
(defn extra [x] (- x 10))     ;; 公开

;; main.tisp
(ns my.core (:require [lib]))
;; 调用: (lib/pub 5) → 10
;; 不可见: priv —— defn- 定义，外部引用 → unbound variable
;; 所有公开定义均可通过 lib/前缀访问
```

### 10.2.2 `:as`（别名）

导入时指定别名，引用时用 `别名/原名`：

```tisp
;; ✅ 可运行
(ns my.core (:require [lib :as l]))
(defn main [] Unit
  (println (l/pub 21)))        ;; → 42
```

### 10.2.3 `:refer`（选择性导入）

`:refer` 白名单过滤：**仅**列表中的符号被导入（同时保留别名与原名两种引用路径）：

```tisp
;; ✅ 可运行
(ns my.app
  (:require [lib :as l])
  (:refer [pub extra]))

;; 现在可用: l/pub (别名限定)  与 pub (原名直达)
;; extra 也以原名导入
;; lib 中的其他公开定义(如未在 :refer 中列出)被过滤
```

规则（`crates/tisp-frontend/src/desugar.rs`，已验证）：
1. 私有定义（`defn-`/`def-`）**总是**随模块导入，但在 `private_aliases` 中标记，外部引用被拒绝。
2. 若 `:refer` 列表非空，则只有列表中的公开定义被保留（其余跳过）。
3. 若提供了 `:as alias`，每个保留的定义额外生成一份 `别名/原名` 的副本。
4. 若既无 `:as` 也无 `:refer`：所有公开定义以原名导入（平铺到当前命名空间）。

---

## 10.3 跨文件项目组织

Tisp 编译器根据输入文件的**父目录**自动解析 `:require` 模块：

```
项目/
├── main.tisp          (ns my.app (:require [lib :as l]))
└── lib.tisp           (defn pub [x] …)
```

CLI 调用 `--typecheck main.tisp` 时：
1. `std::path::Path::new("main.tisp").parent()` → 当前目录
2. `:require [lib]` → 查找 `lib.tisp` → 读取源码 → desugar → 合并到当前程序
3. 防循环：已加载文件（`loaded_files`）跳过重复导入

**不需要**任何 `package.json` 或项目配置文件——文件系统即模块系统。

---

## 10.4 `defn-` 与私有定义

`defn-`（私有函数）和 `def-`（私有值）的行为：

```tisp
;; ✅ 可类型检查
(defn- internal [x] (+ x 100))

(defn public-api [x]
  (internal x))              ;; 同一文件内可用

;; 外部引用 (internal 5) → unbound variable（编译期拒绝）
```

实现原理（`desugar.rs`）：
- `defn-`/`def-` 设置 `Visibility::Private`
- 模块导入时 `private_aliases` 记录这些名字
- 其他文件的表达式 desugar 阶段对 `private_aliases` 中的名字返回 `VarLookupError`

---

## 10.5 完整多文件示例

项目结构：

```
tutorial/examples/
├── ch10-modules.tisp    ← 主模块
└── ch10-lib.tisp        ← 库
```

库文件：

```tisp
;; ch10-lib.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch10-lib.tisp
(defn square [x] (* x x))
(defn double [x] (* x 2))

(defdata (Maybe a) (Nothing) (Just a))

(defn safe-div [a b]
  (if (= b 0) (Nothing) (Just (/ a b))))

(defn- internal-helper [x] (+ x 100))  ;; 私有
(defn extra [x] (- x 10))              ;; 公开但不在 :refer 中
```

主模块：

```tisp
;; ch10-modules.tisp
;; ✅ 可运行      $ tisp --run tutorial/examples/ch10-modules.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch10-modules.tisp
(ns my-app.core
  (:require [ch10-lib :as lib])
  (:refer [square double safe-div]))

(defn- helper [x] (+ x 1))

(defn main [] Unit
  (do
    (println (lib/square 7))               ;; → 49
    (println (lib/double 21))              ;; → 42
    (println (square 6))                   ;; → 36（原名直达）
    (match (safe-div 10 2)
      (Nothing) (println "error")
      (Just v) (println (str-concat "ok: " (str v))))
    (println (helper 10))))                ;; → 11（本地私有）
```

运行输出：

```
49
42
36
ok: 5
11
```

预期类型检查输出包含导入符号、别名符号与私有符号（全部以 `lib/` 前缀命名）以及主模块自身定义。

---

## 练习

1. 创建一个项目：`math.tisp` 定义 `square`/`cube`，`main.tisp` 通过 `ns` 导入并调用全部三个函数。
2. 在上题的基础上，将 `cube` 设为 `defn-`，观察编译期错误。
3. 同时使用 `:as` 别名与 `:refer` 白名单，验证别名限定调用与原名直达两种调用路径。
4. 故意创建循环依赖（A require B，B require A），观察 Tisp 的防循环机制（第二次加载被跳过）。

---

## 本章小结

- `(ns name (keyword vector)…)` —— 声明命名空间与导入导出
- `:require [lib]` —— 导入模块全部公开定义
- `:require [lib :as alias]` —— 别名限定引用
- `:refer [f g]` —— 白名单过滤（配合 `:as` 时同时生成原名与别名副本）
- 跨文件自动解析：`base_dir = 入口文件父目录`，防循环
- `defn-`/`def-` 设置 `Visibility::Private`，外模块不可见

---

> 上一章: [第 09 章 进程演算](09-process-calculi.md) | 下一章: [第 11 章 FFI 与系统级编程](11-ffi-and-system.md) | [返回目录](INDEX.md)