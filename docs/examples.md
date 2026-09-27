# 独立示例宿主

产品源码与示例宿主分别维护。以下公开仓库的默认分支都是 `dev`；测试版统一从 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 分发。

| 仓库 | 生产代码 | 专属测试 | 验证重点 |
| --- | --- | --- | --- |
| [chrono-harness-examples-go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/` | `verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [chrono-harness-examples-ts](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts` | `assays/labels/check.mts` | 无 package.json、lockfile、tsconfig；生产与测试分别类型检查 |
| [chrono-harness-examples-mix](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` | `checks/go-contract/`、`checks/ui/`、`checks/scripts/normalize_test.py` | JSON 合同显式连接 Go 与 TS；独立脚本单独选测 |

三者均采用 `.chrono-harness/` 登记、生成的 `CLAUDE.md`、`AGENTS.md -> CLAUDE.md` 和生成的 GitHub Actions。本地和 CI 调用相同的 `chrono-harness check --config .chrono-harness/ci/check.json --base BASE --candidate CANDIDATE`。宿主自己的语言 SDK 与 harness 发布安装分别登记。

已有实际验证包含：源码选择、文档不选业务测试、真实失败测试、未登记文件、指南漂移、任意源码路径搬移；混合宿主另验共享数据和独立脚本选择，TS 与混合宿主另验生产/测试类型错误边界。原生 integration、PR、dev 均已执行 scoped profile。

这些示例当前使用 `chrono-ci-check/v1`。通用 projects schema 和配对消费者现已接受三个示例现有的无 manifest 登记、自定义 action 与任意目录；完整输入闭包、完整治理启用与跨平台同判尚未认证；示例成功不代表这些目标已完成。三个示例已迁移到 [v0.1.0-beta.3](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.3)：已移除产品源码包和 Rust 构建，仅保留版本、摘要、安装声明与入口。本地 macOS arm64 及原生 CI Linux x86_64 均已通过公开 HTTPS 安装和实际宿主检查。

示例的 `push_baselines` 明确登记 `refs/heads/integration/` 对比 `refs/heads/dev`，每次 integration push 都检查完整分支差异；dev push 保持事件 before/after。登记保存在宿主 `.chrono-harness/ci/github.json`，不由语言或目录推断。
