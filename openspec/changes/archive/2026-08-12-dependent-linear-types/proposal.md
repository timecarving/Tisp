## Why

QTT 的固定等级(0/1/ω)无法表达「资源可用次数由编译期数值决定」的核心场景:如 `(Vec a n)` 的每个元素线性使用一次,则整个向量的资源可用 n 次;`(Matrix a m n)` 可拆分为 m×n 次使用。当前 `Grade` 枚举已定义代数结构(`Nat/Add/Mul/Var`,§11 资源代数与 grades.rs 半环),`tisp-runtime/src/depgraded.rs` 也有依赖等级求值骨架,但**语法、检查与接线全部缺失**——用户无法书写依赖等级,`grade_check` 只处理固定 0/1/ω。本变更把「等级 = 编译期数值表达式」的依赖线性类型完整落地(参考 Idris 2 数量与 Granule)。

## What Changes

- **依赖等级语法**:graded 参数支持等级表达式——数字 `(5 x : a)` → `Grade::Nat(5)`、符号 `(n x : a)` → `Grade::Var(n)`、复合 `((+ n 1) x : a)` → `Grade::Add`;现有 `{0 x : a}`/`{1 x : a}`/`{ω x : a}` 语法保持兼容
- **等级变量绑定**:`Grade::Var(n)` 从依赖类型参数解析(如 `(defn sum-all [n xs : (Vec i64 n)] ...)` 的 n);未绑定等级变量报编译错误
- **检查语义扩展**:`grade_check` 处理 `Nat/Add/Mul/Var` 等级——使用计数须 ≤ 等级表达式(上界语义,与 Idris 2 一致);数字等级常量折叠(grades.rs 半环),符号等级做符号比较(常量可判定时检查,不可判定时警告放行);`0/1/ω` 为特例保持现状
- **分支合并**:if/match 分支对依赖等级变量的使用取合并(计数上界),不破坏现有线性检查
- **运行时接线**:`depgraded.rs` 的等级求值接入解释器(等级值可查询,如 `grade-of` 返回表达式等级)
- **测试与文档**:等级表达式解析/检查/分支合并测试;standard_doc 01/04 与 spec §10 同步

## Capabilities

### New Capabilities

- `dependent-linear-types`:依赖等级表达式语法、等级变量绑定、使用计数 ≤ 等级表达式的检查语义、分支合并的行为规范

### Modified Capabilities

(无)

## Impact

- `crates/tisp-core`:`types.rs`(Grade 已有代数,可能需要显示支持)、`grades.rs`(半环运算已存在)
- `crates/tisp-frontend`:`desugar.rs`(`desugar_graded_param` 接受等级表达式)
- `crates/tisp-middle`:`grade_check.rs`(Nat/Add/Mul/Var 计数检查、分支合并)
- `crates/tisp-runtime`:`depgraded.rs`(等级求值接线)
- `crates/tisp-backend`:`interpreter.rs`(grade-of 扩展、擦除逻辑对 Nat 等级适配)
- 文档:`standard_doc/01-language-core.md`、`standard_doc/04-implementation-status.md`、`docs/spec.md` §10 状态、`CHANGELOG.md`、示例
