## 1. 概率/归纳/模糊/可废止(数值类)

- [x] 1.1 `plp-marginal` 内置:概率事实 + 边际概率(接线 paradigms::marginal)
- [x] 1.2 `ilp-induce` 内置:正/负例归纳(接线 paradigms::induce)
- [x] 1.3 `fuzzy-eval` 内置:真值度组合(接线 paradigms::fuzzy_and/fuzzy_or)
- [x] 1.4 `defeasible-settle` 内置:优先级裁决(接线 paradigms::settle)

## 2. 时序/情境/模态(结构化类)

- [x] 2.1 `temporal-eventually` 内置:时间索引事实 + eventually(接线 TemporalKb)
- [x] 2.2 `context-query` 内置:情境继承/隔离(接线 ContextKb::query)
- [x] 2.3 `modal-possible` 内置:可达世界 possible(接线 ModalKb::possible)

## 3. 高阶/一体化/响应式(函数类)

- [x] 3.1 `higher-order-call` 内置:谓词一等值 + call
- [x] 3.2 `typed-pred` 内置:静态类型谓词(一体化基底)
- [x] 3.3 `reactive-eval` 内置:信号驱动规则(接线 Signal + 派生)

## 4. 类型/效应接入

- [x] 4.1 type_infer 补 12 范式类型签名
- [x] 4.2 effect_infer 效应门控(State/Search/Signal,已注册)

## 5. 测试与文档

- [x] 5.1 各范式端到端测试(源码 → typecheck → run)
- [x] 5.2 standard_doc §31 状态更新 + CHANGELOG
- [x] 5.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告
