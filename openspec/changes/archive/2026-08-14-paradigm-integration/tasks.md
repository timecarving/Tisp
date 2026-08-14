## 1. 接入框架(可接入接口)

- [x] 1.1 定义 `ParadigmFacility` trait(keyword/type_con/effects/eval)+ 范式注册表
- [x] 1.2 新增范式 `CoreExprNode` 变体(AutomatonDef/StateMachineDef/ArrayExpr/SymExpr/StackOp/DfaDef 等)
- [x] 1.3 范式 `Type::Con` 构造器 + `Value` 范式值扩展
- [x] 1.4 reader/desugar 识别范式关键字并脱糖为 `CoreExprNode`
- [x] 1.5 interpreter 经 `ParadigmFacility::eval` 分发求值

## 2. 12 逻辑范式接入

- [x] 2.1 高阶/归纳(ILP)/概率(PLP)实现 `ParadigmFacility` 并接线
- [x] 2.2 时序/描述/可废止实现 `ParadigmFacility` 并接线
- [x] 2.3 模糊/表格化(Tabled)/一体化基底实现 `ParadigmFacility` 并接线
- [x] 2.4 响应式/情境/模态实现 `ParadigmFacility` 并接线
- [x] 2.5 EVOLP(稳定模型/不动点)/DLP(动态稳定模型)/MOP(GetKB/SetKB)实现 `ParadigmFacility` 并接线

## 3. 8 编程范式接入

- [x] 3.1 数组/栈/连接式实现 `ParadigmFacility` 并接线
- [x] 3.2 符号/自动机/状态机实现 `ParadigmFacility` 并接线
- [x] 3.3 数据驱动/基于流实现 `ParadigmFacility` 并接线

## 4. AOP 接入

- [x] 4.1 aspect/pointcut/advice 语法 → 编织接入 OOP 方法组合(经编译器 MOP)

## 5. 端到端与文档

- [x] 5.1 各范式源码端到端 `--run` 测试(书写 → typecheck → 求值)
- [x] 5.2 docs/spec.md + standard_doc + CHANGELOG 同步集成架构与状态
- [x] 5.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告验证
