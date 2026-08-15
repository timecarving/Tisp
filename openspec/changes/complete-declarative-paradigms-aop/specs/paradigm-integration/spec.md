## ADDED Requirements

### Requirement: 范式副作用接入共享效应行

8 类范式与 AOP 的状态副作用 SHALL 经共享效应行执行：栈/状态机/数据驱动操作 SHALL 归属 `State`；基于流 SHALL 归属 `Signal`；数组/符号/自动机 SHALL 为 Pure；AOP 编织后的方法链 SHALL 保留 primary 方法原有效应行并追加切面声明的效应。`effect_infer` 结果 SHALL 与运行时行为一致，不得以设施简化投影绕过。

#### Scenario: 跨范式效应组合

- **WHEN** 程序组合栈编程 + 基于流编程，以 `--typecheck` 运行
- **THEN** 效应行同时包含 `State` 与 `Signal`，纯代码部分保持 Pure

#### Scenario: AOP 效应行合成

- **WHEN** around 切面声明 `State` 效应并包裹 Pure 方法，以 `--typecheck` 运行
- **THEN** 编织后方法链效应行为 `State`，与切面声明一致

### Requirement: 范式设施语义不能绕过

`pf-*` 设施别名与完整范式内置的语义 SHALL 一致：对同一输入 SHALL 产生同一结果与同一错误；任何简化投影（sum%2、+100、默认 0）SHALL 不得作为 `--run` 的公开正确性来源；调用设施别名 SHALL 与调用完整内置走同一效应/类型检查。

#### Scenario: 别名等价

- **WHEN** 同一程序分别经 `pf-*` 别名与完整内置执行同一范式操作
- **THEN** 结果、效应行与错误行为完全一致

#### Scenario: 简化投影失效

- **WHEN** 运行依赖旧简化投影语义（如 `pf-dfa-accept` 的 sum%2、`pf-aop-weave` 的 +100）的程序
- **THEN** 输出为完整语义结果（或对不再支持的形式显式报错），不静默给出旧投影值
