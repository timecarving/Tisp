# 全链路缺口补齐 — 任务清单

规范依据:7 个 delta(data-structures 新增 + 6 个 MODIFIED);方案依据:design.md。按领域分组,每组完成后全量测试 + 零警告。

## 1. 数据持久化(data-structures)

- [x] 1.1 为 interpreter `Value` 实现 `Hash + Eq`(结构相等/哈希),补 `Value` 作为 map/set 键的回归测试
- [x] 1.2 `Value` 增加 `Vector`/`Map`/`Set` 三个 `im` HAMT 变体并接线(替换 `Data("Vec"/"Map"/"Set")`);删除 `persistent.rs` 的孤儿 `PersistentValue` 副本
- [x] 1.3 持久化集合操作:`conj`/`assoc`/`contains?`/`dissoc`/`disj` 返回新结构且旧结构可访问(结构共享),测试(旧引用不被破坏)
- [x] 1.4 `quote` 求值为可操作数据(List/Symbol/数字),支持绑定/遍历/模式匹配;测试(quote 数据可 match)
- [x] 1.5 数据持久化测试固化与全量回归

## 2. 统一约束求解与子类型(type-system-extensions)

- [x] 2.1 middle 新增 `constraint.rs`(`ConstraintGraph`):约束边带跨维度 span,各 pass 改为产约束而非「先到先报错」
- [x] 2.2 新增 `solve.rs` 协调器:fixpoint 迭代至收敛,收敛后统一报告冲突;测试(跨维度联合求解 + 冲突带上下文)
- [x] 2.3 新增 `subtype.rs` 五维子类型格(region/grade/mode/determinism),按协变/逆变位置检查;测试(det ≤ nondet 通过、违反报错)
- [x] 2.4 类型类实例查找改约束求解驱动(替换 `instance_dict` 运行时特例);测试(间接约束经求解器唯一实例)
- [x] 2.5 类型系统测试固化与全量回归

## 3. 模态/等级/代价推理(type-system-extensions + dependent-linear-types)

- [x] 3.1 `□_r`/`◇_ε` 引入与消去参与推断(可判定推导 r/ε,不可判定警告放行);测试(模态消去推导)
- [x] 3.2 grade_check 增加半环加法 `r+s` 依赖等级传播,修 ω 绑定恒过捷径;测试(有限等级传播 + 符号等级)
- [x] 3.3 `Cost` 渐近代价推导:符号代价表达式复合 + `asymptotic` 上界比较;测试(O(n) 复合推导)
- [x] 3.4 模态/等级/代价测试固化与全量回归

## 4. HoTT 补齐(hott-and-deriving)

- [x] 4.1 `hott.rs` 增 `kan_fill`(2-3 维立方面组合),边界不一致报错;测试(多维组合 + 不一致报错)
- [x] 4.2 `♭` 剥离结构、`♯` 嵌入 codiscrete(可区分标记),`--typecheck` 检查 adjoint-triple 组合;测试(flat/sharp 与直通可区分)
- [x] 4.3 HoTT 测试固化与全量回归

## 5. 时序语义保证(temporal-types)

- [x] 5.1 `□_t` 稳定类型检查(稳定类型可跨时刻,非稳定跨时刻报错);测试
- [x] 5.2 受保护递归生产率检查(cons 尾 `⃝` 递归过,无保护报错);测试
- [x] 5.3 时序测试固化与全量回归

## 6. 系统级与编译后端(toolchain-and-macros)

- [x] 6.1 `ptr-read`/`ptr-write` 线性裸指针 + `Unsafe` 门控;测试(读写 + 门控报错)
- [x] 6.2 `with-region`/`region-alloc` 手动区域 + 退出回收 + 区域逃逸报错;测试
- [x] 6.3 LLVM inkwell 路径补 `define`/`call`/闭包环境打包(替换 `ret i64 0`);llc 编译验证;测试(函数调用/闭包)
- [x] 6.4 非 llvm 文本 IR 回退行为一致(可读伪 IR)
- [x] 6.5 `opt-level`/`inline!` 优化器真实接线(迭代次数/内联阈值/强制内联);测试
- [x] 6.6 反射函数补全(名称/定义/参数/效果/等级/模式/确定性全真实,无近似);测试
- [x] 6.7 工具链测试固化与全量回归

## 7. 密码学(process/验证)

- [x] 7.1 `crypto` feature 引入 RustCrypto(`aes`/`chacha20`/`sha2`),`encrypt`/`decrypt`/`hash` 接强算法;测试(往返 + 已知向量)
- [x] 7.2 默认构建保留 XOR/简单 hash 并标注「非加密」,`--run` 输出警告;测试
- [x] 7.3 密码学测试固化与全量回归(默认 + crypto 双构建)

## 8. 递归与运行时(recursion-closure-fixes)

- [x] 8.1 解释器 TCO:尾位置(函数体尾/if 两分支/let 尾体)迭代复用栈帧;测试(`(sum-to 100000)` 不溢出)
- [x] 8.2 修多顶层表达式递归栈溢出:`__top__` 逐表达式求值(不嵌套 Do);测试(多 `println` + fib)
- [x] 8.3 递归/运行时测试固化与全量回归

## 9. 文档与 OpenSpec 收尾

- [x] 9.1 `standard_doc/04-implementation-status.md`:§1 逐章总表与 §2 清单一致化;§2/§4/§9/§11/§16/§17/§18/§19/§26/§27/§29/§30 状态升级 ✅
- [x] 9.2 `standard_doc/02-advanced-features.md`:4 处 stale ⬜ 去 stale(ALP 搜索策略/演算完整语义/实例查找/函数闭包生成)
- [x] 9.3 `docs/spec.md`:章节标题 ⚠️ 标记刷新(§13/§21/§22/§24/§26/§28 升 ✅);§17 实现注更新
- [x] 9.4 `CHANGELOG.md` 记录;`README.md`/`openspec/project.md` 测试数同步
- [x] 9.5 归档 `complete-partial-features`;清理已吸收的 `implement-design-stage-features`
- [x] 9.6 最终验证:`cargo test --workspace` 全绿、`cargo check --workspace` 零警告、`openspec validate --specs` 全过、示例抽查
