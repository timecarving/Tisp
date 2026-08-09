# 03 — Tisp 参考手册

> 覆盖：内置函数表 · CLI 参考 · 类型系统附录 · Core AST 附录 · 实现状态矩阵 · 示例程序
> 内置函数清单与 `crates/tisp-backend/src/interpreter.rs` 注册表一致(0.1.0)

---

## 1. 内置函数表

### 1.1 算术与比较(多参数折叠,至少 2 参;支持部分应用)

| 函数 | 说明 | 示例 |
|------|------|------|
| `+` `-` `*` `/` | 整数算术(折叠);`/` 除零报错 | `(+ 1 2 3)` => 6 |
| `<` `>` `<=` `>=` | 整数比较 | `(> 5 3)` => true |
| `=` `!=` `not=` | 值相等(Int/Bool/Str/Unit) | `(= nil nil)` => true |
| `mod` | 取模(除零报错) | `(mod 7 3)` => 1 |
| `min` `max` | 折叠取最值 | `(min 3 7)` => 3 |
| `abs` | 绝对值(饱和) | `(abs -5)` => 5 |
| `sqrt` | 平方根(Int/Float) | `(sqrt 9)` => 3.0 |
| `pow` | 幂(checked,溢出饱和) | `(pow 2 10)` => 1024 |

### 1.2 字符串

| 函数 | 说明 | 示例 |
|------|------|------|
| `str` | 拼接为字符串 | `(str "a" "b")` => "ab" |
| `str-len` | 字节长度 | `(str-len "hello")` => 5 |
| `str-concat` | 多字符串拼接 | `(str-concat "a" "b")` |
| `str-split` | 按分隔符拆分为列表 | `(str-split "a,b,c" ",")` |
| `str-join` | 用分隔符连接 | `(str-join "-" "ab")` |
| `str-sub` | 子串(UTF-8 安全) | `(str-sub "hello" 1)` => "ello" |

### 1.3 类型转换与反射

| 函数 | 说明 |
|------|------|
| `->string` | 值转字符串 |
| `i64->f64` | 整数转浮点 |
| `type-of` | 运行时类型名(i64/bool/...) |
| `grade-of` `mode-of` `effects-of` `determinism-of` | 静态注解查询(占位) |

### 1.4 IO

| 函数 | 说明 |
|------|------|
| `println` | 打印(可多参数) |
| `print` | 打印不换行(flush) |
| `read-line` | 读一行 |

### 1.5 列表

| 函数 | 说明 | 示例 |
|------|------|------|
| `cons` | 构造链表 | `(cons 1 (Nil))` |
| `first` `rest` | 头/尾 | `(first (range 1 5))` => 1 |
| `nth` | 第 n 个元素(Cons 链或任意构造器字段) | `(nth (range 1 5) 2)` => 3 |
| `take` `drop` | 取/丢前 n 个 | `(take (range 1 5) 2)` |
| `reverse` | 反转 | `(reverse (range 1 5))` => 4 3 2 1 |
| `sort` | 升序排序(Int) | |
| `count` `length` | 元素个数 | `(count (range 1 5))` => 4 |
| `range` | `[s, e)` 升序列表 | `(range 1 5)` => 1 2 3 4 |
| `zip` | 成对合并 | `(zip (range 1 3) (range 10 12))` |
| `concat` | 列表拼接 | `(concat (range 1 3) (range 3 5))` |
| `map` `filter` | 高阶映射/过滤 | `(map (fn [x] (* x 2)) (range 1 5))` |
| `reduce` `foldl` | 左折叠(3 参:fn init list) | `(reduce + 0 (range 1 5))` => 10 |
| `foldr` | 右折叠 | `(foldr + 0 (range 1 5))` => 10 |

### 1.6 布尔与 HoTT

| 函数 | 说明 |
|------|------|
| `not` `~` | 逻辑非 |
| `interval-neg` | 区间取反 |
| `interval-and` `interval-or` | 区间与/或 |

### 1.7 效果操作(§12.3)

`get` `put` `ask` `tell` `throw` `choose` — 经 handler 栈分发;无 handler 报错。

### 1.8 通道(§27)

| 函数 | 说明 |
|------|------|
| `chan` | 创建通道,返回通道名 |
| `send` | 发送值(2 参) |
| `recv` | 接收值(空通道报错) |

### 1.9 FRP(§18)

| 函数 | 说明 |
|------|------|
| `stream` | 惰性数值流(从 start 步进 +1) |
| `stream-take` | 取前 n 个为列表 |
| `advance` | 推进到下一时刻 |
| `delay` | 原样返回(惰性结构) |
| `clock` | 时钟占位("clock@1Hz") |

### 1.10 逻辑编程(§21)

| 函数 | 说明 |
|------|------|
| `fresh` | 创建逻辑变量(`(fresh x)` 或 `(fresh [x y] goal...)`) |
| `==` | 逻辑 unify |
| `search` | 回溯边界(失败返回 false 并恢复) |
| `commit!` | cut |

### 1.11 部分应用(柯里化)

多参内置可用单参调用获得部分应用：

```clojure
((+ 1) 2)         ; => 3
(map (fn [x] (* x 2)))  ; 部分应用,再传列表
```

---

## 2. CLI 参考

```
tisp [OPTIONS] [FILE]

  [FILE]          源文件
  -e, --eval <EVAL>      求值表达式
      --print-ast        打印 AST
      --print-tokens     打印 token 流
      --desugar          打印脱糖后的 Core AST
      --typecheck        运行类型推断
      --run              运行程序(解释执行)
      --verify           运行模型检查
      --ir               生成 LLVM IR 文本
      --compile          JIT 编译运行(需 llvm 特性)
```

示例：

```bash
tisp --run examples/hello.tisp      # 运行
tisp --desugar examples/adt-test.tisp  # 查看脱糖结果
tisp --typecheck examples/adt-test.tisp # 类型检查
tisp --ir examples/run-test.tisp    # 生成 IR
tisp                                # 进入 REPL
```

---

## 3. 类型系统附录

- 基本类型:`i8..i64/u8..u64/f32/f64/bool/String/Unit`
- 复合类型:`(Fun a b)`、`(App t1 t2)`、`(Tuple ...)`、`(Record ...)`
- 高级类型:`Refined`(液态)、`Path`(HoTT)、`Interval`、`Session`、`Modal`、`Temporal`、`Cohesive`、`Meta`
- 效果行:`Pure` / `Closed([EffectLabel])` / `Open(vec, Box<row>)`
- 等级:`Zero/One/Omega`;模式:`In/Free`;确定性:`Det/NonDet`

---

## 4. Core AST 附录(节选)

| 节点 | 说明 |
|------|------|
| `Lit/Var/Lam/App/Let/If/Do/Match/Data` | 核心表达式 |
| `Handle(body, Handler)` / `Perform(op, args)` | 效果系统(§12) |
| `Fresh/Unify/Search/Commit/Domain/Label/Constrain/AllDifferent/Abduce` | 逻辑编程(§21) |
| `ChannelNew/Send/Recv/Async*/Spawn/Join` | 进程演算(§27) |
| `CryptoEncrypt/Decrypt/Sign/Verify/Hash` | 加密(§27.4) |
| `SignalNew/Map/Filter/Fold` / `Delay/Advance/ClockNew` | FRP(§18) |
| `GenericDef/MethodDef/ClassDef/InstanceDef` | OOP(§22/§23) |
| `MacroDef/Comptime` | 宏/编译期(§24) |
| `NSDef/ExternDef` | 命名空间/FFI |

---

## 5. 示例程序

| 文件 | 运行结果 | 说明 |
|------|---------|------|
| `hello.tisp` | `Hello, Tisp!` | 顶层表达式 |
| `fibonacci.tisp` | `55` | 递归 + 顶层 println |
| `adt-test.tisp` | `just/nothing/3` | ADT + match |
| `advanced-test.tisp` | `43/120` | 高阶函数 + 闭包 |
| `run-test.tisp` | `Hello from Tisp!` / `42` | 综合 |
| `type-infer-test.tisp` | `43` | 类型推断 |
| `state-effect.tisp` | `3` | 效果系统(state) |
| `logic-test.tisp` | `OK` | 逻辑编程(fresh/==/search) |
| `frp-counter.tisp` | 定义型(无入口) | FRP 流(:::/⃝/advance) |
| `logic-search.tisp` | 部分支持 | Search effect + Mercury 自由变量 |
| `liquid-types-test.tisp` | 定义型(无入口) | 液态类型 |
| `phase5-test.tisp` | 定义型(无入口) | 洞/确定性 |
| `_qtt-test.tisp` | 定义型(无入口) | QTT |

> 定义型示例只含声明(defn/defdata),无 main 或顶层表达式,`--run` 报 `no main function` 属预期;可用 `--typecheck` 检查。

---

## 6. 测试

```bash
cargo test --workspace   # 105 个测试(0.1.0)
```

覆盖:词法/解析、脱糖、效果处理器、通道/流/加密、CLP、泛型分发、IR 生成、逻辑回溯。
