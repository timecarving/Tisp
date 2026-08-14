## 1. 语言表面补齐(§6 定义 / §7 ADT)

- [x] 1.1 六维注解 `->[ε,ρ,@r,m,d]` 语法解析并贯通 `FunAnnotation`(desugar + type_infer)
- [x] 1.2 私有定义 `defn-`/`def-` 可见性写 `visibility` + ns 引用过滤
- [x] 1.3 `:deriving Ord` 生成 `ord-*` 结构排序函数(desugar)

## 2. 类型/效应/等级补齐(§11 分级模态 / §19 依赖等级)

- [x] 2.1 □_r/◇_ε 引入/消去推理接入 type_infer(unify 补 Modal 臂)
- [x] 2.2 `@Cost` 注解语法 + 渐近代价复合推导接入 grade_check
- [x] 2.3 依赖等级 r+s 对有限等级真实求解(替换对 ω 绑定恒过)

## 3. HoTT / Cohesive / 时序补齐(§16/§17/§18)

- [x] 3.1 完整立方填充:多维 Kan 填充(hott.rs)
- [x] 3.2 Cohesive 完整同伦模型:♭/♯/ʃ adjoint-triple 全语义(hott.rs)
- [x] 3.3 □_t 稳定类型语义保证:跨时刻检查 + 因果性/生产率/空间回收(temporal/type_infer)

## 4. 系统级补齐(§26 FFI / §30 编译指示)

- [x] 4.1 编译期区域逃逸检查(with-region 作用域逃逸判定,region_infer)
- [x] 4.2 inkwell 函数 define/call + 闭包环境打包/解包真代码生成(llc 验证)

## 5. 24 范式全链路接线(§31/§32)

- [x] 5.1 reader 识别 24 范式关键字 + desugar 生成范式 `CoreExprNode`
- [x] 5.2 type_infer/effect_infer 经 `ParadigmFacility` 接入范式类型/效应
- [x] 5.3 interpreter 经 `ParadigmRegistry::eval` 分发范式求值
- [x] 5.4 EVOLP/DLP/MOP 源码端到端 `--run`
- [x] 5.5 12 逻辑范式源码端到端 `--run`
- [x] 5.6 8 编程范式 + AOP 源码端到端 `--run`

## 6. 统一约束求解与演算统一(§2)

- [x] 6.1 六维统一约束求解收尾(共享约束图 + fixpoint)
- [x] 6.2 演算统一收尾(π/ρ/ambient/SKI 观察等价统一框架)

## 7. 文档与验证

- [x] 7.1 standard_doc ⚠️→✅ 升级 + file:line 证据更新
- [x] 7.2 docs/spec.md + CHANGELOG 同步
- [x] 7.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告验证
