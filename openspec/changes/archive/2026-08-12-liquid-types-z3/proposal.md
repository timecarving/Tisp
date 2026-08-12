## Why

§15 液态类型是 Tisp 的核心承诺之一(强静态类型 + 六维注解中的精化维度),但当前只有数据结构、没有真实检查:精化类型 `{x : T | pred}` 与契约 `:requires`/`:ensures` 在 desugar 阶段即被丢弃(CoreDef 字段恒为 `None`,示例文件注释自认 "only parsing, not checking yet");`LiquidChecker` 仅能做常量折叠,变量谓词与未知谓词一律放行;`Z3Bridge`(SMT-LIB2 外部进程)是未接线的死代码,`z3` crate 依赖从未使用。现状与「通过检查即保证无运行时类型错误」的定位不符,需要把这条链路真实打通。

## What Changes

- **精化类型全链路解析**:desugar 支持 `{x : T | pred}` 语法 → `Type::Refined`;类型显示、合一、替换中正确传播 Refined
- **契约解析**:`defn` 的 `:requires`/`:ensures` 解析为 `CoreDef.requires/ensures`(支持多个 `:requires` 合并为 And)
- **谓词 → SMT-LIB2 翻译**:`Predicate`/`Term`(比较、算术、and/or/not/implies、量词、abs/负数等已知函数)翻译为 SMT-LIB2 表达式
- **Z3Bridge 扩展**:多变量声明与断言、`check-sat` 反例模型提取(验证失败时报告反例值)、求解上下文复用
- **Liquid 验证接入 `--typecheck`**:在类型推断后执行——函数体返回精化类型验证、调用点实参精化检查、契约 requires/ensures 验证
- **无 z3 二进制时优雅降级**:保留常量折叠检查,不因缺 z3 而报错(现有行为)
- **清理**:`z3` crate 依赖改为可选(feature 门控,与 `llvm` 约定一致)或移除;消除 `Z3Bridge` 死代码状态
- **测试与文档**:单元测试(翻译/验证/反例)、`examples/liquid-types-test.tisp` 跑通、`standard_doc`/实现状态/CHANGELOG 同步

## Capabilities

### New Capabilities

- `liquid-types`:精化类型 `{x : T | pred}` 的解析、Z3 求解验证、`defn` 契约(`:requires`/`:ensures`)检查的端到端行为规范

### Modified Capabilities

(无 — `openspec/specs/` 目前为空,此为项目首个能力规范)

## Impact

- `crates/tisp-frontend/src/desugar.rs` — Refined 类型与契约解析(desugar 现有 `requires: None` 桩)
- `crates/tisp-middle/src/liquid_types.rs` — 重写:谓词→SMT 翻译、约束收集、验证入口(现仅常量折叠)
- `crates/tisp-middle/src/type_infer.rs` — 接入 `liquid_checker` 检查点(字段已存在,未使用)
- `crates/tisp-backend/src/z3_bridge.rs` — 扩展:翻译 API、反例模型、批量声明
- `crates/tisp-cli/src/main.rs` — `--typecheck` 接线液态检查输出
- `crates/tisp-core/src/types.rs` — 可能微调 `Predicate`/`Term`(函数符号表)
- `Cargo.toml`(workspace + tisp-backend/tisp-cli)— `z3` feature 调整
- 文档:`standard_doc/01-language-core.md`、`standard_doc/04-implementation-status.md`、`CHANGELOG.md`、`examples/liquid-types-test.tisp`
