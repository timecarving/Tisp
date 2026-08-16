# 附录 A2 内置函数速查

> 状态符号：✅ 完全可用（typecheck+run）/ ⚠️ 部分实现 / ⬜ 设计阶段。
> 所有签名以 `--typecheck` 实际推断为准。

## 算术

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `+` | `i64 -> i64 -> i64`（可变参） | 加法/求和 | ✅ |
| `-` | `i64 -> i64 -> i64` | 减法 | ✅ |
| `*` | `i64 -> i64 -> i64` | 乘法 | ✅ |
| `/` | `i64 -> i64 -> i64` | 整数除法 | ✅ |
| `mod` | `i64 -> i64 -> i64` | 取模 | ✅ |
| `pow` | `i64 -> i64 -> i64` | 幂 | ✅ |
| `abs` | `i64 -> i64` | 绝对值 | ✅ |
| `sqrt` | `i64 -> f64` | 平方根 | ✅ |
| `min` / `max` | `a -> a -> a` | 最小值 / 最大值 | ✅ |

## 比较

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `=` | `a -> a -> bool` | 结构相等 | ✅ |
| `!=` / `not=` | `a -> a -> bool` | 不等 | ✅ |
| `<` / `>` / `<=` / `>=` | `a -> a -> bool` | 大小比较 | ✅ |

## 布尔

| 函数 | 说明 | 状态 |
|------|------|------|
| `(and p q)` | 短路合取（特殊形式） | ✅ |
| `(or p q)` | 短路析取（特殊形式） | ✅ |
| `not` | 否定 | ✅ |

## 字符串

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `str` | `a -> String` | 任意值转字符串 | ✅ |
| `str-len` | `String -> i64` | 长度 | ✅ |
| `str-concat` | `String -> String -> String` | 拼接 | ✅ |
| `str-sub` | `String -> i64 -> String` | 子串 | ✅ |
| `str-split` | `String -> String -> List String` | 分割 | ✅ |
| `str-join` | `List String -> String -> String` | 连接 | ✅ |

## 集合（列表）

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `range` | `i64 -> i64 -> List i64` | 区间构造 | ✅ |
| `cons` | `a -> a -> a` | 二元构造 | ✅ |
| `first` | `List a -> a` | 首元素 | ✅ |
| `rest` | `List a -> List a` | 剩余部分 | ✅ |
| `reverse` | `List a -> List a` | 反转 | ✅ |
| `sort` | `List i64 -> List i64` | 排序 | ✅ |
| `count` / `length` | `List a -> i64` | 长度 | ✅ |
| `nth` | `List a -> i64 -> a` | 索引 | ✅ |
| `take` | `List a -> i64 -> List a` | 取前 n | ✅ |
| `drop` | `List a -> i64 -> List a` | 去掉前 n | ✅ |
| `zip` | `List a -> List b -> List (a, b)` | 配对 | ✅ |
| `concat` / `append` | `List a -> List a -> List a` | 拼接 | ✅ |
| `map` | `(a -> b) -> List a -> List b` | 映射 | ✅ |
| `filter` | `(a -> bool) -> List a -> List a` | 过滤 | ✅ |
| `reduce` | `(b -> a -> b) -> b -> List a -> b` | 左折叠 | ✅ |
| `foldl` | `(b -> a -> b) -> b -> List a -> b` | 左折叠 | ✅ |
| `foldr` | `(a -> b -> b) -> b -> List a -> b` | 右折叠 | ✅ |

## IO

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `println` | `a -> Unit`（IO） | 打印一行 | ✅ |
| `print` | `a -> Unit`（IO） | 打印不换行 | ✅ |
| `slurp` | `String -> String` | 读文件 | ✅ |
| `spit` | `String -> String -> Unit` | 写文件 | ✅ |
| `read-line` | `Unit -> String`（IO） | 读一行 | ✅ |

## 类型操作 / 反射

| 函数 | 说明 | 状态 |
|------|------|------|
| `type-of` | 表达式的静态类型 | ✅ |
| `reflect-type` | 定义的签名信息 | ✅ |
| `effects-of` | 定义的效应行 | ✅ |
| `grade-of` | 定义的等级 | ✅ |
| `mode-of` | 定义的模式 | ✅ |
| `determinism-of` | 定义的确定性 | ✅ |

## 效应操作

| 函数 | 签名 | 说明 | 状态 |
|------|------|------|------|
| `get` | `Unit -> s`（State） | 读状态 | ✅ |
| `put` | `s -> Unit`（State） | 写状态 | ✅ |
| `ask` | `Unit -> r`（Reader） | 读环境 | ✅ |
| `tell` | `w -> Unit`（Writer） | 写日志 | ✅ |
| `throw` | `e -> a`（Exception） | 抛异常 | ✅ |
| `choose` | `List a -> a`（Search） | 非确定选择 | ✅ |
| `search` | `bool -> Unit`（Search） | 搜索目标 | ✅ |
| `find-all` | `(Unit -> a) -> List a` | 收集全部解 | ✅ |
| `solve-all` | `goal -> List solution` | 全部解 | ✅ |

## 并发 / 时序 / 进程

| 函数 | 说明 | 状态 |
|------|------|------|
| `chan` | 创建通道 | ✅ |
| `send` / `recv` | 通道收发 | ✅ |
| `spawn` | 启动进程 | ✅ |
| `delay` | 时序延迟 | ✅ |
| `advance` | 推进时序值 | ✅ |
| `clock` | 时钟值 | ✅ |
| `always` / `eventually` | 时序模态 | ⚠️ |

## 编程范式（见第 13 章）

| 函数 | 说明 | 效应 |
|------|------|------|
| `array` | 创建数组 | Pure |
| `array-index` | 索引 | Pure |
| `array-sum-axis0` | 沿轴求和 | Pure |
| `stack-new` / `stack-push` / `stack-pop` / `stack-peek` | 栈操作 | State |
| `concatenate` / `point-apply` | 连接式组合 | Pure |
| `sym-num` / `sym-add` / `sym-eval` | 符号编程 | Pure |
| `dfa-accept` | DFA 识别 | Pure |
| `sm-drive` | 状态机驱动 | State |
| `table-new` / `table-dispatch` | 查表分发 | State |
| `stream` / `stream-take` / `stream-sink` | 惰性流 | Signal |

## 逻辑编程（见第 07 章）

| 函数 | 说明 |
|------|------|
| `fresh` | 引入逻辑变量 |
| `domain` | CLP 域约束 |
| `constrain` | CLP 约束 |
| `label` | CLP 标记 |
| `abduce` | 溯因 |
| `==` | 合一（谓词内） |

## 验证 / 代数

| 函数 | 说明 |
|------|------|
| `model-check` | 可达性模型检查 |
| `verify` | 运行属性 |
| `fun-ext` | 函数点态等价 |
| `monoid-check` | 幺半群检查 |

## HoTT / 区间

| 函数 | 说明 |
|------|------|
| `transp` | 端点传输 |
| `shape` | 路径连通 |
| `interval-neg` / `interval-and` / `interval-or` | 区间逻辑 |

## 其他

| 函数 | 说明 | 状态 |
|------|------|------|
| `gensym` | 生成唯一符号 | ✅ |
| `hash` | 加密散列（spi-calculus） | ⚠️ 占位 |
| `encrypt` / `decrypt` / `sign` / `verify` | 加密原语 | ⚠️ 占位 |
| `ptr-read` / `ptr-write` / `region-alloc` | 系统级 | ⬜ |

---

> 返回 [目录](INDEX.md)