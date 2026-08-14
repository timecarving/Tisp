## 1. 统一所有权载体

- [x] 1.1 新增 `Type::Ref(Box<Type>)` 变体 + Display 臂(tisp-core/types.rs)
- [x] 1.2 引用 `ref`/`deref`/`set!` 建模为 `State` 效应操作(接入 EffectLabel::State)

## 2. 引用即分级值

- [x] 2.1 grade_check 扩展引用所有权:{1 r} 线性写后消费、{ω r} 共享读、{0 r} 擦除
- [x] 2.2 mop.rs 的 `Ref<T>` 从 Rust 级升级为 Tisp 等级运行时引用
- [x] 2.3 单元测试:线性引用写后复用报错、共享引用多次读通过

## 3. 区域分级作用域

- [x] 3.1 区域逃逸检查(region_infer 已补返回值逃逸)并入 grade_check 统一所有权
- [x] 3.2 运行时悬垂检测(freed_addrs)与编译期逃逸检查统一语义
- [x] 3.3 单元测试:区域逃逸报错、退出后悬垂报错

## 4. 手动 Unsafe 统一

- [x] 4.1 `ptr-read`/`ptr-write` 所有权并入 grade_check(1 级线性指针写后不可复用)
- [x] 4.2 Unsafe 门控(effect_infer)与等级检查(grade_check)输出进共享约束图
- [x] 4.3 单元测试:Unsafe 门控 + 线性裸指针

## 5. 统一约束求解收尾

- [x] 5.1 等级/效应/区域三检查串行聚合进共享约束图(solve.rs)
- [x] 5.2 跨维度冲突统一报告(带上下文)
- [x] 5.3 单元测试:引用等级违反 + 区域逃逸联合报告

## 6. 文档重写与 git

- [x] 6.1 重写教程系列文档(standard_doc/ 各篇:统一内存管理模型)
- [x] 6.2 docs/spec.md 同步 + CHANGELOG
- [x] 6.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告
- [x] 6.4 git 提交(追踪所有应追踪文件)
