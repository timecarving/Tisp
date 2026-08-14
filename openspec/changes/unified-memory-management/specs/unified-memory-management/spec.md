## Purpose

定义统一内存管理模型:以线性类型(0/1/ω)+ 分级线性类型(□_r/@Cost)+ 手动 Unsafe 为单一所有权体系,使引用(Ref)、区域(Region)、裸指针(Unsafe)全部接入统一的 Grade + EffectRow,并由纯声明式副作用(代数效应/单子)管理。

## ADDED Requirements

### Requirement: 引用即分级值

`Ref a` 引用 SHALL 作为一等分级值接入等级系统:`{1 r : Ref a}` 为线性可变(写 `set!` 后句柄不可复用)、`{ω r : Ref a}` 为共享读、`{0 r : Ref a}` 编译期擦除;`ref`/`deref`/`set!` SHALL 建模为 `State` 效应操作,所有权由 grade_check 检查。

#### Scenario: 线性引用写后不可复用

- **WHEN** 程序以 `{1 r : Ref a}` 绑定引用并 `set!` 写入后再次使用 r,以 `--typecheck` 运行
- **THEN** 报告等级违反(线性句柄已被消费),不误放行

#### Scenario: 共享引用多次读

- **WHEN** 程序以 `{ω r : Ref a}` 绑定引用并多次 `deref` 读,以 `--typecheck` 运行
- **THEN** 类型检查通过(ω 级共享读)

### Requirement: 区域分级作用域

区域分配 SHALL 受等级/作用域约束:`with-region` 创建区域作用域、`region-alloc` 在区域内分配、退出时回收;区域内分配的指针 SHALL 不可逃出作用域(编译期逃逸检查),退出后访问 SHALL 报悬垂错误(运行时检测)。

#### Scenario: 区域逃逸报错

- **WHEN** 程序把 `region-alloc` 的分配地址作为函数返回值,以 `--typecheck` 运行
- **THEN** 报告区域逃逸错误

#### Scenario: 退出后悬垂报错

- **WHEN** `with-region` 退出后访问区域指针,以 `--run` 执行
- **THEN** 报告悬垂指针错误(而非静默返回默认值)

### Requirement: 手动 Unsafe 逃逸

`ptr-read`/`ptr-write` SHALL 经 `Unsafe` effect 门控:纯代码未经 handler SHALL 无法调用;1 级线性裸指针 SHALL 读写后不可复用;`Unsafe` SHALL 与等级系统一致(是声明式逃逸口,非命令式旁路)。

#### Scenario: Unsafe 门控

- **WHEN** 纯代码未经 handler 调用 `ptr-read`,以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误

#### Scenario: 线性裸指针

- **WHEN** 程序以 `{1 p : (Ptr a)}` 读写裸指针并消费,以 `--run` 执行
- **THEN** 读写正确,线性指针使用后不可复用

### Requirement: 纯声明式副作用

所有内存操作(分配/读写/回收)SHALL 经代数效应/单子管理:引用/区域/裸指针操作 SHALL 声明在效应行中,单处理器路径 SHALL 走 §12.6 直接状态线程;同一输入 SHALL 恒得同一输出(引用透明)。

#### Scenario: 引用透明

- **WHEN** 内存操作(分配 + 读写 + 回收)以纯函数组织并以相同输入执行两次
- **THEN** 结果一致,无副作用泄漏

### Requirement: 统一约束求解

等级/效应/区域 SHALL 由统一约束系统(共享约束图 + fixpoint)检查:引用所有权的等级约束、区域作用域约束、Unsafe 效应约束 SHALL 相互可见并联合求解;跨维度冲突 SHALL 报告带上下文的错误。

#### Scenario: 跨维度联合检查

- **WHEN** 引用的等级违反与区域逃逸同时发生,以 `--typecheck` 运行
- **THEN** 统一约束系统报告全部冲突(而非先到先报错)
