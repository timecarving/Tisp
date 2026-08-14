# 部分实现特性全链路化 — 任务清单

规范依据:3 个 MODIFIED delta;方案依据:design.md。按能力分组,每组完成后全量测试。

## 1. Z3 验证类(等级不等式 + HIT 端点方程)

- [x] 1.1 grade_check 符号等级不等式收集(诊断信息:var/count/span);实施修正:自由等级变量无法静态判定(任何 count 都有 n=0 反例),Z3 严格验证语义不成立,spec/design 已同步修正
- [x] 1.2 LiquidVerifier 等级诊断:自由符号等级记录诊断警告(含使用次数),不误报;数字/可折叠复合等级由 grade_check 常量检查保持严格
- [x] 1.3 用例:自由符号等级通过 + 诊断警告;常量等级违反仍报错(grade_le 路径回归)
- [x] 1.4 HIT 端点方程求解:`:boundary` 等式经 verify_implication 验证可满足性;不可满足 → 边界违反(替换符号检查);测试
- [x] 1.5 Z3 类测试固化与全量回归

## 2. CLP 算术约束编译

- [x] 2.1 constraint.rs 乘法/除法/模传播器(add_mul/add_div/add_mod):域收缩;冲突失败
- [x] 2.2 all-different 全局约束传播器:变量间互斥;排列枚举
- [x] 2.3 clp_constraint 分发扩展:op 表补 * / % all-different;测试(乘法约束解集、全不同排列)
- [x] 2.4 CLP 测试固化与回归

## 3. ALP 多解枚举

- [x] 3.1 abduce-all(多解解释枚举):收集全部一致解释,每解释独立可验证;测试
- [x] 3.2 不可满足原因报告:全部候选失败时返回原因(失败假设数);测试
- [x] 3.3 ALP 测试固化与回归

## 4. 类型系统扩展

- [x] 4.1 Value::Type 变体:Value 枚举加变体;编译器穷尽性暴露 match 点,分批补全(interpreter 求值/转换路径);测试
- [x] 4.2 reflect-type 返回 Value::Type:类型值可绑定/传递/比较(相等性语义);兼容显示;测试
- [x] 4.3 类型族多模式:TypeFamilyInstance 多实例支持;归约按声明序匹配多模式;测试
- [x] 4.4 多模式自动推断:未声明 :mode 的谓词按调用形态自动收集签名;冲突报错提示;测试
- [x] 4.5 隐式绑定默认 0(实施修正):Map 无等级参数存在语法歧义(等级符号 vs 绑定名不可区分),自动默认 0 不落地(记录已知限制);`{0 x : T}` 擦除+引用报错语义确认(既有测试)
- [x] 4.6 类型系统测试固化与全量回归(182 基线)

## 5. HoTT 真实求值

- [x] 5.1 Cohesive 形状代数:ShapeMod 对 Path 计算端点连通(shape-connect 风格);替换最小容器;测试
- [x] 5.2 HComp 真实求值:沿路径填充返回边界一致值(路径 lam 端点求值);测试
- [x] 5.3 Transp 真实求值:沿路径传输返回目标端点值;测试
- [x] 5.4 HoTT 测试固化与回归

## 6. 文档与验收

- [x] 6.1 `standard_doc/04-implementation-status.md`:相关章升级 ✅(§9 类型系统/§10 QTT/§13 模式/§16 HoTT/§17 Cohesive/§21 逻辑/§22 泛型);其余保持
- [x] 6.2 `standard_doc/01-language-core.md`/`02-advanced-features.md`:新语法与语义增补(等级验证、Value::Type、算术约束、abduce-all、形状连通)
- [x] 6.3 示例:为各组新增能力各加 1 个示例(通过型);examples 索引更新
- [x] 6.4 `CHANGELOG.md` 记录;README 测试数与示例同步
- [x] 6.5 最终验证:`cargo test --workspace` 全绿、零警告、示例 `--typecheck`/`--run` 抽查、04 状态核对
