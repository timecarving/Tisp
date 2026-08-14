## 1. §17 态射级自然性

- [x] 1.1 一阶态射表示 `Morphism<A,B>`(函数 A→B 作为值)
- [x] 1.2 自然变换方块:unit/counit 对任意 f 的交换恒等式(♭(f)∘η = η∘f 等)
- [x] 1.3 单元测试:自然性方块交换

## 2. §30 inkwell 闭包堆分配

- [x] 2.1 codegen inkwell 层闭包环境堆分配 display 层(捕获环境打包 + 函数指针)
- [x] 2.2 llc 验证(llvm feature 构建)
- [x] 2.3 默认构建文本 IR 闭包标注(已做)保持

## 3. §26 跨区域/全局别名分析

- [x] 3.1 region_infer 地址流图覆盖跨区域/全局(RegionAlloc → 跨区域赋值/全局 → 逃逸点)
- [x] 3.2 单元测试:跨区域/全局别名逃逸

## 4. 文档与验证

- [x] 4.1 standard_doc ⚠️→✅ 升级 + file:line 证据
- [x] 4.2 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告
