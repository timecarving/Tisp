## 1. 修正陈旧状态标记

- [x] 1.1 审计 §3/§5/§8/§20 实际代码,描述「齐」且实现存在即升 ✅(附 file:line)
- [x] 1.2 更新 §32 note(evolp/dlp/get-kb 已接线)
- [x] 1.3 清理 §3 已知运行时局限(TCO + 多顶层表达式递归已修)

## 2. 维度间 fixpoint 迭代收敛(§2)

- [x] 2.1 solve.rs 串行聚合升级为 fixpoint 循环(迭代至无新冲突 + 上限)
- [x] 2.2 单元测试:fixpoint 收敛 + 上限保护

## 3. □_r/◇_ε 引入消去等级推导(§11)

- [x] 3.1 type_infer 补 □_r 消去等级推导(可推断时推导 r,否则默认警告)
- [x] 3.2 type_infer 补 ◇_ε 效果行推导
- [x] 3.3 单元测试:□_r/◇_ε 推导

## 4. N 维立方 + Cohesive unit(§16/§17)

- [x] 4.1 interpreter HComp 扩展 ≥2 维 Kan 填充(复用 kan_fill_2d 泛化)
- [x] 4.2 Cohesive unit 语义(♯∘♭、♭∘ʃ 嵌入)
- [x] 4.3 单元测试:N 维立方组合 + unit

## 5. 时序因果性 + Z3 等级(§18/§19)

- [x] 5.1 时序因果性检查(输出仅依赖当前/过去)
- [x] 5.2 符号等级不等式交 Z3 求解(无 z3 降级)
- [x] 5.3 单元测试:因果性 + 符号等级

## 6. 数据流逃逸 + inkwell 闭包(§26/§30)

- [x] 6.1 region_infer 数据流逃逸(分配地址被 return/捕获则报逃逸)
- [x] 6.2 codegen inkwell 闭包环境打包(feature 门控)
- [x] 6.3 单元测试:数据流逃逸 + 闭包

## 7. 文档与验证

- [x] 7.1 standard_doc ⚠️→✅ 升级 + file:line 证据
- [x] 7.2 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告
