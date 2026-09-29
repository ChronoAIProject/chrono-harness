# 独立示例宿主

产品源码与示例宿主分别维护。以下公开仓库的默认分支都是 `dev`；测试版统一从 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 分发。

| 仓库 | 生产代码 | 专属测试 | 验证重点 |
| --- | --- | --- | --- |
| [chrono-harness-examples-go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/` | `verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [chrono-harness-examples-ts](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts` | `assays/labels/check.mts` | 无 package.json、lockfile、tsconfig；生产与测试分别类型检查 |
| [chrono-harness-examples-mix](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` | `checks/go-contract/`、`checks/ui/`、`checks/scripts/normalize_test.py` | JSON 合同显式连接 Go 与 TS；独立脚本单独选测 |
| [chrono-harness-examples-initial](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 无；文档宿主 | 无；明确登记空项目及脚本集合 | 生成 v2 CI，真实首次 push 库存检查及后续普通 DELTA；macOS arm64 |

四者均采用宿主 `.chrono-harness/` 登记、生成的 `CLAUDE.md`、`AGENTS.md -> CLAUDE.md` 和生成的 GitHub Actions。当前公开版本为 [v0.1.0-beta.13](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.13)：只保留版本锁、摘要、安装声明与入口，按公开 HTTPS 下载选定工具，不保存产品源码包或二进制，也不编译 Rust。

Go、TS、混合示例使用 `chrono-ci-check/v3` 和 `chrono-github-units/v1`。每个单元有独立 workflow、状态、SDK profile 和重跑；本地及 CI 使用相同的 `chrono-harness check --config .chrono-harness/ci/check.json --base BASE --candidate CANDIDATE --unit UNIT`。汇总使用同入口的 `--collect MANIFEST`，核验原始报告并执行零项业务操作。单元、完整测试计划、SDK 下载／探测与依赖均为显式登记；语言、目录、import 或自动扫描不产生选测权威。harness 和 collection 的 SDK profile 不安装语言工具链。

| 宿主 | 已登记单元 | 当前版本采用 | dev 汇总／检查 |
| --- | --- | --- | --- |
| go | `harness`, `rates` | [PR #14](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/14) | [36591356680](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36591356680) |
| ts | `harness`, `labels` | [PR #14](https://github.com/ChronoAIProject/chrono-harness-examples-ts/pull/14) | [36591398653](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36591398653) |
| mix | `harness`, `labels`, `normalize`, `rates` | [PR #14](https://github.com/ChronoAIProject/chrono-harness-examples-mix/pull/14) | [36591439218](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36591439218) |
| initial | 初始清单；空执行计划 | [PR #9](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/9) | [36591158006](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/36591158006) |

上述升级已通过本地、integration、PR 和 dev 检查。三个业务宿主使用本地 macOS arm64 与原生 Linux x86_64；版本升级只改公开版本锁与说明，CI／SDK 源、生成 workflow、启动命令、指南与业务／测试源码保持原字节，原有测试身份保留。Go、TS、mix 分别执行 4、5、10 个登记操作。宿主配置与生成物的更新方式见[自定义和迁移合同](ci-units.md#host-customization-and-updates)；公开 beta.13 的 macOS `chrono-ci` 已用三个宿主的真实旧／新 provider 验证迁移、配置保留与逐字节生成结果。

初始化示例保留 `chrono-github-ci/v2`，由生成器同时生成 workflow 和独立 `.chrono-harness/ci/initial.json`；没有执行计划或脚本，不添加空单元。历史无父根提交仍用自身 beta.5 锁和 `check --config .chrono-harness/ci/initial.json --candidate ROOT_OID --initial`，已完成[首次 push 库存检查](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/36366092276)。后续版本升级用真实 before/after 的普通 DELTA，执行零业务操作；不追溯更改根提交的版本。本例仅登记 macOS arm64 / `macos-14`，不宣称跨平台初始 profile 或完整治理启用。

各宿主显式登记 integration 分支对比 `origin/dev`，每次检查完整分支差异；dev push 使用事件 before/after。三个业务宿主的 provider 位于 `.chrono-harness/ci/units.json`，初始宿主保留 `.chrono-harness/ci/github.json`。通用 projects 接受无 manifest 登记、自定义 action 和任意目录；这些示例未认证完整输入闭包、完整七判官治理或跨平台同判，也不自动采用 full Git 绑定扩展。

beta.12 的独立原生验证覆盖两类故障：Go 的 rates 单元被取消时，harness 单元独立成功、汇总拒绝；只重跑 rates 和汇总即可恢复，harness 仍为 attempt 1。混合宿主的 Go 构建成功后，`TestContract` 与 `TestSharedInvoice` 命中预定失败，TS 的类型检查与业务测试独立成功，汇总保留失败并拒绝通过。恢复到基线相同的文件树后，本地／原生单元均执行零业务操作，汇总通过；故障未合入 dev，实验分支已用登记工具退休。这些结果不被后续二进制升级重新标为 beta.13 实验。

已有实际验证还覆盖源码选择、文档不选业务测试、未登记文件、指南漂移、任意路径迁移、共享 JSON 合同、独立脚本以及 TS 生产／测试类型错误。早期 beta.9 本地指南测试的原始超时保持未解原因，后续通过不改写该失败。beta.11 已使用四个真实公开 origin 验证 `chrono-worktree start`／`cleanup`；后续版本采用也使用公开工具核验本地和远端升级分支的登记退休。beta.12 混合宿主的一次远端清理收到服务器错误，原失败保留，显式重试核验缺席。

beta.8 的历史验证中，四个已合入宿主在本地 macOS arm64 使用当时的 `chrono-worktree`，针对各自真实公开 `origin/dev` 验证了两个中断入口：checkout hook 终止自有进程后，通过 `recover-interrupted` 验证显式协调的 HEAD／暂存树；fetch 的引用事务 hook 终止自有进程后，通过 `cleanup-fetch-interrupted` 验证保留提交并删除临时引用。核对了 Git 进程字节、原始意图和空结果文件、未提交 README 内容的保留；协调后的暂存内容先提交到保留分支，再由 `cleanup` 验证工作树与原分支实际移除。两个原进程均实际停止，原操作结果仍为未知。测试仅覆盖上述明确操作与状态；该历史消费未覆盖丢失收据或损坏的 Git 元数据；这些状态的显式重建由下述 beta.11 消费覆盖。中断重建续跑与 PR／合并编排仍未交付；远端分支退休从 beta.10 起有独立合同，治理和确定性同判均未建立。

## beta.11 原始 checkout 与显式重建

已从升级后的 mix 公开 dev 克隆固定落地提交，使用公开安装入口安装 beta.11。在本地 macOS arm64，`core.filemode=false` 隐藏 README 执行位变化时，Git diff 无路径，但规范 check 退出 1、cleanup 退出 2，工作保持；恢复执行位后同命令通过。

同一消费显式准备暂存／未暂存内容、二进制、Unicode 路径、可执行文件和字面 symlink，损坏 index 并移除旧生产者收据。`inspect-rebind`／`rebind` 按给定 branch、HEAD、index tree、备份和 donor 重建，核对可见工作、选定索引及残存旧 metadata 均保留。原操作结果仍未知；保存该工作到保留提交后，登记 cleanup 成功。该结果不包含中断 rebind 续跑、历史索引推断、完整治理或同判。
