# 固定修复源码的验证

当前完整 runner、workflow execution 和 owner-publication 独立预期在严格 Verus
调用中触发 `VerusErasureCtxt has not been initialized`。上游
[#3269](https://github.com/verus-lang/verus/pull/3269) 的固定提交
`6ae5f7b97c6403789fc1f12117ea8b2f0092289c` 修复同类 const／erasure 问题，
但尚未证明可以修复本仓。现有官方普通及 rolling 发布不包含它；该提交的 macOS
arm64 构建被取消，没有对应产物。因此先按上游 BUILD.md 验证源码构建，再决定正式采用。

**本批同时修改宿主操作登记和工具声明。** 新增方法只准备当前失败所需的固定输入，
不授予证明资格，也不把源码获取、工具链安装或普通构建当作证明成功。所有原严格参数、
完整根、独立预期、真实依赖与正式准入继续适用。源码构建、证明及存储总成本未知。

## 输入取得与复用

两个操作均从本仓已登记的 `chrono-worktree use --operation ID --path WORKTREE` 执行：

| ID | 输入、行为与结果 |
| --- | --- |
| `investigate.verus-source.acquire` | Git 直接获取上述完整提交的单一浅检出；原进程及固定方法由 worktree 保存。获取成功后核实际 HEAD、tree、干净状态和上游 `rust-toolchain.toml`。失败或已有路径不覆盖原工作。 |
| `investigate.verus-source.toolchain` | Rustup 安装固定 Rust 1.99.1 minimal，加 rustfmt、rustc-dev 和 llvm-tools。先记录原默认；首次安装可能自动设置默认，须恢复原选择。安装成功后仍须核实际工具身份与组件。 |

上游源码及后续编译输出复用
`.chrono-harness/cache/bootstrap-owner-tooling/verus/source-6ae5f7b9/`；
依赖下载复用宿主当前 CARGO_HOME，Rustup 复用原有工具链存储。
源码只在缺失时获取；后续构建先核同一完整 HEAD 和干净的跟踪源码，不重复 clone。
工具链准备只用于缺失输入，后续构建直接选择固定已安装工具链。

## 源码构建入口

`investigate.verus-source.bootstrap` 委托现有 native bootstrap，消费
`bootstrap-verus-source.json`。该 profile 按原生工具绑定登记实际 Rust 1.99.1
cargo／rustc，复用上游 Cargo 的 `--locked --release` 构建方法，只构建真实使用的
验证器、驱动、Cargo 前端及宏／builtin。既有 Rust 1.95.0 bootstrap 前端仅负责启动，
构建操作实际选用已绑定的 1.99.1。编译结果不安装到当前正式工具目录。

Z3 4.16.0 的 macOS arm64 包摘要来自上游 release asset 的 `digest`，下载和命中
复用现有 `download` 所有者；tar 只取实际 z3 文件。其原包保留在同一个有界 Verus
输入缓存槽位，下载响应头归现有 state 原件保留。重复构建复用已核原包和上游
`target-verus`／Cargo 缓存，不反复下载源码或工具链。

native 原报告保存固定宿主提交、配置、完整操作、实际工具和双流；构建完成后还须
核上游跟踪源码、实际版本和产物。同一入口在构建后通过原 Cargo 方法查询实际版本，
再由上游 cargo-verus 构建同一源码的 vstd，保持两线程。vstd 直接使用上游声明的
方法和依赖；这一步不授予本仓严格根或扩展输入正式资格。

该固定检出归本实验负责，消费者是 #1058 的完整根证明修复。它使用已有 Verus 输入缓存
保留范围，最多一个本次固定源码槽位；失败时保留原输入和构建现场，采用或放弃该候选、
消费结束且必要原件已接管后，由该缓存所有者核无活消费者再退役。
源码不是本仓 feature／integration 工作树，不改动其它参与者的检出。

上游独立 `vstd/Cargo.lock` 不随 Git 提供。首次构建已保留 `--locked` 拒绝缺 lock
的原失败。缺失时先运行 `investigate.verus-source.vstd-lock`，用固定 Cargo 1.99.1
和现有下载缓存离线生成该独立锁文件；保留实际摘要与原命令结果。后续构建沿用该文件，
不重复生成或解除 `--locked`。缓存缺依赖时保留原失败，按真实缺项准备输入。

## 采用前验收

`investigate.verus-source.check` 继续委托现有 native bootstrap，消费
`bootstrap-verus-source-check.json`。先用同一 Rust 1.99.1、当前 runner manifest／lock
和已有下载缓存离线构建真实 runner 库；独立编译目录留在本次固定源码缓存槽位中。
原 Cargo JSON 决定实际 Serde 等依赖的产物路径，不能从目录扫描或猜测 hash 选择。
这份生产记录供原 workflow 严格根的修复实验使用，不授予正式证明资格。
随后 `investigate.verus-source.execution` 直接验证原来的
`crates/judge-workflow/src/verified_execution.rs`，包含真实 response／status 源码及
Serde derive；完整根、严格参数、编译和 dep-info 保留。所选 Serde 库路径来自上述
原 Cargo 产物记录；依赖变化后须重新核实际记录和字节，不能静默选择另一个库。
该实验方法不登记正式 `output_contract`，普通退出成功不绕过原资格检查。
执行复用已登记的 Cargo 和上游原 `verus` binary，显式消费同一 manifest／配置；
保持宿主 cwd 及全部证明参数，不增加 CI 不认识的实验工具别名或另一个启动脚本。

补丁改变验证器字节但保留上游版本号时，原 Cargo 版本指纹可能仍命中旧 vstd。
此时先核本实验固定源码、目标 manifest／package、原工具及输出身份，并确认自有
构建与使用方全部结束；再执行登记的 `investigate.verus-source.vstd-refresh`。
它使用 Cargo `clean --package vstd --release`，只失效本实验的 vstd 编译输出，
随后委托原 `investigate.verus-source.vstd` 方法重建。源码、下载、lock、工具链、
验证器和其它依赖缓存继续复用；原报告按已有引用保留。这是工具变化后的显式准备，
普通重复构建不清理 vstd。实际重建和原流须分别核验，新增成本未知。

固定上游的 Map／Set 类型已保证有限，旧 BTreeMap 有限性公理已删除。真实 tree／index
解析循环改为显式保持 domain 有限的不变量，从空库存和每次插入传递该性质；不改变
生产控制流、规格结果或必要证明，也不增加版本分支。完整根仍按上述同一入口复验。

同一固定 vstd 的 Vec 与 BTreeMap 迭代器改用 `IteratorSpec::remaining`，不再提供旧的
位置／完整库存二元 view。真实循环以剩余序列等于原库存的未消费后缀连接完整前缀规格；
已有输出数量或状态轨迹给出消费位置，去重循环只增加被擦除的 ghost 计数。借用树库存
通过标准 `unref` 保留原键值，排序仍由相同的比较定律证明。独立预期、完整字段、首错、
顺序和数量义务不变，不保留旧版本分支；当前工具采用和证明结果仍须分别取得。

源码构建应复用上游 Cargo 构建方法和固定 lock，不另造编译器构建框架。
其编译器、后端、库、宏、vstd、Z3 及生产原件必须与实际构建实例匹配。
当前 Rust 1.95.0 生成的依赖不能充当 Rust 1.99.1 的输入。

## 块内 crate 别名的工具修复

`nested-extern-crate.patch` 是固定上游输入的明确补丁，归本实验的外部 Verus 工具
所有者维护；实际使用方是上面的完整 workflow 根。Serde 展开在匿名 const 块内声明
crate 别名。上游逐项翻译器已经接受 `ExternCrate`，语句翻译器却遗漏同一类声明。
补丁只让这个编译期声明沿既有 `use` 路径不产生执行语句；原 Rust 名称解析、嵌套
函数和 impl 的独立检查不变，不把 Serde 派生或其它仓内函数列为受信接口。

补丁还在上游原 Rust 测试项目增加一个明确测试目标，核执行块与证明块的别名成功，
并要求匿名 const 内故意错误的函数仍被拒绝。测试驱动与断言均使用原 Rust 方法。
登记的 `investigate.verus-source.regression-check` 委托 native bootstrap 消费
`bootstrap-verus-source-regression.json`，执行下文登记目标的完整库存，保持单测试线程；
每个证明仍用两个线程及原严格选项。原 Cargo 配置负责已构建工具、同源 vstd 与 Z3，
不另造启动器、测试框架或下载路径。
严格选项由上游测试宏传递；两线程参数通过其现有 `VERUS_EXTRA_ARGS` 接口传给
验证器，避免交给只接受固定白名单的测试宏参数解析器。

先只加入补丁中的测试文件，沿该入口核原工具的预期失败，再应用编译器 hunk，
沿原 `investigate.verus-source.bootstrap` 构建并复验同一库存及真实完整根。
重复运行复用当前补丁和缓存；不再次应用已应用的 hunk，也不把修改后的工具称为
未修改的上游版本。源码必须精确对应固定 HEAD 加本候选登记的补丁，实际差异和工具字节
另核；补丁、配置及原报告由本候选共同固定，仍不授予正式采用或证明资格。

**外部工具与宿主方法登记同时修改。** 影响是工具对声明语句的翻译及一个独立回归目标；
完整根、派生实现、严格参数和后续资格义务保留。构建和回归复用原缓存，新增成本未知。
补丁及回归输入只在该工具的采用／放弃决定和必要失败验收完成后，沿已有缓存所有者退役。

## 常量数组转切片的工具回归

实际 Serde 字段表使用命名常量把数组借用为切片。`const-array-unsizing.patch`
在同一上游 Rust 测试项目增加明确目标，分别核常量的完整字节、空切片、函数内字段表和
错误内容断言；另核无需切片转换的数组借用、普通执行代码对照和任意执行调用仍拒绝。
测试继续使用原严格选项，
不把字段表或派生实现排除出证明。

原工具把只读数组的隐式切片转换表示为执行函数调用，双用途命名常量的规格检查因此
拒绝它。补丁将这项纯转换表示为内部一元操作，模式检查继续核原操作数；AST 简化随后
直接使用 vstd 已有的 `spec_array_as_slice`。它与原执行 helper 后置条件中的结果是同一
函数，不新增外部公理，不放宽任意函数调用的模式检查；可变切片仍走原路径。
首次整 crate 剪枝也显式保留这项转换依赖的规格函数，避免在后续简化引入调用前
移除必要模型；不通过扩大全部函数保留范围来掩盖缺失依赖。
双用途常量的初始化体始终保留原执行值模式，包括在 `verus!` 外声明的常量；
关闭显式 ghost 块检查不能把其中临时借用提升为 proof 值。常量自身的规格模式、
返回模式和任意函数调用准入保持原合同，原 `consts` 库存及明确拒绝回归共同验收。

同一 `investigate.verus-source.regression-check` 入口显式选择这个目标、块内别名目标和
上游原 `arrays`／`consts` 完整库存，没有名称过滤。先应用块内别名补丁，再应用本补丁；
源、工具及同源 vstd 继续沿原构建入口准备。补丁沿上述缓存和有限保留边界管理。
常量的规格与执行两种用途、实际切片内容及错误函数拒绝都须验证；回归失败不授予工具
或本仓完整证明资格。新增构建、证明与维护成本未知。

## 字符串与字节串字面量模式

完整 workflow 根的真实 Serde 展开使用字符串与字节串字面量模式。固定上游只接受
可直接使用 SMT 相等的模式类型，因此在翻译这些合法 Rust 模式时失败。
`literal-patterns.patch` 沿现有模式翻译所有者补齐这两种字面量：字符串核完整字符
数量及每个 Unicode 字符，字节串核完整长度及每个原字节。两者复用现有长度、索引和
逻辑组合，不把引用身份相等当作内容相等，也不增加公理或跳过派生函数。
命名常量及其他类型仍走既有准入；浮点模式的原拒绝继续适用。
字节串切片模式在首次剪枝时保留后续展开实际需要的切片长度和索引函数，
不扩大为保留所有函数。成功预期保持完整序列的双向等价，并用显式逐元素相等证明
建立序列相等；不减少长度、内容或反向条件。

同一 `investigate.verus-source.regression-check` 登记七项独立成功／失败预期，覆盖
空值、完整字段、Unicode、NUL、255 字节、数组／切片、嵌套或模式及 guard，
错误内容和空值的错误断言必须真实失败。先只加入测试并观察原工具失败，再实现并构建工具。
嵌套或模式与 guard 分别验证；同一臂同时使用两者仍沿上游原回归拒绝，
不为真实 Serde 消费不需要的组合新增特性。
该入口同时保留原 `arrays`、`consts`、块内别名及常量切片库存，
并运行上游原 `match`／`strings` 完整库存；没有测试名过滤，单测试线程与两证明线程不变。
工具变化后沿前述显式 vstd 刷新方法准备同源模型，再复验同一真实完整根。

**外部工具源码与宿主验证登记同时修改。** 新增范围仅服务已观察的完整根失败；
编译和回归总成本仍未知。字面量模式翻译通过不代表 Serde trait、JSON、完整根、
正式工具采用或本仓完整证明通过；实际结果继续由原报告负责。

## Formatter 写入的外部接口

完整 workflow 根的真实 Serde `Expected::fmt` 实现将完整预期文本交给
`core::fmt::Formatter::write_str`，并返回该调用的原始 `Result`。固定 vstd 已声明
`Formatter` 与 `fmt::Error`，但没有这个方法的合同。`formatter-write.patch` 由同一外部
工具所有者维护，在现有 `std_specs::fmt` 补齐该标准库接口；不为仓内派生实现或适配器
增加外部函数豁免。

接口接受原 `&mut Formatter` 与完整 `&str`，返回标准库原 `Result<(), fmt::Error>`。
返回结果与写入后的 formatter 状态保持不透明：调用方须处理两种结果，不能据合同
声称成功、失败、输出内容或 formatter 状态不变。这里仅采用真实标准库调用边界，
不建立格式化输出语义模型，也不声明完整 Serde 证明完成。

先只登记三项 Rust／Verus 预期，观察原工具的未支持错误，再补方法声明并重建同源 vstd。
成功预期直接转交文本和返回值，两个独立负例分别要求拒绝无依据的成功／失败断言；
负例必须收到 `assertion failed`，方法未支持不算符合预期。
同一 `investigate.verus-source.regression-check` 保留原全部目标，并增加上游 `std` 和
`formatter_write` 的完整库存；线程、严格选项及无名称过滤的方法不变。
新增构建、证明和回归成本未知，沿当前缓存及有限保留边界管理。

**工具接口合同与宿主回归登记同时修改。** 源码只在该接口的自然所有者处增加标准库声明，
完整 workflow 根、真实派生、其他外部合同及正式资格仍需各自验收。

## Serde UTF-8 转换的外部接口

完整 workflow 根的实际派生代码调用 Serde 1.0.229 的
`serde::__private229::from_utf8_lossy`。固定依赖源码在当前 std／alloc 配置下直接
转发 `String::from_utf8_lossy`，返回 `Cow<'_, str>`；vstd 已有该 Cow 类型和完整文本 view，
不另造字符串或 JSON 模型。`serde-lossy.patch` 先登记三项 Rust／Verus 预期：
原字节及原返回值可转交，不能无依据断言结果文本为空或非空。两个负例必须实际收到
`assertion failed`；未支持错误不算预期拒绝。

回归沿原完整库存增加 `serde_lossy`，保留严格选项、单测试线程和两证明线程。
通过上游既有 `VERUS_EXTRA_ARGS` 连接实际 runner Cargo 原件选定的同一 Serde rlib
及其依赖目录；相对地址以 Cargo 测试包工作目录为准，修改前实际执行已核相同原路径。
不扫描或猜测产物 hash，依赖变化后须重新核实际 Cargo 记录；不以此实验授予正式资格。

真实 vstd 的 `serde_specs` 只在 std 及保留 ghost 时声明该外部函数，保留原切片、
借用寿命和原 Cow 返回值，不提供内容、分配或 Borrowed／Owned 保证。bootstrap 在
vstd 前复用原 runner 依赖构建；vstd 通过 Cargo-Verus 既有额外参数连接上述相同产物，
相对地址以 vstd 独立包工作目录为准，实际编译及完整根仍须核验。额外参数由上游
DepTracker 写入 Cargo 环境依赖，变更会失效原指纹；同址依赖字节改变时仍须沿既有
定向 vstd 刷新重新准备，不声称完整 Cargo 输入资格已经解决。

**外部接口合同与宿主回归登记同时修改。** 成本仍未知，复用当前源码、下载及编译缓存，
没有完整根、原派生、实际输入绑定或正式采用豁免。先核三项原失败，再实现接口并重建。

## Serde Expected 的外部 trait

完整 workflow 根的九个实际派生实现使用 `serde::de::Expected`。固定依赖的该 trait
仅有 `fmt(&self, &mut Formatter) -> fmt::Result`，没有额外 supertrait 或结果保证。
`serde-expected.patch` 由同一外部工具所有者维护，顺序应用在 `serde-lossy.patch` 后。
先登记四项独立 Rust／Verus 预期：泛型外部调用与正确的仓内实现可接受，无依据的
成功及失败断言均拒绝，仓内 trait 实现中的错误断言也必须拒绝。
负例必须收到实际 `assertion failed`，trait 未声明或方法未支持不算预期拒绝。

同一完整回归入口新增 `serde_expected` 库存，复用相同 Serde、vstd、线程、严格参数
和缓存；不放行仓内 impl，不授予完整 Serde、编译输入或正式工具资格。
实际声明位于真实 vstd 的既有 `serde_specs`，逐字保留 trait 的 self／Formatter 借用
和原 `Result` 类型；不增加 requires、ensures、supertrait 或格式化成功假设。
**外部 trait 合同与宿主验证登记同时修改。** 新增构建、证明与回归成本未知，
先核原失败，再实现合同并重建同源 vstd，最后核真实完整根及全部保留回归。

## Serde Unexpected 的透明外部类型

完整 workflow 根的真实派生代码构造 `serde::de::Unexpected` 错误值。固定 Serde
1.0.229 的该枚举有十八个实际变体；上游 `external_type_specification` 已能从 Rust
类型信息直接读取完整枚举及字段，不需要另造映射、复制类型或工具翻译机制。
`serde-unexpected.patch` 由同一外部工具所有者维护，顺序应用在 `serde-expected.patch` 后。

先登记七项独立 Rust／Verus 预期：完整布尔／整数／字符字段、浮点变体身份、
带原借用寿命的全文／字节／Other 字段、全部十个无字段变体，以及错误变体、错误整数、
Unicode／NUL 文本截断和含 255 字节的截断拒绝。浮点用例只核变体身份，不声明
浮点相等或 IEEE 语义。负例必须实际收到 `assertion failed`，未支持错误不算预期拒绝。

原完整回归入口只增加 `serde_unexpected` 库存，复用同一真实 Serde 产物与 vstd；
严格选项、两证明线程、单测试线程、无名称过滤及全部原回归保持。
**外部类型合同与宿主验证登记同时修改。** 新增成本未知；先核原失败，再采用现有透明
类型声明，重建同源 vstd，复验完整库存与真实完整根，不授予正式资格或仓内豁免。
声明位于同一真实 vstd 的 `serde_specs`，没有 `external_body`；全部变体和字段直接
来自原类型。字段预期用标准模式匹配，不要求外部枚举没有生成的 Verus 字段访问器。

## Serde 错误构造的外部 trait

真实派生通过 `serde::de::Error` 构造错误，Serializer 后续接口也要求
`serde::ser::Error`。固定 Serde std 配置的两 trait 都有 `Sized + StdError` 父约束；
de 提供八种方法，ser 提供 `custom`，不能漏掉真实父约束或替换成自造错误类型。
`serde-errors.patch` 由同一外部工具所有者维护，顺序应用在 `serde-unexpected.patch` 后。

先登记五项 Rust／Verus 预期：九种真实接口原参数及结果转交、两个正确的本地实现、
不能无依据断言两次结果相等或不同，以及两个错误本地实现必须都被拒绝。
结果保持不透明，不增加消息文字、结果身份、分配或格式化成功保证；借用、动态
Expected、完整字段及静态候选列表保持原签名。负例必须实际收到 `assertion failed`，
方法／类型未支持不算预期拒绝。

原完整回归入口增加 `serde_errors` 库存，继续使用同一 Serde、完整库存、严格参数、
两证明线程和单测试线程。**外部接口合同与宿主验证登记同时修改。** 新增成本未知；
先核原失败，再声明实际标准库父 trait 和 Serde 方法，重建 vstd 并核完整根。
标准库父 trait 在现有 `std_specs::core` 所有者处声明，不把其未声明方法列为已支持，
也不为仓内 impl 或适配器增加外部函数豁免。
实际标准库声明保留 Debug／Display 两个父约束；Serde 两个声明保留 Sized／StdError
及各方法实际泛型、借用和返回类型。所有方法均无额外 requires／ensures，本地
impl 仍有完整证明义务；编译器的外部 trait 规格匹配也须通过，不能放宽来规避失败。

## 已导入动态 trait 的最终检查

Serde Error 的真实参数包含 `&dyn Expected`。原工具的前置检查已读取导入的 trait，
最终动态兼容性检查却只查当前 crate，因而把 vstd 已声明的真实 Expected 误报为未声明。
`imported-dyn-traits.patch` 顺序应用在 `serde-errors.patch` 后，由原 well-formed 所有者
在最终检查中同时消费已导入和当前 trait 的已计算兼容性；不跳过检查、不猜允许结果。

先增加三项严格 Rust／Verus 预期：动态调用保留原 Result，无依据的成功与失败断言
均须实际因 assertion failed 拒绝。完整原 `traits_dyn` 和 `safe_api` 库存加入同一登记
回归，保留不兼容合同、unsized blanket impl、未声明 trait 及原有安全边界；
不改原断言、线程、严格选项或回归范围。工具判断与宿主验证登记同时修改，新增成本未知。

## Serde 序列化接口

真实完整根的 Serialize 派生需要 Serializer、Serialize 和七种复合状态接口。
固定 Serde 1.0.229 的 std 配置是本实验唯一外部接口来源；保留原关联类型及相等约束、
所有公开方法的泛型、借用、静态名称和 Result，不增加结果、消息或格式化成功保证。
`serde-serialization.patch` 顺序应用在 `imported-dyn-traits.patch` 后，仍由真实 vstd
的 `serde_specs` 承载外部接口，不复制 Serde 类型或为仓内实现加 external_body。

先登记六项独立 Rust／Verus 预期：标量和命名参数、全部七种复合状态及原错误类型、
正确本地实现和真实结构／枚举派生、不能无依据断言成功或失败，以及错误本地实现必须拒绝。
负例必须实际收到 assertion failed，未声明接口不算预期拒绝。沿原完整登记回归追加库存，
保留全部原测试、严格参数、线程和 ignored；真实完整 workflow 根继续复验。
外部接口合同与宿主验证登记同时修改，新增证明、构建、CI 和存储成本未知。

Serialize 的方法依赖 Serializer，后者的关联状态方法又依赖 Serialize，原循环检查
因此拒绝完整声明。复用上游已在 Iterator 使用的 `impls_cannot_extend_spec`，只标记
`Serialize::serialize`：允许该 exec 接口的递归依赖，同时禁止本地 impl 增加 ensures，
防止通过调用后置条件制造不良定义；本地方法体和错误断言继续检查。其余方法不加此标记。
增加第七项独立拒绝预期，并沿同一回归运行完整 `traits_extend_ensures` 库存，核原循环、
spec／proof 限制及后置条件边界；不修改工具算法、Serde bounds 或原安全断言。

真实派生的纯表达式失败同时保留所属规格函数的名称和位置，复用现有错误的次级标签。
该诊断只补充原错误上下文，不改变纯表达式准入或派生的证明义务。

Serde 生成的 `const _: ()` 是占位常量项，不是 rustc 的 `AnonConst`。未显式指定模式时，
这两类无具名规格值的常量沿既有 exec 初始化路径检查；具名常量和显式模式仍保持原规则。
原完整 `consts` 库存加入占位常量的可执行初始化正例、初始化前置条件的拒绝预期，
以及具名常量的拒绝对照；
真实结构／枚举派生仍由原 Serde 回归验收，不跳过初始化体或降低纯表达式要求。

## Serde 反序列化接口

完整 workflow 根的实际 Deserialize 派生仍需要 Deserializer、Visitor、DeserializeSeed、
SeqAccess、MapAccess、EnumAccess 和 VariantAccess。接口依据当前 Cargo 原件选定的
Serde／serde_core 1.0.229 std 配置，不复制派生实现或为仓内方法添加 external_body。
`serde-deserialization.patch` 顺序应用在 `serde-serialization.patch` 后。

八个 trait 声明固定 std 配置下的八十个公开方法，保留真实生命周期、泛型、关联值／错误
类型、Variant 相等约束及原参数。结果与修改后的状态保持不透明，不增加 requires／ensures。
没有当前调用的隐藏 `__deserialize_content_v1` 和 `__private_visit_untagged_option`
尚未声明，不能据本批声称支持这两个内部入口或全部 Serde 行为。

先登记八项独立 Rust／Verus 预期：反序列化和 Visitor 的真实参数转交、种子／默认访问及
原关联错误类型、正确的本地实现和真实结构／枚举派生、无依据成功／失败断言，以及错误
Deserialize／Visitor 本地实现均须实际因 assertion failed 拒绝。未声明接口不算预期拒绝。
同一完整登记回归只追加这个目标，保留原库存、严格选项、线程和 ignored。

**外部接口合同与宿主验证登记同时修改。** 新增构建、证明及回归成本未知，继续复用
同一源码、依赖、工具链和编译缓存。完整根、正式资格及实际调用绑定仍须分别验收。

真实 Visitor 与反序列化／容器访问接口互相引用。原工具实际拒绝这项规格循环后，
复用既有 `impls_cannot_extend_spec`，只标记 Visitor 的 `visit_some`、`visit_newtype_struct`、
`visit_seq`、`visit_map` 和 `visit_enum`，切断这五种递归 exec 接口的规格扩展。
本地方法体继续检查，增加第九项拒绝本地后置条件的独立预期；原完整
`traits_extend_ensures` 库存仍由同一入口验收，不修改循环判断或增加外部公理。

真实结构／枚举派生还使用 `IgnoredAny` 和 `__private229::de::missing_field`。
前者复用透明外部类型声明，字段直接来自固定依赖；后者保留原静态字段名、生命周期、
Deserialize／Error 约束和原 Result，不保证成功或失败。真实缺失 Option 字段可能成功，
不能把函数名当作恒失败合同。增加参数／结果转交及无依据成功／失败断言的三项独立预期，
原真实派生和枚举 tuple 变体继续保留，未支持错误不算负例通过。

## 构造器作为函数值

真实枚举 Deserialize 派生把 tuple 变体构造器交给 Result::map。固定上游尚不支持这个
合法函数值，不能以改写派生、去掉变体或不透明外部函数替代。`constructor-function-values.patch`
顺序应用在 `serde-deserialization.patch` 后，先登记五项独立严格预期，核结构与枚举的
完整字段、变体、泛型、借用、FnOnce 原合同及错误字段／变体的实际断言拒绝。
同一回归入口追加 `constructor_function_values` 和原完整 `fndef_types` 库存。
原两个零参数 tuple 结构用例随当前支持目标改为真正调用并核构造值，保留 Self 路径；
其余原断言、真实派生、严格参数、线程及 ignored 不变。

**外部工具与宿主验证登记同时修改。** 构建、证明和回归成本未知，沿已有固定源码、
工具链与编译缓存复用；实际失败、实现及完整根须分别验收，不授予正式工具资格。

只有实际消除原失败、完成严格整 crate 证明及独立预期、补齐正式资格载体，并通过
适用 bootstrap、canonical、integration 与 dev 落地验收后，才可宣称相应交付完成。
