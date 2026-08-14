## 1. N(≥3)维立方组合(§16)

- [x] 1.1 hcomp-nd:递归泛化 hcomp-2d 到 N 维(2N 个 (N-1) 维面 + 共享面一致性)
- [x] 1.2 单元测试:3 维立方组合 + 边界不一致报错

## 2. adjoint-triple 自然性(§17)

- [x] 2.1 自然变换方块:unit/counit 的组合恒等式(♭(f)∘η = η'∘f 等)
- [x] 2.2 单元测试:自然性条件

## 3. 空间回收(§18)

- [x] 3.1 ⃝(next)值两时刻后回收(时刻计数器 + 回收队列)
- [x] 3.2 单元测试:advance 两次后值回收(无泄漏)

## 4. 完整别名分析(§26)

- [x] 4.1 region_infer 地址流图(RegionAlloc → 绑定/分支/闭包捕获 → 逃逸点)
- [x] 4.2 单元测试:别名逃逸(跨赋值/闭包捕获)

## 5. inkwell 闭包堆分配(§30)

- [x] 5.1 codegen inkwell 层闭包环境堆分配 display 层(llvm feature 门控)
- [x] 5.2 默认构建文本 IR 闭包标注(已做)保持

## 6. 文档与验证

- [x] 6.1 standard_doc ⚠️→✅ 升级 + file:line 证据
- [x] 6.2 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告
