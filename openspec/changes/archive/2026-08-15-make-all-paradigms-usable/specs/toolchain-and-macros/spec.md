## ADDED Requirements

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

## MODIFIED Requirements

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
