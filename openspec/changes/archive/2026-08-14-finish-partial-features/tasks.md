# 部分实现补齐 — 任务清单

规范依据:5 个 delta(module-visibility 新增 + 4 个 modified);方案依据:design.md(D1-D8)。按依赖分层,语法层先行,每组完成后 `cargo test --workspace` 全绿。

## 1. 语法层(lexer/parser,前置)

- [x] 1.1 lexer 补 `□` token;parser 为 `@` 标注 token 加解析分支(替换 `Token::At` 被拒)
- [x] 1.2 parser 增加 `->[...]` 六维括号注解解析(ε/ρ/@r/m/d 槽位);测试
- [x] 1.3 parser 增加 `{n : T}` 隐式绑定语法(与显式 `{0 n : T}` 区分);测试
- [x] 1.4 parser 增加 `defresource-algebra` 关键字形式(`:semiring`/`:lattice`/`:order`/`:asymptotic`);测试
- [x] 1.5 parser 增加 `□_r` 分级必然与 `@[n]` 分级应用语法;测试

## 2. 类型/等级/模式系统

- [x] 2.1 `CoreDef` 增加 region 字段;desugar 把六维注解填进 `FunAnnotation`(D1);测试
- [x] 2.2 desugar 实现 QTT 隐式绑定默认 0(`{n : T}` → `Grade::Zero`);测试
- [x] 2.3 grade_check 实现隐式绑定默认等级检查(替换 `grade_check.rs:158` 空注释);测试
- [x] 2.4 资源代数:desugar 解析 `:semiring` 关键字形式并接线 `grades.rs` 的 `Semiring`/`Order` 模型;测试
- [x] 2.5 资源代数 Cost:类型携带代数语义并做代价/复杂度检查(不可判定则明确警告);测试
- [x] 2.6 类型族:单声明多模式语法(替换 `desugar.rs:3217` 单模式)+ 未声明族正确报错;测试
- [x] 2.7 类型族 rewrite 规则解析与实例间简化重写;测试
- [x] 2.8 类型一等值:类型值显示(`show_value`/`value_to_string` 加 `Value::Type` 分支);测试
- [x] 2.9 类型一等值:类型值模式匹配(`match_pattern_into` 支持 `Value::Type`);测试
- [x] 2.10 类型一等值:EffectRow/Grade/Mode/Determinism 运行时值(六维一等值);测试
- [x] 2.11 Mercury:内联 `:in`/`:out` 参数模式解析(替换静默丢弃);测试
- [x] 2.12 Mercury:接线 `infer_modes` 自动模式推断(替换死代码);测试
- [x] 2.13 Mercury:同名多模式重载(`mode_sigs` 支持同名多模式,非覆盖);测试

## 3. 模块可见性(module-visibility)

- [x] 3.1 `CoreDef` 增加 visibility 字段;desugar 区分 `defn-`/`def-` 与公开定义;测试
- [x] 3.2 `ns` 解析保留 `:refer` 列表;加载时按导出表过滤符号;测试
- [x] 3.3 私有定义跨文件不可见检查 + 导出边界生效;测试

## 4. 宏/泛型/反射/Monad

- [x] 4.1 宏卫生:`substitute_macro_hygienic` 增加 fn/lambda 参数卫生臂;测试
- [x] 4.2 宏卫生:if-let/when-let/match 绑定卫生;测试
- [x] 4.3 宏卫生:`~x`/`~@x` unquote 参与宏参数替换;测试
- [x] 4.4 泛型特化:由 `Pattern::Lit` 改为 `Pattern::Con`(构造器类型)驱动 + 多参数组合;测试
- [x] 4.5 泛型特化:特化结果接入 `--run` 执行路径(替换仅 `--typecheck` 展示);对拍测试
- [x] 4.6 反射:`mode-of`/`effects-of`/`determinism-of` 查询真实签名(替换硬编码常量);测试
- [x] 4.7 反射:`type-of` 返回静态推断类型(替换运行时值标签);测试
- [x] 4.8 Monad:desugar 增加 `mlet`/`get-m`/`put-m`/`pure` monadic 语法;测试
- [x] 4.9 Monad:真单处理器/无嵌套检测 + 直接状态传递编译(重写 `inline_state_passing`);测试

## 5. 逻辑/CLP/ALP

- [x] 5.1 续延式回溯:Search 节点改续延 + 选择点 + trail 恢复(接线 `logic.rs` 引擎);测试
- [x] 5.2 逐分支解隔离:`find-all`/`solve-all` 替换全局 collect-mode 累加器;测试
- [x] 5.3 结构化值统一:`val_to_logic` 支持 Cons/结构化值(替换 `Int(0)` 折叠);测试
- [x] 5.4 CLP:非线性约束精确收窄(结果变量 z 也收窄);测试
- [x] 5.5 CLP:精确除法(替换截断除);测试
- [x] 5.6 CLP:线性表达式(`+`/`-`)编译为域传播;测试
- [x] 5.7 ALP:domain 感知假设生成(替换 0..5 盲搜);测试
- [x] 5.8 ALP:`assign` 域相交(替换域)+ 逻辑变量溯因;测试

## 6. HoTT/deriving/演算

- [x] 6.1 HIT:`:boundary` 结构化子句解析(替换不透明字符串);测试
- [x] 6.2 HIT:端点代入 `i := i0/i1` + 唯一一致性检查;测试
- [x] 6.3 HIT:接线 `hott.rs`(替换解释器内联占位)+ 端点值构造;测试
- [x] 6.4 deriving:desugar 代码生成 `eq-*`/`show-*`/`ord-*`(替换运行时内置);测试
- [x] 6.5 deriving:Ord 按构造器序+字段比较;未知 trait 与不可派生字段报错;测试
- [x] 6.6 演算:补 async→sync / applied→π / ρ→π 三种编码;测试
- [x] 6.7 演算:修复 `SKI::reduce` 丢弃 K 负载;测试
- [x] 6.8 演算:观察等价(迹/互模拟)检查,接在原进程与编码结果之间;测试

## 7. 文档与验收

- [x] 7.1 04 清单:17 条 ⚠️→✅ 同步 + 双向失真修正(5/6/15/16 标实际状态,2/3/10/11/13/14 修正过度声称)
- [x] 7.2 示例:各领域综合示例(类型一等值/多解逻辑/deriving Ord/六维注解)+ examples 索引更新
- [x] 7.3 `CHANGELOG.md` 记录;README 同步
- [x] 7.4 最终验收:`cargo test --workspace` 全绿、`cargo check --workspace` 零警告、示例抽查、04 状态核对
