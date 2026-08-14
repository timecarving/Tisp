## ADDED Requirements

### Requirement: 编译指示全处理

编译指示 SHALL 有真实语义(§30,替换「仅语法接受」):`opt-level` SHALL 控制优化级别(影响优化器迭代/内联阈值);`inline!`/`specialize!` SHALL 标记目标函数强制内联/特化;`suppress-warning` SHALL 抑制指定警告;未识别的编译指示 SHALL 报错。

#### Scenario: opt-level 生效

- **WHEN** 程序声明 `(opt-level 2)` 且含可优化调用,以 `--typecheck` 运行
- **THEN** 优化统计反映更高优化级别(如更多内联)

#### Scenario: inline 标记

- **WHEN** 函数标记 `(inline! f)` 且被调用,以 `--typecheck` 运行
- **THEN** 优化统计显示 f 被强制内联

#### Scenario: suppress-warning

- **WHEN** 程序声明 `(suppress-warning "grade")` 且含等级警告,以 `--typecheck` 运行
- **THEN** 对应警告被抑制

#### Scenario: 未知编译指示报错

- **WHEN** 程序使用未识别的编译指示,以 `--typecheck` 运行
- **THEN** 报告编译指示错误

## MODIFIED Requirements

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
