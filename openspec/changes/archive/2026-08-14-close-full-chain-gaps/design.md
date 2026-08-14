## Context

动机见 proposal.md — Why。现状:middle 层是 7 个独立 pass(type/effect/grade/mode/determinism/region/specialize),无共享约束图;`runtime/persistent.rs` 已用 `im` 实现 HAMT 但从未被 interpreter `Value` 引用(死代码);`codegen.rs::compile_app` 对非柯里化二元运算回退 `ret i64 0`;加密是 XOR/简单 hash;`instance_dict` 运行时字典替代约束求解。约束:保持 `cargo test --workspace` 全绿、零警告;LLVM/Z3 代码 feature 门控,默认构建不破坏;注释简体中文。

## Goals / Non-Goals

**Goals:**
- 六维注解收敛为共享约束图 + fixpoint,各维度约束联合求解(替换串行 pass 的「先到先报错」)
- 持久化集合与 quote-as-data 真正进入运行时 `Value`
- 模态/等级/子类型/代价的「解析 → 推理」补齐,以「可判定时检查、不可判定时警告放行」为一致策略
- LLVM 函数/闭包真实代码生成 + 裸指针/区域/Unsafe 门控 + 密码学换强算法
- 解释器 TCO + 多顶层递归栈溢出修复

**Non-Goals:**
- 不追求完全形式化(不引入 kernel/elaboration 架构、不证明 soundness)
- 不做完整 category-theoretic Cohesive model(♭/♯ 落地为可区分的运行时语义 + 上下文检查,非全模型)
- 立方填充限定有限维度(2-3 维)可判定,不实现任意维度 Kan 组合
- 不重写 lexer/parser(语法层已齐)

## Decisions

### D1: 统一求解用「增量收敛」而非「全量重写」

现有 7 个 pass 各维护自己的状态(type env、effect row、grade counter、mode、determinism、region)。把六个维度重写成单一 union-find + HM 求解器风险极高、且会破坏已绿的 230 测试。

- **决策**:middle 引入 `constraint.rs`(`ConstraintGraph`),各 pass 从「检查即报错」改为「产出约束边(带跨维度 span)」;新增 `solve.rs` 协调器按 fixpoint 迭代,收敛后统一报告冲突。
- **替代**:全量重写(否决——回归面过大);仅文档宣称统一(否决——自欺)。

### D2: 持久化接线用 `im` 直接替换 Value 内集合

`persistent.rs` 已有 `PersistentValue`(Vector/HashMap/HashSet),但与 interpreter `Value` 是两套。合并两套会带来 hash/eq 派生负担。

- **决策**:让 interpreter 的 `Value::List` 改为 `im::Vector<Value>`、对象改为 `im::HashMap`;为 `Value` 派生/实现 `Hash + Eq`(引用语义或按结构);`persistent.rs` 的 `PersistentValue` 改为被 interpreter 复用(删除孤儿副本)。
- **替代**:保留两套、仅文档改口(否决——「全链路」要求运行时真实持久化)。

### D3: 模态/等级/子类型推理用「可判定即查、不可判定警告放行」

`□_r`/`◇_ε` 引入消去、五维子类型、依赖等级 `r+s` 传播、Cost 渐近推导,全部共享这一策略,与既有符号等级「不可判定时警告放行」一致,避免引入 SMT 依赖。

- **决策**:grade_check 增加半环加法(`r+s`)与子类型序;effect_infer 增加 `◇_ε` 消去;新增 `subtype.rs` 处理五维子类型;`Cost` 复用 grade 半环 + 渐近标记(`asymptotic`)做符号上界复合。
- **替代**:全量 Z3(否决——默认构建无 z3);仅语法(否决——正是本轮要补的缺口)。

### D4: Cohesive ♭/♯ 落地为「运行时可区分 + 上下文检查」

不做全模型 adjoint triple(超出 0.1.0 范围),但 ♭/♯ 不再直通。

- **决策**:`♭` 求值为「剥离结构」标记(对离散类型恒等、对含拓扑结构类型剥离)、`♯` 求值为 codiscrete 标记;`--typecheck` 检查 crisp 上下文与模态组合合法性;`--run` 结果与直通可区分。
- **替代**:纯直通(现状,否决);全模型(否决,Non-Goal)。

### D5: 立方填充限定有限维、复用 hott.rs

- **决策**:`hott.rs` 增 `kan_fill`(2-3 维面组合),边界一致性复用既有端点方程求解;`--typecheck` 对不一致边界报错。
- **替代**:完整 cubical type checker(否决,超出范围)。

### D6: LLVM 用 inkwell 真发射,非 llvm 回退文本 IR

inkwell 已是依赖(llvm feature)。`--ir` 当前有两套生成器(文本 `IrGenerator` + inkwell `llvm_generate`)。

- **决策**:在 inkwell 路径补齐 `define`/`call`/闭包环境打包;文本回退保持行为一致(可读伪 IR);`llc` 编译验证。函数调用不再回退 `ret i64 0`。
- **替代**:只修文本 IR(否决——§30 要求真编译)。

### D7: 系统级裸指针/区域/Unsafe 沿用「Unsafe effect + feature 门控」

- **决策**:新增 `ptr-read`/`ptr-write`/`with-region`/`region-alloc` 内置,签名携带 `[Unsafe]` 效应与线性指针等级;纯代码调用报效应缺失;`ffi` feature 关时这些内置报明确错误。
- **替代**:无门控裸指针(否决——违反声明式设计原则)。

### D8: TCO 用显式尾调用消除(解释器 eval 循环)

- **决策**:interpreter 的 `apply`/`eval` 对尾位置(函数体最后表达式、if 两分支、let 尾体)做迭代复用栈帧;多顶层 `__top__` 的 `Do` 包装改为逐表达式求值(不嵌套 Do)。
- **替代**:trampoline 全量改写(否决——侵入面大);仅加栈大小(否决——治标)。

### D9: 密码学用 RustCrypto,feature 门控,保留 XOR 作非加密回退

- **决策**:`spi` 加密/哈希在 `crypto` feature 下用 `aes`/`chacha20`/`sha2`;默认构建保留 XOR/简单 hash 并标注「非加密」,`--run` 输出警告。
- **替代**:默认引入 RustCrypto(否决——默认构建不应新增重量级依赖)。

## Risks / Trade-offs

- **[统一求解破坏既有推断]** → 增量收敛保留各 pass 测试基线,先让约束图「可叠加」再切「统一报错」,分步提交。
- **[Value 派生 Hash/Eq 改变相等语义]** → 为 `Value` 实现结构相等/哈希,补回归测试覆盖 map/set 键。
- **[模态/子类型推理误放行]** → 不可判定一律「警告放行」而非「报错」,宁可保守放行也不误报;误放行仅影响静态保证强度,不产生运行时错误。
- **[LLVM 闭包环境打包复杂]** → 先覆盖「无捕获闭包 + 有限捕获」子集,超出子集回退解释器路径并 `--ir` 提示。
- **[立方填充/Cohesive 语义可能过度设计]** → 严格限定有限维/可区分标记,保持 task 小步可验证。
- **[crypto feature 增加构建矩阵]** → 默认构建零新增依赖,crypto 单独 feature + CI 覆盖。

## Migration Plan

1. 每领域独立小步,每步保持 `cargo test --workspace` 绿 + 零警告(顺序见 tasks.md 分组)。
2. 文档/OpenSpec 收尾最后做(归档残留 change、刷状态标记),避免中途账目漂移。
3. 无破坏性 API 变更;新增 flag/内置为增量,不删既有行为。

## Open Questions

- (无——各决策已定,不阻塞 task 拆分)
