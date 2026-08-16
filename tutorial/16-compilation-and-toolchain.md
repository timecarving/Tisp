# 第 16 章 编译与工具链

## 目标

- 掌握全部 CLI flags 的功能与输出
- 理解 `comptime` 全链路：desugar 内联 → typecheck → run 不重复
- 使用编译指示（opt-level / inline! / specialize! / suppress-warning）

---

## 16.1 CLI Flags 全览

```bash
tisp [OPTIONS] [FILE]
```

| Flag | 功能 | 行为 |
|------|------|------|
| `--eval EXPR` | 单表达式求值 | 全链路：lex → desugar → typecheck → interpret → 输出 |
| `--print-ast` | 打印 S 表达式 AST | 输出 Spanned AST 树 |
| `--print-tokens` | 打印词法 tokens | tokens + 位置区间 |
| `--desugar` | 脱糖 + 打印 Core AST | 含 comptime 内联 + 宏展开 + AOP 编织后的 Core |
| `--typecheck` | 静态检查 | 类型/效果/等级/模式/确定性/区域/液态类型 七维检查 |
| `--run` | 解释执行 | typecheck 成功后再解释执行 |
| `--verify` | 模型检查 | 执行 `defprop` 中的 `model-check` 属性 |
| `--ir` | LLVM IR | 生成文本 IR（默认 feature 回退解释器 `ret i64 0`） |
| `--compile` | JIT 编译 + 运行 | 需要 `--features llvm`（默认构建报错或链接失败） |

**运行示例**：

```bash
# eval 单表达式
$ tisp --eval '(+ 1 2)'
=> 3

# 打印 tokens
$ tisp --print-tokens examples/hello.tisp
LParen @ 0..1
Ident("println") @ 1..8
Str("Hello, Tisp!") @ 9..23
RParen @ 23..24

# 脱糖查看 Core AST
$ tisp --desugar examples/hello.tisp
(def __top__ : ? = (Lam [] (App (Var 'println) (Lit (String "Hello, Tisp!")))))

# 类型检查
$ tisp --typecheck examples/hello.tisp
__top__ : Unit
...
; type checking passed

# 模型检查
$ tisp --verify examples/verify-user.tisp
; property reachable-5: holds: true
; property upper-bound-100-unreachable: holds: false
```

---

## 16.2 `--eval` 全链路求值

```tisp
$ tisp --eval '(+ 1 2)'       ;; 正常 → 3
$ tisp --eval '(+ 1 true)'    ;; 类型错误 → 非零退出码
```

`--eval` 完成读取 → 脱糖 → 静态检查 → 求值 → 打印结果的全链路，而非仅打印读取数量。

---

## 16.3 `comptime` 全链路

`comptime` 在 desugar 阶段完成编译期求值，内联结果到 Core AST；`--typecheck` 检查内联后的程序；`--run` 执行时**不再重复求值**。

```tisp
;; ✅ 可运行  $ tisp --run
(defn main [] -> [[State], rho1, @omega, in, det] Unit
  (println (comptime (+ 1 2))))  ;; → 3（编译期折叠）

;; 编译期 KB：side effects 只在编译期发生一次
(defn main [] -> [[State], rho1, @omega, in, det] Unit
  (do
    (comptime (set-kb [1 2]))   ;; 编译期执行，写入编译期 KB
    (println (get-kb))))        ;; 运行时 KB → []（编译期 KB 不泄漏到运行时）
```

**全链路验证**（均以 `--desugar`/`--typecheck`/`--run` 测试）：

| 阶段 | 行为 |
|------|------|
| `--desugar` | 输出内联后的 Core AST（`comptime (+ 1 2)` → `3`） |
| `--typecheck` | 检查内联后程序的全部维度 |
| `--run` | 执行内联后的程序，comptime 副作用不重复 |

---

## 16.4 编译指示

编译指示 **仅语法接受**——当前实现中部分为统计占位，真实优化行为待 `llvm` feature 完整化。

```tisp
;; ✅ 可类型检查（当前为语法占位）
(opt-level 2)                  ;; 控制优化级别 0-3
(inline! f)                    ;; 强制 f 内联
(specialize! map [i64] (List i64))  ;; 强制特化
(suppress-warning "grade")     ;; 抑制特定警告
```

> 当前实现状态：这些编译指示被 parser 接受但优化器不执行相应操作（`--typecheck` 输出 `optimizations: 0 inlined, 0 folded`）。真实语义正在按 `docs/spec.md` §30 推进。

---

## 16.5 LLVM 代码生成（需要 `llvm` feature）

启用 `llvm` feature 时的行为：

```bash
$ cargo build --release --features llvm

$ tisp --ir examples/run-test.tisp      # 生成真实 LLVM IR（含 call 指令、闭包环境）
$ tisp --compile examples/hello.tisp    # JIT 编译 + 运行
```

默认构建的 `--ir` 回退为最小文本 IR（`ret i64 0` 占位），`--compile` 尝试调用 llc/clang 链接。

---

## 示例

```tisp
;; tutorial/examples/ch16-toolchain.tisp
;; ✅ 可运行  $ tisp --run tutorial/examples/ch16-toolchain.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch16-toolchain.tisp

(defn main [] -> [[State], rho1, @omega, in, det] Unit
  ;; comptime 常量折叠
  (println (comptime (+ 1 2)))
  ;; comptime KB 写入（编译期执行一次）
  (comptime (set-kb [1 2]))
  ;; 运行时 KB（分离）
  (println (get-kb))
  ;; 基本 eval
  (println (+ 40 2)))
```

预期输出：
```
3
[]
42
```
（注意 `(get-kb)` 返回空列表 `[]`，因为运行时 KB 与编译期 KB 分离）

---

## 练习

1. 对比 `tisp --desugar` 与 `tisp --print-ast` 的输出差异，解释 S 表达式 AST 与 Core AST 的区别。
2. 编写一个含 `comptime (set-kb ...)` 的程序，用 `--desugar` 观察 `set-kb` 如何被内联，再用 `--typecheck` 确认编译期 KB 写入可见。
3. 运行 `tisp --verify examples/verify-user.tisp`，阅读输出找出哪些属性 holds、哪些不 holds，尝试修改深度值观察结果变化。
4. 运行 `tisp --eval '(+ 1 true)'`，观察类型错误报告。

---

## 本章小结

- CLI flags：`--eval`/`--print-ast`/`--print-tokens`/`--desugar`/`--typecheck`/`--run`/`--verify`/`--ir`/`--compile`
- `comptime`：编译期求值内联 → `--desugar` 可见 → `--run` 不重复
- 编译指示：语法接受，真实优化待 llvm feature 完整化
- LLVM：`llvm` feature 生成真实 IR/JIT；默认回退占位

---

> 上一章: [第 15 章 HoTT 与 deriving](15-hott-and-derived.md) | 下一章: [第 12 章 验证](12-verification.md) | [返回目录](INDEX.md)