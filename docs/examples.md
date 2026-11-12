# 独立示例宿主

示例仓库用于演示按明确登记接入指南生成、发布安装、本地检查和 CI。产品源码归本仓维护，业务代码、工具版本和宿主政策归各示例维护；它们不随本仓源码自动升级。

| 仓库 | 生产代码与专属测试 | 接入用途 |
| --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/`、`verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts`、`assays/labels/check.mts` | 无 package.json、lockfile、tsconfig，分别检查生产与测试类型 |
| [混合语言](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` 及各自独立测试 | JSON 合同明确连接 Go 与 TS，独立 Python 脚本单独选测 |
| [首次接入](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 文档宿主，无项目及脚本 | 首次文件清单检查及后续普通变更，不添加空测试单元 |

## 当前接入方式

四个示例的默认分支都是 `dev`，当前采用的固定版本由各仓 `.chrono-harness/distribution.json` 负责；迭代期间各仓可能处于不同迁移阶段。安装由该版本的清单与所选工具表生成 `install.sh`，校验 installer、清单和资产的摘要、长度及真实版本；不编译本仓 Rust 源码，也不提交工具、SDK 或缓存。各宿主的指南选择保留，`AGENTS.md` 仍是指向 `CLAUDE.md` 的相对符号链接。

Go、TS、混合语言使用 `chrono-ci-check/v3` 和 `chrono-github-units/v1`，collection provider 为 v4。父 workflow 先检测明确登记的 DELTA，再运行所选独立 job 并汇总原件。Go 的 harness／rates、TS 的 harness／labels、混合的 harness／labels／normalize／rates 分配与业务 SDK profile 均保留；harness 和汇总不安装无关业务 SDK。首次接入使用独立 v4 provider，保留单独的初始清单 profile、普通 DELTA profile，以及空 projects／scripts／units，runner 明确选 macOS arm64／`macos-26`。

四个宿主现已采用 schema4 短命令：普通 `.chrono-harness/bin/chrono-harness check`，单元追加 `--unit ID`，汇总追加无值 `--collect`。profile 的 registration_config 和 facts_config 都绑定根入口；Go 的根入口选择 macOS／Linux 完整政策，保留其精确 Git pin、global 配置摘要和缺席 include。其余宿主不套用 Go 的预期值。schema、安装、生成和行为升级仍须按[宿主定制与更新](ci-units.md#host-customization-and-updates)显式迁移。

## 验证范围

首轮 v0.1.0 中，四个宿主先完成同源码最终测试版的采用，再为正式版分别完成 feature → integration → dev 的真实合并；该轮本地 canonical、dev 落地检查、适用 CodeQL 与原安装回执均已核验通过。历史交付原件分别见 [Go PR #32](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/32)、[TS PR #29](https://github.com/ChronoAIProject/chrono-harness-examples-ts/pull/29)、[混合语言 PR #29](https://github.com/ChronoAIProject/chrono-harness-examples-mix/pull/29)、[首次接入 PR #22](https://github.com/ChronoAIProject/chrono-harness-examples-initial/pull/22)。该轮 Go／TS／混合语言的原生 CI 为 Ubuntu 24.04；首次接入为 macOS 26。Linux／macOS 安装由[该轮正式发布](https://github.com/ChronoAIProject/chrono-harness/actions/runs/43022416702)分别验收。这些历史结果不替代新版本的采用验收；每轮结果由对应 PR／Release 引用原件。

Go 的原 Linux Git pin 漂移已按该候选 bootstrap 原日志显式更新，未放宽约束。首次接入旧 runner 的不可调度注释已定位并迁移。首轮 Python 安装器退役后，远端旧扫描仍固定 Python 并以提取 0 个模块失败，现有凭据访问扫描配置返回 403；原件保留在 PR #18。本轮首次接入的 Actions CodeQL 与汇总检查均成功，实际树仍无 Python；这不证明远端配置为何变化，不回填历史失败，也没有增加空源文件。

这些结果覆盖当前 scoped 采用，不证明全部 SPEC、完整有效输入、七判官全面启用或跨平台同判。当前版本没有重新执行无父初始 push；该项仅保留历史事实，不把普通 DELTA 结果冒充首次清单验收。原失败和 unknown 仍按各自原件解释；原件不可取得时保持未验证。具体源码、run／attempt、报告和摘要由各宿主与发布原件维护，本仓不复制长期运行表。当前产品缺口归[覆盖表](spec-coverage.md)。

## 可复制的当前源码示例

[`examples/ci-host-job-gating`](../examples/ci-host-job-gating/README.md) 演示当前短命令、一个按需启动单元的父工作流，以及原报告汇总。工具获取、detector／harness 需要和业务依赖均明确登记；Shell 只做安装与验证转发，Python 用于示例业务及其测试。

源码示例不能替代公开二进制发布、外部宿主的明确迁移和真实 Actions 采用验收。
