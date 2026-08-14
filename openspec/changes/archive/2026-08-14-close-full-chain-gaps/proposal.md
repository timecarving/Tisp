## Why

`docs/spec.md`(30 章)与 `standard_doc/` 宣称的「强静态类型 + 演算 > 效应 + 全程声明式」在 0.1.0 中仍有约 22 章停留在 ⚠️。前两轮(`finish-partial-features`/`finish-design-stage-features`)已把 ⬜ 主清单清零,但源码级审计暴露出三类「半成品」:**写了没接线**(`persistent.rs` 的 `im` HAMT 集合从未被 interpreter 的 `Value` 使用)、**接线是最小占位**(`codegen.rs::compile_app` 回退 `ret i64 0`、`instance_dict` 用运行时字典替代约束求解、加密用 XOR)、**语义只到解析**(六维是 7 个独立 pass 而非统一求解、`□_r`/`◇_ε`/`□_t` 无推理、`♭`/`♯` 直通、`r+s` 等级传播恒过)。现在补齐这些缺口,是兑现 0.1.0 设计承诺的最后一环。

## What Changes

- **统一六维约束求解**:把 middle 的 7 个独立 pass 收敛为共享约束图 + fixpoint 迭代,类型/效果/等级/模式/确定性/区域作为同一约束系统的投影;补五维子类型格;类型类实例查找改为约束求解驱动。
- **持久化数据结构落地**:`im` HAMT 的 `Vec/Map/Set` 接线到解释器 `Value`(结构共享语义),`quote` 产生可运行时操作的数据。
- **模态语义补齐**:`□_r`/`◇_ε` 推理、`Cost` 注解语法与全推导、`♭`/`♯` 真 adjoint-triple 语义、`□_t` 因果/生产率保证、依赖等级 `r+s` 传播真正检查。
- **HoTT 深水区**:完整立方填充(Kan composition)补齐 HComp/Transp 之外的组合。
- **系统级 + 编译后端正名**:`defextern` 之外补裸指针读/写、手动区域管理、`Unsafe` 效应门控;LLVM `--ir`/`--compile` 生成函数定义/调用/闭包(替换 `ret i64 0` 占位);`opt-level`/`inline!` 优化器真实接线;反射函数补全;密码学从 XOR/简单 hash 换强算法(AES/ChaCha/SHA-256)。
- **运行时收尾**:解释器加 TCO 缓解深递归栈溢出;修复多顶层表达式 + 递归的 `__top__` 栈溢出。
- **账目对齐**(收尾):`standard_doc/04` §1/§2 一致化、`02-advanced-features.md` 4 处 ⬜ 去 stale、`docs/spec.md` 章节标记刷新;归档 `complete-partial-features`、清理已吸收的 `implement-design-stage-features`。

## Capabilities

### New Capabilities

- `data-structures`: 持久化数据结构(im-backed HAMT `Vec/Map/Set`)与 `quote`-as-data 的运行时语义。

### Modified Capabilities

- `type-system-extensions`: 统一六维约束求解、五维子类型格、类型类实例查找约束求解驱动、`□_r`/`◇_ε` 推理、`Cost` 注解全推导。
- `hott-and-deriving`: 完整立方填充(Kan composition)、Cohesive `♭`/`♯` 完整 adjoint-triple 语义。
- `temporal-types`: `□_t` 因果性/生产率/无空间泄漏的语义保证。
- `dependent-linear-types`: 依赖等级传播真正检查(`r+s`,不再对 ω 绑定恒过)。
- `toolchain-and-macros`: 裸指针/手动区域/`Unsafe` 门控、反射函数补全、优化器真实接线、LLVM 函数/闭包代码生成。
- `recursion-closure-fixes`: 解释器 TCO 与多顶层表达式递归栈溢出修复。

## Impact

- **crates 全景**:`tisp-core`(约束图/子类型/等级/Cost)、`tisp-middle`(统一求解器、模态/等级推理、优化器)、`tisp-backend`(interpreter 持久化 Value + TCO、codegen 函数/闭包、加密)、`tisp-runtime`(persistent 接线、hott 立方填充)、`tisp-cli`(新增 flag 透传)。
- **依赖**:`im` 从「死代码」变为真依赖;加密引入 RustCrypto(`aes`/`chacha20`/`sha2`)或等价 crate,LLVM/Z3 保持 feature 门控,默认构建不破坏。
- **文档**:`standard_doc/01/02/03/04`、`docs/spec.md` 章节标记、`CHANGELOG.md`、`openspec/project.md` 测试数。
- **OpenSpec 流程**:归档 `complete-partial-features`,删除/归档已吸收的 `implement-design-stage-features`。
