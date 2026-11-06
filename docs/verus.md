# Verus 规范性迁移

本仓库的真实消费者是 `chrono-harness check` 及其本地／CI 汇总。它们必须对登记、选择、状态和失败结果给出可复查的语义。普通 Rust 测试只能执行有限样例，不能证明边界、组合和状态转移没有一条未覆盖的分支；因此影响这些结果的逻辑以 Verus 源码为权威，并由 coverage gate 检查证明和生产调用是否相连。

当前实现已接入 runner 的实际边界：完整 commit OID、CI 单元 ID、POSIX 相对路径，以及 registration／routes 共用的执行资源上限。`facts::full_oid`、`units::id`、`runner::relative_path` 直接调用同源 `verified_identifiers.rs`／`verified_paths.rs`；`judge-registration::Bounds::valid` 与 `judge-routes::Execution::validate` 共同调用 `verified_limits.rs`。本批还把 full CI 和 release 配置中的字面量准入接到 `verified_literals.rs`：值必须是单行字节，不能含 NUL、CR、LF 或 `${{` 模板起始串；`crates/ci/src/full.rs` 和 `crates/ci/src/release.rs` 都消费同一个 `valid_literal`。新增的 `verified_paths::valid_state_path` 由输入快照、保留 blob 和 execution-unit 报告注册直接消费，证明运行时证据引用只能落在 `.chrono-harness/state/` 下；它不证明证据内容完整，也不把 state 变成 Git policy。独立 Rust 测试通过 Verus 宏检查边界；现有 full/release 行为测试继续验证两个配置消费者的拒绝和生成路径。Verus 的唯一规范入口是 `.chrono-harness/verus.json`；`.chrono-harness/ci/bootstrap-verus.json` 只引用它，`.chrono-harness/ci/verus_bootstrap.py` 下载并校验归档，归档和解压出的工具都属于可重建宿主状态，不入 Git。此批只证明这些局部谓词和调用接线，不证明 JSON 解码、Git／文件系统事实、registration 图选择或其它状态逻辑。

这项要求有明确的终点：如果移除 Verus proof，check 仍可运行，却无法把通过／拒绝／未知绑定到已证明的规范谓词，当前 SPEC 的核心正确性就失去可观察保证。纯 IO、进程运输、Git／CI provider 观察和不改变裁决的 fixture 没有这个终点，不因项目使用 Rust 或存在 Cargo 文件而强行加入 Verus。

## 权威边界

以下逻辑必须迁移到同一份可验证的 Verus 源码，并由实际调用方消费其结果：

- 输入字段、登记引用和必要性链的准入；
- DELTA、影响闭包、路由和项目选择；
- CI 汇总、缓存／报告／工作树状态与生命周期转移；
- 生成器、bootstrap 的政策判断以及这些判断的测试 oracle。

Rust、Python、Shell 和 YAML 可以实现外部接口适配。它们必须登记输入、输出、边界假设和唯一的 Verus 调用；不能在 wrapper 里重新判断同一结果，也不能把 `external_body` 当成仓库逻辑的豁免。OS、Git、CI provider、Rust/Cargo、Verus/Z3 等只是假设接口，仓库自己的解析、错误处理和状态更新仍要证明。

非 Rust 业务继续使用自己的语言，只要它不决定本仓的规范性状态，或把决定委托给已登记且已证明的 Verus 核心。若脚本自身决定准入、报告结论或状态，它仍属于待迁移逻辑。

## 登记和证明合同

后续采用会在 `.chrono-harness/verus.json` 固定 Verus 版本、commit、编译器／宏版本和允许的外部接口，并在 FILEMAP 为每个 program／specification 文件列出唯一 proof root。每个 proof operation 必须是显式 argv，使用 `--no-cheating`、`--no-external-by-default` 和整 crate 检查；验证结果、源码摘要、实际调用和 coverage response 通过不可变候选绑定。

缺少 proof root、proof 未执行、proof 与生产调用不一致、或只运行普通 Cargo 测试时，coverage gate 返回 `E_VERUS_COVERAGE`，保持 `proposed`／`incomplete`，不回退到未证明的 wrapper。声明 `language: verus`、能编译 proof crate 或一次 CI 绿色都不等于已建立对应关系。

## 分批迁移

PR #286 先建立递归必要性链合同，后续候选批次把 Verus 谓词接入 runner、registration 和 routes 的真实使用方。Verus 工具固定为 `0.2026.04.24.f8e1704`，proof operation 使用整 crate、`--no-cheating`、`--no-external-by-default` 和编译／依赖输出；宿主 bootstrap 负责摘要校验和安装。coverage 仍是 `proposed`，不伪装当前 `dev` 已完成全仓采用。后续 PR 按真实消费者推进：

1. 固定 Verus 工具和宿主登记，接入现役 runner／registration／routes 谓词及其同语言证明测试；当前也覆盖 full CI／release 的 literal admission，但范围仅限上述四类字节边界；
2. 迁移组合选择、必要性链和状态判定，保留未完成 coverage 的真实失败；
3. 迁移 CI、bootstrap、生成器和生命周期适配，核对原始输入／输出绑定；
4. 只有所有受治理规范性对象的 proof root、调用对应、输入边界和适用行为验收闭合后，才评估把 coverage gate 从 `proposed` 切到 `active`。

每批都必须报告 proof 范围、未迁移对象、输入假设、成本和真实消费者。局部 proof 通过不代表全仓迁移完成；没有当前消费者的逻辑不因“以后可能需要”继续保留。
