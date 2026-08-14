## Context

动机与范围见 `proposal.md`(⬜ 10 项 + ⚠️ 5 处深度缺口 + deriving 移 desugar)。本变更吸收 `implement-design-stage-features`(其 D1-D11 决策沿用,见下),并新增深度缺口决策。关键现状:上一轮 `finish-partial-features` 已落地「已实现」半边,本变更补「缺」半边——hott.rs 接线、Cost 注解、Monad 状态线程、Prolog 续延回溯、演算互模拟、deriving 移 desugar。

## Goals / Non-Goals

**Goals**
- 10 项 ⬜ + 5 处深度缺口 + deriving 全部实现至 ✅(04 全绿,无 ⬜ 项)
- 04 清单过时条目同步(类型族 rewrite/类型一等值已由上一轮完成)

**Non-Goals**
- 完整形式化语义(HoTT 公理化、LTL 完整时序逻辑)——以「端点/时刻语义一致」为验收
- 协议安全的形式化证明(教学级攻击者模型/互模拟)
- 统一约束求解(§2 Principle 3)与 LLVM 真编译链——属 PLAN.md 另一套后续方向,不在本变更

## Decisions

### D1-D11:沿用 implement-design-stage-features(⬜ 项)

类型族 rewrite=实例递归归约(D1)、类型一等值纯文档(D2)、类型类 fun-deps/超类/kind(D3)、依赖会话值依赖(D4)、fun-ext/幺半等价有限域枚举(D5)、HIT 端点 CLP 验证(D6)、Cohesive 连通图(D7)、时序时刻化流(D8)、dlopen 全签名(D9)、编译指示接线优化器(D10)、dolev-yao 知识合成(D11)。

### D12:HIT hott.rs 接线 — 解释器引用模块,替换内联占位

`tisp-runtime/src/hott.rs` 已导出(Interval/PathTerm/Circle)。解释器的内联 HoTT(IntervalEndpoint→Bool、HComp→KanFill、Shape→连通)改为引用 hott.rs 类型:区间值经 `hott::Interval` 表示,路径值经 `hott::PathTerm` 表示,HIT 构造器经 `hott::Circle`(Base/Loop)构造。**理由**:消除死代码与内联重复;现有 HoTT 测试(内联语义)保持通过。

### D13:Cost 注解与推导 — `@Cost` 等级 + grade_le 上界

`@Cost` 作为等级注解(复用 `@` 前缀语法),声明了 `:asymptotic` 代数的 Cost 等级参与 `check_cost_bound`(grade_le 上界);可判定时报错,符号/不可判定时警告放行。**理由**:`grades.rs` 的 `from_declared`/`check_cost_bound` 已就位,补注解语法与推导接线即可。

### D14:Monad 直接状态线程 — 解释器状态槽线程化

单处理器 + 无嵌套时,解释器不走 ActiveHandler 栈,改为直接状态槽:handler 状态存解释器字段,`perform get/put` 直接读写该槽;续延 `k` 内联展开。**理由**:替换「计数占位」,让 §12.6 的零开销承诺真实落地;monadic 语法(D14 前置,上一轮已 desugar)复用同一状态槽后端。

### D15:Prolog 完整续延回溯 — Search 返回解流

Search 节点改为续延式:每个 Match arm 成功即产出解并挂起续延,`find-all` 逐续延重入收集全部解;选择点栈 + trail 恢复复用 `logic.rs` 的 `backtrack`/`restore_to`。**理由**:替换「首解/全局累加器」,修复递归/or/结构化参数的 0 解与垃圾解;分支隔离(上一轮已做局部 trail 快照)升级为逐分支解流。

### D16:演算互模拟 — 迹等价比较器

在 `process.rs` 为演算项加迹等价比较:比较通道 I/O 迹(或 barbed 等价),接在编码结果与原项之间;`check_observational_equivalence`(上一轮 π→SKI 特例)推广为任意演算项的迹比较。**理由**:5 编码已齐,补「保持观察等价」的可执行检查。

### D17:deriving 移 desugar — 生成 eq-/ord-/show- 函数

deriving 从运行时内置移到 desugar:解析 defdata 的 `:deriving` 时生成结构递归的 `eq-*`/`ord-*`/`show-*` 函数定义(进 CoreProgram.defs),`--desugar` 可见;含函数字段或未知 trait 报错。**理由**:满足 spec §7.5「编译期生成」与「--desugar 可见」;替换运行时内置的不可见/静默忽略。

## Risks / Trade-offs

- [hott.rs 接线改变区间表示] → 现有 HoTT 测试兜底;区间仍以 Bool 对外,内部 Interval 化
- [Cost 注解语法歧义] → 复用 `@` 前缀,记录于 04 文档
- [状态线程化改变 handler 语义] → 对拍测试(状态线程 vs 通用 handler 输出一致)
- [续延回溯重写 Search] → 默认 DFS 首解行为不变,多解经 find-all/solve-all 暴露;现有单解测试保持
- [deriving 移 desugar 改变 --desugar 输出] → 更新受影响测试与示例
- [dolev-yao/互模拟教学级] → 攻击者规则集/迹等价限定,04 标注

## Migration Plan

无部署概念。实施顺序:类型系统(依赖会话/类型类/Cost)→ HoTT/时序(hott.rs/fun-ext/连通图/deriving)→ 工具链(dlopen/编译指示/Monad)→ 验证(dolev-yao/续延回溯/互模拟)→ 文档与 04 同步。回滚:git revert。归档顺序:先 `finish-partial-features`,再本变更(存在 delta 重叠)。

## Open Questions

- 无。各「最小语法/语义」落地均记录于 04 文档(近似度透明)。
