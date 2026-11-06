# Verus 实施范围

本仓按批次把实际决策和状态逻辑迁到 Verus。完整目标是所有 SPEC 与仓内程序逻辑都有证明，
外部接口只采用明确登记的假设。目前只完成下列局部范围；全仓强制覆盖门和其他程序的迁移仍在实施。
Python 实现暂缓，不把未迁移程序列为已证明。

## runner 标识符与路径

`crates/runner/src/verified_identifiers.rs` 是实际 OID 和 CI 单元 ID 判断的唯一实现。
OID 必须恰为 40 或 64 个小写十六进制字节，且不全为零；ID 必须非空，
每个字节都属于 ASCII 字母、数字或 `._-`。循环证明返回值与各自规格充要相等，包含边界和终止。
`facts::full_oid` 与 `units::id` 直接消费这些结果。

`crates/runner/src/verified_paths.rs` 是 `relative_path` 的实际判断实现。
它精确描述当前 POSIX 词法路径合同：拒绝空、绝对路径、开头的当前目录、任意完整父目录分量，
以及 NUL、换行、回车和反斜杠；保留原本允许的内部 `.`、重复分隔符和末尾分隔符。
调用方的 `str` 保证 UTF-8 有效性。规格与执行循环证明全部字节的拒绝条件、分量边界及终止；
不据此主张文件系统解析、符号链接或目录穿越的完整安全合同。

独立 Verus 根直接读取生产源码，声明成功、失败和边界预期。Cargo 消费同一源码并擦除 ghost；
Rust 测试不能代替证明。其他 runner 逻辑、调用适配器与完整 SPEC 对应仍未证明。
把错误长度的 OID 改为接受，会直接违反返回值后置条件（6 verified / 1 error）；
正确源码按 SHA-256 恢复，再运行正式检查。

实际启动准备判定也使用同源 `verified_consumption::matches_preparation`，证明命令、宿主配置、
唯一先行 verifier、执行工具和项目登记必须精确对应。消费关系判官读取显式 preparation，
再核原共享 core 合同；解码和外层组合仍待证明。独立根与 Rust 回归核缺前置、重复、
把校验放入普通操作、工具／schema／命令／项目错绑及缺文档。registration 原计划增加两项整 crate 证明。

## 固定执行入口

本地和 CI 都使用宿主上下文登记的 bootstrap 与 canonical check。
`test:runner-tests` 的执行计划先运行 runner 的四项整 crate Verus 操作，再运行原 Rust 构建和未过滤测试。
生产和独立根均强制 `--no-cheating --no-external-by-default`，并编译产物及源依赖清单。
未增加仓内受信接口、函数过滤或证明预算。

官方 Verus 固定为 `0.2026.04.24.f8e1704`、Rust 1.95.0。
宿主使用现有通用 bootstrap 执行已登记的 HTTPS 下载、SHA-256 核对和解包操作；
runner 与 registration 的原生启动先核共享 core，再安装该工具。Python bootstrap 源码和测试均无修改。
当前工具归档适用于 macOS arm64；其他平台尚未采用这份证明执行计划。

**产品和宿主政策同时变更：**产品判断改为同源 Verus，FILEMAP、工具、启动和执行计划登记实际证明。
新增固定宏依赖通过 runner 传播到其现役依赖方的独立 lockfile；不改变其他包版本。
判断仍为线性字节遍历，未增加运行时进程；构建、下载、验证和 CI 总成本未测。
