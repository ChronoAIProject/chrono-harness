# 独立示例宿主

示例仓库用于演示按明确登记接入指南生成、发布安装、本地检查和 CI。产品源码归本仓维护，业务代码、工具版本和宿主政策归各示例维护；它们不随本仓源码自动升级。

| 仓库 | 生产代码与专属测试 | 接入用途 |
| --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `engine/rates/`、`verification/contracts/` | 两个独立 Go 模块，无根 workspace |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `logic/text/label.mts`、`assays/labels/check.mts` | 无 package.json、lockfile、tsconfig，分别检查生产与测试类型 |
| [混合语言](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `backend/calc/`、`client/view/`、`jobs/normalize.py` 及各自独立测试 | JSON 合同明确连接 Go 与 TS，独立 Python 脚本单独选测 |
| [首次接入](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 文档宿主，无项目及脚本 | 首次文件清单检查及后续普通变更，不添加空测试单元 |

## 当前接入方式

四个示例的默认分支都是 `dev`，安装声明位于各自的 `.chrono-harness/distribution.json`，当前均锁定测试版 `v0.1.5-dev.43478399135.1.g07876e055ad618bf830788d37458377dcca6a9e6` 及其 `release.json` 摘要。它们下载选定的发布工具，不编译本仓 Rust 源码，也不提交工具二进制、SDK 或缓存。指南选择由各自 `.chrono-harness/instructions/` 维护，`AGENTS.md` 是指向 `CLAUDE.md` 的相对符号链接。

Go、TS、混合示例采用 `chrono-ci-check/v3` 和 `chrono-github-units/v1`，在 `.chrono-harness/ci/units.json` 明确登记单元、工作流、SDK、测试计划及依赖。Go 有 harness／rates 两个单元，TS 有 harness／labels 两个单元，混合示例有 harness、labels、normalize、rates 四个单元；harness 与汇总不安装业务 SDK。首次接入示例采用独立 `chrono-github-ci/v2`，只登记 macOS arm64／`macos-14`。

四个示例沿用 `feature → integration → dev` 晋升链，当前 preview 的四个 `dev` 分支业务检查和必要安全检查均已通过；每个宿主仍保留自己的业务代码、SDK 选择和 CI 政策。上一正式版 `v0.1.4` 仍是稳定基线；版本和行为升级按[宿主定制与更新](ci-units.md#host-customization-and-updates)明确迁移。

## 验证范围

当前 preview 已有四个宿主的本地 macOS 真实安装和 `dev` 原生 CI 采用，包括单元执行、原报告汇总及宿主安全检查。Linux 采用由四个宿主对应的 `dev` CI 实际验证。具体候选、运行、原报告和失败直接查各宿主的 PR、Actions 和发布原件；本仓的逐轮身份、落地 SHA 和运行号记录在[发布与下游迭代](release-iteration.md)。

稳定版 workflow 本身的 `linux_install` 和 `macos_install` 在本轮手动 stable run 中被跳过，因此采用证据由公开 stable 资产的本地 macOS 安装和四个下游 `dev` CI 覆盖，不能把被跳过的 job 写成通过。Go 预览阶段的 Linux Git 摘要绑定失败已在负责的 Git 政策层修复并复验；旧失败和未知结果仍按各自原件解释，后续成功不补作历史证据。当前产品缺口归[覆盖表](spec-coverage.md)。

## 可复制的当前源码示例

[`examples/ci-host-job-gating`](../examples/ci-host-job-gating/README.md) 演示当前短命令、一个按需启动单元的父工作流，以及原报告汇总。工具获取、detector／harness 需要和业务依赖均明确登记；Shell 只做安装与验证转发，Python 用于示例业务及其测试。

源码示例不能替代公开二进制发布、外部宿主的明确迁移和真实 Actions 采用验收。
