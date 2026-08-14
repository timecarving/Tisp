## ADDED Requirements

### Requirement: 裸指针与手动区域

系统级 SHALL 支持裸指针与手动区域(§26.2-26.4):`ptr-read`/`ptr-write` SHALL 以线性指针(1 级)读写裸内存并经 `Unsafe` 效应门控;`with-region` SHALL 创建区域、在区域内分配(`region-alloc`)、退出时回收;所有系统级操作 SHALL 要求 `Unsafe` 效应——纯代码未经 handler SHALL 无法调用;默认构建(无 `ffi` feature)下这些操作 SHALL 报明确错误而非静默回退。

#### Scenario: 线性裸指针读写

- **WHEN** 程序以 `{1 p : (Ptr a)}` 读写裸指针并消费,以 `--run` 执行
- **THEN** 读写正确,线性指针使用后不可复用

#### Scenario: 手动区域回收

- **WHEN** 程序以 `with-region` 分配并运行 f,退出后访问区域指针,以 `--typecheck` 运行
- **THEN** 区域退出后指针不可用(报告区域逃逸或悬垂错误)

#### Scenario: Unsafe 门控

- **WHEN** 纯代码未经 handler 调用 `ptr-read`,以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误

### Requirement: LLVM 函数与闭包代码生成

`--ir`/`--compile` SHALL 生成完整的函数定义、调用与闭包(§30):函数定义 SHALL 生成 LLVM `define`;函数调用 SHALL 生成真实 `call`(替换 `ret i64 0` 占位);闭包(捕获环境)SHALL 生成环境打包/解包结构;`llc` 编译验证 SHALL 通过。非 `llvm` feature 构建 SHALL 回退文本 IR 且行为一致。

#### Scenario: 函数调用生成

- **WHEN** 程序含函数定义与调用(如 `(f 42)`),以 `--ir` 运行并经 llc 编译
- **THEN** 生成真实 call 指令,llc 编译通过且运行结果与解释器一致

#### Scenario: 闭包生成

- **WHEN** 程序含捕获自由变量的闭包,以 `--ir` 运行
- **THEN** 生成环境打包与调用,llc 编译通过,结果与解释器一致

## MODIFIED Requirements

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
