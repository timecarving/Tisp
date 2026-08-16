# 第 11 章 FFI 与系统级编程

## 目标

- 使用 `defextern` 调用 C 动态库函数
- 了解 ABI 签名（i64→i64 / f64→f64 / str→i64 / 指针透传）
- 理解裸指针与手动区域的概念（⬜ 设计阶段）
- 掌握 `Unsafe` 效应门控

---

## 11.1 外部函数声明 `defextern`

### 基本 FFI 调用

```tisp
;; ⚠️ 运行需 --features ffi 构建；默认构建 --typecheck 通过、--run 报「未启用 ffi feature」
(defextern c-abs "abs" "libc.so.6")
(defn main [] (c-abs -42))  ;; → 42（ffi feature 构建）
```

语法：`(defextern <tisp-name> "<c-symbol>" "<library>" [:abi "<signature>"])`

- `<tisp-name>`：Tisp 侧使用的函数名
- `"<c-symbol>"`：动态库中的符号名
- `"<library>"`：共享库路径（如 `"libc.so.6"`、`"libm.so.6"`）
- `:abi "<signature>"`（可选）：指定调用约定，用于类型检查分派

### ABI 签名

```tisp
;; ⚠️ 需 ffi feature —— 浮点签到浮点
(defextern c-sin "sin" "libm.so.6" :abi "f64->f64")

;; ⚠️ 需 ffi feature —— 字符串长度
(defextern c-strlen "strlen" "libc.so.6" :abi "str->i64")

;; 调用
(c-sin 0.5)       ;; → 约 0.479
(c-strlen "hello") ;; → 5
```

支持的 ABI 签名格式：`"<input>-><output>"`，其中 input/output 为 `i64` / `f64` / `str` / `ptr`。

### 签名不匹配报错

```tisp
;; ❌ sin 以 i64 签名声明 → 参数/签名不匹配
(defextern bad-sin "sin" "libm.so.6" :abi "i64->i64")
;; 调用时报告错误，不以错误 ABI 调用
```

### 符号缺失报错

```tisp
;; ❌ 库中无此符号
(defextern nope "no-such-symbol" "libc.so.6")
```

---

## 11.2 裸指针与手动区域（⬜ 设计阶段）

以下特性为 `docs/spec.md` §26.2-26.4 规划内容，当前实现未覆盖：

```tisp
;; ⬜ 设计阶段语法展示
(defn ptr-read [{1 p : (Ptr a)}] -> [Unsafe] a ...)
(defn ptr-write [{1 p : (Ptr a)}, {1 v : a}] -> [Unsafe] Unit ...)
(defn with-region [f : (Region -> [ε] a)] -> [ε] a ...)
(defn region-alloc [r : Region, {1 v : a}] -> (Ptr a) ...)
```

区域逃逸检查与 `Unsafe` 效应门控方案详见 `docs/spec.md` §26。

---

## 11.3 `Unsafe` 效应门控

系统级操作需要 `Unsafe` 效应声明。纯代码未经 handler 无法调用含 `Unsafe` 效应的函数。

```tisp
;; ⬜ 设计阶段（ptr-read 等尚未实现，Unsafe 效应门控待落实）
;; (defn safe-fn [x] -> [Unsafe] i64 (ptr-read x))
```

当前 `defextern` 调用的效应管理为运行时隐式处理，未来版本将统一纳入 `Unsafe` 效应检查。

---

## 11.4 feature 门控行为

- 默认构建（无 `ffi` feature）：`defextern` 声明可通过 `--typecheck`，但 `--run` 调用时报「未启用 ffi feature,无法加载动态库符号」
- 启用 ffi：`cargo build --features ffi` 后可通过 `libloading` 实际加载库并调用
- 无 `llvm` feature 时：`--ir` 回退为文本 IR（解释器输出），`--compile` 报「未启用 llvm feature」错误

---

## 示例

```tisp
;; tutorial/examples/ch11-ffi.tisp
;; ✅ 可类型检查  $ tisp --typecheck tutorial/examples/ch11-ffi.tisp（默认构建）
;; ⚠️ 运行需 ffi feature  $ tisp --run tutorial/examples/ch11-ffi.tisp
(defextern c-abs "abs" "libc.so.6")
(defextern c-strlen "strlen" "libc.so.6" :abi "str->i64")

(defn main []
  (println (c-abs -42))
  (println (c-strlen "hello")))
```

预期输出：
```
42
5
```

---

## 练习

1. 尝试声明 `(defextern s-sin "sin" "libm.so.6" :abi "f64->f64")` 并调用 `(s-sin 0.5)`，对比预期值 0.479。
2. 有意以错误 ABI（如 `"i64->i64"`）声明 `sin`，调用并观察报错。
3. 阅读 `docs/spec.md` §26.2-26.3 关于裸指针与手动区域的草案，写一段注释总结设计意图。

---

## 本章小结

- `(defextern name "sym" "lib" [:abi "sig"])` 调用 C 动态库
- ABI 签名：`i64`/`f64`/`str`/`ptr` 的组合
- 裸指针 / 区域 / Unsafe gating 为 ⬜ 设计阶段
- 当前默认构建直接支持 dlopen 加载

---

> 上一章: [第 10 章 模块与命名空间](10-modules-and-namespaces.md) | 下一章: [第 09 章 进程演算](09-process-calculi.md) | [返回目录](INDEX.md)