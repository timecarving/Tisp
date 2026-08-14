## Why

前几轮把大量特性以「运行时模块 + 语义助手」形式落地并标注 ⚠️(语言表面接线缺),但这不等于完成:部分特性仍停留在语义演示或只接入了一两层,无法从 Tisp 源码端到端运行。本变更澄清——**该完全实现就必须完全实现**,不因「组合优先」而降低标准。目标是把一切「部分实现(⚠️)/未实现(⬜)」特性继续实现下去,贯通 lexer → reader → desugar → 类型/效应/等级 → 解释器/代码生成,直到全链路可用。

## What Changes

- **六维注解语法**:`def`/`defn` 的 `->[ε,ρ,@r,m,d]` 签名解析并贯通 `FunAnnotation`(§6.6)。
- **私有定义语义**:`defn-`/`def-` 的可见性在跨文件加载与 `ns` 引用中强制(§6.5)。
- **deriving Ord**:`defdata :deriving Ord` 生成 `ord-*` 结构排序函数(§7.5)。
- **分级模态推理**:□_r/◇_ε 引入/消去接入 type_infer/unify(§11)。
- **Cost 全推导**:`@Cost` 注解语法 + 渐近代价复合推导接入 grade_check(§11.1)。
- **HoTT 完整立方填充**:HComp 扩到多维 Kan 填充(§16)。
- **Cohesive 完整同伦模型**:♭/♯/ʃ 的 adjoint-triple 全语义(§17)。
- **时序 □_t 语义保证**:稳定类型跨时刻检查 + 因果性/生产率/空间回收(§18)。
- **依赖等级传播**:r+s 对有限等级真实求解(替换对 ω 绑定恒过)(§19)。
- **编译期区域逃逸检查**:with-region 作用域逃逸判定(§26)。
- **inkwell 闭包代码生成**:函数 define/call + 闭包环境打包/解包,llc 验证(§30)。
- **24 个范式全链路接线**:EVOLP/DLP/MOP、12 逻辑范式、8 编程范式、AOP 从源码可书写并贯通全链路(§31/§32)。
- **统一约束求解与演算统一收尾**:六维约束共享求解 + 演算统一(§2)。

## Capabilities

(无新增/修改能力——本变更为既有需求的**实现完成**,不改变 spec 级行为;`.openspec.yaml` 已设 `skip_specs: true`。上述各项的 spec 已存在于 `docs/spec.md` 与各主/增量规范中。)

## Impact

- **tisp-frontend**:lexer/reader/desugar 补六维注解、私有定义、deriving Ord、范式关键字与 AOP 语法。
- **tisp-middle**:type_infer(□_r/◇_ε 推理、范式类型)、grade_check(依赖等级 r+s、Cost 全推导)、region_infer(逃逸检查)、约束求解统一。
- **tisp-backend**:interpreter(范式求值经 `ParadigmRegistry`)、codegen(inkwell 闭包)、hott/temporal(立方填充、□_t 保证)。
- **tisp-runtime**:范式设施经 `ParadigmFacility` 接入 interpreter;语义助手并入求值器内核。
- **tisp-core**:补齐范式 AST/类型构造与统一值/类型抽象。
- **docs/spec.md** / **standard_doc** / **CHANGELOG**:⚠️ 项升级为 ✅,同步实现状态。
