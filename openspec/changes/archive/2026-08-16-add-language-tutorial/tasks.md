## 1. 教程骨架与导航

- [x] 1.1 创建 `tutorial/` 目录与 `tutorial/INDEX.md`：包含学习路线（新手/类型系统深入/系统编程/效果与元编程/逻辑编程/并发与进程）、完整章节列表、状态符号说明（✅/⚠️/⬜）
- [x] 1.2 创建 `tutorial/examples/` 目录并确定示例文件命名规范（`chNN-*.tisp`）

## 2. 基础篇

- [x] 2.1 第 01 章 `01-getting-started.md`：安装与构建、`hello.tisp`、REPL 使用（`:type`、定义行累积、`(exit)`）、字面量/变量/函数定义/let/if/cond/类型标注等基本语法，含练习题
- [x] 2.2 第 02 章 `02-types-and-patterns.md`：ADT（含 Record 语法）、GADT、模式匹配（同名变量、or-pattern、guard、if-let/when-let）、穷尽性检查，含 `--typecheck` 可验证示例与练习题

## 3. 类型系统深入篇

- [x] 3.1 第 03 章 `03-type-system-deep.md`：QTT 等级（0/1/ω、线性资源协议、参数性）、区域系统、依赖类型（Π/Σ）、液态类型 `{x : T | pred}`（标注 Z3 feature 要求），含示例与练习题
- [x] 3.2 第 15 章 `15-hott-and-derived.md`：HoTT Path/Interval、HIT、`deriving` 与代数结构，标注实现状态，含示例与练习题

## 4. 效果与元编程篇

- [x] 4.1 第 04 章 `04-effect-system.md`：effect 声明、`handle`/`perform`、续延 `k`、内置效果（get/put/ask/tell/throw/choose）、效应行消减、monadic 优化路径（mlet/get-m/put-m/pure），含 `--run` 可验证示例与练习题
- [x] 4.2 第 05 章 `05-macros-and-metaprogramming.md`：`defmacro`、语法引号与 `~x`/`~@x`、卫生展开、`gensym`、comptime 编译期求值与编译期 KB（get-kb/set-kb），含 `--desugar` 可验证示例与练习题

## 5. OOP 与逻辑编程篇

- [x] 5.1 第 06 章 `06-oop-and-typeclasses.md`：`defgeneric`/`defmethod`（模式匹配分发）、方法组合（`:around`/`:before`/`:after`/`:primary`）、`call-next-method`、编译期特化、`defclass`/`definstance` 类型类与函数依赖，含示例与练习题
- [x] 5.2 第 07 章 `07-logic-programming.md`：`defpred` 子句、合一与回溯、`find-all`、搜索效应、CLP（`domain`/`label`/`constrain`/`solve-all`）、溯因 `abduce`、12 类逻辑范式的组织方式，含示例与练习题

## 6. 并发、进程与系统篇

- [x] 6.1 第 08 章 `08-concurrency-and-frp.md`：`stream`/`stream-take`/`advance`、Signal 节点（Map/Filter/Fold）、`chan`/`send`/`recv`/`spawn` 基础并发，含示例与练习题
- [x] 6.2 第 09 章 `09-process-calculi.md`：SKI 组合子、π-calculus、Async π、applied π、spi-calculus（加密 encrypt/decrypt/sign/verify/hash）、Safe Ambients、ρ-calculus、κ-calculus、ς-calculus 与编码，标注实现状态，含示例与练习题
- [x] 6.3 第 10 章 `10-modules-and-namespaces.md`：`ns` 声明、`:require`/`:refer`/`:as` 导入导出、跨文件项目组织、模块可见性，含多文件示例与练习题
- [x] 6.4 第 11 章 `11-ffi-and-system.md`：`defextern` 签名分派（i64/f64/str/指针）、`ptr-read`/`ptr-write` 线性裸指针、`with-region`/`region-alloc` 手动区域与逃逸检查、`Unsafe` 效应门控、feature 门控行为（默认构建显式报错），含示例与练习题
- [x] 6.5 第 16 章 `16-compilation-and-toolchain.md`：CLI flags 全览（--eval/--print-ast/--print-tokens/--desugar/--typecheck/--run/--verify/--ir/--compile）、comptime 全链路、编译指示（opt-level/inline!/specialize!/suppress-warning），含示例与练习题

## 7. 验证、编程范式与 AOP 篇

- [x] 7.1 第 12 章 `12-verification.md`：`defprop` 属性声明、`verify` 模型检查、等价检查、攻击重建、验证即 effect handler 的视角，含示例与练习题
- [x] 7.2 第 13 章 `13-programming-paradigms.md`：8 类编程范式逐一详解——数组（创建/形状/索引/切片/map/reduce）、栈（new/push/pop/peek/dup/swap/rotate）、连接式（compose/apply/branch）、符号（构造/匹配/代换/化简/求值）、自动机（DFA 声明/识别/组合）、状态机（事件驱动/entry-exit 动作）、数据驱动（DispatchTable 分发）、基于流（source/map/filter/take/sink 惰性流水线），含非法输入报错示例与练习题
- [x] 7.3 第 14 章 `14-aop.md`：AOP 切面定义、pointcut、`before`/`after`/`around` 建议、`call-next-method` 内层链、comptime 编译期编织、AOP 效应行合成、与 OOP 语义保持，含 `--desugar`/`--run` 可验证示例与练习题

## 8. 附录与示例收录

- [x] 8.1 附录 `A1-syntax-reference.md`：完整语法 BNF、保留字表、运算符优先级
- [x] 8.2 附录 `A2-builtins-catalog.md`：内置函数速查表（算术/比较/布尔/字符串/集合/IO/类型操作）
- [x] 8.3 附录 `A3-quick-reference.md`：常用模式速查卡（类型标注、效应声明、宏模板、等级标注等）
- [x] 8.4 附录 `A4-error-messages.md`：常见错误诊断与修复建议（类型错误/效应缺失/等级违反/区域逃逸等）
- [x] 8.5 为每章创建独立可运行示例文件到 `tutorial/examples/`（`chNN-*.tisp`），与教程代码块保持一致

## 9. 验证与验收

- [x] 9.1 交叉链接检查：INDEX.md 与各章「上一章/下一章」导航链接全部有效
- [x] 9.2 全量运行验证：`tutorial/examples/` 下所有标注 ✅ 的示例经 `tisp --typecheck` 通过、可运行的经 `--run` 输出与教程所述一致；标注 ⚠️/⬜ 的与 `standard_doc/04-implementation-status.md` 状态一致
- [x] 9.3 覆盖性审查：对照 `docs/spec.md` 32 章逐节核对，确认每个特性在教程中均有使用讲解与示例（无遗漏）；不完整时补齐缺口章节/示例
- [x] 9.4 最终一致性检查：教程与 `docs/spec.md`/`standard_doc/` 术语、语法一致；更新 `CHANGELOG.md` 记录新增教程