# chrono-harness

chrono-harness 帮仓库按登记的依赖选择检查，让本地和 CI 使用同一条检查命令。它用 Rust 编写，还提供工作树管理、发布安装、CI 配置生成和 AI 工作指南生成工具。

这里的“宿主”就是接入工具的仓库；“判官”就是按规则给出结果的检查程序；“DELTA”就是两次提交之间的变化。工具只按明确登记的文件、依赖和操作做事，不靠目录名、语言或扫描代码猜依赖。

**目前已实现指南生成、CI 生成、共用的 `check` 入口，以及各检查程序的一部分功能。本仓正在使用按变更范围检查的 CI；五份完整检查登记仍标为 `proposed`，尚未全面启用。** 完整要求见 [SPEC.md](SPEC.md)，进度和缺口见[覆盖表](docs/spec-coverage.md)。

[公开测试版 v0.1.0-beta.29](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.29) 提供 macOS arm64 和 Linux x86_64 可执行文件及一键安装脚本。在接入仓库根目录运行：

```sh
curl -fsSL https://github.com/ChronoAIProject/chrono-harness/releases/download/v0.1.0-beta.29/install.sh | sh -s -- --host-root .
```

默认安装全部 16 个工具，无需 Python、Rust 工具链或 sudo。可用 `--tool NAME` 选择工具，已有不同安装用 `--replace` 显式升级。安装登记和生成入口归宿主维护，详见[发布与安装](docs/distribution.md#一键安装)。

## 从哪里开始

| 要做的事 | 阅读位置 |
| --- | --- |
| 生成 AI 工作指南和技能文件 | [指南生成](docs/instructions.md) |
| 生成 CI、准备事件输入 | [CI 使用说明](docs/ci.md) |
| 提高合并效率、配置默认合并规则 | [宿主默认合并方案](docs/merging.md) |
| 按单元检查、汇总原始报告 | [CI 单元与汇总](docs/ci-units.md) |
| 理解如何按变更选择测试 | [文件影响范围](docs/filemap-impact.md)、[执行检查](docs/execution.md) |
| 同时修改检查规则和产品 | [混合修改警告](docs/mixed.md)、[成本报告](docs/costs.md) |
| 检查分支、集成结果和迁移 | [工作流检查](docs/workflow.md)、[工作流选测](docs/workflow-selection.md) |
| 创建、恢复或清理工作树 | [工作树管理](docs/worktree.md) |
| 收集外部输入、核对 Git 实际状态 | [输入快照](docs/inputs.md)、[Git 状态核对](docs/git-facts.md) |
| 管理缓存、日志和保留文件 | [CI 缓存](docs/ci-cache.md)、[诊断](docs/diagnostics.md)、[文件保留](docs/artifact-retention.md) |
| 接入发布或完整检查流程 | [发布 CI](docs/release-ci.md)、[完整 CI](docs/full-ci.md) |
| 增加 Cargo 专用检查 | [Cargo 项目检查](docs/cargo-projects.md) |

## 生成 AI 工作指南

按上面的命令安装后，在接入仓库根目录初始化指南：

```sh
.chrono-harness/bin/chrono-instructions init --host-root .
```

生成器内置[默认规则](assets/instructions/catalog.json)和只生成根指南的默认计划，复制可执行文件后也能使用。初始化在目标仓库的 `.chrono-harness/instructions/` 下建立独立的规则目录、输出计划和空上下文，再生成 `CLAUDE.md`，让 `AGENTS.md` 以相对符号链接指向它。

默认使用中文。新仓库可用 `--locale en` 选英文，用 `--methodology M` 导入自定义 UTF-8 方法文件，用 `--host-context C` 导入仓库背景。导入文件未指定语言时记为 `und`；生成器不会猜测或翻译内容。

以后修改仓库自己登记的规则或输出计划，再运行：

```sh
.chrono-harness/bin/chrono-instructions generate --host-root .
```

规则可组合成 Markdown 指南或专用技能文件。未指定布局时先输出依赖，再输出使用它的规则；指定布局时，每条选中的非空规则必须出现一次。布局只影响阅读顺序。缺少翻译、循环依赖或无效引用都会报错。完整格式和示例见[指南生成](docs/instructions.md)。

生成器只接受 schema 2、`atomic-rules/relative-alias/v3` 的输出计划。重复初始化且省略选项时保留已有选择；显式传入不同内容或语言时拒绝覆盖。根指南标记块外的内容由接入仓库维护；额外输出必须有正确的生成器标记。删除或改名输出登记不会自动删旧文件，旧技能文件需要明确清理。内容没变时不写文件，也不重建已有的正确链接。

目前支持 Unix，行为验证范围是当前 macOS。普通读写失败会尝试回滚，报告未恢复路径和备份；尚不保证崩溃或多个写入者同时操作时的完整恢复。退出码：`0` 成功，`2` 用法错误，`1` 生成或读写失败。

规则的[来源说明](docs/methodology-extraction.md)、[逐条处理表](docs/methodology-clause-map.md)和[许可说明](docs/licenses/methodology-attribution.md)另行保留。生成指南只说明文件已生成，不能证明 AI 已阅读、遵守规则或执行检查。

## 在本仓运行检查

本仓约束和工具选择登记在 [.chrono-harness/](.chrono-harness/config.json)。先从当前候选源码安装工具，再使用日常检查入口：

```sh
/bin/bash .chrono-harness/ci/bootstrap-native.sh . .chrono-harness/ci/bootstrap-core.json
# 先提交候选改动，确保工作树干净
.chrono-harness/bin/chrono-harness check
```

同一入口支持 `check --unit ID` 检查单元，以及 `check --collect` 汇总已有结果。汇总不重跑业务操作。本地从登记的远端、目标分支和干净的已提交 `HEAD` 取得输入；生成的 CI 设置 `CHRONO_CHECK_SOURCE=ci`，从事件取得输入。工具保留实际参数、配置、提交范围和输入生产过程。已采用短命令的仓库不能再用旧的长参数拼写。

本仓 CI 的配置源是 `.chrono-harness/ci/units.json`，生成结果是 `.github/workflows/chrono-ci.yml`。流程先确定 Git 变更范围，只启动被选中的单元，最后汇总原始报告。每个单元有独立环境、时限和重跑入口。只要求汇总检查 **chrono / collection** 通过；这里不修改远端分支保护。完整说明见 [CI 单元与汇总](docs/ci-units.md)。

更新工作流时使用登记的生成入口：

```sh
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
```

`verify` 只检查生成结果有没有漂移，不先修复文件。未知路径、缺少 Git 对象、未提交改动、无效登记或命令失败都会返回非零。只改文档且没有依赖边选中业务检查时，报告会明确写出未选择项目检查。

完整检查依次涉及 `registration`、`filemap`、`routes`、`projects`、`cost`、`mixed` 和 `workflow`，分别负责登记核验、影响范围、操作计划、真实执行、成本、混合修改和工作流验证。当前本仓的完整输入和工具绑定仍不齐全；创建工作树、生成配置或局部测试通过，都不能算全面启用。

检查状态必须来自实际报告。固定文本的 `spec status` 和独立 `parity` 比较命令已移除。现有输入生产程序仍报告 `completeness_proven:false`，报告仍标为 `parity:unestablished`。相同命令是本地与 CI 的共同要求；要证明结果必然相同，还须有完整且相同的有效输入，以及确定性的求值过程。两个相同报告本身不足以证明这些条件。

## 示例仓库

| 仓库 | 用途 |
| --- | --- |
| [Go 示例](https://github.com/ChronoAIProject/chrono-harness-examples-go) | 生产模块与独立测试模块，分别登记构建、测试和文件 |
| [TypeScript 示例](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | 不使用 package.json 或 tsconfig，分别检查生产与测试类型 |
| [混合语言示例](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | Go、TypeScript 和独立 Python 脚本，通过明确依赖连接共享数据 |
| [首次接入示例](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 只有文档，登记首次文件清单检查和后续变更检查 |

这四个仓库用于验证安装、本地检查和真实 CI；采用版本、证据和限制见[示例索引](docs/examples.md)。本仓提供可复制的[当前 CI 宿主示例](examples/ci-host-job-gating/README.md)，使用已安装工具和按需运行的独立单元。

## 源码和工具分工

`crates/` 只用于放置独立 Rust 项目，不决定构建和选测。每个生产项目都有对应的 `*-tests` 测试项目，各有自己的 `Cargo.toml`、锁文件和构建目录，没有根 workspace。

| 项目 | 负责什么 |
| --- | --- |
| `runner` | 调用外部检查程序，提供统一 `check` 入口并保存报告 |
| `judge-registration`、`judge-filemap` | 核对登记和输入，计算变更影响范围 |
| `judge-routes`、`judge-projects` | 制定操作计划，执行并核对原始结果 |
| `judge-cargo` | 可选的 Cargo 一致性检查 |
| `judge-cost`、`judge-mixed`、`judge-workflow` | 报告成本、混合修改、集成和迁移结果 |
| `judge-ci`、`ci` | 执行按范围选择的 CI 检查，生成工作流和事件输入 |
| `cache`、`inputs` | 规划缓存，收集并保留明确登记的输入 |
| `worktree` | 创建、重建、恢复和清理工作树 |
| `distribution` | 打包发布、锁定版本和安装工具 |
| `instructions` | 从登记规则生成指南和技能文件 |
| `diagnostics` | JSON Lines 日志和明确的错误链；本仓运行日志位于 `.chrono-harness/state/logs/runtime.jsonl`，每份 10 MiB、共 5 份自动滚动 |

`assets/instructions/` 提供可分发的产品默认规则。接入仓库的选择放在自己的 `.chrono-harness/`，升级产品不会自动覆盖它们。`docs/` 放说明、规范解释和来源资料，`examples/` 放示例，`skills/` 放本仓生成的技能文件。

使用 `sha2` 的独立 Cargo 项目在 dev 和 test 配置中显式设置该依赖的 `opt-level = 3`，减少摘要计算成本，同时保留调试断言。该设置由每个调用项目自己声明，依赖库的构建配置不会自动传给调用方。

## 工作树和交付

`chrono-worktree start` 从登记目标创建新工作树并保存 Git 原始结果；`reconstruct` 按逐路径计划重放要保留的旧改动。冲突和旧工作都会保留，之后仍须重新提交和检查。PR、合并及落地核验由调用方完成。

已采用自动清理的仓库，用 `use --operation` 保护正在消费的文件。调用方等待全部自有任务完成、核对实际交付后，从仍存活的协调仓库运行 `finish --path`；`maintain` 是独立维护入口。未完成或有活跃消费者时保留工作，清缓存不代表任务完成。

恢复、远端分支清理、中断恢复和元数据重建均使用明确计划，保留原始失败和未知结果。各入口的版本、命令和剩余限制见[工作树管理](docs/worktree.md)。最终原件归谁保管、何时可以清理，见[文件保留](docs/artifact-retention.md)。
