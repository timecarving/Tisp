# 全部缺口补齐 — 设计

## Context

动机与范围见 `proposal.md`;行为要求见 4 个能力 spec(`type-system-extensions` / `logic-and-verification` / `toolchain-and-macros` / `hott-and-deriving`)。关键现状(2026-08 实测):

- 类型系统:`Type::Pi/Sigma` 变体已存在(§19.1 语法/显示/推断统一),缺等级维传播;`MetaQuery` 节点存在但硬编码返回 `"meta"`;`defresource-algebra` 走 `desugar_stub_defn` 占位;`:deriving` 只收集名字;grade_check 已有 0/1/ω 跟踪
- 逻辑:`constrain` 挂恒真 propagator;`abduce` 返回占位字符串;Search 节点已有多解收集(solve-all/find-all)
- 工具链:`defextern` 走模拟 C 函数表(abs/strlen/sqrt);宏展开无 hygiene;`EffectCompiler::detect_single_handler` 已接入 cli 但只检测不编译;GenericDef 运行时分发可用
- HoTT:`FlatMod/SharpMod` 直通求值,无 ʃ(Shape)节点;HIT 的 `:boundary` 被忽略
- 文档:`standard_doc/04-implementation-status.md` 章节号错位(23 章起 -1)、两张清单大面积过时;`docs/spec.md` 附录 A/B/F 与实现脱节
- 约束:全量 139 测试保持全绿、`cargo check --workspace` 零警告、默认构建可用(LLVM/Z3 风格 feature 门控)、注释简体中文

## Goals / Non-Goals

**Goals**
- 按 A→B→C→D 四组补齐 20 项功能缺口,每组独立提交、可验证
- 文档修复随实施同步(每组合并更新 CHANGELOG/04),最后统一做 spec 附录 + 状态内联
- 保持默认构建可用:dlopen FFI 用 feature 门控(`ffi` feature),与 `llvm` 风格一致

**Non-Goals**
- 类型系统结构级重构(如给 Type::Pi 加等级字段)——等级传播在 grade_check 层实现,避免全链改动
- Cohesive HoTT 的完整同伦语义(ʃ 以最小可区分语义落地,见 D7)
- 攻击搜索的密码学完备性(有限深度符号执行,标注为教学级)
- 性能优化(特化/降级以行为正确为准,不做基准)

## Decisions

### D1:实施顺序 — A(类型系统)→ B(逻辑/验证)→ C(工具链)→ D(HoTT/小件),文档组 E 贯穿

依赖关系:A 的 desugar/type_infer 改动为其他组铺路(类型族/一等值被反射用到);B 独立(runtime 层);C 的 gensym 被 D 的 deriving 生成用到(生成代码需要唯一符号);D 的 Cohesive 最大件放组尾,失败可隔离。E 文档随每组提交同步 04 对应条目,最后统一修 spec 附录与状态内联。

### D2:QTT 擦除与移动 — 解释器层擦除 + grade_check 扩展线性检查

0 级参数擦除在 `interpreter.rs` 的 apply/let:0 级绑定不求值、不入闭包 env(闭包捕获时跳过 0 级自由变量)。1 级移动检查扩展 `grade_check.rs`(已有使用计数):1 级绑定第二次引用报错(移动语义)。Type 结构不动。
**备选**:codegen 层擦除。否决:当前主执行路径是解释器,codegen 未覆盖全语言。

### D3:多模式谓词 — 模式签名表 + mode_analysis 扩展

desugar 解析 `:mode (i, o)` 风格注解存 CoreDef(新增 `modes: Vec<ModeSig>` 字段或并行 HashMap);`mode_analysis.rs`(已有 `ModeAnalyzer`)扩展:调用点按实参 free/ground 匹配模式,无匹配报错。运行时:现有 `:free` 参数已走逻辑变量路径,多模式只影响检查层。
**备选**:合并进 type_infer。否决:与现有 mode_analysis 职责重叠。

### D4:类型族 — 新变体 + 实例表 + 调用点归约

`types.rs` 加 `TypeFamily(Symbol, Vec<Type>)` 变体与 `TypeFamilyInstance` 结构;desugar 解析 `(typefamily 名称 参数模式 结果)` 声明;type_infer 维护实例表,遇到类型族应用尝试归约,失败报错(严格模式,符合 spec)。显示与 unify 加显式分支(避免影响既有匹配的穷尽性)。
**风险**:类型系统核心改动 → 所有 `match Type` 点需补分支;编译器会强制暴露遗漏(穷尽性检查),回归风险可控。

### D5:类型一等值 — 复用 MetaQuery 节点真实化

不新增节点:扩展 `MetaQuery` 求值(interpreter.rs:1543 处)——按查询目标返回真实类型显示(`Value::Str` 或新 `Value::Type`)、环境信息;`Value::Type` 变体加入 `Value` 枚举(interpreter 与 runtime 共享的 value 定义在 tisp-runtime 或 backend?——按现有 `Value` 定义位置定,最小改动)。`--typecheck` 输出与反射结果同源(共用类型显示函数)。

### D6:等级传播 — grade_check 层实现 r+s

不给 `Type::Pi/Sigma` 加字段(避免全链重构):grade_check 在检查依赖绑定时按「绑定等级 + 使用次数」实现 r+s 线性约束,违反报错。`--typecheck` 输出等级信息。spec 的「Π/Σ 携带等级维」以检查语义满足,结构不变。
**备选**:Type 加等级字段(更大改动,收益低)。

### D7:Cohesive 最小语义 — ʃ 作为路径代数容器 + crisp 检查

spec §17 只有概念,无操作语义。实现最小可区分语义:ʃ 求值 = 对值构造「区间端点对」(i0/i1 与 Path 数据关联),`♭` 仅在 crisp 上下文允许解包(类型检查在 type_infer 加 crisp 标记传播),`♯` 保持受限直通。`--typecheck` 输出 crisp 违规错误。
**风险**:语义未定义 → 以「可区分 + 不破坏现有程序」为验收标准,并在 04 文档标注实现语义为最小近似。

### D8:dlopen FFI — `ffi` feature 门控 + libloading

`defextern` 支持 `(defextern name "lib.so" "sym")` 形式;`ffi` feature 下用 libloading 解析并调用(C ABI:整数/浮点/指针参数转换),默认构建回退模拟函数表(现状)。符号缺失报错不崩溃。feature 名与 `llvm`/`z3` 风格一致。

### D9:Monad 优化接线 — 检测→标注→状态传递求值

三阶段落地:① 现状检测(已有);② `--typecheck` 输出「可降级」标注(已有 monad_candidates);③ 解释器对单处理器 get/put 状态效果走直接状态传递求值路径(不建 monad 库,直接状态传递即语义等价)。嵌套/多处理器保持 handler 语义。

### D10:find-attack — ModelChecker 扩展 + 攻击者知识

复用 `process.rs` 的 `ModelChecker`(可达性搜索已有):攻击者建模 = 拦截/转发/重放轨迹探索;`find-attack` 在深度限制内找「机密泄露/角色偏离」轨迹;`check-equivalence` 比较两进程的状态可达集。教学级(不做密码学完备)。

### D11:deriving 生成 — desugar 层生成函数定义

`defdata` 的 `:deriving (Eq, Ord, Show)` 在 desugar 时生成对应函数定义(结构递归比较/打印,构造器序定 Ord),使用 gensym 保证不冲突;含函数字段报错。纯 desugar 工作,零运行时改动。

### D12:演算互编码 — runtime 转换函数

`tisp-runtime/src/process.rs` 加转换函数:π 通道操作 → SKI 组合子应用序列(S/K/I 已有);ambient enter/exit/open → 通道消息编码。转换结果仍走解释器执行,验收 = 观察等价(现有效果操作语义)。

## Risks / Trade-offs

- [类型系统核心改动(类型族/一等值)破坏既有行为] → 显式 match 分支 + 每组合并全量测试(139+);类型族挂起报错为严格模式,回归由测试兜底
- [Cohesive 语义未定义导致实现漂移] → 最小可区分语义 + 文档标注;验收以「不破坏现有程序 + 可区分」为准
- [dlopen 平台差异] → `ffi` feature 门控 + 回退模拟表;仅 linux 验证
- [大变更的文档同步压力] → 文档随每组提交增量同步,最后统一 spec 附录;避免一次性大重写
- [宏卫生改变既有展开行为] → 卫生化只对新增绑定生效,已存在宏测试(27 个 frontend 测试)回归验证
- [CLP/ALP 传播改变求解顺序] → 语义以「解集正确」验收,不做求解顺序保证

## Migration Plan

无部署概念。实施路径:每组(功能 + 测试 + CHANGELOG/04 增量同步)独立提交;组 E 文档收尾(spec 附录 + 状态内联)最后提交。回滚:git revert 对应组提交。feature 说明:`ffi` feature 默认关闭,`--features ffi` 启用真实 dlopen。

## Open Questions

- Cohesive ʃ 的具体操作语义(spec §17 无定义):以「最小可区分语义」实施并在 04 标注近似度,不阻塞本变更的 spec 与任务拆分。
- 类型族语法细节(spec §9 无 BNF):以设计提案语法 `(typefamily 名称 参数模式 结果)` 为准,实施时同步进 spec 附录 A。
