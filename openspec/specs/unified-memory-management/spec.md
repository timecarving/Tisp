# unified-memory-management

## Purpose

定义统一内存管理模型:以线性类型(0/1/ω)+ 分级线性类型(□_r/@Cost)+ 手动 Unsafe 为单一所有权体系,使引用(Ref)、区域(Region)、裸指针(Unsafe)全部接入统一的 Grade + EffectRow,并由纯声明式副作用(代数效应/单子)管理。

## Requirements

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

### Requirement: 范式运行时值统一区域跟踪

范式运行时持有的可变状态（逻辑变量表/回溯 trail、CLP 域存储、通道缓冲、流/信号缓存、进程运行时、知识库）SHALL 经统一区域栈分配并在作用域退出时回收；`--run` 的区域统计 SHALL 计入这些分配；范式程序重复执行 SHALL 不产生跨次残留。

#### Scenario: 范式分配计入统计

- **WHEN** 运行创建通道/逻辑变量/流并退出的程序
- **THEN** 区域统计的分配与回收次数大于 0 且配对，统计随范式状态规模变化

#### Scenario: 重复执行无泄漏

- **WHEN** 同一范式程序连续执行两次
- **THEN** 两次区域统计一致，第二次不高于第一次

### Requirement: 范式状态接入完整统一内存体系

范式状态 SHALL 以「Unsafe + 依赖线性类型 + QTT + 分级线性类型」为唯一所有权与资源体系接入：范式句柄（流、通道、逻辑存储、知识库）SHALL 是分级值，0 级句柄编译期擦除、1 级句柄移交后不可复用、ω 级句柄共享读；携带值依赖（长度 n、时钟、容量）的范式结构 SHALL 由依赖等级参与检查；范式操作的资源上界 SHALL 可经 `□_r`/`@Cost` 声明并在检查中判定；裸指针访问范式状态 SHALL 经 `Unsafe` 门控。区域栈仅是该体系之下的底层分配/回收载体。

#### Scenario: 线性通道句柄移交

- **WHEN** 程序以 `{1 c : (Chan Int)}` 绑定通道并在 `send!` 移交后再次使用 c，以 `--typecheck` 运行
- **THEN** 报告等级违反错误

#### Scenario: 依赖线性流缓冲

- **WHEN** 程序以依赖长度 n 声明流缓冲并超过 n 消费，以 `--typecheck` 运行
- **THEN** 依赖等级检查报告违反，不静默放行

#### Scenario: 分级代价上界

- **WHEN** 范式操作以 `@Cost`/`□_r` 声明资源上界且实际使用超界，以 `--typecheck` 运行
- **THEN** 报告代价/等级违反，或明确警告

#### Scenario: Unsafe 访问范式状态

- **WHEN** 纯代码以 `ptr-read`/`ptr-write` 访问范式内部存储且无 `Unsafe` handler，以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误

### Requirement: 范式内存错误显式化

范式持有的内存状态在生命周期外被访问（已回收通道/流/逻辑存储）SHALL 报告悬垂错误；指针/区域原语与范式状态之间的转换 SHALL 保持等级（0/1/ω）与效应行一致，不得绕过 `Unsafe`/`State` 门控。

#### Scenario: 已回收状态访问报错

- **WHEN** 通道或流在其作用域/区域退出后仍被引用访问，以 `--run` 执行
- **THEN** 报告悬垂/已回收错误，不返回默认值
