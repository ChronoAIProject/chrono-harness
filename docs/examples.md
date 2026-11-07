# 独立示例宿主

示例仓库用于演示按明确登记接入指南生成、发布安装、本地检查和 CI。产品源码归本仓维护，业务代码、工具版本和宿主政策归各示例维护；它们不随本仓源码自动升级。

| 仓库 | 生产代码与专属测试 | 接入用途 |
| --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/`、`verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts`、`assays/labels/check.mts` | 无 package.json、lockfile、tsconfig，分别检查生产与测试类型 |
| [混合语言](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` 及各自独立测试 | JSON 合同明确连接 Go 与 TS，独立 Python 脚本单独选测 |
| [首次接入](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 文档宿主，无项目及脚本 | 首次文件清单检查及后续普通变更，不添加空测试单元 |

## 当前接入方式

四个示例的默认分支都是 `dev`，安装声明位于各自的 `.chrono-harness/distribution.json`，锁定独立发布 beta.20。它们下载选定的发布工具，不编译本仓 Rust 源码，也不提交工具二进制、SDK 或缓存。指南选择由各自 `.chrono-harness/instructions/` 维护，`AGENTS.md` 是指向 `CLAUDE.md` 的相对符号链接。

Go、TS、混合示例采用 `chrono-ci-check/v3` 和 `chrono-github-units/v1`，在 `.chrono-harness/ci/units.json` 明确登记单元、工作流、SDK、测试计划及依赖。Go 有 harness／rates 两个单元，TS 有 harness／labels 两个单元，混合示例有 harness、labels、normalize、rates 四个单元；harness 与汇总不安装业务 SDK。首次接入示例采用独立 `chrono-github-ci/v2`，只登记 macOS arm64／`macos-14`。

这几份旧发布配置仍使用该版本的显式参数命令，不能据此推导本仓当前短命令已经在这些宿主采用。Go 的平台 Git 政策接入见 [PR #18](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/18)，公开产物清单排除接入见 [PR #19](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/19)。版本和行为升级按[宿主定制与更新](ci-units.md#host-customization-and-updates)明确迁移。

## 验证范围

固定旧发布已有本地 macOS 和原生 CI 的有限采用，包括单元执行、原报告汇总、取消后的选择性重试，以及业务失败时无关单元继续。具体候选、运行、原报告和失败直接查各宿主的 PR、Actions 和发布原件；本仓不维护运行表、测试数量、报告大小或安装摘要的副本。

这些观察不证明当前候选或 beta.20 的原生采用、完整有效输入、七判官全面启用、跨平台同判，也不覆盖全部恢复状态。旧失败和未知结果仍按各自原件解释，后续成功不补作历史证据；原件不可取得时保持未验证，本文说明不能代替原字节。当前产品缺口归[覆盖表](spec-coverage.md)。

## 可复制的当前源码示例

[`examples/ci-host-job-gating`](../examples/ci-host-job-gating/README.md) 演示当前短命令、一个按需启动单元的父工作流，以及原报告汇总。工具获取、detector／harness 需要和业务依赖均明确登记；Shell 只做安装与验证转发，Python 用于示例业务及其测试。

源码示例不能替代公开二进制发布、外部宿主的明确迁移和真实 Actions 采用验收。
