# grammar-dsl

## Purpose

定义声明式语法框架(§草稿 `draft`):`::>` 语法定义 + meta-tag 特殊形式,使语言自身能声明语法结构并据此生成扫描器,替换/旁挂手写 lexer/parser。

## Requirements

### Requirement: ::> 语法定义

`::>` 形式 SHALL 定义「语法结构 → 具体语法结构」:左侧为语法结构名(元标签所指对象),右侧为产生状态序列的具体结构;同一结构名 SHALL 可多次定义(视为「或」);扫描器 SHALL 按具体结构逐字符匹配,匹配成功继续、源文件穷尽为止。

#### Scenario: 语法结构定义与匹配

- **WHEN** 以 `identifier ::> | <atozLetter> <AtoZLetter> [{<ASCIILetter>}]` 声明语法结构,并以扫描器匹配 `abc1`
- **THEN** 匹配成功,返回 identifier 结构

#### Scenario: 同名结构多定义

- **WHEN** 同一结构名以两条 `::>` 分别定义(如 `number ::> | <integer> <float>`),扫描器匹配任一分支
- **THEN** 匹配任一分支即视为该结构成功

### Requirement: meta-tag 特殊形式

语法具体结构 SHALL 支持以下特殊形式:`<tag>`(元标签,展开同名结构)、`\x`(字面转义)、`[x]`(可选)、`{x}`(重复 1+)、`[{}]`(重复 0+)、`| a b`(逻辑或)、`multi|`(多路或,接收 count(`|`)+1 个)、`<nonterm>`(空格/换行分隔)、`<error>`(永不满足,展开即报错)。

#### Scenario: 可选与重复

- **WHEN** 语法含 `[{<expression>}]`(0+ 重复)与 `[x]`(可选),扫描器匹配空序列
- **THEN** 空序列匹配成功(0 个满足)

#### Scenario: 多路或

- **WHEN** 语法含 `|| a b c`(三路或),扫描器匹配 `b`
- **THEN** 匹配成功(命中第二分支)

#### Scenario: error 元标签

- **WHEN** 具体结构展开到 `<error>`,扫描器匹配到此处
- **THEN** 停止并报错(该状态永不满足)

### Requirement: 内置 meta-tag

框架 SHALL 内建以下字符类标签:`<ztonLetter>`(0-9)、`<atozLetter>`(a-z)、`<AtoZLetter>`(A-Z)、`<ASCIILetter>`(A-Z/a-z/0-9)、`<ASCIILetterAll>`(全部合法 ASCII)、`<ASCIISpecLetter>`(ASCIILetterAll 除去 ASCIILetter 的部分)。

#### Scenario: 字符类匹配

- **WHEN** 语法含 `<ztonLetter>`,扫描器匹配字符 `7`
- **THEN** 匹配成功;匹配字母 `a` 则失败

### Requirement: 扫描器生成

声明式语法表 SHALL 生成扫描器:对目标源文件逐字符扫描,按状态判断;生成的扫描器 SHALL 输出匹配到的语法结构序列,匹配失败 SHALL 定位到失败位置并报错。

#### Scenario: 声明式语法驱动扫描

- **WHEN** 以 `integer ::> [| + -]{<ztonLetter>}` 声明并扫描 `-42`
- **THEN** 生成扫描器返回 integer 结构,值为 `-42`

#### Scenario: 匹配失败定位

- **WHEN** 源文件在某位置无法匹配任何声明结构
- **THEN** 扫描器报告该位置的语法错误,而非静默跳过
