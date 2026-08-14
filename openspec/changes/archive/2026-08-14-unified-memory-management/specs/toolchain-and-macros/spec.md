## MODIFIED Requirements

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
