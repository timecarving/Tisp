# 02 — Tisp 高级特性

> 覆盖：效果系统 · 模式/确定性 · 区域 · HoTT · FRP · 逻辑编程 · 进程演算 · 宏 · OOP · 验证
> 状态符号：✅ 全链路可用 | ⚠️ 部分实现 | ⬜ 设计阶段(docs/spec.md)

---

## 1. Quantitative Type Theory (QTT)

### 1.1 等级(Grade)✅

```clojure
;; 等级:{0, 1, ω},用于线性资源检查
(defn linear-use [x : Int] -> Int  ; 默认 ω
  x)
```

- `Grade::Zero/One/Omega` 三种等级
- `grade_check` 在类型检查阶段校验线性变量使用(编译器实现于 tisp-middle/grade_check.rs)

### 1.2 依赖等级(depgraded)⚠️

- `depgraded.rs` 提供依赖等级演算的运行时骨架
- 与 QTT 的完整融合(编译期消除)仍在设计中

---

## 2. 效果系统 ✅

### 2.1 效果声明

```clojure
(defeffect State s
  (get [] -> s)
  (put [s] -> Unit))
```

### 2.2 效果处理器(handle)

```clojure
(defn run-state [init f]
  (handle (let [_ (put init)]
            (f))
    (State s)
    (get [] [k s] (k s s))
    (put [v] [k _s] (k Unit v))))
```

语义(§12.2)：
- `handle` 建立 handler 作用域(运行时 handler 栈)
- body 中的效果操作按 **操作名** 从栈顶向下分发到最近的 handler clause
- clause 的 `[k state]`：`k` 是续延闭包，`(k result new_state)` 把新状态写回 handler 状态槽并返回 `result`；单参 `(k v)` 直接返回 `v`(Search 等续延)

### 2.3 内置效果操作(§12.3)

| 操作 | 说明 |
|------|------|
| `get` / `put` | State：读/写可变状态 |
| `ask` / `tell` | Reader / Writer |
| `throw` | Except |
| `choose` | Search：选择候选值 |

无 handler 时调用报错 `perform <op> not in handler`。

### 2.4 效果行与子类型(§12.4/12.5)✅

```clojure
;; 效果行标注
g : (Int ->[{IO}] Unit)
h : (Int ->[{IO, State Int}] Bool)
```

`EffectRow::Pure/Closed/Open`、效果子类型(纯函数可用于效果上下文)在类型系统中实现。

---

## 3. 模式系统(Mercury 风格)✅

```clojure
;; :free 输出逻辑变量;:ground 输入
(defpred member [a :free, List a :ground] :nondet
  ([X [X . _]])
  ([X [_ . Xs]] (member X Xs)))
```

- `:free` → `Mode::Free`,`:ground` → `Mode::In`
- `:det` / `:nondet` 确定性注解
- 子句形式(§21.2)编译为参数打包 `__tuple` 的 Match + Search 包装(失败回溯返回 false)

---

## 4. 确定性分析 ✅

- `Determinism::Det/NonDet` 在 `determinism_analysis` 中推断
- `defpred` 默认 `NonDet`，普通函数默认 `Det`

---

## 5. 区域推断与运行时 ✅

- `RegionStack`(tisp-runtime/region.rs)：无 GC 的栈式区域分配
- `region_infer` 推断区域类型
- 每次 `--run` 结束输出区域统计：`; region stats: N allocs, ...`

---

## 6. 液态类型(Liquid Types)⚠️

```clojure
;; 精化类型(refinement type)
(defn safe-div [x : Int, y : Int] -> {Int | y != 0}
  (/ x y))
```

- `liquid_types.rs`、`holes.rs` 提供精化类型与洞(hole `?name`)的骨架
- Z3 集成(z3_bridge)为可选后端，无 z3 时跳过

---

## 7. Homotopy Type Theory (HoTT) ⚠️

- Interval 类型：`i0` / `i1` 端点、`interval-neg/and/or` 运算 ✅
- Path 类型、`~`(取反)、`FunExt` 节点存在 ⚠️
- HIT(defdata-hit)语法解析 ✅,运行时语义 ⚠️

---

## 8. 时序类型与 FRP(§18)✅

### 8.1 流(Stream)

```clojure
(stream 1)                    ; 从 1 开始的自然数流(惰性,步进 +1)
(stream-take (stream 1) 5)    ; => (1 2 3 4 5)
(advance s)                   ; 推进到下一时刻
(delay x)                     ; ⃝ A 的语义:值已是惰性结构
```

- 流由 `temporal::Stream<i64>` 支持(惰性 thunk)
- `⃝ A` 时态算子 desugar 为 `(delay A)`(§18.1)

### 8.2 构造器 `:::`(§18.2)

```clojure
(defdata (Stream a)
  (::: [a, (⃝ (Stream a))]))
```

### 8.3 信号(Signal)✅

```clojure
(signal-new 0)                ; 创建信号(返回 Signal 值)
```

- `Signal<T>` 由 `frp::Signal` 支持(可订阅/映射/折叠)
- `signal-new/map/filter/fold` 节点求值为**值管道**语义(立即求值,非响应式订阅)

### 8.4 时钟 ✅

```clojure
(clock)                       ; => "clock@1Hz"(占位)
```

---

## 9. 逻辑编程(§21)✅

### 9.1 谓词定义

```clojure
(defpred member [X Xs]
  ([X [X . _]])
  ([X [_ . T]] (member X T)))

(member 3 [1 2 3])            ; 成功 => ()
(member 9 [1 2 3])            ; 失败 => false(不报错)
```

子句形式三种写法：
1. `([P1 P2] body...)` 向量模式列表
2. `[([P1 P2])]` 向量包圆括号模式列表
3. `(P1 P2)` 圆括号模式列表

### 9.2 逻辑变量

```clojure
(fresh [x y z] goal...)       ; 多变量;嵌套 Fresh + Do
(== a b)                      ; unify
(search goal)                 ; 回溯边界(失败恢复 trail)
(commit! g)                   ; cut
```

- 约束存储 `ConstraintStore`(tisp-runtime/logic.rs)支持 unify/回溯/trail
- Search 节点失败时恢复 trail 并清理 choice point(不泄漏)

### 9.3 约束逻辑编程(CLP-FD,§21.5)✅

```clojure
(let [x (domain x 1 5)]       ; 声明域
  (label x 1)                 ; 标签搜索,解回绑 x(域升序第一个解)
  x)                          ; => 1
```

- `ClpStore`:Domain(BTreeSet 有序)/add_lt/add_eq/all_different/propagate/label
- `label` 枚举解并按升序返回(修复了无序域的问题)
- `constrain` 接受已求值为 true/false 的约束;对 **CLP 变量的算术约束编译**(如 `(> x 2)` 中 x 为变量 id)尚未实现 ⚠️

### 9.4 溯因(ALP,§21.6)⚠️

- `abduce` 节点接线 `AbductionEngine`(hypothesis 生成)
- 完整 ALP 搜索策略 ⬜

---

## 10. 进程演算与通信(§27)

### 10.1 通道(π-calculus)✅

```clojure
(let [c (chan)]               ; 创建通道
  (send c 42)                 ; 发送
  (recv c))                   ; => 42
```

- 接线 `ProcessRuntime`(共享 Arc<Mutex>,send/recv 经缓冲通道)
- `spawn` 子解释器共享通道运行时(§27.2 结构化并发)
- `send!`/`recv!` 为 session 协议操作(协议状态机)

### 10.2 加密(applied π,§27.4/27.5)✅

```clojure
(secret! "k1")                ; 声明密钥
(let [enc (encrypt "hello" "k1")]
  (decrypt enc "k1"))         ; => "hello"
```

- `encrypt/decrypt/sign/verify/hash` 接线 `CryptoEngine`
- **算法为 XOR/简单哈希占位**——生产环境应替换为 AES/ChaCha/SHA-256(代码注释已标注)

### 10.3 其他演算 ⚠️

- SKI 组合子、ambients(enter/exit/open)、ρ-calculus(quote/drop/lift)、κ-calculus(bind/unbind/react)节点存在并求值参数 ⚠️
- 完整语义 ⬜

---

## 11. 宏系统(§24)✅

```clojure
(defmacro unless [cond then]
  (if cond nil then))

(unless false 42)             ; => 42
```

- `defmacro` 注册宏表(desugar 阶段)
- 调用点展开:参数替换模板,递归 desugar(支持嵌套宏)
- 多表达式模板自动包 `do`
- **未实现**:syntax-quote、卫生宏(hygiene)

---

## 12. OOP 泛型函数(§22、§23)✅

### 12.1 泛型函数与方法

```clojure
(defgeneric area)

(defdata Shape
  (square [Int])
  (circle [Int]))

(defmethod area [(s square)] (* (nth s 0) (nth s 0)))
(defmethod area [(c circle)] (* 3 (nth c 0) (nth c 0)))

(area (square 4))             ; => 16
(area (circle 2))             ; => 12
```

- 方法模式 `(name Type)` 绑定整个值并匹配类型;`(Type)` 无绑定匹配
- 分发器运行时查 `generic_table`,按模式顺序匹配(§22.3 组合顺序 around→before→primary→after 已登记,分发取首个匹配)
- 无匹配报错 `no method for generic <name>`

### 12.2 类型类(§23)⚠️

- `defclass` / `definstance` 解析并登记 `instance_dict`
- 实例方法查找(约束求解驱动)⬜

---

## 13. 验证(Model Checking)✅

- `ModelChecker`(tisp-backend/process.rs):BFS 可达性验证,生成反例 trace
- CLI:`--verify <file>`

---

## 14. LLVM 代码生成(§30)⚠️

```bash
tisp --ir examples/run-test.tisp   # 生成文本 LLVM IR
```

- `IrGenerator` 生成文本 IR(无 inkwell 依赖):算术/if-phi/let
- 已修复:函数头语法、phi 寄存器一致性
- 真编译需 llvm 工具链(llc);函数调用/闭包生成 ⬜
