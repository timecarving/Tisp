## Why

`drafts/` 下有三份早期设计稿(`draft` 语法 DSL、`type-system`/`type-system2` 类型系统),描述了当前 Tisp 尚未实现的几个方向:声明式语法框架(`::>` + meta-tag)与类型层 λ 演算(tlambda/defpoly/where 约束)以及 `conj`/`disj` 和积类型字面量。这些稿子是「从零构建语言工具链」的原始构想,现在是时候把它们补进全链路,兑现「类型一等值 + 类型 λ」的设计闭环。

## What Changes

- **语法/词法 DSL(新增)**:落地 `::>` 语法定义 + meta-tag 系统(`<tag>`、`\x`、`[]`、`{}`、`[{}]`、`|`、`multi|`、`<nonterm>`、`<error>`)与内置标签(`<ztonLetter>`/`<atozLetter>`/`<AtoZLetter>`/`<ASCIILetter>`/`<ASCIILetterAll>`/`<ASCIISpecLetter>`),生成可驱动扫描器的声明式语法表。
- **类型 λ(tlambda)**:`A => B`(tlambda)/`=> B`(无输入 tlambda)类型字面量;tlambda 作为编译期变量/参数参与类型推导,通过 `[]` 与类型系统通信。
- **多态类型(defpoly + where)**:`(defpoly Name ['a 'b ... where 约束...] body)` 定义带约束的多态类型,`where` 约束(如 `Number`、`BiggerThan[60]`)参与编译期检查。
- **conj/disj 类型字面量**:`(conj A B)` 乘积类型(= 现有 Tuple/Record 的别名形式);`(disj A B)` 和类型——按你确认的语义,**就是 defdata 多构造器 ADT 的语法糖**。
- **类型字面量补齐**:`()`→`Unit`、`(A B C)`→Tuple(List 包装)、`A -> B`→lambda、`A => B`→tlambda。
- **trait 语法糖**:`deftrait`/`defabsmember`/`defmember`/`polytrait`/`(with ...)`/`(with-static ...)` 映射到现有 `defclass`/`definstance` 类型类系统,作为等价语法。

## Capabilities

### New Capabilities

- `grammar-dsl`: `::>` 声明式语法定义与 meta-tag 扫描器框架,替换/旁挂手写 lexer/parser。

### Modified Capabilities

- `type-system-extensions`: 类型 λ(tlambda)、多态类型(defpoly/where)、`conj`/`disj` 类型字面量、trait 语法糖(`deftrait`/`polytrait`/`with`)。

## Impact

- **crates**:`tisp-frontend`(grammar-dsl 解析器 + 类型字面量/defpoly/deftrait 脱糖)、`tisp-core`(Type 增加 tlambda 相关变体或别名、conj/disj 别名)、`tisp-middle`(where 约束求解、tlambda 编译期求值)、`tisp-backend`(运行时类型值扩展)。
- **语法层**:新增 `::>` 语法文件解析、`defpoly`/`deftrait`/`polytrait` 顶层形式、`(conj ...)`/`(disj ...)`/`A => B` 类型字面量。
- **兼容性**:`conj`/`disj` 为新增字面量,不与现有 `Tuple`/`defdata` 冲突;`deftrait` 等为等价语法糖,不改变既有 `defclass` 行为。
- **文档**:`standard_doc/01/02/03` 语法增补、`04` 状态升级、`docs/spec.md` 相关章节、`CHANGELOG.md`。
