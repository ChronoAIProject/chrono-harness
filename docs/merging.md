# 宿主默认合并方案

独立 PR 并行检查，通过必需检查后及时合并；只按实际依赖、冲突和有效输入决定先后。默认不要求分支追平目标，也不按 PR 编号或创建时间排队。

本仓的整体开发方式见[开发、集成与发布流程](development.md)。仓库已允许 GitHub 原生自动合并；调用方在每个就绪 PR 上提前启用，入口归[宿主自动交付授权](../.chrono-harness/instructions/host-context.md#自动交付授权)。必需 CI 和现有保护满足后，GitHub 直接合并。AI 负责修复失败及核对落地结果，不增加合并前放行步骤。

此默认由产品的 `candidate.landing` 原子提供，新宿主运行 `chrono-instructions init` 即采用。现有宿主按[默认采用步骤](instructions.md#新默认的宿主采用边界)保留定制并更新登记源。GitHub CI 由 `chrono-ci init/generate` 生成；生成器不修改远端保护规则或自动取得合并权限。

## GitHub 配置

宿主使用 GitHub 原生规则集，关闭 `strict_required_status_checks_policy`，保留必需汇总检查、既有评审要求和禁止绕过的设置。已有宿主先验证并合入合并树校验工作流，再关闭 strict。默认无需 merge queue、独立合并机器人、轮询服务或新增权限。

本仓唯一采用文件为 [.chrono-harness/ci/admission.json](../.chrono-harness/ci/admission.json)，覆盖 dev 与所有层级的 integration 分支，必需检查为 `chrono / collection`，评审人数保持现有的 0。规则集的 `refs/heads/integration/**/*` 使用 GitHub 的分支匹配语法；工作流已对目标为 dev 或 `integration/**` 的 PR 运行相同汇总。工作流触发和必需检查保护须分别配置，能运行 CI 不表示合并已受保护。部署入口是：

```sh
gh api --method PUT repos/ChronoAIProject/chrono-harness/rulesets/24355777 --input .chrono-harness/ci/admission.json
```

这个入口只使用调用方已有的仓库规则管理权限；普通 CI 仍只需 `contents: read` 和 `actions: read`。权限不足时保留原失败，不新增运输或激活机制。其他宿主复制采用文件后，明确修改仓库、规则集 ID、目标分支、汇总检查名称和既有评审要求，并登记其自己的部署入口。部署后读取实际规则集，并分别核 dev、单层与多层 integration 目标分支的有效规则，确认 strict 已关闭、必需检查及其他保护仍在。文件提交或请求退出成功不能代替生效核验。

`do_not_enforce_on_create` 允许首次发布工作分支，避免要求一个尚未能运行 PR CI 的新引用先有检查；创建不表示候选通过。已有 integration 分支的后续更新与 dev 一样走 PR，修复从 feature 分支提交到该 integration；最终交付 dev 的 PR 仍验其固定合并树。直接 push 更新受保护目标继续受原生规则限制。

## 检查与落地

| 时点 | 固定输入 | 验证范围 |
| --- | --- | --- |
| PR | 原生 workflow 的合并提交 M；第一父 B；第二父须等于事件 head H | 所有作业检出同一个 M，按 B→M 的完整 DELTA 和明确依赖选测 |
| 目标分支 push | 事件 before／after | 检查实际落地变化，包括一次 push 的多个提交 |
| 同一 PR 更新 | 新事件固定输入 | 原生 concurrency 替换该 PR 的旧运行 |
| 不同 PR 或目标 push | 各自固定输入 | 独立调度；push 不互相取消，不丢弃落地检查 |

GitHub 运行、job 和 artifact API 的 `head_sha` 对 PR 指 H，不能拿它代替正在测试的 M。检测结果分别绑定 `candidate=M` 和 `event_head=H`；汇总按 H 核对原生运行归属，按 M 核对实际测试和原始报告。缺失、错配、失败或意外跳过的必需结果继续拒绝，不能只凭同名绿灯合并。

新 feature／integration 仍从创建当时最新 dev 开始。目标分支前移本身不要求重建；实际冲突、必要输入或明确依赖变化时，保全工作并补验受影响范围。依赖 PR 可以先后交付，或在明确 integration 候选中组合验收。规则、CI、门控、发布链和稳定性改动先走 feature → 新 integration 的真实 PR，验收并实际合入后再交付 dev；共同合同的冲突表示与局限归[开发流程](development.md#并行改动与真实冲突)。

检查时的 M1 与落地时的 M2 可能不同。这套默认方案通过目标 push 发现新增交互并及时修复，不能声称准入时严格同树。有明确同树要求的发布、信任根或不可逆操作，须由宿主另行登记并验证能构建实际落地树的合并执行方案；未验证的 `merge_group` 或队列名称不构成保证。完整治理的 integration 凭据仍按其明确绑定消费，不能将 M1 的凭据冒用到 M2。

## 参考与验收

参考 trureturing 固定修订 [`b6bf8a2f`](https://github.com/the-omega-institute/trureturing/blob/b6bf8a2fbae0e4e6fbef60c6985362066171ade1/CLAUDE.md) 的 §7.7、§9.5，以及其 [CI 入口](https://github.com/the-omega-institute/trureturing/blob/b6bf8a2fbae0e4e6fbef60c6985362066171ade1/.github/workflows/ci-current.yml)。当前规则以实际 API 结果为准；历史规则查询和运行耗时直接查原 PR／CI，不在本文维护观察副本。成本比较须固定输入与测量窗口，区分排队、执行和计费范围。

验收需覆盖：PR 合并树包含 head 之外的目标分支变化；旧事件 base 不覆盖固定第一父；head 检出或错误父身份拒绝；运行／工件归属与被验树分别正确绑定；完整 push DELTA 和独立运行保留。Rust 行为测试只证明相应程序行为，原生事件、工件传递、规则实际生效和交付后检查仍须真实核验。尚未实测的新吞吐、总成本和严格同树保证保持未知。
