# 发布与下游迭代验收

本文是测试版到正式版的唯一过程记录。它记录可回读的源码、工作流、发布、安装和下游落地身份；状态只推进到证据覆盖的阶段。`docs/release-ci.md` 负责发布器合同，`docs/distribution.md` 负责安装器合同，`docs/examples.md` 负责现役下游清单，本文件负责把它们串成一轮可重复验收。

## 生命周期

每轮按以下顺序执行：

1. 从最新 `dev` 审计发布源、安装声明、受影响的测试和四个下游的当前版本；固定候选 source/tree、适用 CI run/attempt 和 workflow blob。
2. 只使用固定身份发布一个测试版。核两平台 frontend、业务验证、collector、公开资产、标签源码和两平台安装；任一步失败都保留原始回执，不能把重试或新候选的成功补给旧身份。
3. 在四个下游分别从最新目标分支建立 `feature → integration → dev` 晋升链。采用固定版本的生成安装声明，保留宿主定制，运行各自登记的本地入口和真实 CI。
4. 失败先读原 stdout、stderr、退出码、实际输入和候选树，在产生错误的负责层修复。修复后重新走受影响的 feature、integration 和 dev 检查；独立下游继续推进，旧失败保留为历史证据。
5. 只有最终测试版的发布、安装和四个下游 dev 落地均通过，且没有未解释的候选相关失败，才把同一源码晋升到 `main` 并派发 stable。stable 必须携带最终测试版 run/attempt，不能重新构建或移动测试版标签。
6. 核 stable Release 的公开状态、标签源码、完整资产和安装结果，再以相同三段晋升链迁移四个下游。正式采用完成后关闭本轮目标，另设下一轮目标；没有真实净改动就不创建新版本。

每个候选的身份至少包含：源码 commit 和 tree、版本字符串、上游 landing CI run/attempt、发布 workflow head/blob、公开 `release.json` 摘要和长度、两个平台 installer 摘要和长度，以及每个下游 feature/integration/dev 提交和落地 CI run。只凭 commit SHA、PR 状态或一个绿色 job 不足以证明交付。

等待使用原生完成通知或真正阻塞的 `gh run watch`，一次等待后读取原始结果。通道丢失、认证失败或超时保留 run 句柄和 unknown 状态，不重复派发；禁止用短周期轮询或新运行掩盖原结果。

## 当前 v0.1.3 记录

本轮最终测试版固定为：

| 项目 | 身份 |
| --- | --- |
| preview 版本 | `v0.1.3-dev.43314625058.1.ga7b59227bb2323162d7531a0bd5e857175f72589` |
| source / tree | `a7b59227bb2323162d7531a0bd5e857175f72589` / `8d2f7b092056a56df06f4e9ac738040bfa913107` |
| landing CI | run `43314625058`, attempt `1` |
| preview release | run `43317169856`, attempt `1` |
| stable release | run `43326911672`, attempt `1` |
| stable tag | `v0.1.3`, target `a7b59227bb2323162d7531a0bd5e857175f72589` |
| stable manifest | `release.json`, SHA-256 `83caaa4e79485e0ad2ad0f45d84701d53dd34e0fa3234ea0570c6ee2dbb3651d`, 6589 bytes |

preview 的 34 个公开资产、两平台安装和上游单元已通过。正式发布为公开、非 draft、非 prerelease，稳定版公开资产完整。stable workflow 的 `linux_install` 和 `macos_install` 在本次手动 stable run 中被跳过，因此不把它们写成正式发布作业的通过证据；稳定版的 macOS 安装由四个下游本地安装回执覆盖，Linux 安装由四个下游 dev CI 覆盖。

四个下游最终 dev 落地如下：

| 下游 | dev 落地 SHA | 稳定版 dev CI | CodeQL | 实际单元 |
| --- | --- | --- | --- | --- |
| Go | `973d31e4c99a4c55dcaaa5dab2d7ab3a95891c9a` | `43328703853` | `43328703462` | `harness`, `rates` |
| TypeScript | `fa58c86d5b33805a13229e40ce4ddbea6e9f87fa` | `43328714243` | `43328713560` | `harness`, `labels` |
| Mix | `5ab8e778bfd5e1d45d9983690d50a6b3840f4164` | `43328725136` | `43328724187` | `harness`, `labels`, `normalize`, `rates` |
| Initial | `010a7fd82dda6c4c0daebf579b19ea07493dc4c1` | `43328736750` | `43328735430` | 文档宿主检查 |

四个 dev 的 `.chrono-harness/distribution.json` 都锁定 `v0.1.3` 和上述 manifest 摘要；每个下游的 stable feature→integration→dev PR 均已实际合并。Go 预览迁移曾在 run `43323417443` 因 Linux Git 摘要绑定失败，诊断 run `43324156277` 确认检测 job 与汇总 job 使用了不同的 mutable runner Git 字节；修复在 Go 的 Linux schema4 政策层改用短绑定，最终 preview run `43325200790` 全部通过。该失败原件仍保留，不能由后续绿色结果抹除。

## 稳定门槛与下一轮

下列条件全部满足时，候选才可 stable：上游 preview 公开资产和两平台安装通过；四个下游的 feature、integration、dev 链均落地；dev 上的真实单元、汇总和必要安全检查通过；候选相关失败已经在负责层修复并重新验证；stable Release 的标签、源码、资产和下游稳定版声明一致。任一条件缺失都保持 preview 或 unknown。

正式采用完成后，下一轮目标从稳定版源码的下一个真实改动开始。先审计当前使用方和 SPEC，确认改动的最小范围，再产生新的 feature、测试版和下游波次；不要因为流程本身生成版本。

