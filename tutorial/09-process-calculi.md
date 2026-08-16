# 第 09 章 进程演算

## 目标

- 理解 Tisp 中 8 种进程演算的实现状态与交互方式
- 掌握 SKI 组合子（`S`/`K`/`I` + `ski-app`/`ski-reduce`）
- 掌握 π-calculus `chan`/`send`/`recv` 与 `spawn`/`join`
- 了解 applied π / spi-calculus 的加密原语：`encrypt`/`decrypt`/`sign`/`verify!`/`hash`
- 了解 Safe Ambients（`ambient-new`/`enter`/`exit`/`open`）
- 了解 ρ-calculus（`rho-quote`/`rho-drop`/`rho-lift`）与 ς-calculus（`invoke`/`update!`）
- 了解演算互编码：`pi-to-ski` 等（⚠️ 运行时可用，无类型签名）
- 如实标注各实现状态

---

## 9.1 概览：8 种进程演算

Tisp 将所有进程演算视为 **Communication 效应族的特化**（`docs/spec.md` §27）。
下表给出每种演算的实现状态（数据源：`standard_doc/04-implementation-status.md`、`crates/` 源码）：

| 演算 | spec | 状态 | 说明 |
|------|------|------|------|
| SKI 组合子 | §27.1 | ✅ | `S`/`K`/`I` + `ski-app`/`ski-reduce` 完全可用 |
| π-calculus | §27.2 | ✅ | `chan`/`send`/`recv`（FIFO 缓冲通道 + Condvar） |
| Async π | §27.3 | ✅ | `async-send`/`async-recv`（分支通道类型） |
| applied π | §27.4 | ✅ | `encrypt`/`decrypt`/`sign`/`verify!`/`hash`（XOR 占位） |
| spi-calculus | §27.5 | ✅ | `secret!`/`commit!`/`check!` |
| Safe Ambients | §27.6 | ⚠️ | `ambient-new`/`enter`/`exit`/`open` 可用；`co-enter`/`co-exit`/`co-open` 未暴露 |
| ρ-calculus | §27.7 | ⚠️ | `rho-quote`/`rho-drop`/`rho-lift` 可用；完整通道通信语义待外接 |
| κ-calculus | §27.8 | ⚠️ | `KappaBind`/`KappaUnbind`/`KappaReact` AST 节点已定义；无表面语法（`complex`/`site`/`bind`/`unbind`/`react` 未接入 desugar） |
| ς-calculus | §27.9 | ✅ | `invoke`/`update!` 对象自分发可用 |
| 互编码 | §27.10 | ⚠️ | `pi-to-ski`/`async-to-pi`/`applied-to-pi`/`rho-to-pi`/`ambient-to-channel` 已接线；仅运行时（无类型签名，typecheck 报 `unbound variable`） |

---

## 9.2 SKI 组合子（§27.1）

SKI 组合子是 Tisp 的**闭包转换编译目标**（`S f g x = (f x) (g x)`、`K x y = x`、`I x = x`）。
它以三内置 `S`/`K`/`I` 加应用形式 `ski-app` 暴露：

```tisp
;; ✅ 可运行
(ski-app (ski-app (ski-app S K) K) 5)    ;; → 5  (S K K 5 = K 5 (K 5) = 5)
(ski-app (ski-app K 40) 2)               ;; → 40 (K 40 2 = 40)
(ski-app I 7)                            ;; → 7  (I 7 = 7)
```

> **注意**：`S`/`K`/`I` 是**裸符号引用**——不要写成 `(S)`（那会零参调用而返回 Unit）。`ski-app` 包裹二元应用，内部 `SkiApp` 节点求值后经 `apply` 分发。

`ski-reduce` 归约组合子项（单步）：

```tisp
;; ✅ 可运行
(ski-reduce (ski-app I 9))               ;; → 9
```

**演算互编码**`pi-to-ski` 将 π 通道操作编码为 SKI 项（⚠️ 仅运行时可用，无类型签名）：

```tisp
;; ⚠️ 仅运行时（typecheck 报 unbound）；drives 解释器内部测试
(pi-to-ski ["send:42" "recv"])           ;; → Vector[包含 42 的 SKI 编码]
```

---

## 9.3 π-calculus（§27.2/27.3）

通道是 Tisp 并发的基础（第 08 章已简介）：

```tisp
;; ✅ 可运行
(let [c (chan)] (send c 42) (recv c))    ;; → 42
```

- `chan`（0 参）→ `(Chan a)` 类型（推导为 `Chan i64`）
- `send` 两参 `[Chan a, a]` → `Unit`（多态 `a`）
- `recv` 一参 `[Chan a]` → `a`（空通道或已关闭通道均报错）
- 效应自动推导为 `Channel` + `Session`，无需在签名中手工声明

**Async π**（§27.3）另有两个非阻塞变体：

```tisp
;; ✅ 可运行
(async-send c 42)                        ;; 非阻塞发送
(async-recv c)                           ;; 从异步通道接收
```

---

## 9.4 Applied π 与 spi-calculus（§27.4/27.5）

Applied π 在通道操作上叠加**密码学原语**（`docs/spec.md` §27.4）：

| 操作 | 签名 | 语义 |
|------|------|------|
| `secret! key` | `a → Unit` | 注册密钥（XOR 占位） |
| `encrypt data key` | `a × Key → Cipher` | 加密 |
| `decrypt cipher key` | `Cipher × Key → a` | 解密 |
| `sign data key` | `a × Key → Signature` | 签名 |
| `verify! cv key` | `CryptoValue × Key → Bool` | 验签 |
| `hash data` | `a → CryptoValue` | 散列 |

```tisp
;; ✅ 可运行
(secret! "k1")                             ;; 注册密钥
(let [enc (encrypt "hello" "k1")]
  (println (decrypt enc "k1")))            ;; → "hello"
(println (verify! (sign "msg" "k1") "k1"))  ;; → true
(println (hash "hello"))                    ;; → (CryptoValue d218e905... hash)
```

> ⚠️ **密码学警告**：默认构建采用 XOR 与简单散列占位（非加密）。
> 解释器运行时会打印 `; warning: 密码学为 XOR/简单 hash 占位`。
> 生产环境应启用 `crypto` feature 替换为 AES/ChaCha/SHA-256。

spi-calculus（§27.5）额外暴露承诺/验证：

```tisp
;; ✅ 可运行
(secret! "k2")
(let [c (commit! "secret-message" "k2")]
  (println (check! c "secret-message")))  ;; → true
```

- `commit! msg key` 加密并返回十六进制摘要
- `check! commit msg` 用全部已注册密钥逐一加密 `msg`，比对摘要

---

## 9.5 Safe Ambients（§27.6）⚠️

Safe Ambients 将进程组织为有名称的嵌套空间（`ambient` 注册 → `enter`/`exit`/`open` 移动）：

```tisp
;; ✅ 可运行（部分实现）
(println (ambient-new room))              ;; → ambient-room
(println (enter "room" 42))               ;; → false（未注册的 ambient 返回 false）
(println (exit "room" 99))                ;; → 99（总是求值 body）
(println (open "room" 0))                 ;; 移除 ambient 注册
```

状态说明：
- `ambient-new` 以**裸符号**命名（如 `room`，不是字符串 `"room"`）
- `enter`/`exit`/`open` 的第一参求值为字符串（`"room"`）
- `co-enter`/`co-exit`/`co-open`（并发移动）在 spec 中声明但**未暴露为表面语法** ⚠️
- Ambient 注册存储在 `Interpreter.ambients`（`HashMap<Symbol, Value>`）

---

## 9.6 ρ-calculus（§27.7）⚠️

ρ-calculus（反射演算）让**进程**本身作为可传递值：

```tisp
;; ✅ 可运行
(rho-quote 42)                             ;; → (Rho 42) —— 包装值
(rho-drop (rho-quote 42))                  ;; → 42      —— 拆包
(rho-lift (rho-quote 1) (+ 2 3))           ;; → 5       —— 在引用环境中求值
```

- `rho-quote`（⌜P⌝）：将值包装为 `Rho` 标签
- `rho-drop`（⌊x⌋）：拆包 `Rho`，取出原始值
- `rho-lift`：在一参引用的上下文中求值二参（当前实现约化为直接求值二参）
- 完整通道通信 `rho-lift` 的通道语义待外接 ⚠️

---

## 9.7 ς-calculus（§27.9）✅

ς-calculus（对象演算）通过 `invoke`/`update!` 实现对象自分发：

```tisp
;; ✅ 可运行
(let [obj (update! 0 (fn [self] (+ 1 1)))]
  (println (invoke obj "_update")))        ;; → 2
```

- `update! obj closure`：为 `obj` 添加名称为 `_update` 的方法（返回值变为 `Object<HashMap<Symbol, Value>>`）
- `invoke obj method-name`：以方法名查找 `obj` 的方法表并以 `obj` 自身为参数调用

---

## 9.8 κ-calculus（§27.8）⚠️

κ-calculus（化学演算）的 `complex`/`site`/`bind`/`unbind`/`react` 在 AST 中定义为
`KappaBind`/`KappaUnbind`/`KappaReact` 节点，解释器有对应求值分支（默认行为：
`KappaBind` 绑定续延变量后求值 body→cont；`KappaUnbind` 求值一参 → 求值二参；
`KappaReact` 恒等求值）。但这些操作**无表面语法**——`complex`/`site`/`bind` 等
函数名未在 `desugar.rs` 中映射到对应节点。目前仅可通过 Core AST 直接构建测试 ⚠️。

---

## 完整示例

见 `tutorial/examples/ch09-process.tisp`（所有 ✅ 演算均通过 typecheck 与 run）：

```tisp
;; tutorial/examples/ch09-process.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch09-process.tisp
;; ✅ 可运行      $ tisp --run tutorial/examples/ch09-process.tisp
;; ⚠️ pi-to-ski/async-to-pi/applied-to-pi/rho-to-pi/ambient-to-channel 仅运行时（无类型签名）
;; ⚠️ κ-calculus(bind/unbind/react)无表面语法（仅 CoreExprNode）
```

预期输出：

```
42               ;; π
5 40 7           ;; SKI
hello            ;; applied π (decrypt)
true             ;; applied π (verify!)
(CryptoValue …)  ;; applied π (hash)
true             ;; spi
ambient-room     ;; ambient
false 99         ;; ambient
42 5             ;; ρ
2                ;; ς
42               ;; spawn
```

---

## 练习

1. 用 `S`/`K`/`I` + `ski-app` 组合出等价于 `(fn [x] x)` 的恒等函数（提示：`(ski-app (ski-app S K) K)`）。
2. 编写 `(defn ping-pong [c n])`：在通道 `c` 上 `send n`，然后 `recv` 等待回应；编写 `main` 创建一个通道，spawn 发送方然后接收。
3. 注册两个密钥 `"k1"`/`"k2"`，分别加密不同消息，验证 `decrypt` 用错密钥时返回 `"decrypt failed"`。
4. 用 `ambient-new` 注册一个 ambient `room`，然后用 `enter "room" 42` 验证 `enter` 返回 `false`（未注册），思考为什么 ambient 注册不匹配。

---

## 本章小结

- SKI：`S`/`K`/`I`（裸符号）+ `ski-app`/`ski-reduce`——组合子驱全体
- π-calculus：`chan`/`send`/`recv`（FIFO 缓冲通道，Session 效应自动推导）
- Async π：`async-send`/`async-recv` 分支通道
- applied π / spi：`encrypt`/`decrypt`/`sign`/`verify!`/`hash`（⚠️ XOR 占位）
- Ambients：`ambient-new`/`enter`/`exit`/`open`（⚠️ co-* 未暴露）
- ρ-calculus：`rho-quote`/`rho-drop`/`rho-lift`（⚠️ 完整通道语义待外接）
- ς-calculus：`invoke`/`update!` ✅
- κ-calculus：AST 节点存在，无表面语法 ⚠️
- 互编码：5 条编码 + trace-equivalence 运行时可用，无类型签名 ⚠️

---

> 上一章: [第 08 章 并发与 FRP](08-concurrency-and-frp.md) | 下一章: [第 10 章 模块与命名空间](10-modules-and-namespaces.md) | [返回目录](INDEX.md)