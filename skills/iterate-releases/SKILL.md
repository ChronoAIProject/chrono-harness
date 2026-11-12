---
name: iterate-releases
description: 持续发布测试版，迁移和验证真实下游，修复反馈，并在稳定门槛满足后发布正式版和完成下游正式采用；每轮把真实证据写回迭代文档。
---

# 持续发布与下游迭代

在用户已授权的仓库和发布范围内工作。先读取各仓库的 AGENTS、host_context、发布源、安装声明、当前 SPEC 和 `docs/release-iteration.md`，再审计真实使用方、验收入口、保留证据和失败现场。个人目录中的 skill 是安装副本，仓库 `skills/iterate-releases/SKILL.md` 是唯一维护源。

## 每轮执行

1. 固定候选身份：源码 commit/tree、计划版本、landing CI run/attempt、发布 workflow head/blob、公开资产以及两个平台安装结果。检查是否已有同一身份的成功自动发布；没有原生运行时才按固定 ref 派发，不能把移动的 `dev` 或 `latest` 当作候选。
2. 先发布测试版。检查 frontend、所有登记验证、collector、公开 Release、标签源码、manifest/installer 字节和两平台安装。发布输入必须同时固定 `source`、`--ref`、run/attempt 和 workflow 修订；失败保留原 stdout、stderr、退出码和 artifacts，不改写旧回执、不移动标签、不按 SHA 单独复用。
3. 四个现役示例按 `feature → integration → dev` 逐仓迁移。使用候选安装器的显式 adopt/replace 更新版本和摘要，保留业务测试、SDK、工作流和宿主定制；本地检查、feature PR、integration PR 和 dev 落地 CI 都要核实际提交 SHA。自动合并不可用时使用已有合并权限，不申请新权限。
4. 失败先读原始结果和实际输入，修复产生错误的负责层。若 runner、工具或环境是 mutable，不能把一次观察写成跨 job 的硬摘要；固定 runner 才使用 executable pin，否则使用 schema4 短绑定并保留每次实际身份。修复后只重跑受影响的依赖闭包，独立下游继续推进。
5. 所有等待使用原生完成通知或一次 `gh run watch`。保留原 run 句柄；认证失败、丢通知和超时保持 unknown，不重复派发或用新 run 覆盖旧结果。
6. 只有稳定门槛全部满足才晋升：最终 preview 资产和两平台安装通过、四个下游 dev 落地并全绿、候选相关失败已修复并复验、源码没有漂移。按发布合同将 `main` 快进到同一源码，使用最终 preview run/attempt 派发 stable，核公开状态、标签、完整资产和安装。
7. stable 发布后再次运行四个下游的同一晋升链。确认 dev 锁定正式版本且落地 CI 通过，再关闭本轮目标并创建下一轮目标；没有真实净改动不产生新版本。

## 证据和报告

每轮在 `docs/release-iteration.md` 写入版本、source/tree、run/attempt、workflow blob、Release 状态、公开资产、平台安装、下游 PR/merge/dev SHA 和 CI run。报告只覆盖已核实阶段：PR 合并、提交、工作流完成或状态为 CLOSED 都不能单独证明交付。失败原件保留到调试和验收义务结束，unknown 不得写成稳定。

stable workflow 如果跳过安装 job，明确记录跳过状态，不宣称该 job 通过；可以由同一公开 stable 资产的真实下游 macOS 安装和 Linux dev CI 补齐平台采用证据，但仍保留 workflow 缺口和恢复条件。

执行结束后更新本 skill 和其负责文档中暴露出的非显然问题，运行 FILEMAP/登记判官及受影响的文档检查，并同步个人安装副本。不要新增第二套发布器、常驻轮询器、人工审批关卡或为未来需求预建扩展层。
