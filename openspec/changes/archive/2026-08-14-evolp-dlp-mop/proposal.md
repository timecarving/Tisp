## Why

Tisp 已具备扎实的逻辑编程基底(`defpred`/CLP/ALP/Search 效应/回溯),但知识库仍是静态的:规则一经定义便不再随时间演化,程序无法把「规则本身」「约束」「项传播」当作一等数据来操作。这使「纯声明式 + 一切皆数据」的核心承诺(Principle: Everything is an Annotated Relation / Effects are the Universal Glue)在逻辑编程侧留下一块空白——无法表达动态信念修订、反射式元编程,也无法把高阶/归纳/概率/时序/描述/可废止/模糊/表格化/情境/模态/响应式等逻辑编程范式组合到一个统一 ADT 基底上。本变更落地「Everything as ADT」这一最终核心思想:把逻辑编程的规则、约束、项传播、OOP 对象全部视为 ADT,在纯声明式约束下实现 EVOLP、DLP 与 MOP,并以组合优先原则覆盖其余逻辑编程范式。

## What Changes

- **Everything as ADT 统一**:逻辑规则、约束、项传播、OOP 对象 SHALL 成为可绑定/传递/匹配的一等 ADT 值;`defpred` 子句、`constrain` 约束、统一项、对象均降为数据构造器,而非仅编译器内部表示。
- **EVOLP(演化逻辑编程)**:规则可携带演化指令(`assert`/`retract`);程序「当前状态」为不可变值(`Program`);演化操作是纯函数,整个演化过程可用 `foldl` 实现;求值迭代计算稳定模型直至不动点(递归/`fix`/反射性状态单子)。
- **DLP(动态逻辑编程)**:知识库为状态序列,更新=向序列末尾追加新状态;「动态稳定模型」按拒绝/接受语义定义(对每状态拒绝被后续状态否定的规则,对剩余规则做约化取最小模型)。
- **MOP(元对象协议)**:`GetKB`/`SetKB` 定义为 Effect 操作,Handler 充当元解释器;元编程能力在编译期即可满足(宏展开/部分求值),运行时 handler 为回退路径。
- **State Effect 引用管理**:在既有 `State s`(get/put)之上设计可变引用(`ref`/`deref`/`set!`),以线性/分级等级约束所有权,作为 State 效应操作建模。
- **逻辑编程范式扩展(组合优先)**:在纯声明式、静态类型、函数-并发-内存管理(依赖/分级线性类型)约束下,按「可用既有特性组合则组合、否则新增少量特性」原则设计高阶、归纳(ILP)、概率(PLP)、时序、描述、可废止、模糊、表格化(Tabled)、类型化函数-面向对象-并发一体基底、代数效应 FRP 响应式、情境、模态共 12 类逻辑编程能力。

## Capabilities

### New Capabilities

- `everything-as-adt`: 「一切皆 ADT」统一基底——规则/约束/项/OOP 对象的一等数据化与纯声明式约束。
- `evolp-dlp`: 演化逻辑编程(EVOLP)与动态逻辑编程(DLP),含不可变 Program、演化指令、稳定模型不动点与动态稳定模型。
- `meta-object-protocol`: 元对象协议(MOP),含 GetKB/SetKB 效应操作、Handler 元解释器、编译期元编程与 State Effect 引用管理。
- `logic-programming-paradigms`: 12 类逻辑编程范式(高阶/归纳/概率/时序/描述/可废止/模糊/表格化/一体化基底/响应式/情境/模态),组合优先设计。

### Modified Capabilities

(无——本变更全部为新增能力,不改动既有 `logic-and-verification` 等规范的需求块。)

## Impact

- **tisp-core**:新增 `Program`(不可变程序)、动态程序序列、演化指令(`Evolve`/`Assert`/`Retract`)、元对象与 KB 操作的 AST/类型节点;`Type` 扩展 ADT 化类型构造。
- **tisp-frontend**:`desugar.rs` 新增 `defevolp`/`defdlp`/`deftrait`-风格元对象等语法的脱糖;reader 支持规则/约束的字面量数据形式。
- **tisp-middle**:类型/效果/等级/确定性推断扩展——GetKB/SetKB 效应行、State 引用线性等级、演化操作纯函数性校验、稳定模型不动点类型。
- **tisp-backend**:interpreter 新增稳定模型求解器(约化 + 最小模型 + 不动点迭代)与动态稳定模型求值;State 引用运行时。
- **tisp-runtime**:`logic.rs` 扩展为规则/程序数据结构;新增 `evolp`/`dlp`/`mop` 模块;各 LP 范式作为组合层或少量新模块。
- **docs/spec.md**:新增/扩展逻辑编程与元编程章节;同步 `standard_doc/` 与 `CHANGELOG.md`。
