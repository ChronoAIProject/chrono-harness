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

固定上游的 Map／Set 类型已保证有限，旧 BTreeMap 有限性公理已删除。真实 tree／index
解析循环改为显式保持 domain 有限的不变量，从空库存和每次插入传递该性质；不改变
生产控制流、规格结果或必要证明，也不增加版本分支。完整根仍按上述同一入口复验。

源码构建应复用上游 Cargo 构建方法和固定 lock，不另造编译器构建框架。
其编译器、后端、库、宏、vstd、Z3 及生产原件必须与实际构建实例匹配。
当前 Rust 1.95.0 生成的依赖不能充当 Rust 1.99.1 的输入。

只有实际消除原失败、完成严格整 crate 证明及独立预期、补齐正式资格载体，并通过
适用 bootstrap、canonical、integration 与 dev 落地验收后，才可宣称相应交付完成。
