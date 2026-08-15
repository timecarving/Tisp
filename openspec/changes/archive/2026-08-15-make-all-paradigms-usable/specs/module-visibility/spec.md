## ADDED Requirements

### Requirement: 命名空间别名限定引用

`ns` 的 `(:require [lib :as alias])` SHALL 建立限定命名空间：被加载模块中的公开定义 SHALL 可经 `alias/name` 引用；`alias` SHALL 隔离不同模块的同名定义；未声明别名时的直名引用行为保持不变；别名模块中的私有定义 SHALL 仍不可经别名引用。

#### Scenario: 别名限定调用

- **WHEN** 模块 B 以 `(:require [A :as a])` 加载模块 A 并调用 `(a/helper)`，以 `--run` 执行
- **THEN** 调用成功，返回 A.helper 的结果

#### Scenario: 别名隔离同名

- **WHEN** 模块 B 同时以不同别名加载定义同名函数的两个模块并分别经别名调用
- **THEN** 各调用解析到各自模块的定义，互不冲突

#### Scenario: 别名尊重私有边界

- **WHEN** B 经别名引用 A 中以 `defn-` 声明的私有符号，以 `--typecheck` 运行
- **THEN** 报告该符号不可见，不静默通过

### Requirement: 命名空间定义不产生可调用函数

`(ns name ...)` SHALL 仅声明模块边界与导入导出关系，SHALL 不注册名为 `name` 的可调用函数定义，也不影响入口 `main`/`__top__` 的解析。

#### Scenario: ns 无函数副作用

- **WHEN** 程序以 `(ns app (:require [lib]))` 开头且无 main，以 `--run` 执行
- **THEN** 报告无 main 函数，而非尝试调用名为 `app` 的定义
