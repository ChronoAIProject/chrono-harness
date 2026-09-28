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

Go、TS、混合示例当前使用 `chrono-ci-check/v1`。通用 projects schema 和配对消费者现已接受三个示例现有的无 manifest 登记、自定义 action 与任意目录；完整输入闭包、完整治理启用与跨平台同判尚未认证；示例成功不代表这些目标已完成。三个示例已迁移到 [v0.1.0-beta.6](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.6)：宿主仅保留版本、摘要、安装声明与入口，不保存产品源码包或二进制，不编译 Rust。本地 macOS arm64 及原生 integration、PR、dev 的 Linux x86_64 检查均通过公开 HTTPS 安装和实际宿主检查。

初始化示例当前也采用 beta.6，并明确选择 `chrono-github-ci/v2`，由 `chrono-ci generate` 同时生成工作流和独立的 `.chrono-harness/ci/initial.json`。其历史无父根提交使用自身的 beta.5 锁定配置，登记 18 个文件、空生产项目与脚本集合，不依赖语言或业务目录，并已完成[首次 push 检查](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/36366092276)；本地和 CI 均使用 `chrono-harness check --config .chrono-harness/ci/initial.json --candidate ROOT_OID --initial`，结果为库存完成、`governance: not-evaluated`。随后[文档 DELTA](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/1)在本地、integration、PR 和 dev 使用普通 profile，选择零业务测试。升级至 beta.6 使用普通 DELTA，并已通过本地、integration、PR、dev 检查；不追溯声称根提交使用 beta.6。该实例仅登记 macOS arm64 / `macos-14`，初始判官摘要绑定该平台；不宣称跨平台初始 profile 或完整治理已启用。

示例的 `push_baselines` 明确登记 `refs/heads/integration/` 对比 `refs/heads/dev`，每次 integration push 都检查完整分支差异；dev push 保持事件 before/after。登记保存在宿主 `.chrono-harness/ci/github.json`，不由语言或目录推断。

四个宿主的 beta.6 采用已合入 dev：[Go #7](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/7)、[TS #7](https://github.com/ChronoAIProject/chrono-harness-examples-ts/pull/7)、[mix #7](https://github.com/ChronoAIProject/chrono-harness-examples-mix/pull/7)、[initial #2](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/2)。每个宿主明确安装 `chrono-worktree` 并登记 `.chrono-harness/worktree.json`；所需的共享登记仍为 proposed，不代表完整治理启用。Go、TS、mix 的普通检查分别执行 4、5、10 个既有操作，初始化示例选择零业务操作；原有测试身份与命令保留。

四个已合入的公开宿主还在本地 macOS arm64 使用发布的二进制，针对各自真实 `origin/dev` 完成 `start` 和带 README 变更的 `reconstruct`，核对原源保留、暂存路径、树和 Git 进程字节。该消费验证不把工作树结果升级为治理成功；临时验证工作树及分支由调用方在核对身份后清理。自动恢复／清理和 PR／合并编排仍未交付。
