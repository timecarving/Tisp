## Context

Tisp 的内存管理能力已分散存在:`Grade`(0/1/ω/Nat)只约束函数参数(type-system-extensions §10);`Ref<T>`(mop.rs)是 Rust 级所有权演示,未接入 Tisp 等级;区域逃逸(region_infer)是独立 pass;裸指针靠 `Unsafe` effect 门控(toolchain-and-macros §26)。本变更把它们统一到「线性 + 分级 + 手动 Unsafe」一套等级/效应模型。动机见 proposal.md。

## Goals / Non-Goals

**Goals:**
- 引用/区域/裸指针三类内存能力全部接入统一 `Grade` + `EffectRow`,所有权由 grade_check 裁决。
- 内存副作用统一经代数效应/单子管理,保持纯声明式 + 引用透明。
- 等级/效应/区域由统一约束系统检查。

**Non-Goals:**
- 不引入新的等级范畴(复用既有 Zero/One/Omega/Nat/Add/Mul/Var/Custom)。
- 不改变既有 QTT/分级模态/区域的已 ✅ 语义。
- 不做真实 GC 或栈内存布局——本变更统一「所有权模型」,非「内存分配器实现」。

## Decisions

### D1: 统一所有权载体 = Grade + EffectRow

引用、区域、裸指针三类能力都落到既有 `Grade`(所有权/复用次数)与 `EffectRow`(副作用)上,不再各自为政:

```
所有权(线性/共享/擦除) → Grade(0/1/ω + Nat/Custom)
副作用(可观测效应)     → EffectRow(State / Unsafe / ...)
作用域(存活范围)       → Region(区域) + grade_check 逃逸判定
```

**理由**:四个抽象已贯穿全链路,统一到它们即天然获得类型/效应/等级检查。
**备选**:新增 `MemoryKind` 平行体系(被否——无法跨范式组合,重复造轮子)。

### D2: 引用即分级值

把 mop.rs 的 `Ref<T>` 从 Rust 级所有权升级为「Tisp 分级值」:新增 `Type::Ref(Box<Type>)`;`ref`/`deref`/`set!` 建模为 `State` 效应操作;`{1 r : Ref a}` 线性(写后消费句柄)、`{ω r}` 共享读、`{0 r}` 擦除,由 grade_check 检查。
**理由**:引用所有权与参数所有权共用一套 Grade,兑现「引用即分级值」。
**备选**:独立 `Ref` 效应(被否——State 已覆盖,独立效应徒增行复杂度)。

### D3: 区域分级作用域

区域分配/回收接入统一检查:`with-region` 创建作用域、`region-alloc` 分配、退出回收;编译期逃逸检查(region_infer 已补「返回值逃逸」)与 grade_check 合并;运行时悬垂检测(已补 `freed_addrs`)保留。
**理由**:区域存活范围是所有权的一部分,应受统一等级约束。
**备选**:区域继续独立 pass(被否——违背「统一」目标)。

### D4: Unsafe 与等级一致

`ptr-read`/`ptr-write` 保持 `Unsafe` effect 门控,但所有权并入 grade_check:1 级线性裸指针写后不可复用,`Unsafe` 是声明式逃逸口(经 handler),非命令式旁路。
**理由**:Unsafe 不绕过等级系统,是「受控逃逸」而非「关闭检查」。

### D5: 副作用统一经代数效应/单子

所有内存操作(分配/读写/回收)声明于效应行,单处理器路径走 §12.6 直接状态线程降级;引用透明由效应行消减保证。
**理由**:兑现「效应是万能胶」,内存副作用与既有效应共享检查。

## Risks / Trade-offs

- [Ref 接入 HM 推断复杂] → `Ref a` 用 `Type::App(Con(Ref), a)` 具体类型 + grade_check 所有权,不做 rank-n 引用推断。
- [区域/等级/效应三检查合并复杂] → 先串行聚合进共享约束图(solve.rs),fixpoint 迭代作为后续。
- [mop.rs `Ref<T>` 升级破坏既有测试] → 保留 Rust 级 `StateRuntime` 作为运行时后端,新增 Tisp 等级层,既有测试继续绿。
- [Unsafe 门控与等级检查交互] → Unsafe 门控在 effect_infer,所有权在 grade_check,两 pass 输出进共享约束图统一报告。

## Migration Plan

增量:①新增 `Type::Ref` + `ref`/`deref`/`set!` 的 grade 所有权 → ②区域逃逸并入 grade_check → ③Unsafe 所有权统一 → ④统一约束求解收尾 → ⑤文档重写 + git。每步 `cargo test --workspace` 全绿、`cargo check --workspace` 零警告。

## Open Questions

- 引用是否支持区域标注(`Ref a` 带 `Region` 维度):可延后,初版 `Ref a` 不带区域维,区域独立于引用。
- `Ref` 的 0 级擦除是否真正从闭包环境剥离:可延后,初版类型级检查,运行时擦除与既有 QTT 0 级一致。
