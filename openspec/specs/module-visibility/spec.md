# module-visibility

## Purpose

为 Tisp 的模块与定义系统补齐可见性语义:私有定义(§6.5)与命名空间导入导出(§25.2)之间的边界,使跨文件加载遵循明确的公开/私有契约。

## Requirements

### Requirement: 私有定义可见性

`defn-`/`def-` 声明的定义 SHALL 为命名空间私有:同命名空间内 SHALL 可正常引用,其他命名空间经 `ns` 加载 SHALL 无法引用该私有定义;引用私有定义 SHALL 为编译错误(未定义或不可见)。`defn`/`def` 声明 SHALL 保持公开。

#### Scenario: 私有定义同空间可用

- **WHEN** 命名空间内以 `defn-` 声明辅助函数并在同空间调用,以 `--run` 执行
- **THEN** 调用成功,行为与公开定义一致

#### Scenario: 跨空间引用私有定义报错

- **WHEN** 模块 B 经 `ns` 加载模块 A,并引用 A 中以 `defn-` 声明的符号,以 `--typecheck` 运行
- **THEN** 报告该符号不可见或未定义,不静默通过

### Requirement: 导入导出过滤

`ns` 的 `:require`/`:refer` SHALL 过滤导入符号:未在导出范围内的符号 SHALL 不可被引用;`:refer [f]` SHALL 仅导入显式列出的符号。导出表 SHALL 由公开定义构成。

#### Scenario: refer 列表过滤

- **WHEN** 模块 B 以 `(:require [A :refer [f]])` 加载模块 A,并尝试引用 A 中未列出的符号 g
- **THEN** 引用 g 报错,引用 f 正常

#### Scenario: 导出边界生效

- **WHEN** 模块 A 未显式导出某公开定义,模块 B 未通过 refer 导入却直接引用该定义
- **THEN** 按导出/导入契约判定可见性,违反时报告错误

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
