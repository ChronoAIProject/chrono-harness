# 独立示例宿主

产品源码与示例宿主分别维护。以下公开仓库的默认分支都是 `dev`；测试版统一从 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 分发。

| 仓库 | 生产代码 | 专属测试 | 验证重点 |
| --- | --- | --- | --- |
| [chrono-harness-examples-go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/` | `verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [chrono-harness-examples-ts](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts` | `assays/labels/check.mts` | 无 package.json、lockfile、tsconfig；生产与测试分别类型检查 |
| [chrono-harness-examples-mix](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` | `checks/go-contract/`、`checks/ui/`、`checks/scripts/normalize_test.py` | JSON 合同显式连接 Go 与 TS；独立脚本单独选测 |
| [chrono-harness-examples-initial](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 无；文档宿主 | 无；明确登记空项目及脚本集合 | 生成 v2 CI，真实首次 push 库存检查及后续普通 DELTA；macOS arm64 |

四者均采用 `.chrono-harness/` 登记、生成的 `CLAUDE.md`、`AGENTS.md -> CLAUDE.md` 和生成的 GitHub Actions。普通 DELTA 的本地和 CI 调用相同的 `chrono-harness check --config .chrono-harness/ci/check.json --base BASE --candidate CANDIDATE`。宿主自己的语言 SDK 与 harness 发布安装分别登记。

已有实际验证包含：源码选择、文档不选业务测试、真实失败测试、未登记文件、指南漂移、任意源码路径搬移；混合宿主另验共享数据和独立脚本选择，TS 与混合宿主另验生产/测试类型错误边界。原生 integration、PR、dev 均已执行 scoped profile。

Go、TS、混合示例当前使用 `chrono-ci-check/v1`。通用 projects schema 和配对消费者现已接受三个示例现有的无 manifest 登记、自定义 action 与任意目录；完整输入闭包、完整治理启用与跨平台同判尚未认证；示例成功不代表这些目标已完成。三个示例已迁移到 [v0.1.0-beta.11](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.11)：宿主仅保留版本、摘要、安装声明与入口，不保存产品源码包或二进制，不编译 Rust。本地 macOS arm64 及原生 integration、PR、dev 的 Linux x86_64 检查均通过公开 HTTPS 安装和实际宿主检查。

初始化示例当前也采用 beta.11，并明确选择 `chrono-github-ci/v2`，由 `chrono-ci generate` 同时生成工作流和独立的 `.chrono-harness/ci/initial.json`。其历史无父根提交使用自身的 beta.5 锁定配置，登记 18 个文件、空生产项目与脚本集合，不依赖语言或业务目录，并已完成[首次 push 检查](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/36366092276)；本地和 CI 均使用 `chrono-harness check --config .chrono-harness/ci/initial.json --candidate ROOT_OID --initial`，结果为库存完成、`governance: not-evaluated`。随后[文档 DELTA](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/1)在本地、integration、PR 和 dev 使用普通 profile，选择零业务测试。升级至 beta.11 使用普通 DELTA，并已通过本地、integration、PR、dev 检查；不追溯声称根提交使用 beta.11。该实例仅登记 macOS arm64 / `macos-14`，初始判官摘要绑定该平台；不宣称跨平台初始 profile 或完整治理已启用。

示例的 `push_baselines` 明确登记 `refs/heads/integration/` 对比 `refs/heads/dev`，每次 integration push 都检查完整分支差异；dev push 保持事件 before/after。登记保存在宿主 `.chrono-harness/ci/github.json`，不由语言或目录推断。

四个宿主的 beta.11 采用已合入 dev：[Go #12](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/12)、[TS #12](https://github.com/ChronoAIProject/chrono-harness-examples-ts/pull/12)、[mix #12](https://github.com/ChronoAIProject/chrono-harness-examples-mix/pull/12)、[initial #7](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/7)。每个宿主明确安装 `chrono-worktree` 并登记 `.chrono-harness/worktree.json`；所需的共享登记仍为 proposed，不代表完整治理启用。Go、TS、mix 的普通检查分别执行 4、5、10 个既有操作，初始化示例选择零业务操作；原有测试身份与命令保留。beta.10 提供 full config v3、scoped v2 和 CI provider v3 的显式 Git 绑定，但此次升级保留各宿主既有 profile，不自动启用这些扩展。beta.9 升级的历史首轮本地检查曾在未修改的指南生成测试超时；原始失败保留，后续规范检查及原生检查通过，首轮原因未确认。本次 beta.11 结果不改写该历史。

本次 beta.11 验证另在本地 macOS arm64 使用安装后的公开 `chrono-worktree`，从四个宿主各自的真实 `origin/dev` 创建新 feature 工作树，并用登记的 `cleanup` 核对工作树与分支已移除；创建基线、工具摘要和原始 Git 进程证据均已核对。本次消费验证覆盖 `start`／`cleanup`，并用安装后的公开二进制对四条已交付升级分支执行登记的 `cleanup-remote`；核对原始过程字节、保留提交及远端分支缺席。

beta.8 的历史验证中，四个已合入宿主在本地 macOS arm64 使用当时的 `chrono-worktree`，针对各自真实公开 `origin/dev` 验证了两个中断入口：checkout hook 终止自有进程后，通过 `recover-interrupted` 验证显式协调的 HEAD／暂存树；fetch 的引用事务 hook 终止自有进程后，通过 `cleanup-fetch-interrupted` 验证保留提交并删除临时引用。核对了 Git 进程字节、原始意图和空结果文件、未提交 README 内容的保留；协调后的暂存内容先提交到保留分支，再由 `cleanup` 验证工作树与原分支实际移除。两个原进程均实际停止，原操作结果仍为未知。测试仅覆盖上述明确操作与状态；该历史消费未覆盖丢失收据或损坏的 Git 元数据；这些状态的显式重建由下述 beta.11 消费覆盖。中断重建续跑与 PR／合并编排仍未交付；远端分支退休从 beta.10 起有独立合同，治理和确定性同判均未建立。

## 当前采用身份

以下记录固定本次已合入的候选、落地提交和实际 dev 检查；原始检查产物由各自 CI 保留。

| 宿主 | 候选 | 落地提交 | dev 检查 |
| --- | --- | --- | --- |
| go | `8ac0d4e6b629d2b03a79369ad2eb98ee6ca0b5a9` | `bff83ea6e7e84b7b0d44fd12f631fb2d393cf0df` | [36569072104](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36569072104) |
| ts | `2ed96ef466dbb197c2bf9afd8d4a1ac7fec126c7` | `5c3537e6073f4923f141da213208585cfb4d0664` | [36569090125](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36569090125) |
| mix | `e0565faa24711cb23596d3f37471e767aa2e3754` | `69e5d041abc18edef63b1e5a7d98cd60368238cc` | [36569113047](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36569113047) |
| initial | `d92f113e46c7f57c70e88c6d0ceff99021a5b99a` | `2b0dd825d031746e28f2da6e6710f04f10dcee38` | [36569134544](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/36569134544) |

四者继续显式使用既有 scoped profile。发布／full CI 生成能力的分发不自动改变宿主配置，也不证明原生 full dispatch、完整输入闭包或确定性同判。

## beta.11 原始 checkout 与显式重建

已从升级后的 mix 公开 dev 克隆固定落地提交，使用公开安装入口安装 beta.11。在本地 macOS arm64，`core.filemode=false` 隐藏 README 执行位变化时，Git diff 无路径，但规范 check 退出 1、cleanup 退出 2，工作保持；恢复执行位后同命令通过。

同一消费显式准备暂存／未暂存内容、二进制、Unicode 路径、可执行文件和字面 symlink，损坏 index 并移除旧生产者收据。`inspect-rebind`／`rebind` 按给定 branch、HEAD、index tree、备份和 donor 重建，核对可见工作、选定索引及残存旧 metadata 均保留。原操作结果仍未知；保存该工作到保留提交后，登记 cleanup 成功。该结果不包含中断 rebind 续跑、历史索引推断、完整治理或同判。
