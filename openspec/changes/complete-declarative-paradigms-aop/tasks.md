## 1. 运行时范式完整实现

- [x] 1.1 数组：多维数组创建/形状/索引/切片/map/reduce/沿轴求和全部经 `tisp_runtime::programming::Array`，越界索引返回显式错误（补 `Array::new_checked`/`index_checked`）
- [x] 1.2 栈：Stack 状态句柄 API（new/push/pop/peek/dup/swap/rotate）全部经纯函数栈变换，空栈 pop/peek 显式错误
- [x] 1.3 连接式：compose/apply/branch 高阶组合子保持点自由语义，参数类型错误显式报错
- [x] 1.4 符号：SymExpr 构造/代换/化简/求值，含自由变量的求值返回明确错误
- [x] 1.5 自动机：Dfa::accepts_checked 已存在，补齐自动机组合（并/串）与未声明符号错误；输入字符集非法显式报错
- [x] 1.6 状态机：StateMachine::drive 已存在，补多状态/entry-exit 动作与非法转移后状态不变测试
- [x] 1.7 数据驱动：DispatchTable 查表分发，缺失键显式报错（不再返回 None 静默）
- [x] 1.8 基于流：source/map/filter/take/sink 惰性流水线与 FRP `Signal` 接线，无限流取前 n 不卡死测试

## 2. 源码表面与声明式效应

- [x] 2.1 desugar：8 范式源码形式映射到 CoreExprNode 受控内置（数组/栈/自动机/状态机/数据驱动/基于流），非法形态报 DesugarError
- [x] 2.2 effect_infer：Stack/SM/DataDriven 操作注册 `State`，Stream 节点注册 `Signal`，Array/Sym/DFA/连接式为 Pure；Pure 定义调用状态操作被拒绝
- [x] 2.3 type_infer：8 范式完整内置签名（含 Stack a/SM/Table a 句柄类型），错误类型调用报类型错误
- [x] 2.4 grade_check：栈/状态机句柄按 QTT 等级检查（线性句柄移交后复用报错、ω 多读放行）
- [x] 2.5 interpreter：8 范式内置走 `perform_effect`/handler 栈或纯函数执行；非法输入返回 EvalError（无默认值）

## 3. 单子优化路径

- [x] 3.1 `effect_compile`：单处理器 State handler + `mlet/get-m/put-m/pure` 重写为直接状态线程（非仅计数）
- [x] 3.2 interpreter：monadic 状态线程与 handler 语义等价；`--run` 报告降级数量与实际路径
- [x] 3.3 补等价性测试：同一栈/状态机程序 effect 风格 vs monadic 风格输出一致；多处理器保持 handler 语义

## 4. pf-* 别名与语义一致性

- [x] 4.1 `pf-array-sum`/`pf-stack-top`/`pf-sym-eval`/`pf-dfa-accept`/`pf-sm-drive`/`pf-dispatch`/`pf-stream-take`/`pf-compose`/`pf-aop-weave` 改为调用完整内置同一实现
- [x] 4.2 移除/替换旧简化投影语义（sum%2、+100、默认 0）；对不可保留形式显式报错
- [x] 4.3 别名与完整内置的效应行/类型签名一致，补 `pf-*` 与完整内置结果等价测试

## 5. comptime 编译期 pass

- [x] 5.1 `tisp-backend::ComptimePass`：遍历 Comptime 节点，受限解释器编译期求值并替换回 AST（字面量/Data/闭包内联）
- [x] 5.2 编译期求值失败返回带 span 的编译错误；求值结果不可内联时显式报告
- [x] 5.3 CLI 在 desugar 后、静态检查前运行 ComptimePass；`--desugar`/`--typecheck`/`--run` 共用内联结果
- [x] 5.4 补 comptime 全链路测试：常量折叠、编译期错误、运行时不重复求值

## 6. 编译期 MOP 知识库

- [x] 6.1 ComptimePass 持独立编译期 KB（`tisp_core::evolp::Program`），comptime 内 `get-kb`/`set-kb` 读写该 KB
- [x] 6.2 编译期 KB 写入对同一编译单元后续宏展开/切面编织/类型检查可见
- [x] 6.3 编译期 KB 与运行时 KB 分离测试：运行时 `get-kb` 不包含编译期写入

## 7. AOP 编译期编织

- [x] 7.1 `(defaspect name (pointcut Gen) [:around|:before|:after] body)` 脱糖为 `AspectDef` 节点（含 pointcut 与 advice 类别）
- [x] 7.2 ComptimePass 执行切面编织：命中 MethodDef 集合按 around(注册序)→before→primary→after 重写，`call-next-method` 指向内层链，写回 CoreProgram
- [x] 7.3 `--desugar` 输出可见编织后的方法链；未命中方法不受影响；非法 pointcut 报编译错误
- [x] 7.4 编织与 specialize 交互：primary-only 可特化，含组合链保持运行时分发；补特化前后结果等价测试
- [x] 7.5 AOP 效应行合成：切面声明 State 时编织后方法链效应行含 State，Pure 切面保持 Pure

## 8. 示例、验收与文档

- [x] 8.1 为 8 范式各新增/更新 `.tisp` 示例（`examples/paradigm-matrix.tisp` 扩展），全部 `--typecheck` + `--run` 通过
- [x] 8.2 新增 AOP/comptime/MOP 示例（如 `examples/aop-mop.tisp`）：编译期 KB 写入 + 切面编织 + OOP 方法链，`--desugar`/`--run` 验证
- [x] 8.3 更新 `scripts/check-paradigm-matrix.sh` 覆盖 8 范式 + AOP/MOP + 非法输入拒绝 + monadic 等价
- [x] 8.4 全量验收：`cargo test --workspace` 全绿零警告、`cargo build --workspace` 零警告、`--features llvm,ffi` 构建通过、矩阵全 PASS
- [x] 8.5 同步 README/PLAN/CHANGELOG/standard_doc 与 OpenSpec 主规范（含旧 pf-* 投影语义变化的 BREAKING 说明）
