# 全部缺口补齐 — 任务清单

规范依据:4 个能力 spec;方案依据:design.md。每组独立提交,每组完成后跑全量测试。

## 1. A 组:类型系统深化(type-system-extensions)

- [x] 1.1 QTT 0 级参数运行时擦除:解释器 apply/let/闭包捕获跳过 0 级绑定;单元测试(0 级参数不求值、不占闭包)
- [x] 1.2 QTT 1 级移动检查:grade_check 扩展,1 级绑定二次引用报错(带 span);单元测试(复用报错)
- [x] 1.3 多模式谓词解析:defpred 解析 `:mode (i, o)` 注解,模式签名存 CoreDef 新字段;desugar 测试
- [x] 1.4 多模式调用检查:mode_analysis 按实参 free/ground 匹配模式,无匹配报错;`--typecheck` 输出模式签名;测试
- [x] 1.5 类型族结构与解析:`types.rs` 加 `TypeFamily` 变体与实例结构;desugar 解析 `(typefamily 名称 参数模式 结果)` 声明;desugar 测试
- [x] 1.6 类型族归约:type_infer 维护实例表,应用处归约,悬挂报错;显示/unify 补分支;测试
- [x] 1.7 类型一等值:`Value::Type` 变体;类型反射查询(MetaQuery 类型分支)返回真实类型;测试(与静态推断一致)
- [x] 1.8 依赖等级传播:grade_check 对依赖绑定实现 r+s 线性约束,违反报错;`--typecheck` 输出;测试
- [x] 1.9 资源代数:desugar 真实解析 `defresource-algebra`(替换 stub);Cost 注解检查;desugar/typecheck 测试
- [x] 1.10 committed-choice 运行时语义:解释器按 CcMulti/CcNonDet 在首解后提交,不再回溯;测试
- [x] 1.11 A 组合并:CHANGELOG + 04 文档增量同步、`cargo test --workspace` 全绿、零警告

## 2. B 组:逻辑与验证(logic-and-verification)

- [x] 2.1 CLP 域间传播:constraint.rs 将 constrain 编译为区间/不等式传播(域收缩),冲突使搜索失败;测试(解集正确)
- [x] 2.2 ALP 溯因真实化:abduction.rs 返回可满足性验证的假设集,缺失时报告原因;测试
- [x] 2.3 find-attack:ModelChecker 扩展攻击者知识(拦截/转发),深度限制内返回攻击轨迹;测试
- [x] 2.4 check-equivalence:比较两进程状态可达集,输出等价或区分轨迹;测试
- [x] 2.5 MPST 角色与投影:defsession 解析 `:role` 标注,角色投影为单方 SessionType;`--desugar` 保留完整协议;测试
- [x] 2.6 MPST 类型级检查:会话操作顺序违反报错;`--typecheck` 测试
- [x] 2.7 B 组合并:CHANGELOG + 04 增量同步、全量测试全绿、零警告

## 3. C 组:工具链与宏(toolchain-and-macros)

- [x] 3.1 宏卫生:desugar 宏展开对引入符号加唯一后缀(捕获避免);现有宏测试回归;测试
- [x] 3.2 gensym 内置:每次调用唯一符号,宏展开多次互不冲突;测试
- [x] 3.3 编译期特化:middle 新模块 specialize,GenericDef ground 调用 monomorphize,`--typecheck` 报告特化数;测试
- [x] 3.4 dlopen FFI:新增 `ffi` feature + libloading 依赖;defextern 库/符号解析与 C ABI 调用;默认构建回退模拟表;测试
- [x] 3.5 反射环境查询:MetaQuery 环境分支返回真实定义/参数/效果信息(与 A 组共享节点);cli 输出;测试
- [x] 3.6 Monad 优化接线:解释器对单处理器 get/put 走直接状态传递求值,输出优化标注;嵌套保持 handler 语义;测试
- [x] 3.7 C 组合并:CHANGELOG + 04 增量同步、全量测试全绿、零警告

## 4. D 组:HoTT 与小件(hott-and-deriving)

- [x] 4.1 HIT :boundary 解析:desugar 解析 defdata-hit 的 `:boundary` 声明;desugar 测试
- [x] 4.2 HIT 边界检查:路径构造器端点与 boundary 一致性检查,违反报错;测试
- [x] 4.3 deriving 生成:desugar 对 `:deriving (Eq, Ord, Show)` 生成结构递归实现(gensym 防冲突),函数字段报错;测试
- [x] 4.4 演算互编码:runtime/process.rs 加 π→SKI 与 ambient 能力编码函数;观察等价测试
- [x] 4.5 Cohesive ʃ 节点与语义:ʃ 求值为路径代数容器(区间端点),`♭`/`♯` 上下文检查;测试
- [x] 4.6 crisp 上下文检查:type_infer 传播 crisp 标记,非 crisp 解包报错;`--typecheck` 测试
- [x] 4.7 D 组合并:CHANGELOG + 04 增量同步、全量测试全绿、零警告

## 5. E 组:文档收尾与全量验证

- [x] 5.1 重建 04 总表:修正章节号(23 章起偏移 -1)、按 CHANGELOG+代码更新全部 30 章状态
- [x] 5.2 重建 04 两张清单:剔除已实现条目,保留真缺口并更新 file:line 证据
- [x] 5.3 spec 附录 A:BNF 补块注释 `#| |#`、构造器名 `:::`、时序算子 `⃝`、类型族语法
- [x] 5.4 spec 附录 B:保留字补 defsession/verify!/solve-all/find-all/gensym 等
- [x] 5.5 spec 附录 F:示例索引重建(14 个实际示例 + 运行结果)
- [x] 5.6 spec 状态内联:30 章正文标注 ✅/⚠️/⬜(与 04 同符号);README/INDEX 链接描述同步
- [x] 5.7 示例补充:为 A-D 组新增能力各加 1 个示例(可运行或 typecheck 型);examples 索引更新
- [x] 5.8 最终验证:`cargo test --workspace` 全绿、零警告、`--typecheck`/`--run` 抽查示例;CHANGELOG 汇总
