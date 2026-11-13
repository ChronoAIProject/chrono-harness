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

## v0.1.4 测试版迭代记录（2026-11-13）

第一轮候选固定为 `v0.1.4-dev.43341524786.1.gf5c541e2a5cbfa2ca78760f55730fac9922717b7`：source `f5c541e2a5cbfa2ca78760f55730fac9922717b7`，tree `a302b329d9f3ae81a92ecc572cb2b20e5f46050e`，landing run `43341524786` attempt `1`。公开 Release 有 34 个资产，`release.json` 为 6649 bytes、摘要 `6ced0c0a6129f28bf0ddccc62d0f87c71c59c4c9e7d7a0c12b41f9bba070b1c2`；Linux/macOS installer 分别为 `6c3002b7c6d24025a963353ae14f6f7656839a99d25abcfee4c87417da8b733a` 和 `a556921e2baf7f8dc26e47cfa3eebda631f064bbf3eaf371b849a5381acc94ba`，两平台安装和发布 workflow（`43342038213`、`43345275243`）均成功。四仓第一轮 feature→integration→dev 均通过：Go PR `49→50`、dev `374986cf`（collection `43349580288`，CodeQL `43349579551`）；TypeScript PR `46→47`、dev `281407df`（`43349927656`、`43349927171`）；Mix PR `46→47`、dev `6df1c18e`（`43350150391`、`43350149883`）；Initial PR `39→40`、dev `cc780ec6`（`43349832098`、`43349831435`）。

第一轮四仓落地后，上游 `dev` 合入 PR #1014，源码前进到 `1ca9aa00be1147202d1cf3935ced4ba186eb3bba`，所以不能把第一轮候选晋升 stable；该源码漂移作为稳定门槛失败证据保留。

第二轮候选固定为 `v0.1.4-dev.43348924098.1.g1ca9aa00be1147202d1cf3935ced4ba186eb3bba`：source `1ca9aa00be1147202d1cf3935ced4ba186eb3bba`，tree `bb52845468736985d0aa8e351a0a8229d5705d6b`，landing run `43348924098` attempt `1`，release workflow `43351005007`。公开 Release 为 34 个资产、`release.json` 6649 bytes、SHA-256 `aba17a9b40bdb7bc3a939275e2f4fa2512829ae358d1ed3f93f1ed6244c99a0b`；Linux/macOS installer 分别为 `6c3002b7c6d24025a963353ae14f6f7656839a99d25abcfee4c87417da8b733a`（1653840 bytes）和 `a556921e2baf7f8dc26e47cfa3eebda631f064bbf3eaf371b849a5381acc94ba`（1330032 bytes），公开、非 draft、prerelease，两平台安装 job 成功。

第二轮四仓 feature→integration→dev 已实际落地，且落地 SHA 对应的 push collection/CodeQL 均成功：Go feature PR `51`（`85ad778d`）→ integration PR `52`（`316e06ec`）→ dev PR `53`，dev `13e3615647f9069c6df9b4b7f876723c24dd831a`，collection `43355979882`，CodeQL `43355979057`；TypeScript `48`（`790e158f`）→ `49`（`a3e5008b`）→ `50`，dev `454690c48cbc318bae942ec0efaffe4bb752d0c4`，collection `43355978693`，CodeQL `43355978227`；Mix `48`（`1a6c716f`）→ `49`（`c0ec8e65`）→ `50`，dev `49558d0c1816a8670ae9638daa56fb90b3cb49b5`，collection `43355979868`，CodeQL `43355979637`；Initial `41`（`36478118`）→ `42`（`1d67d1b5`）→ `43`，dev `37488c04f62c6557d80ec797d339ef4082e6a0e9`，collection `43355982324`，CodeQL `43355981204`。四个 dev 的 `distribution.json` 均锁定第二轮 preview 和摘要，真实业务单元也通过。

第二轮四仓落地后，上游再次合入维护 PR #1015，`dev` 从候选 `1ca9aa00` 漂移到 `5af328f4565fd10408ac359ef2f10d5ea6a71128`（tree `6a926598234e48719f0387a5018dc276dd20084e`）。因此第二轮也保持 preview，不晋升 stable。当前第三轮候选以 `5af328f4` 为 source，landing collection `43356015909` 仍在运行；对应 CodeQL `43356014786` 已成功。collection 完成前不派发新 release、不迁移下游，避免把未固定的源码或未完成的上游结果写成稳定证据。

## 稳定门槛与下一轮

下列条件全部满足时，候选才可 stable：上游 preview 公开资产和两平台安装通过；四个下游的 feature、integration、dev 链均落地；dev 上的真实单元、汇总和必要安全检查通过；候选相关失败已经在负责层修复并重新验证；stable Release 的标签、源码、资产和下游稳定版声明一致。任一条件缺失都保持 preview 或 unknown。

正式采用完成后，下一轮目标从稳定版源码的下一个真实改动开始。先审计当前使用方和 SPEC，确认改动的最小范围，再产生新的 feature、测试版和下游波次；不要因为流程本身生成版本。

