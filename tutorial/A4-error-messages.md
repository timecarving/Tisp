# 附录 A4 常见错误诊断

> 所有错误以 `--typecheck` 或 `--run` 的实际输出为准。本附录收集典型错误与修复建议。

## 1. 类型错误

### `cannot unify A with B`

**症状**：

```
Error:   × 约束求解冲突(跨维度上下文):
  │   [type] cannot unify Con(TypeCon { name: 'i64, ... }) with
  │   Con(TypeCon { name: 'bool, ... }) (span ...)
```

**常见原因与修复**：

| 原因 | 修复 |
|------|------|
| `if` 两分支类型不同 | 统一两个分支的返回类型 |
| 函数返回值与声明不符 | 修正 `-> Type` 或 body |
| 算术/比较操作数类型错误 | `(= 1 true)` → 两侧同为数值或布尔 |
| `cond` 分支与默认分支类型不同 | 令所有分支 + 默认分支类型一致 |

### `unbound variable: <name>`

**症状**：

```
Error:   × [type] unbound variable: ilp-induce (span ...)
```

**原因**：调用了未定义或不存在的函数/变量。

**修复**：
- 检查拼写（`str` 而非 `->string`；`range` 而非 `list`）
- 确认该函数是否属于特定 feature（如部分内置需 z3/llvm）
- 用 `grep -rn '"函数名"' crates/` 在源码中确认存在性

## 2. 效应错误

### `State 效应缺失`

**症状**：

```
Error:   × [effect] State 效应缺失:定义 counter 调用状态/信号类范式操作,但效应
行未声明或处理该效应(纯声明式副作用管理)
```

**原因**：函数使用了 `get`/`put`/`stack-*`/`sm-drive`/`table-*`/`set-kb` 等操作，但签名没有声明 `State`。

**修复**：在签名加效应行：

```tisp
;; ❌ 缺少效应行
(defn counter [] (get))

;; ✅ 声明 State
(defn counter [] -> [State] i64 (get))

;; ✅ 或使用完整六维注解
(defn main [] -> [[State Signal], rho1, @omega, in, det] Unit ...)
```

### `Signal 效应缺失`

使用 `stream`/`stream-take`/`stream-sink` 时同理——声明 `Signal`。

## 3. 等级错误（QTT）

### `grade violation`

**症状**：等级 1（线性）的值使用两次，或使用次数超过标注等级。

```tisp
;; ❌ {1 p} 用了两次
(defn bad [{1 p : i64}] -> i64 (+ p p))

;; ✅ 声明等级 ≥2
(defn ok [{2 p : i64}] -> i64 (+ p p))
```

### `unbound grade variable`

**症状**：等级变量未绑定（如 `{m x : i64}` 中的 `m` 没有来源）。

**修复**：让等级变量由类型参数绑定（如 `xs : (Vec i64 n)` 引入 `n`），或使用具体数字。

## 4. 模式匹配错误

### `match is non-exhaustive`

**症状**：

```
[type] match is non-exhaustive for type Color — missing constructors: [Blue]
```

**修复**：补全所有构造子，或加通配符 `_` 分支。

### `pattern constructor must be a symbol`

**症状**：在模式中使用了非符号构造子（如 `[head . tail]` 中缀模式）。

**修复**：使用构造子前缀模式 `(Cons h t)`；无参构造子在 `or` 模式中直接写名字 `(or Red Green)`。

## 5. 液态类型错误（需要 z3）

### 精化违反

**症状**：

```
[liquid] 调用违反参数精化 (sqrt -1): x 不满足 (>= n 0)
```

**修复**：调用点实参满足谓词；或给函数加 `:requires` 前置条件缩小调用域。

### 契约违反

`requires` 不满足 / `ensures` 不成立 → 检查调用点参数与函数体的所有返回路径。

## 6. 编译期错误

### comptime 求值失败

```
[comptime] 编译期求值失败: undefined-fn 未定义 (span ...)
```

**修复**：comptime 内只使用编译期可见的定义与纯表达式。

### 未启用 llvm feature

```
Error: 需要 llvm feature
```

**修复**：`cargo build --release --features llvm`（需要 LLVM 17 与 `LLVM_SYS_170_PREFIX=/usr/lib/llvm-17`）。

## 7. FFI 错误

### 符号缺失

```
[ffi] 符号 no-such-symbol 在 libc.so.6 中不存在
```

**修复**：检查符号名与库路径。

### 签名不匹配

```
[ffi] 参数/签名不匹配
```

**修复**：用正确的 `:abi` 签名（`i64->i64`/`f64->f64`/`str->i64` 等）。

## 8. 其他

### `no main function`

`--run` 要求文件定义 `main`；否则仅输出该提示（typecheck 仍可通过）。纯 `defprop` 文件用 `--verify` 而非 `--run`。

### `failed to read ...: No such file or directory`

文件路径错误——使用相对于当前工作目录的正确路径。

---

> 返回 [目录](INDEX.md)