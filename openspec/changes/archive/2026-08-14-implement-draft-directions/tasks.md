# 草案方向落地 — 任务清单

规范依据:2 个 delta(grammar-dsl 新增 + type-system-extensions 新增);方案依据:design.md。按领域分组,每组完成后全量测试 + 零警告。

## 1. 语法 DSL(grammar-dsl)

- [x] 1.1 frontend 新增 `grammar` 模块:`::>` 定义解析器 → `GrammarTable`(结构名 → 具体结构 AST);测试(结构定义解析)
- [x] 1.2 meta-tag 特殊形式解析:`<tag>`/`\x`/`[]`/`{}`/`[{}]`/`|`/`multi|`/`<nonterm>`/`<error>`;测试(可选/重复/多路或/error)
- [x] 1.3 内置字符类标签:`<ztonLetter>`/`<atozLetter>`/`<AtoZLetter>`/`<ASCIILetter>`/`<ASCIILetterAll>`/`<ASCIISpecLetter>`;测试(字符类匹配)
- [x] 1.4 扫描器:按语法表逐字符状态匹配,输出结构序列,失败定位报错;测试(integer 扫描 + 失败定位)
- [x] 1.5 语法 DSL 测试固化与全量回归

## 2. 类型 λ(tlambda)

- [x] 2.1 `Type` 增 `TLambda(Box<Type>, Box<Type>)` 变体(类型级抽象);补 unify/display/serde/reduce/collect;测试
- [x] 2.2 `A => B` / `=> B` 类型字面量解析(desugar);`=> B` 脱糖为 `TLambda(Unit, B)`;测试
- [x] 2.3 tlambda 作编译期变量/参数:绑定/传递/`[]` 应用;运行时无动态变量;测试
- [x] 2.4 tlambda 测试固化与全量回归

## 3. 多态类型(defpoly + where)

- [x] 3.1 `(defpoly Name [params where 约束...] body)` 解析 + `where` 约束捕获;测试
- [x] 3.2 `[]` 类型实参应用按参数序匹配;约束(Number/BiggerThan[60])编译期检查;测试(匹配 + 违反报错)
- [x] 3.3 defpoly 测试固化与全量回归

## 4. conj/disj 类型字面量

- [x] 4.1 `(conj A B)` → Tuple、`()`→Unit、`(A B C)`→Tuple、`A -> B`→Fun 解析;测试
- [x] 4.2 `(disj A B)` → defdata 多构造器 ADT 脱糖;测试(`--desugar` 可见构造器 A/B)
- [x] 4.3 conj/disj 测试固化与全量回归

## 5. trait 语法糖

- [x] 5.1 `deftrait` → defclass、`defabsmember` → 抽象方法、`polytrait` → 带类型参数 defclass;测试(`--desugar`)
- [x] 5.2 `(with ...)`/`(with-static ...)`/`(with-cons ...)` → definstance 方法绑定;测试(运行时按类型分发)
- [x] 5.3 trait 语法糖测试固化与全量回归

## 6. 文档与验收

- [x] 6.1 `standard_doc/01-language-core.md`/`02-advanced-features.md` 语法增补(::>/tlambda/defpoly/conj-disj/trait)
- [x] 6.2 `standard_doc/04-implementation-status.md`:§9 类型系统、新增 grammar-dsl 状态升级
- [x] 6.3 `docs/spec.md` 相关章节增补;`CHANGELOG.md` 记录;`README.md` 示例/测试数同步
- [x] 6.4 最终验证:`cargo test --workspace` 全绿、`cargo check --workspace` 零警告、`openspec validate --specs` 全过、示例抽查
