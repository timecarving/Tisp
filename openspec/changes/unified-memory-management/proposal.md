## Why

Tisp 已有线性类型(QTT `Grade` 0/1/ω)、分级线性类型(`□_r`/`@Cost`)、手动区域(`with-region`/`region-alloc`/`ptr-read`/`ptr-write` + `Unsafe` effect)与 State 效应引用(`Ref<T>`),但这些内存管理能力**分散在四处、互不贯通**:`Grade` 只约束函数参数,mop.rs 的 `Ref<T>` 是 Rust 级所有权演示(未接入 Tisp 等级系统),区域逃逸是独立的 region_infer pass,裸指针只靠 `Unsafe` effect 门控。本变更把它们统一到「线性 + 分级 + 手动 Unsafe」一套一致的等级/效应模型上,使「所有资源所有权都由 Grade 裁决、所有内存副作用都由代数效应/单子管理」的纯声明式承诺在内存层落地。

## What Changes

- **引用即分级值**:`Ref a` 引用 SHALL 作为一等分级值接入等级系统——`{1 r : Ref a}` 线性可变(写后不可复用)、`{ω r : Ref a}` 共享读、`{0 r : Ref a}` 编译期擦除;`ref`/`deref`/`set!` 建模为 State 效应操作,经 grade_check 检查所有权。
- **区域分级作用域**:区域分配 SHALL 受等级/作用域约束,`with-region` 退出后区域指针不可用(编译期逃逸检查 + 运行时悬垂检测统一)。
- **手动 Unsafe 逃逸**:`ptr-read`/`ptr-write` SHALL 经 `Unsafe` effect 门控,与等级系统一致——纯代码未经 handler 无法调用,1 级线性指针读写后不可复用。
- **纯声明式副作用**:所有内存操作(分配/读写/回收)SHALL 经代数效应/单子管理,单处理器路径走 §12.6 直接状态线程,保持引用透明。
- **统一约束求解**:等级/效应/区域 SHALL 由统一约束系统(共享约束图)检查,替换分散的独立 pass。

## Capabilities

### New Capabilities

- `unified-memory-management`: 线性 + 分级 + 手动 Unsafe 的统一内存管理模型(引用/区域/裸指针接入统一 Grade + EffectRow)。

### Modified Capabilities

- `type-system-extensions`: 分级模态 □_r/◇_ε 与资源代数声明(§11)扩展为「引用/区域亦为分级值」。
- `toolchain-and-macros`: 裸指针与手动区域(§26)扩展为「线性指针所有权 + 区域分级作用域」。
- `dependent-linear-types`: 依赖等级传播(r+s)扩展到引用/区域的所有权判定。

## Impact

- **tisp-core**:`Ref` 类型构造器 + `Grade` 与 `EffectLabel::State`/`Unsafe` 的统一语义;`Type` 扩展 `Ref a`/`Region a`。
- **tisp-middle**:grade_check 扩展引用/区域所有权(线性写消费、共享读);region_infer 与 grade_check 统一;effect_infer 接入 State/Unsafe 门控。
- **tisp-runtime**:mop.rs 的 `Ref<T>` 从 Rust 级所有权升级为接入 Tisp 等级的运行时引用;region.rs 与 effect.rs 统一。
- **tisp-backend**:interpreter 的 `ref`/`deref`/`set!`/`region-alloc`/`ptr-read` 接入统一等级/效应检查。
- **docs/spec.md** / **standard_doc** / **CHANGELOG**:教程系列文档重写,记录统一内存管理模型。
