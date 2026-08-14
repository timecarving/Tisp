## 1. 补齐 ⚠️ 特性到全链路可用

- [x] 1.1 分级模态 □_r/◇_ε 引入/消去推理(type_infer/unify 补 Modal 臂)
- [x] 1.2 Cost 注解 `@Cost` 语法 + 渐近代价全推导(desugar + grade_check)
- [x] 1.3 HoTT 完整立方填充(多维 Kan 填充,hott.rs)
- [x] 1.4 Cohesive 完整同伦模型(♭/♯/ʃ adjoint-triple 全语义)
- [x] 1.5 时序 □_t 稳定类型语义保证(因果性/生产率/空间回收)
- [x] 1.6 编译期区域逃逸检查(region_infer)
- [x] 1.7 inkwell 函数/闭包真代码生成(环境打包/解包)
- [x] 1.8 密码学真原语(ChaCha20/SHA-256 替换 XOR 占位)
- [x] 1.9 EVOLP/DLP/MOP 语言表面接线(语法/desugar/类型/interpreter)
- [x] 1.10 统一约束求解与演算统一收尾
- [x] 1.11 各补齐项单元测试

## 2. 8 类编程范式(组合优先)

- [x] 2.1 数组编程:多维数组类型 + 索引/切片/归约(映射/折叠/扫描)
- [x] 2.2 栈编程:数据栈 + 栈操作(压栈/弹栈/交换/复制/旋转,State effect)
- [x] 2.3 连接式编程:点自由组合子(compose/apply/branch)
- [x] 2.4 符号编程:符号 ADT + 代换/化简/模式匹配
- [x] 2.5 自动机编程:DFA/PDA 转移表 + 识别 + 组合(并/串/星)
- [x] 2.6 状态机编程:状态/事件/转移/动作(entry/exit/transition)
- [x] 2.7 数据驱动编程:查表/策略/解释器
- [x] 2.8 基于流编程:数据流网络(源/变换/汇)+ 惰性流
- [x] 2.9 各范式单元测试

## 3. AOP(基于编译器纯声明式 MOP)

- [x] 3.1 aspect/pointcut/advice 语法与脱糖
- [x] 3.2 编译器 MOP 编织(before/after/around → :before/:after/:around 方法组合)
- [x] 3.3 AOP 单元测试

## 4. 文档与收尾

- [x] 4.1 docs/spec.md 新增/扩展编程范式与 AOP 章节
- [x] 4.2 同步 standard_doc/ 与 CHANGELOG.md,更新实现状态
- [x] 4.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告验证
