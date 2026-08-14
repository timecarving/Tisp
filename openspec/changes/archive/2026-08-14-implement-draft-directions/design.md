## Context

动机见 proposal.md — Why。现状:`tisp-frontend` 用 logos 手写 lexer + 手写 parser/desugar,无声明式语法框架;`tisp-core` 的 `Type` 已有 `Forall`(多态量化)与 `Fun`(运行时函数),但无「类型级 λ」(tlambda)与 `conj`/`disj` 字面量;类型类已是 `defclass`/`definstance`(§23 ✅)。约束:保持 `cargo test --workspace` 全绿、零警告;注释简体中文;`Type` 变更会波及 type_infer/unify/display。

## Goals / Non-Goals

**Goals:**
- `::>` 语法 DSL 落地为可解析、可生成扫描器的声明式语法表(不立刻替换 logos,先旁挂)
- tlambda/defpoly/where 作为编译期类型级特性进入类型系统
- `conj`/`disj` 类型字面量 + `deftrait` 等 trait 语法糖,以脱糖方式复用既有 Tuple/defdata/defclass 语义

**Non-Goals:**
- 不做完整「自举 lexer」——语法 DSL 生成扫描器,但不重写现有 logos 词法器
- 不引入运行时动态类型变量(tlambda 仅静态语义)
- 不做 mixin(type-system2 明确「暂时不做」)

## Decisions

### D1: 语法 DSL 用「语法表 + 扫描器」模块旁挂 logos

`draft` 的 `::>` + meta-tag 是一套声明式语法定义语言。当前 lexer 用 logos,parser 手写。

- **决策**:frontend 新增 `grammar` 模块,解析 `::>` 语法定义 → 生成 `GrammarTable`(结构名 → 具体结构 AST)→ `Scanner` 按 meta-tag 逐字符匹配。作为独立旁挂能力,先用于语法 DSL 自描述与增量语法定义,不替换 logos。
- **替代**:替换 logos(否决——回归面大);仅文档(否决——无行为)。

### D2: tlambda 用 `Type::TLambda` 变体承载类型级 λ

`A => B` 是「类型 → 类型」的编译期函数,与 `Fun`(运行时函数)、`Forall`(量化)都不同。

- **决策**:`Type` 增 `TLambda(Box<Type>, Box<Type>)`(类型级抽象,参数类型 → 返回类型);`=> B` 脱糖为 `TLambda(Unit, B)`(无输入)。tlambda 值作为编译期一等值(可绑定/传递/匹配),不进入运行时。
- **替代**:复用 `Forall`(否决——语义是量化非抽象);类型族(否决——tlambda 是值,类型族是声明)。

### D3: conj/disj 走脱糖,不改 Type 结构

- **决策**:`(conj A B)` → `Type::Tuple`(或 `Record`);`(disj A B)` → `defdata` 多构造器 ADT(语法糖);`()`→`Unit`、`(A B C)`→`Tuple`、`A -> B`→`Fun`。全部在 desugar 层完成。
- **替代**:新增 `Type::Sum`(否决——与 defdata 重复,徒增 unify 分支)。

### D4: trait 语法糖脱糖到 defclass/definstance

- **决策**:`deftrait`→`defclass`、`defabsmember`→抽象方法、`polytrait`→带类型参数的 `defclass`、`(with ...)`/`(with-static ...)`/`(with-cons ...)`→`definstance` 方法绑定。行为完全复用 §23 既有实现。
- **替代**:独立 trait 运行时(否决——重复)。

### D5: where 约束编译期检查,复用 kind/等级检查

- **决策**:`where` 约束(如 `Number`、`BiggerThan[60]`)在 type_infer 的实例化点做编译期检查,复用既有 `kind_of`/等级比较;不可静态判定则警告放行(与项目「可判定即查、不可判定放行」一致)。
- **替代**:新约束求解器(否决——超出本轮;close-full-chain-gaps 的统一求解器是增量第一步)。

## Risks / Trade-offs

- **[Type 增变体破坏 unify/display]** → `Type::TLambda` 需同步补 unify、display、serde、type_infer 的 reduce/collect 分支;分步小改,每步跑测试。
- **[语法 DSL 与 logos 语义漂移]** → 语法 DSL 先只做「声明 → 扫描器」,不承担真实词法任务,避免两套词法冲突。
- **[where 约束过度检查]** → 不可判定一律警告放行,不误报。
- **[trait 语法糖覆盖不全]** → 先覆盖草案明列的 7 个形式,未列出的暂不实现(记录在 tasks)。

## Migration Plan

1. 语法 DSL 模块独立,不影响现有 frontend 词法路径。
2. tlambda/conj/disj/trait 均为新增语法或等价糖,不改既有程序行为;逐项加测试后全量回归。
3. 文档最后同步(`standard_doc`/`docs/spec.md`/`CHANGELOG`)。

## Open Questions

- (无——scope/版本/语义已由用户确认)
