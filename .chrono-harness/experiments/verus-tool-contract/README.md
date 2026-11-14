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
| `investigate.verus-source.toolchain` | Rustup 安装固定 Rust 1.99.1 minimal，加 rustfmt、rustc-dev 和 llvm-tools；不选择全局默认工具链。安装成功后仍须核实际工具身份与组件。 |

上游源码及后续编译输出复用
`.chrono-harness/cache/bootstrap-owner-tooling/verus/source-6ae5f7b9/`；
依赖下载复用宿主当前 CARGO_HOME，Rustup 复用原有工具链存储。
源码只在缺失时获取；后续构建先核同一完整 HEAD 和干净的跟踪源码，不重复 clone。
工具链准备只用于缺失输入，后续构建直接选择固定已安装工具链。

该固定检出归本实验负责，消费者是 #1058 的完整根证明修复。它使用已有 Verus 输入缓存
保留范围，最多一个本次固定源码槽位；失败时保留原输入和构建现场，采用或放弃该候选、
消费结束且必要原件已接管后，由该缓存所有者核无活消费者再退役。
源码不是本仓 feature／integration 工作树，不改动其它参与者的检出。

## 采用前验收

源码构建应复用上游 Cargo 构建方法和固定 lock，不另造编译器构建框架。
其编译器、后端、库、宏、vstd、Z3 及生产原件必须与实际构建实例匹配。
当前 Rust 1.95.0 生成的依赖不能充当 Rust 1.99.1 的输入。

只有实际消除原失败、完成严格整 crate 证明及独立预期、补齐正式资格载体，并通过
适用 bootstrap、canonical、integration 与 dev 落地验收后，才可宣称相应交付完成。
