# toolchain-and-macros

## Purpose

补全宏、工具链与系统级能力(§12.6/§22-26/§29/§30):宏卫生与 gensym、泛型编译期特化、编译指示全处理、真实动态库 FFI、反射函数真实化与 Monad 优化路径接线,使「宏/FFI/优化」工具链从骨架变为可用。

## Requirements

### Requirement: 宏卫生与 gensym

`defmacro` 展开 SHALL 避免捕获用户变量(卫生展开):展开中引入的绑定(含 `let`、`fn`/lambda 参数、`if-let`/`when-let` 绑定、`match` 模式变量)SHALL 不与调用点上下文冲突;syntax-quote 的 `~x`/`~@x` SHALL 正确绑定宏参数并参与展开;`gensym` 内置 SHALL 生成每次调用唯一的符号;同一宏展开多次 SHALL 产生互不冲突的符号。

#### Scenario: 无捕获展开

- **WHEN** 宏展开引入的绑定名与调用点同名变量并存
- **THEN** 两者互不干扰,展开后程序行为与卫生语义一致

#### Scenario: fn 参数卫生

- **WHEN** 宏模板内引入 `fn`/lambda 参数且调用点存在同名自由变量
- **THEN** 模板参数被重命名,不捕获调用点变量,展开后行为正确

#### Scenario: unquote 绑定宏参数

- **WHEN** 宏模板以 syntax-quote 的 `~x` 引用宏参数并以实参调用
- **THEN** `~x` 处替换为调用点实参,展开结果正确

#### Scenario: gensym 唯一性

- **WHEN** 同一宏在程序中展开两次且使用 gensym
- **THEN** 两次生成的符号互不相同,无变量冲突

### Requirement: 泛型编译期特化

GenericDef SHALL 在 middle 层被识别并可按参数**类型** monomorphize:对构造器类型(如 `Circle`)的调用 SHALL 特化为专用方法(不再走运行时分发);多参数调用 SHALL 按参数类型组合特化;特化 SHALL 作用于 `--run` 执行路径(非仅 `--typecheck` 展示);非特化调用保持运行时分发。`--typecheck` SHALL 报告特化数量。特化 SHALL 保持语义透明：含方法组合的泛型调用（around/before/after）经特化后运行结果 SHALL 与直接运行时分发完全一致。

#### Scenario: ground 类型特化

- **WHEN** 泛型函数以 `i64` 等具体类型调用且存在匹配方法,以 `--typecheck` 运行
- **THEN** 报告特化发生,运行结果与运行时分发一致

#### Scenario: 类型驱动特化

- **WHEN** 泛型函数以构造器类型实参(如 `area(circle)`)调用且存在匹配方法,以 `--run` 执行
- **THEN** 走特化路径,运行结果与运行时分发一致

#### Scenario: 多参数特化

- **WHEN** 多分派泛型函数以具体构造器类型组合调用(如 `collide(circle, rect)`),以 `--typecheck` 运行
- **THEN** 报告该调用特化,生成对应专用方法

#### Scenario: 非特化调用回退

- **WHEN** 泛型函数以无法静态判定类型的实参调用
- **THEN** 保持运行时分发,行为正确

#### Scenario: 方法组合语义保持

- **WHEN** 泛型函数含 `:around` 方法与 `call-next-method` 且调用可特化，以 `--run` 执行
- **THEN** 结果与未特化的运行时分发一致（如 around 翻倍结果），不得丢失 around 链
### Requirement: 编译指示全处理

编译指示 SHALL 有真实语义(§30,替换「仅语法接受」):`opt-level` SHALL 控制优化级别(实际改变优化器迭代次数/内联阈值,而非仅统计);`inline!`/`specialize!` SHALL 强制目标函数在优化器中内联/特化;`suppress-warning` SHALL 抑制指定警告;未识别的编译指示 SHALL 报错。

#### Scenario: opt-level 生效

- **WHEN** 程序声明 `(opt-level 2)` 且含可优化调用,以 `--typecheck` 运行
- **THEN** 优化器按更高迭代/内联阈值运行,优化统计反映更高优化级别

#### Scenario: inline 标记

- **WHEN** 函数标记 `(inline! f)` 且被调用,以 `--typecheck` 运行
- **THEN** 优化器强制内联 f,优化统计显示 f 被内联

#### Scenario: suppress-warning

- **WHEN** 程序声明 `(suppress-warning "grade")` 且含等级警告,以 `--typecheck` 运行
- **THEN** 对应警告被抑制

#### Scenario: 未知编译指示报错

- **WHEN** 程序使用未识别的编译指示,以 `--typecheck` 运行
- **THEN** 报告编译指示错误

### Requirement: 真实动态库 FFI

`defextern` SHALL 支持经动态库加载的真实函数(`libloading`):按名称从指定库解析符号并按 C ABI 调用;全签名 SHALL 支持——在 i64/f64 基础上补指针(整数地址透传)、字符串(UTF-8 → CString → 结果回转)与可变参;库不存在或符号缺失 SHALL 在调用时报明确错误;现有模拟函数表 SHALL 保留为回退。

#### Scenario: 动态库调用

- **WHEN** 声明外部函数指向 libc(如 `abs`)并以 `--run` 执行
- **THEN** 调用返回真实 libc 结果

#### Scenario: 字符串签名

- **WHEN** 外部函数接受/返回字符串(C 风格),以 `--run` 执行
- **THEN** 字符串正确转换(UTF-8 → CString → 结果回转)

#### Scenario: 指针签名

- **WHEN** 外部函数接受整数地址(指针透传),以 `--run` 执行
- **THEN** 地址值正确传递,结果正确

#### Scenario: 符号缺失报错

- **WHEN** 声明的外部符号在库中不存在并调用
- **THEN** 报告符号解析错误,不崩溃

### Requirement: 反射函数真实化

类型反射内置(`reflect-type`/`type-of`/`effects-of`/`determinism-of`/`grade-of`/`mode-of`)SHALL 返回真实静态类型与运行环境信息(名称、定义、参数、效果、等级、模式、确定性),替换硬编码占位与近似实现;`type-of` SHALL 返回静态推断类型(而非运行时值标签);`--typecheck` 通过的程序反射结果 SHALL 与静态信息一致。

#### Scenario: 反射真实类型

- **WHEN** 程序对已定义函数执行类型反射并以 `--run` 运行
- **THEN** 返回该函数的真实签名(与 `--typecheck` 输出一致)

#### Scenario: 反射效果与模式

- **WHEN** 程序对含效果/模式的函数执行 `effects-of`/`mode-of` 反射并以 `--run` 运行
- **THEN** 分别返回真实效果行与模式,而非常量 `"Pure"`/`"in"`

#### Scenario: type-of 返回静态类型

- **WHEN** 程序对某表达式执行 `type-of` 并以 `--run` 运行
- **THEN** 返回该表达式的静态推断类型(如 `i64`),而非运行时值标签

#### Scenario: 反射完整信息

- **WHEN** 程序对已定义函数执行反射以获取名称/定义/参数/效果/等级/模式/确定性,以 `--run` 运行
- **THEN** 全部字段返回真实信息,无近似或占位

### Requirement: Monad 优化路径接线

EffectCompiler SHALL 从「检测单处理器」扩展到「编译降级」:对可安全降级的单 handle 代码,`--typecheck` 与 `--run` 生成/执行等价的直接状态传递路径——状态在线程化中贯穿调用(替换计数占位);monadic 风格(`mlet`/`get-m`/`put-m`/`pure`)SHALL 编译为零开销状态链;不可降级时保持 handler 语义,行为 SHALL 与未优化一致。

#### Scenario: 单处理器降级

- **WHEN** 单处理器 handle 代码满足降级条件并以 `--run` 执行
- **THEN** 结果与 handler 语义一致,且实际走状态传递路径(非仅计数)

#### Scenario: monadic 风格编译

- **WHEN** 程序以 `mlet`/`get-m`/`put-m`/`pure` 编写状态热路径并以 `--run` 执行
- **THEN** 解析成功并执行,结果与 effect 风格等价

#### Scenario: 不可降级保持原义

- **WHEN** 嵌套/多处理器 handle 不满足降级条件
- **THEN** 按 handler 语义执行,结果正确

### Requirement: 裸指针与手动区域

系统级 SHALL 支持裸指针与手动区域(§26.2-26.4),并接入统一内存管理模型:`ptr-read`/`ptr-write` SHALL 以线性指针(1 级)读写裸内存并经 `Unsafe` 效应门控,所有权由 grade_check 检查(写后不可复用);`with-region` SHALL 创建分级区域作用域、在区域内分配(`region-alloc`)、退出时回收,区域内分配地址不可逃出作用域(编译期逃逸检查);所有系统级操作 SHALL 要求 `Unsafe` 效应——纯代码未经 handler SHALL 无法调用;默认构建(无 `ffi` feature)下这些操作 SHALL 报明确错误而非静默回退。

#### Scenario: 线性裸指针读写

- **WHEN** 程序以 `{1 p : (Ptr a)}` 读写裸指针并消费,以 `--run` 执行
- **THEN** 读写正确,线性指针使用后不可复用

#### Scenario: 手动区域回收

- **WHEN** 程序以 `with-region` 分配并运行 f,退出后访问区域指针,以 `--typecheck` 运行
- **THEN** 区域退出后指针不可用(报告区域逃逸或悬垂错误)

#### Scenario: Unsafe 门控

- **WHEN** 纯代码未经 handler 调用 `ptr-read`,以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误

#### Scenario: 区域逃逸编译期检查

- **WHEN** 程序把 `region-alloc` 的分配地址作为函数返回值,以 `--typecheck` 运行
- **THEN** 报告区域逃逸错误(统一等级/效应/区域检查)

### Requirement: LLVM 函数与闭包代码生成

`--ir`/`--compile` SHALL 生成完整的函数定义、调用与闭包(§30):函数定义 SHALL 生成 LLVM `define`;函数调用 SHALL 生成真实 `call`(替换 `ret i64 0` 占位);闭包(捕获环境)SHALL 生成环境打包/解包结构;`llc` 编译验证 SHALL 通过。非 `llvm` feature 构建 SHALL 回退文本 IR 且行为一致。

#### Scenario: 函数调用生成

- **WHEN** 程序含函数定义与调用(如 `(f 42)`),以 `--ir` 运行并经 llc 编译
- **THEN** 生成真实 call 指令,llc 编译通过且运行结果与解释器一致

#### Scenario: 闭包生成

- **WHEN** 程序含捕获自由变量的闭包,以 `--ir` 运行
- **THEN** 生成环境打包与调用,llc 编译通过,结果与解释器一致

### Requirement: --eval 真实求值

`--eval EXPR` SHALL 对表达式完成「读取 → 脱糖 → 静态检查 → 求值 → 输出结果」全链路；非法表达式 SHALL 报告对应诊断；SHALL 不得只打印读取数量。

#### Scenario: eval 求值表达式

- **WHEN** 以 `--eval '(+ 1 2)'` 运行
- **THEN** 输出 3（或含结果的求值输出），不出现未求值的“N form(s) read”

#### Scenario: eval 报错

- **WHEN** 以 `--eval '(+ 1 true)'` 运行
- **THEN** 报告类型错误并以非零码退出

### Requirement: --run 静态检查前置

`--run` SHALL 在执行前运行与 `--typecheck` 相同的全部静态检查；任一维度（类型/效果/等级/模式/确定性/区域/液态类型）失败 SHALL 拒绝执行并报告诊断；通过后执行的程序 SHALL 与直接解释一致。

#### Scenario: 类型错误拒绝执行

- **WHEN** 含类型错误的程序以 `--run` 运行
- **THEN** 不执行 main，报告类型错误

#### Scenario: 检查通过正常执行

- **WHEN** 合法程序以 `--run` 运行
- **THEN** 执行结果与现有正确行为一致，且额外报告静态检查通过信息

### Requirement: --compile 可执行语义

启用 `llvm` feature 时，`--compile FILE` SHALL 生成 LLVM IR、经 llc/clang 编译为可执行文件并运行，输出程序运行结果；编译或链接失败 SHALL 报告明确错误。未启用 `llvm` feature 时，`--compile` SHALL 报告「未启用 llvm feature」错误，不得只打印 IR 加提示。

#### Scenario: compile 运行程序

- **WHEN** 以 `--features llvm` 构建后对简单程序执行 `--compile`
- **THEN** 程序被编译并运行，输出结果（如 42）

#### Scenario: 缺 feature 显式报错

- **WHEN** 默认构建下执行 `--compile`
- **THEN** 报告需要 llvm feature 的错误

### Requirement: FFI 按声明签名安全分派

`defextern` SHALL 以显式或可推导的 ABI 签名（i64→i64、f64→f64、str→str/str→i64、指针透传）解析符号并调用；解析到的函数指针 SHALL 以该签名调用，不得先按 i64 签名试探导致其他签名函数被错误调用或崩溃；参数不匹配 SHALL 报告明确错误。默认构建（无 `ffi` feature）对声明了真实库路径的外部函数 SHALL 报告「未启用 ffi feature」错误；模拟回退仅允许用于显式标记为模拟的符号，且必须输出警告，不得对未知符号静默返回实参。

#### Scenario: 浮点签名正确

- **WHEN** 声明 `sin` 为 f64→f64 并传入 0.5，以 `--run` 执行
- **THEN** 返回约 0.479，而非 0 或实参直通

#### Scenario: 字符串签名正确

- **WHEN** 声明 `strlen` 为 str→i64 并传入 "hello"，以 `--run` 执行
- **THEN** 返回 5，进程不崩溃

#### Scenario: 签名不匹配报错

- **WHEN** 以 i64 签名声明 `sin` 并传入浮点实参，以 `--run` 执行
- **THEN** 报告参数/签名不匹配错误，不按错误 ABI 调用

#### Scenario: 默认构建显式报错

- **WHEN** 默认构建（无 ffi feature）执行带真实库路径的 `defextern` 调用
- **THEN** 报告未启用 ffi feature，不静默返回错误结果

### Requirement: 范式 monadic 状态链真实编译

`mlet`/`get-m`/`put-m`/`pure` SHALL 在 8 类范式的状态线程中真实可用：monadic 风格编写的栈/状态机/数据驱动程序 SHALL 经单处理器检测降级为直接状态传递并执行；结果 SHALL 与代数效应风格等价；`--run` SHALL 报告降级数量与实际路径，而非仅语法接受。

#### Scenario: monadic 栈程序

- **WHEN** 以 `mlet`/`get-m`/`put-m`/`pure` 编写数据栈操作并以 `--run` 执行
- **THEN** 执行结果与 effect 风格一致，且报告走直接状态线程

#### Scenario: 效应风格等价

- **WHEN** 同一栈/状态机程序分别以 handle/perform 与 monadic 风格编写并运行
- **THEN** 两种风格输出一致

### Requirement: comptime 工具链全链路

`comptime` SHALL 贯通工具链：lexer/reader 识别 → desugar 执行编译期求值与 MOP 编织 → `--desugar` 输出内联/编织后的 Core AST → `--typecheck` 检查内联后的程序 → `--run` 执行内联后的程序；`--run` 在执行前 SHALL 已完成 comptime 阶段，运行时不再次求值。

#### Scenario: 全链路 comptime

- **WHEN** 程序含 `comptime` 常量表达式，依次以 `--desugar`/`--typecheck`/`--run` 运行
- **THEN** desugar 显示内联结果，typecheck 通过，run 输出与内联语义一致

#### Scenario: 运行时不重复求值

- **WHEN** comptime 表达式带副作用（如编译期 KB 写入），以 `--run` 执行
- **THEN** 副作用只发生一次（编译期），运行阶段不重复执行
