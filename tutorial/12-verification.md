# 第 12 章 验证

## 目标

- 使用 `defprop` 声明程序属性
- 运行模型检查器（可达性搜索）
- 理解验证即 effect handler 的视角
- 使用 `--verify` CLI 与 `verify` 内置

---

## 12.1 属性声明 `defprop`

`defprop` 声明一个命名属性，值来自模型检查：

```tisp
;; ✅ 可验证  $ tisp --verify <file>
(defprop reachable-5
  (model-check 0
    (fn [n] (= n 5))              ;; goal：状态谓词
    (fn [n] [(+ n 1) (+ n 2)])    ;; next：状态 → 后继状态列表
    20))                          ;; 最大搜索深度
```

参数顺序：`(model-check init goal next max-depth)`

| 参数 | 含义 |
|------|------|
| `init` | 初始状态 |
| `goal` | 命中即 property holds 的谓词 |
| `next` | 转换关系（返回可达后继列表） |
| `max-depth` | 搜索深度上界 |

**不可达属性**：

```tisp
;; ✅ 可验证（expected: holds: false）
(defprop upper-bound-100-unreachable
  (model-check 0
    (fn [n] (> n 100))
    (fn [n] [(+ n 1)])
    10))
```

---

## 12.2 `--verify` 运行模型检查

```bash
$ tisp --verify examples/verify-user.tisp
; property reachable-5: (VerifyResult true 3 ["depth 0: 0" "depth 1: 1" "depth 2: 3" "5"]) (holds: true)
; property upper-bound-100-unreachable: (VerifyResult false -1 []) (holds: false)
; verification result: 2 property/properties checked
```

输出包含：
- 每个属性的验证结果 `VerifyResult holds depth trace`
- 命中路径的深度轨迹（depth 0/1/2 的中间状态）
- 属性总数

---

## 12.3 等价检查

对纯函数做等价性验证（结合 HoTT 的 `fun-ext` 与验证工具）：

```tisp
;; ✅ 可运行
(defn id [x] x)
(fun-ext id (fn [x] (+ x 0)) (range 0 10))  ;; 点态等价 → true
```

---

## 12.4 验证即 Effect Handler 视角

Tisp 设计哲学中，**验证器 = 探索所有路径的 effect handler**（`docs/spec.md` §28.5）：

- 搜索效应（`Search`）为验证器提供回溯能力
- `choose` 操作枚举候选状态
- 模型检查器以 handler 形式捕获搜索，探索可达状态空间

```tisp
;; ✅ 概念示例（search/choose 的效应语义）
(handle (search (member x (range 1 5)))
  (Search)
  (choose [xs] [k] (fold (fn [acc v] (or (k v) acc)) false xs)))
```

---

## 12.5 攻击重建（Attack Reconstruction）

`docs/spec.md` §28.4 规划对安全协议（spi-calculus）做攻击重建：当属性不成立时，验证器给出反例路径（攻击序列）。当前实现以 `VerifyResult false` + 轨迹的形式给出反例。

---

## 示例

```tisp
;; tutorial/examples/ch12-verification.tisp
;; ✅ 可验证  $ tisp --verify tutorial/examples/ch12-verification.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch12-verification.tisp
(defprop reachable-5
  (model-check 0
    (fn [n] (= n 5))
    (fn [n] [(+ n 1) (+ n 2)])
    20))

(defprop unreachable-100
  (model-check 0
    (fn [n] (> n 100))
    (fn [n] [(+ n 1)])
    10))
```

预期输出：
```
; property reachable-5: (VerifyResult true 3 ["depth 0: 0" "depth 1: 1" "depth 2: 3" "5"]) (holds: true)
; property unreachable-100: (VerifyResult false -1 []) (holds: false)
; verification result: 2 property/properties checked
```

---

## 练习

1. 用 `model-check` 验证：从 0 出发，每次 +1 或 +3，能否到达 10？写出属性并运行。
2. 修改 `next` 为每次 *2，验证从 1 能否到达 16（depth 上限 5）。
3. 将 `goal` 改为永远不满足的谓词，观察 `holds: false` 输出。
4. 阅读 `docs/spec.md` §28.5，用一段文字解释「验证器 = effect handler」。

---

## 本章小结

- `(defprop name (model-check init goal next depth))` 声明可检查属性
- `--verify` 运行模型检查并输出结果/轨迹
- 验证器按 effect handler 语义探索状态空间
- 反例路径是攻击重建的基础

---

> 上一章: [第 16 章 编译与工具链](16-compilation-and-toolchain.md) | 下一章: [第 13 章 八类编程范式](13-programming-paradigms.md) | [返回目录](INDEX.md)