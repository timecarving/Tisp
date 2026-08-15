## ADDED Requirements

### Requirement: 设施元数据强制

每个经 ParadigmRegistry/Facility 注册的范式设施 SHALL 携带完整的六维元数据（类型构造、效应行、区域、等级、模式、确定性）与声明式来源标记；缺失或占位元数据的设施 SHALL 不得注册成功；type_infer 对范式内置的签名 SHALL 从该元数据生成，而非手写单态补丁。

#### Scenario: 元数据完整注册

- **WHEN** 注册全部范式设施并以 `--typecheck` 编译调用范式内置的程序
- **THEN** 每个设施的六维元数据齐全，类型/效应/确定性检查结果与元数据一致

#### Scenario: 缺失元数据拒绝

- **WHEN** 某设施未声明效应或等级元数据而尝试注册
- **THEN** 注册失败并报告缺项，不得以默认占位放行

### Requirement: 范式执行经统一内存跟踪

范式设施的执行 SHALL 经统一内存体系约束：设施句柄与状态 SHALL 携带 QTT 等级（0/1/ω）并参与 grade_check；值依赖结构经依赖线性类型检查；资源上界经 `□_r`/`@Cost` 判定；`Unsafe` 访问经效应门控；底层分配与回收经统一内存入口并由 `--run` 区域统计反映。重复执行同一范式程序 SHALL 不累积未回收状态。

#### Scenario: 分配回收一致

- **WHEN** 同一含范式状态的程序连续执行两次
- **THEN** 每次的区域统计一致（分配与回收配对），无跨次泄漏

#### Scenario: 设施句柄等级检查

- **WHEN** 范式设施句柄以线性等级使用后复用，以 `--typecheck` 运行
- **THEN** 报告等级违反，设施元数据中的等级信息与检查结果一致

#### Scenario: 设施资源上界

- **WHEN** 设施声明 `□_r`/`@Cost` 资源上界且调用超界，以 `--typecheck` 运行
- **THEN** 报告资源违反或明确警告

#### Scenario: 设施 Unsafe 门控

- **WHEN** 设施内部存储被裸指针访问且无 `Unsafe` 效应声明，以 `--typecheck` 运行
- **THEN** 报告 `Unsafe` 效应缺失错误
