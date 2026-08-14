# 设计阶段 + 深度缺口实现 — 任务清单

规范依据:5 个 delta(temporal-types 新增 + 4 个 modified);方案依据:design.md(D1-D17)。按领域分组,每组完成后 `cargo test --workspace` 全绿。

## 1. 类型系统(type-system-extensions)

- [x] 1.1 类型族 rewrite 状态同步(纯文档,上一轮已实现)
- [x] 1.2 类型一等值状态同步(纯文档,上一轮已实现)
- [x] 1.3 类型类 `:fun-deps`:defclass/definstance 解析 + 实例对冲突检测;测试
- [x] 1.4 类型类超类与 kind:超类方法存在性校验 + kind 校验;测试
- [x] 1.5 依赖会话:defsession 协议体引用依赖值;投影与顺序检查保持;测试
- [x] 1.6 Cost 注解:`@Cost` 等级语法 + 接线 `check_cost_bound`(上界检查/警告放行);测试
- [x] 1.7 类型系统测试固化与回归

## 2. HoTT 与时序(hott-and-deriving + temporal-types)

- [x] 2.1 fun-ext 内置(有限域点态比较);测试
- [x] 2.2 幺半等价内置(结合律/单位元枚举验证,反例给出);测试
- [x] 2.3 HIT 端点方程符号求解:边界等式经 CLP 验证可满足性;测试
- [x] 2.4 Cohesive 连通图:shape-graph 内置(节点+连通边);测试
- [x] 2.5 时序时钟:`(clock name rate)` 注册替换 ClockNew 占位;`(next A)` 时刻语义;测试
- [x] 2.6 always/eventually 流判定(有限窗口);测试
- [x] 2.7 LTL-as-types:时序类型与流操作匹配检查(advance 输入 next 值);测试
- [x] 2.8 多时钟重采样(按速率抽值)+ 时钟不匹配报错;测试
- [x] 2.9 HIT hott.rs 接线:解释器引用 `tisp-runtime::hott`(Interval/PathTerm/Circle),替换内联占位;测试
- [x] 2.10 deriving 移 desugar:生成 `eq-*`/`ord-*`/`show-*` 函数定义,`--desugar` 可见;未知 trait/不可派生字段报错;测试
- [x] 2.11 演算互模拟:迹等价比较器,接在编码结果与原项之间(替换 π→SKI 特例);测试
- [x] 2.12 HoTT/时序测试固化与回归

## 3. 工具链(toolchain-and-macros)

- [x] 3.1 dlopen 字符串签名:UTF-8 ↔ CString 转换;测试
- [x] 3.2 dlopen 指针签名:整数地址透传;测试
- [x] 3.3 编译指示 opt-level:优化器迭代/内联阈值参数化;`--typecheck` 统计反映;测试
- [x] 3.4 inline!/specialize! 标记:优化器强制内联/特化;测试
- [x] 3.5 suppress-warning:警告过滤(等级/液态);未知编译指示报错;测试
- [x] 3.6 Monad 直接状态线程:解释器状态槽线程化(替换 ActiveHandler 栈/计数占位);对拍测试
- [x] 3.7 工具链测试固化与回归

## 4. 验证(logic-and-verification)

- [x] 4.1 dolev-yao 攻击者知识:窃听/转发/合成/解密规则;find-attack 扩展;测试(已知漏洞命中)
- [x] 4.2 check-equivalence 保持 + dolev-yao 集成;测试
- [x] 4.3 Prolog 完整续延回溯:Search 返回解流 + 选择点重入(接线 logic.rs 引擎);测试
- [x] 4.4 验证测试固化与回归

## 5. 文档与验收

- [x] 5.1 04 清单:⬜ 10 项升级 + ⚠️ 5 项升 ✅ + 过时条目同步(类型族 rewrite/类型一等值)
- [x] 5.2 standard_doc 01/02/03:新语法与内置(clock/fun-ext/monoid-check/shape-graph/编译指示/Cost 注解)增补
- [x] 5.3 示例:各领域综合示例;examples 索引更新
- [x] 5.4 `CHANGELOG.md` 记录;README 同步
- [x] 5.5 最终验证:`cargo test --workspace` 全绿、`cargo check --workspace` 零警告、示例抽查、04 状态核对
