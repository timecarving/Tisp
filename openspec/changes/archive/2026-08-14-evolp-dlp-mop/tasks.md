## 1. Everything-as-ADT 数据模型

- [x] 1.1 在 tisp-core 新增 `Rule`/`Program`/`EvolInstr` 类型(`Program` 用 `im::HashSet<Rule>` 等不可变表示)
- [x] 1.2 规则/约束/项/OOP 对象的一等值构造器与字面量(reader/desugar 暴露为数据)
- [x] 1.3 规则数据的增删查纯函数(与文本定义语义等价)
- [x] 1.4 约束组合与项传播的 `foldl` 纯函数应用
- [x] 1.5 单元测试:规则绑定/匹配、约束数据组合、对象即值、引用透明

## 2. EVOLP 演化逻辑编程

- [x] 2.1 语法与解析:`assert`/`retract` 演化指令
- [x] 2.2 不可变 `Program` + `evolve` 纯函数 + `foldl` 折叠演化
- [x] 2.3 稳定模型求解器:grounding + Gelfond-Lifschitz 约化 + 最小模型
- [x] 2.4 不动点迭代求值(递归/`fix`,至程序不再变化)
- [x] 2.5 单元测试:assert/retract、foldl 折叠、不动点收敛、稳定模型判定

## 3. DLP 动态逻辑编程

- [x] 3.1 DLP 状态序列(`Vec<Program>`)与追加新状态更新
- [x] 3.2 动态稳定模型:拒绝被后续状态否定的规则 + 约化 + 最小模型
- [x] 3.3 单元测试:状态序列、拒绝/接受语义、动态稳定模型结果

## 4. MOP 元对象协议与 State Effect

- [x] 4.1 `GetKB`/`SetKB` 效应操作(效应行声明 + 纯代码门控报错)
- [x] 4.2 handler 元解释器(捕获 GetKB/SetKB 并解释语义)
- [x] 4.3 编译期元编程(宏展开/部分求值解析静态 KB 操作)+ 运行时 handler 回退
- [x] 4.4 `Ref a` 类型与 `ref`/`deref`/`set!` 的 State 效应操作
- [x] 4.5 引用线性/分级等级所有权检查(接入 QTT grade_check)
- [x] 4.6 单元测试:效应门控、handler 元解释、编译期解析、引用等级约束

## 5. 12 类逻辑编程范式(组合优先)

- [x] 5.1 高阶 LP:谓词一等值 + `call` 组合子
- [x] 5.2 归纳逻辑编程 ILP:`induce` 内置(正/负例 → 假设规则)
- [x] 5.3 概率逻辑编程 PLP:概率事实 + 边际概率(默认精确枚举)
- [x] 5.4 时序逻辑编程:时间索引事实 + LTL 算子(组合 §18 时序类型)
- [x] 5.5 描述逻辑编程:概念/角色/子概念推理(组合类型类 + 子类型)
- [x] 5.6 可废止逻辑编程:优先级/击败者(组合泛型方法组合 + committed-choice)
- [x] 5.7 模糊逻辑编程:真值度 + min/max 组合
- [x] 5.8 表格逻辑编程 Tabled:记忆表(默认全记忆)使左递归终止
- [x] 5.9 静态类型-函数-OOP-并发一体化基底:谓词静态类型 + 互操作
- [x] 5.10 响应式逻辑编程:FRP 信号驱动规则(组合 Signal + 代数效应)
- [x] 5.11 情境逻辑编程:情境层次/继承/隔离(组合模块 + Reader)
- [x] 5.12 模态逻辑编程:`possible`/`necessary`(组合 §11 分级模态)
- [x] 5.13 各范式单元测试(逐范式可独立验证)

## 6. 文档与收尾

- [x] 6.1 docs/spec.md 新增/扩展逻辑编程与元编程章节
- [x] 6.2 同步 standard_doc/ 与 CHANGELOG.md,更新实现状态
- [x] 6.3 `cargo test --workspace` 全绿 + `cargo check --workspace` 零警告验证
