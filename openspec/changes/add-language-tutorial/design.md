## Context

参见 proposal.md —— Tisp 现有文档 (`docs/spec.md` + `standard_doc/`) 为设计/参考性质的规范，缺乏面向学习者的渐进式教程。`examples/` 中的 24 个示例文件可作为各章节的示范来源，但无结构化学程编排。

本设计约定教程的目录结构、章节编排、内容模板与示例代码规范，不涉及编译器或运行时变更。

## Goals / Non-Goals

**Goals:**
- 定义 `tutorial/` 目录下教程文档的完整章节目录
- 定义每章的内容模板（目标、讲解、示例、练习）
- 定义教程示例代码的可验证性约定（`--typecheck` / `--run` 标注）
- 定义导航结构（`INDEX.md`、前后章节链接）
- 给定 20+ 章完整覆盖 Tisp 全部语言特性（基础 → 高级 → 系统级）

**Non-Goals:**
- 不编写实际的教程内容（实施阶段完成）
- 不修改 `docs/spec.md` 或 `standard_doc/`（保持现有参考文档不变）
- 不修改编译器/运行时/示例代码
- 不新增语言特性或变更任何规范

## Decisions

### 1. 教程目录结构：`tutorial/` 下按篇-章二级组织

```text
tutorial/
├── INDEX.md                    # 导航索引 + 学习路线建议
├── 01-getting-started.md       # 安装 · Hello World · REPL · 基本语法
├── 02-types-and-patterns.md    # ADT · GADT · 模式匹配 · 级配模式
├── 03-type-system-deep.md      # QTT 等级 · 区域 · 依赖类型 · 液态类型
├── 04-effect-system.md         # effect declare · handle/perform · 续延
├── 05-macros-and-metaprogramming.md  # defmacro · syntax-quote · hygiene · gensym
├── 06-oop-and-typeclasses.md   # defgeneric · defmethod · method combination · typeclass
├── 07-logic-programming.md     # defpred · 回溯 · CLP · 溯因
├── 08-concurrency-and-frp.md   # FRP · Signal · Stream · 通道基础
├── 09-process-calculi.md       # π-calculus · ρ · κ · spi · ambient · SKI
├── 10-modules-and-namespaces.md # ns · require · refer · 多文件项目
├── 11-ffi-and-system.md        # defextern · 裸指针 · 手动区域 · Unsafe effect
├── 12-verification.md          # defprop · verify · model checking · 等价检查
├── 13-programming-paradigms.md # 8 类编程范式详解（数组/栈/连接式/符号/自动机/状态机/数据驱动/基于流）
├── 14-aop.md                   # AOP 切面 · pointcut · before/after/around · comptime 编织
├── 15-hoTT-and-derived.md      # HoTT Path/Interval · deriving · HIT
├── 16-compilation-and-toolchain.md  # CLI flags · --ir · --compile · comptime
├── A1-syntax-reference.md      # 完整语法 BNF
├── A2-builtins-catalog.md      # 内置函数速查表
├── A3-quick-reference.md       # 常用模式速查卡
└── A4-error-messages.md        # 常见错误诊断与修复建议
```

**理由**：按学习难度递进编排——从基础语法到高级类型再到系统级特性；篇内各章独立可读，但有前后导航链接。附录集中参考信息。

### 2. 章节内容模板

每章按固定结构组织：

```markdown
# <章标题>

## 目标 (Objectives)
<!-- 3-5 条学习目标 -->

## <节 1: 概念讲解>
<!-- 概念定义、设计动机、与语言其他特性的联系 -->

### 示例 (Examples)
<!-- 带注释的可运行代码块，标注 `--typecheck`/`--run` 状态 -->

### 练习 (Exercises)
<!-- 2-4 道练习题 -->

<继续各节…>

## 本章小结 (Summary)

---

> 上一章: `<link>` | 下一章: `<link>` | [返回目录](INDEX.md)
```

### 3. 示例代码可验证性约定

每个代码块在前导注释中标注状态：

```tisp
;; ✅ 可运行  $ tisp --run <file>
;; ✅ 可类型检查  $ tisp --typecheck <file>
(defn add [x y] (+ x y))
```

- ✅ 完全可运行：示例能经 `--run` 输出预期结果
- ⚠️ 部分可运行：示例通过 `--typecheck` 但运行时语义未全实现
- ⬜ 语法展示：示例仅展示语法形态，当前实现未覆盖

**理由**：如实反映语言实现现状（与 `standard_doc/04-implementation-status.md` 一致），避免误导用户。

### 4. 所有代码示例必须收录到专用示例文件

对应每章在 `tutorial/examples/` 下提供独立可运行文件，如：
`tutorial/examples/ch01-hello.tisp`、`tutorial/examples/ch04-effect-handler.tisp`

这些文件应能被 `tisp --run` 或 `tisp --typecheck` 直接执行，与教程中的代码块保持一致。

### 5. INDEX.md 提供多入口学习路径

```markdown
# Tisp 教程

## 学习路线
- **新手路线**: 01 → 02 → 04 → 10 → A3
- **类型系统深入**: 01 → 02 → 03 → 15 → 10
- **系统编程**: 01 → 02 → 10 → 11 → 16
- **效果与元编程**: 01 → 02 → 04 → 05 → 14
- **逻辑编程**: 01 → 02 → 07 → 12
- **并发与进程**: 01 → 02 → 08 → 09

## 章节列表
<!-- 完整章节目录 -->

## 状态说明
<!-- ✅ ⚠️ ⬜ 标记说明 -->
```

## Risks / Trade-offs

- **[风险] 部分高级特性当前实现状态为 ⚠️ 或 ⬜，教程示例可能沦为「语法展示」无运行结果**
  → **缓解**：如实标注状态，提供预期的运行输出注释供未来验证
- **[风险] 教程内容与 `docs/spec.md` / `standard_doc/` 可能出现信息重复或不一致**
  → **缓解**：教程侧重「如何使用」而非「规范定义」；引用 `docs/spec.md` 对应章节供深入阅读
- **[权衡] 20 章一次性编写可能过于庞大**
  → 任务按章拆分为独立子任务，可逐步实施、逐章交付