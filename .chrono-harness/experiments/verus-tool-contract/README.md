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
错误内容断言；另保留普通执行代码中的数组转切片对照。测试继续使用原严格选项，
不把字段表或派生实现排除出证明。

原工具把只读数组的隐式切片转换表示为执行函数调用，双用途命名常量的规格检查因此
拒绝它。补丁将这项纯转换表示为内部一元操作，模式检查继续核原操作数；AST 简化随后
直接使用 vstd 已有的 `spec_array_as_slice`。它与原执行 helper 后置条件中的结果是同一
函数，不新增外部公理，不放宽任意函数调用的模式检查；可变切片仍走原路径。

同一 `investigate.verus-source.regression-check` 入口显式选择这个目标、块内别名目标和
上游原 `arrays`／`consts` 完整库存，没有名称过滤。先应用块内别名补丁，再应用本补丁；
源、工具及同源 vstd 继续沿原构建入口准备。补丁沿上述缓存和有限保留边界管理。
常量的规格与执行两种用途、实际切片内容及错误函数拒绝都须验证；回归失败不授予工具
或本仓完整证明资格。新增构建、证明与维护成本未知。

只有实际消除原失败、完成严格整 crate 证明及独立预期、补齐正式资格载体，并通过
适用 bootstrap、canonical、integration 与 dev 落地验收后，才可宣称相应交付完成。
