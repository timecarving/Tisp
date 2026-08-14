# data-structures

## Purpose

定义 Tisp 数据结构的持久化语义(§4):List/Vector/Map/Set SHALL 为不可变持久化结构(HAMT,结构共享),quote SHALL 产生可运行时操作的数据值,使「所有数据结构不可变」的设计承诺全链路落地。

## Requirements

### Requirement: 持久化集合语义

Vector/Map/Set SHALL 为持久化结构(HAMT-based,结构共享):更新操作 SHALL 返回新结构且旧结构保持可用(结构共享,非全量复制);List SHALL 为持久化单链表(头插共享尾部)。运行时的 `Value` 表示 SHALL 使用持久化表示(替换 std 可变集合)。

#### Scenario: 结构共享更新

- **WHEN** 程序对 vector/map/set 执行 `conj`/`assoc` 更新并保留旧引用,以 `--run` 执行
- **THEN** 旧结构仍可访问且内容不变,新结构反映更新(持久化语义)

#### Scenario: 持久化 List 头插

- **WHEN** 程序对 List 执行 `cons` 并保留旧 List,以 `--run` 执行
- **THEN** 新旧 List 共享尾部,行为正确且旧 List 未被破坏

### Requirement: quote 产生数据

`quote`(或 `'`)SHALL 在运行时产生可操作的数据值(而非仅在解析期处理):quoted 表达式 SHALL 求值为 List/Symbol/数字等数据,可被绑定、传递、遍历与模式匹配。

#### Scenario: quote 求值为数据

- **WHEN** 程序绑定 `'(1 2 3)` 并遍历/取元素,以 `--run` 执行
- **THEN** 返回可操作的数据结构(元素 1/2/3),而非报错或仅解析期占位

#### Scenario: quote 数据可模式匹配

- **WHEN** 程序对 quoted 数据执行 `match` 模式匹配,以 `--run` 执行
- **THEN** 匹配按数据结构正确分支
