# 第 13 章 八类编程范式

## 目标

- 逐一掌握 8 类编程范式的创建与使用
- 理解各范式的副作用归属（Pure / State / Signal）
- 掌握效应行声明的正确写法
- 区分合法与非法输入的错误行为

---

Tisp 提供 8 类编程范式作为**一等内置设施**，所有范式经统一的类型/效应/等级检查。

## 13.1 效应归属速览

| 范式 | 副作用归属 | REPL 提示符直接可用 | 典型操作 |
|------|-----------|--------------------|---------|
| 数组（Array） | **Pure** | ✅ | `array`/`array-index`/`array-sum-axis0` |
| 栈（Stack） | **State** | ❌ 需经带效应行 main + `--run` | `stack-new`/`stack-push`/`stack-pop`/`stack-peek` |
| 连接式（Concatenative） | **Pure** | ✅ | `concatenate`/`point-apply` |
| 符号（Symbolic） | **Pure** | ✅ | `sym-num`/`sym-add`/`sym-eval` |
| 自动机（DFA） | **Pure** | ✅ | `dfa-accept` |
| 状态机（State Machine） | **State** | ❌ 需经带效应行 main + `--run` | `sm-drive` |
| 数据驱动（Data-Driven） | **State** | ❌ 需经带效应行 main + `--run` | `table-new`/`table-dispatch` |
| 基于流（Stream） | **Signal** | ❌ 需经带效应行 main + `--run` | `stream`/`stream-take`/`stream-sink` |

> **重要**：使用 `State` 或 `Signal` 效应的入口，必须在签名中显式声明效应行：
> ```tisp
> (defn main [] -> [[State Signal], rho1, @omega, in, det] Unit ...)
> ```

---

## 13.2 数组编程

```tisp
;; ✅ 可运行（需 State Signal 效应声明于 main）
(array [2 2] [1 2 3 4])               ;; 创建 2×2 矩阵
(array-index (array [2 2] [1 2 3 4]) [1 1])  ;; 第 2 行第 2 列 → 4
(array-sum-axis0 (array [2 2] [1 2 3 4]))    ;; 沿轴 0 求和 → [4 6]
```

- 数组不可变（Pure 效应）
- 越界索引报错

---

## 13.3 栈编程

```tisp
;; ✅ 可运行（State 效应）
(stack-new)                              ;; 新建空栈
(stack-push (stack-new) 7)              ;; 压入 7
(stack-peek (stack-push (stack-new) 7)) ;; 查看栈顶 → 7
(stack-pop (stack-push (stack-new) 7))  ;; 弹栈
```

- 栈操作是纯函数（栈 → 栈）
- 空栈 pop/peek 显式报错
- 其他操作：`dup` / `swap` / `rotate`

---

## 13.4 连接式（点自由）编程

```tisp
;; ✅ 可运行（Pure 效应）
(defn f [x] (+ x 1))
(defn g [x] (* x 2))
(concatenate f g)                         ;; 串联 f∘g
(point-apply (concatenate f g) 3)         ;; (g (f 3)) → 8
```

- 无需显式参数传递
- `compose` / `apply` / `branch` 组合子

---

## 13.5 符号编程

```tisp
;; ✅ 可运行（Pure 效应）
(sym-num 1)                               ;; 符号常数 1
(sym-num 2)                               ;; 符号常数 2
(sym-add (sym-num 1) (sym-num 2))         ;; 构造 (+ 1 2)
(sym-eval (sym-add (sym-num 1) (sym-num 2)))  ;; 化简求值 → 3
```

- 构造 → 模式匹配 → 代换 → 化简 → 求值
- 含自由变量的求值报错

---

## 13.6 自动机编程（DFA）

```tisp
;; ✅ 可运行（Pure 效应）
;; DFA: 状态 0 --a--> 1 --a--> 0, 状态 0 接受
(dfa-accept 0 [0] [0 97 1 1 97 0] "aa")   ;; 接受 "aa" → true
(dfa-accept 0 [0] [0 97 1 1 97 0] "ab")   ;; 未声明符号 → 报错
```

参数：`(dfa-accept start-state accepting-states transitions input-string)`

- `transitions`：`[from char to ...]` 扁平三元组列表
- 未声明符号报错
- 自动机组合：并（union）/ 串（concat）

---

## 13.7 状态机编程

```tisp
;; ✅ 可运行（State 效应）
;; 状态 0 --1--> 1
(sm-drive 0 1 [0 1 1])                  ;; drive start target events → 1
```

参数：`(sm-drive initialState targetState eventList)`

- 事件驱动状态转移
- entry / exit 动作（当前实现：部分支持 ⚠️）
- 非法转移报错且状态不变

---

## 13.8 数据驱动编程

```tisp
;; ✅ 可运行（State 效应）
(table-new [1] [(fn [x] (+ x 1))])       ;; key=1 → action+1
(table-dispatch (table-new [1] [(fn [x] (+ x 1))]) 1 41)  ;; 查表 → 42
```

- `table-new keys actions`：创建查找表
- `table-dispatch table key arg`：按 key 查表并调用对应 action
- 缺失 key 报错（不再静默返回 None）

---

## 13.9 基于流编程

```tisp
;; ✅ 可运行（Signal 效应）
(stream 1)                                ;; 从 1 开始的递增流
(stream-take (stream 1) 5)                ;; 取前 5 个 → [1 2 3 4 5]
(stream-sink (stream 1) 5)                ;; 收集到 sink → [1 2 3 4 5]
```

- 惰性流水线（source → map → filter → take → sink）
- Signal 效应归入 FRP
- 无限流取前 n 不卡死

---

## 示例

`tutorial/examples/ch13-paradigms.tisp`：

```tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch13-paradigms.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch13-paradigms.tisp
(defn main [] -> [[State Signal], rho1, @omega, in, det] Unit
  ;; 数组
  (println (array-sum-axis0 (array [2 2] [1 2 3 4])))
  ;; 栈
  (println (stack-peek (stack-push (stack-new) 7)))
  ;; 连接式
  (println (point-apply
             (concatenate (fn [x] (+ x 1)) (fn [x] (* x 2)))
             3))
  ;; 符号
  (println (sym-eval (sym-add (sym-num 1) (sym-num 2))))
  ;; DFA
  (println (dfa-accept 0 [0] [0 97 1 1 97 0] "aa"))
  ;; 状态机
  (println (sm-drive 0 1 [0 1 1]))
  ;; 数据驱动
  (println (table-dispatch (table-new [1] [(fn [x] (+ x 1))]) 1 41))
  ;; 基于流
  (println (stream-sink (stream 1) 5)))
```

预期输出：
```
[4 6]
7
8
3
true
1
42
[1 2 3 4 5]
```

---

## 练习

1. 构造一个 2×3 矩阵，用 `array-index` 取第二行第三列元素。
2. 组合数组与流：先创建数组，再把沿轴求和的中间结果用流收集。
3. 声明一个 DFA 识别语言 `a*b*`，运行 `(dfa-accept ... "aab")` 与 `(dfa-accept ... "aba")`。
4. 修改 state machine 示例，用 `(sm-drive 0 2 [0 1 1 1 2])` 三步到达目标状态。

---

## 本章小结

- 8 范式各有专用内置函数；Pure / State / Signal 效应归属清晰
- 入口函数声明效应行：`-> [[State Signal], rho1, @omega, in, det] Unit`
- 非法输入（越界/未声明符号/非法转移/缺失 key）报错而不静默

---

> 上一章: [第 12 章 验证](12-verification.md) | 下一章: [第 14 章 AOP 面向切面编程](14-aop.md) | [返回目录](INDEX.md)