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

第二轮四仓落地后，上游再次合入维护 PR #1015，`dev` 从候选 `1ca9aa00` 漂移到 `5af328f4565fd10408ac359ef2f10d5ea6a71128`（tree `6a926598234e48719f0387a5018dc276dd20084e`）。因此第二轮也保持 preview，不晋升 stable。

第三轮候选固定为 `v0.1.4-dev.43356015909.1.g5af328f4565fd10408ac359ef2f10d5ea6a71128`：source `5af328f4565fd10408ac359ef2f10d5ea6a71128`，tree `6a926598234e48719f0387a5018dc276dd20084e`，landing collection `43356015909`，CodeQL `43356014786`，发布 workflow `43360491835`。公开 Release 有 34 个资产，`release.json` 为 6649 bytes、SHA-256 `d96e7f633d8c7d2ea0f05080558cd4695f3c416a965698bad32b8bc3c4fe37af`；Linux/macOS installer 分别为 `87fbb05fe031f1cdba57d6ca85a2ec22325e4aef6c81dae81ca7a8270797d334`（1654480 bytes）和 `a556921e2baf7f8dc26e47cfa3eebda631f064bbf3eaf371b849a5381acc94ba`（1330032 bytes）。上游随后漂移到 `48b3ee73544547a0b557ef1d834a98fb7e68df4a`、`ea81dd400cdb4a47c85aa72172c2a5f60683cebb` 和 `3e914ac44a191ca7f63456fb50cc251b6d230f9b`，未把第三轮候选写入下游。

第四轮候选固定为 `v0.1.4-dev.43364900625.1.g3e914ac44a191ca7f63456fb50cc251b6d230f9b`：source `3e914ac44a191ca7f63456fb50cc251b6d230f9b`，tree `73995f9f764b407dd1a2584e94514dc9f01895eb`，landing collection `43364900625`，CodeQL `43364899728`，发布 workflow `43367390789`。公开 Release 有 34 个资产，`release.json` 为 6649 bytes、SHA-256 `131a3f46d1eaf7fdd0d44137e11a3f81006ad9838db052162cdfb516b0464876`；Linux/macOS installer 分别为 `a2e20b8e4eb823a9fa455b78f37f730962f5abb8cee3f8ea95e5a548841b15fd`（1656256 bytes）和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`（1329600 bytes），两平台安装成功。上游随后又漂移到 `eb40976ebd99b7844420fcf49c95c4ee3ab234ca`（tree `3e2519351cab706162a85b8c984ce5d5cd13beff`），因此第四轮完成真实下游采用但仍不能晋升 stable。

第四轮四仓 feature→integration→dev 已实际落地，且落地 SHA 对应的 dev push collection/CodeQL 均成功：Go feature PR `54`（`0698faef6794a97c907b7cdd4ab522a9b86d78b3`）→ integration PR `55`（merge `15c5f3c6672dd604ab5e4a1b39c1a0297f806934`）→ dev PR `56`，dev `d5e925ea5f5daec9ce763774d6af185ad5cffdfe`，collection `43371569631`，CodeQL `43371569429`；TypeScript `51`（`3c3941dd97f0bd1f5134d170a76e5ee3fdd416b3`）→ `52`（merge `f497449628368f58d78301d91bd255ed3b7f5acf`）→ `53`，dev `6c82959729a633759b062717c877772bfe2dfaa7`，collection `43371578545`，CodeQL `43371578767`；Mix `51`（`8dc063f0a2542be562f8319fa339b183800d0f92`）→ `52`（merge `b5bebe1aa90db064a207be29f4a7b240d54a3142`）→ `53`，dev `39b1b68448093725e7678f355a00f1e072d92cb3`，collection `43371589020`，CodeQL `43371587821`；Initial `44`（`e81a84a172dfd0a5e2e8d1e0a23c4301f2bbb318`）→ `45`（merge `8d2b7318923dfb568017fc9314eeb0db09383ea6`）→ `46`，dev `1a8acc29d02688cea2ee5e3a46e5334e1987fbcc`，collection `43371597705`，CodeQL `43371596844`。四个 dev 的 `distribution.json` 都锁定第四轮 preview 和 manifest 摘要；本地单元、汇总和 CI 配置核验全部通过。

`43362310653` 是第四轮前一次发布尝试的失败原件：执行 workflow 修订为 `ea81dd400cdb4a47c85aa72172c2a5f60683cebb`，固定候选 workflow 修订为 `48b3ee73544547a0b557ef1d834a98fb7e68df4a`，因 `E_RELEASE_IDENTITY` 拒绝；其派生的 package 缺失不能另计根因。该 run 的原 stdout、stderr、退出码和 artifacts 继续保留。

第五轮候选固定为 `v0.1.4-dev.43368268229.1.geb40976ebd99b7844420fcf49c95c4ee3ab234ca`：source `eb40976ebd99b7844420fcf49c95c4ee3ab234ca`，tree `3e2519351cab706162a85b8c984ce5d5cd13beff`，landing collection `43368268229`、CodeQL `43368267500`，均为 attempt `1`。发布 workflow 为 `43370230567`，工作流文件 `.github/workflows/chrono-release.yml` 的 blob 为 `ec83d23d5b9122d126898854dfb34d8ad0c4a5f4`。公开 Release 有 34 个资产，公开、非 draft、prerelease；`release.json` 为 6649 bytes、SHA-256 `17c7f7366b2f7309ac13615618274e392cecf8b33b9b7b39d643fb51080a94cf`；Linux/macOS installer 分别为 `a2e20b8e4eb823a9fa455b78f37f730962f5abb8cee3f8ea95e5a548841b15fd`（1656256 bytes）和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`（1329600 bytes），发布及两平台安装均成功。

第五轮四仓先完成固定候选的本地单元与汇总检查，再实际走完 feature→integration→dev：Go PR `57` → `58`，dev 落地 `5db7065b8a9048faffb3555fed4cfc8111ef4443`，collection `43374575914`，CodeQL `43374575438`；TypeScript PR `54` → `55`，dev 落地 `31dcb0a9fe763c9dffd9f19de6b25e8e908cc8d0`，collection `43374576232`，CodeQL `43374575880`；Mix PR `54` → `55`，dev 落地 `38ea9cc2f4d1be01d86e59f452ae7869f3267422`，collection `43374578929`，CodeQL `43374578086`；Initial PR `47` → `48`，dev 落地 `89130ad2f158e243af2303d0ad0a91844af7ca67`，宿主检查 `43374576343`，CodeQL `43374576025`。四个下游的 `distribution.json` 均锁定上述 preview 和 manifest 摘要；Go、TS、Mix 的真实单元及汇总、Initial 的登记检查均通过。

第五轮下游落地后，上游 `dev` 又合入 PR `#1026`，从候选 `eb40976ebd99b7844420fcf49c95c4ee3ab234ca` 漂移到 `7561f6232e5a7ee4c0e5904dbab017235f2388d5`（tree `019e1bdb1882290c55fa9cef1d1fc5427437ecdb`）。因此第五轮已完成真实 preview 采用，但稳定门槛再次因源码漂移失败；当前仍保持 preview，不能把第五轮发布成 stable。

第六轮候选固定为 `v0.1.4-dev.43374900168.1.g7561f6232e5a7ee4c0e5904dbab017235f2388d5`：source `7561f6232e5a7ee4c0e5904dbab017235f2388d5`，tree `019e1bdb1882290c55fa9cef1d1fc5427437ecdb`，landing collection `43374900168`、CodeQL `43374899692`，均为 attempt `1`。发布 workflow 为 `43377717333`；公开 Release 有 34 个资产，公开、非 draft、prerelease；`release.json` 为 6649 bytes、SHA-256 `8194712eec7a146ffc8bab0ffa508714850066c6ca2c09ef425f64b55b3530ab`；Linux/macOS installer 分别为 `a2e20b8e4eb823a9fa455b78f37f730962f5abb8cee3f8ea95e5a548841b15fd`（1656256 bytes）和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`（1329600 bytes），两平台安装成功。

第六轮四仓 feature→integration→dev 已实际落地，且落地 SHA 对应的 dev push collection/CodeQL 均成功：Go PR `59`（`1045d3f1c3ed654d3102137bebe78e42def85f1c`）→ `60`，dev `33d7536dc36351cfb3f283cb0daf2aedd12006bd`，collection `43384347888`，CodeQL `43384347500`；TypeScript `56`（`82d2a6cb066c967d6e8a07d2c9f5eb75c4c318a1`）→ `57`，dev `84b4590b52cbec49d6dfcee9262142a277bff96d`，collection `43384347439`，CodeQL `43384347195`；Mix `56`（`e8744c714df22ba3c8cc8bdc561a34a4f260da95`）→ `57`，dev `71d05b45e47758b7e60917237e7f3eb744a360d8`，collection `43384347025`，CodeQL `43384346919`；Initial `49`（`34f2b47f8ff1e76cacf95b17bbd8727add1faaa3`）→ `50`，dev `aac829a59254b89aa70216902b51543dd6c82e73`，collection `43384197301`，CodeQL `43384196780`。四个 dev 的安装声明均锁定第六轮 preview 和摘要，真实业务单元、汇总和宿主检查均通过。落地后上游继续前进到 `8b7eb292fc1beea9d3d11067288cea7fcfaa9a20`，所以第六轮保持 preview。

第七轮候选固定为 `v0.1.4-dev.43380493081.1.g8b7eb292fc1beea9d3d11067288cea7fcfaa9a20`：source `8b7eb292fc1beea9d3d11067288cea7fcfaa9a20`，tree `3469489408c6105e2536f65e4b6ef86a9dbab0fd`，landing collection `43380493081`、CodeQL `43380492384`，均为 attempt `1`。发布 workflow 为 `43383080388`；公开 Release 有 34 个资产，公开、非 draft、prerelease；`release.json` 为 6649 bytes、SHA-256 `08ace8b0b594ef52facda0696554a420e4009ee38500944aa77a7e244717a02f`；Linux/macOS installer 分别为 `a2e20b8e4eb823a9fa455b78f37f730962f5abb8cee3f8ea95e5a548841b15fd`（1656256 bytes）和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`（1329600 bytes），两平台安装成功。

第七轮四仓 feature→integration→dev 已实际落地，且落地 SHA 对应的 dev push collection/CodeQL 均成功：Go PR `61`（`ca94e5863fd8938af9f1e58d1376f64b60107a7e`）→ `62`，dev `4f49a517000906d26c3264df3f3679668551c62d`，collection `43392781517`，CodeQL `43392781706`；TypeScript `58`（`8a9c50dc8e5c583ae87abc56bfdd67138e18febb`）→ `59`，dev `e109ddeb2ce52703ac533a78b1103f936081f8cd`，collection `43392782389`，CodeQL `43392781989`；Mix `58`（`5eab380caf7e20c03d2f726ce43d4da40960ac52`）→ `59`，dev `3315bb3310249dd4b48086fcf8a5c3dec53990ba`，collection `43392782200`，CodeQL `43392781858`；Initial `51`（`d0553bc9036ec13300cde02f3e5db454feb6cd7f`）→ `52`，dev `c5d82580d11d6a75a8f96b661febfdd2612b10e9`，collection `43392012605`，CodeQL `43392012160`。四个 dev 的安装声明均锁定第七轮 preview 和摘要，真实业务单元、汇总和宿主检查均通过。落地后上游继续前进到 `4d31cde6446dadfc4b098ababe4e69ad4b3f9e66`，所以第七轮保持 preview。

第八轮候选固定为 `v0.1.4-dev.43388849772.1.g4d31cde6446dadfc4b098ababe4e69ad4b3f9e66`：source `4d31cde6446dadfc4b098ababe4e69ad4b3f9e66`，tree `d5c54644d5bb6eebfd3f7a9a354fc06cc925de29`，landing collection `43388849772`、CodeQL `43388848678`，均为 attempt `1`。发布 workflow `43392259585`（另一次同源发布 `43392083172` 对应更早 `87efce…`，不作为本候选证据）；公开 Release 有 34 个资产，公开、非 draft、prerelease；`release.json` 为 6649 bytes、SHA-256 `3ef8c66517be508af210cba7a72cfe258f18fbe4584cfdf3e373292ef8231f60`；Linux/macOS installer 分别为 `a2e20b8e4eb823a9fa455b78f37f730962f5abb8cee3f8ea95e5a548841b15fd`（1656256 bytes）和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`（1329600 bytes），两平台安装成功。

第八轮四仓 feature→integration→dev 已实际落地，且落地 SHA 对应的 dev push collection/CodeQL 均成功：Go PR `63`（`28dfc7714192b59239bf062ab96d4ed8bf172594`）→ `64`，dev `63ec065beaa75fa6333ff3b5bfd63a8a4092676d`，collection `43403825909`，CodeQL `43403826028`；TypeScript `60`（`74cab33fd355d620469713aac71d7b9516c211a6`）→ `61`，dev `a23b54c25972a5ca924a31e7a325bc8b725a7889`，collection `43403827987`，CodeQL `43403826845`；Mix `60`（`398ac883a20e59cc204e479fe4e95c531a9395fa`）→ `61`，dev `7e39ecfee64a189129706dce0bde7f2d6cd3a507`，collection `43403826176`，CodeQL `43403825849`；Initial `53`（`da96b053410594eed771521ceb5b099e7aed1e9d`）→ `54`，dev `76c57e172d272b5df15805aab613d614ddd86d20`，collection `43403827252`，CodeQL `43403826445`。四个 dev 的安装声明均锁定第八轮 preview 和摘要，真实业务单元、汇总和宿主检查均通过。落地后上游继续前进到 `f14b11005a034283d15e144d950ea9524968c2d6`（tree `6a883d3e15de400968741804c6ec63df3f78e2b9`），因此第八轮也保持 preview。

第九轮候选 source `f14b11005a034283d15e144d950ea9524968c2d6`、tree `6a883d3e15de400968741804c6ec63df3f78e2b9`，landing collection `43403262640`、CodeQL `43403261370`，均为 attempt `1`。发布 run `43406619666` 的 attempt `1` 中，所有 Linux/macOS frontend、构建和大部分验证通过；`linux_verify_runner_tests` 因 Linux `ETXTBSY (Text file busy)` 失败，`linux_package` 以退出码 `2` 终止，publish/install 被跳过，原始日志和失败 artifact 保留。该 run 的 attempt `2` 已完成：`linux_verify_runner_tests`、两平台 package、publish 和安装均成功，发布的 preview 为 `v0.1.4-dev.43403262640.1.gf14b11005a034283d15e144d950ea9524968c2d6`，`release.json` 为 6649 bytes、SHA-256 `1b24d2ccbebd6fd04aaf5c490645f6689479fa999c332f25a76aac27b3d1c3d0`；Linux/macOS installer 分别为 `e8df377c71fe77bcd9726f7ae2c96cb34ec484f0b9b4bce117553eb17a9313ee` 和 `95545bfad80f793a4e40e7527490ba98610a4a16111c952f816bd9829aca7047`。另一个成功 run `43406750088` 的日志实际绑定旧 source `9b706fa695e3c5c48a1c56f46efc539c16c98635` 与 CI `43403018872`，生成旧 preview 标签，不能作为第九轮证据。

第九轮四仓最终采用已落地：Go 经 PR `67→68`，dev `7d28cd1bd2e084a9a4ab284ec86068a976561a9f`，collection `43416658642`、CodeQL `43416658322`；TypeScript 经 PR `62→63`，dev `cbc0e78901c6db3ecd9f6e19ec88b590d022abd2`，collection `43418331102`、CodeQL `43418329863`；Mix 经 PR `62→63`，dev `cc5ab541ec3f838647ef03262b1495538d0ef09e`，collection `43418333870`、CodeQL `43418333184`；Initial 经 PR `55→56`，dev `1966e7a1d813a31943041b75a0f4c9de23dee418`，宿主检查 `43418330862`、CodeQL `43418330637`。Go 早先直接合入的 PR `65` 产生 collection 失败 `43415798646`，随后按真实顺序回退 PR `66`（落地 collection `43416177786`、CodeQL `43416177118`），再以 PR `67→68` 完成；这些失败和修复原件均保留。四仓随后继续采用第十轮 preview，作为 stable 晋升前的最终候选。

## 第十轮 preview 与 stable `v0.1.4`（2026-11-13）

第十轮 preview 固定为 `v0.1.4-dev.43411762402.1.gf348ea7425b3430a0134fa346e8d97bc2d9f527a`：source `f348ea7425b3430a0134fa346e8d97bc2d9f527a`，tree `2bda5bc561d3531a36aa0522a1e8900f3be678a1`，landing collection `43411762402`、CodeQL `43411761404`，均为 attempt `1`，发布 workflow `43416115837` attempt `1`，workflow blob `cdf3abed13df84828bb847b341f48d034647ff2e`。公开 Release 有 34 个资产，公开、非 draft、prerelease；`release.json` 为 6649 bytes、SHA-256 `fc9392c1af6a87bbe67d586f72db7de0656a9332514d93ac8de65c880930ba93`；`install.sh` 为 5186 bytes、SHA-256 `8c006294ecacba22681725fcd06cf258ecee1a9994c49c97b221d8a14da52fba`；Linux/macOS installer 分别为 `e8df377c71fe77bcd9726f7ae2c96cb34ec484f0b9b4bce117553eb17a9313ee`（1656256 bytes）和 `6abc37db7bec4f1e1a3013b4ded5090b7078ba587d63f4b433e84f88d6434c29`（1329600 bytes），两平台安装成功。四仓 feature→integration→dev 均通过：Go PR `69→70`、dev `2ed9f5e308c795a415d20cf97f10ea71a0671026`，collection `43421082713`、CodeQL `43421082405`；TypeScript PR `64→65`、dev `0548473cf80a023af167b5cb87342fe617b9bfdd`，collection `43421002241`、CodeQL `43421001724`；Mix PR `64→65`、dev `9203c6216d35a815b68e69f1e5b3cc9036e687c9`，collection `43421082848`、CodeQL `43421083176`；Initial PR `57→58`、dev `d7fb940a771350a0734976c67cdb2b3473b646cc`，宿主检查 `43421002939`、CodeQL `43421003330`。

稳定晋升前保留了三类失败输入：错误 workflow 文件名返回 HTTP 404；用裸 SHA 作为 workflow ref 返回 HTTP 422；stable 首次派发 run `43421760311` 的 publish job `131873970303` 因缺少 `preview_run=43416115837` 和 `preview_attempt=1` 返回 `E_RELEASE_PROMOTION: explicit positive preview run and attempt required`。这些失败原件没有被后续成功覆盖。

最终 stable 以同一 source 快进 `main` 后派发，workflow run `43422705025` attempt `1`，publish job `131877092958`，workflow blob `cdf3abed13df84828bb847b341f48d034647ff2e`。Release `v0.1.4` 公开、非 draft、非 prerelease，标签目标为 `f348ea7425b3430a0134fa346e8d97bc2d9f527a`。公开资产为 34 个；`release.json` 为 6589 bytes、SHA-256 `6be5d88992d4157c315238a0816f1e5264c17bdf5986c02246555d42f5ee8147`；`install.sh` 为 5066 bytes、SHA-256 `038d7074f7acb7d54f75a4510ee9f22ddc93228ddc78a703c1143e17c8a92bb5`；Linux/macOS installer 摘要和长度分别为 `e8df377c71fe77bcd9726f7ae2c96cb34ec484f0b9b4bce117553eb17a9313ee`（1656256 bytes）和 `6abc37db7bec4f1e1a3013b4ded5090b7078ba587d63f4b433e84f88d6434c29`（1329600 bytes）。stable workflow 的 `linux_install` 与 `macos_install` 均为 `skipped`，不能记作通过；真实下游采用提供了 macOS 安装和 Linux dev CI 的覆盖证据。

stable 正式采用已在四个下游完成：Go feature PR `71`（head `b29756902c138b5ffe17446d8c43caa1d5ad6134`）→ integration merge `a7ad985e666427824f8a1698428b0c1a59dfdcf2` → dev PR `72`、dev `c0ddc8b2022d7ad0e328cfc5ca14e85ac781a9cf`，collection `43425092561`、CodeQL `43425092278`；TypeScript PR `66`（head `0054aeca21f2044b48e98dc8c1a4b3f937764bec`）→ integration merge `d9996ba54476138613d3726f36bcc80d877dbd43` → dev PR `67`、dev `b87ece774789798d8b32b20883c9396891b89c43`，collection `43425092614`、CodeQL `43425091115`；Mix PR `66`（head `a57edc5e583e5f6776ac61b7cb3abe8103326a7e`）→ integration merge `b8de615b391adfb356de8ea90f4fefc0aafb71e9` → dev PR `67`、dev `6a00ee5a10c283096784a1ec07d145d41e8740c0`，collection `43425092793`、CodeQL `43425092195`；Initial PR `59`（head `a0ef26d05fcd515e749b1d22e2b73cf52083ec07`）→ integration merge `f931b9edc7000b39897ff36a6dc39e80a006f3da` → dev PR `60`、dev `965750bf973395ef221fbca2e0cf042309128708`，宿主检查 `43425093505`、CodeQL `43425093027`。四个 `dev` 的 `distribution.json` 均锁定 `v0.1.4`、stable `release.json` 摘要/长度和两平台 installer 摘要/长度；macOS 本地安装和业务检查通过，Linux 采用由对应 dev CI 实际验证。

## 第十一轮 preview 采用（2026-11-13）

正式版 `v0.1.4` 之后审计发现上游存在真实净改动，固定源码为 `7c20c55c12e26f31c1afefecbc75ef045dd70d6f`，tree 为 `d9b0fb3b81d6187b570ae8085619794891a74865`。本轮先发布测试版并完成四仓真实采用：

| 项目 | 身份 |
| --- | --- |
| preview 版本 | `v0.1.4-dev.43435181626.1.g7c20c55c12e26f31c1afefecbc75ef045dd70d6f` |
| landing collection / CodeQL | `43435181626` / `43435180637`，均 attempt `1` |
| 发布 workflow | `43439682171`，attempt `1`；workflow blob `1e81d8754bd39a18d5132b94bda2d152ed3692dc` |
| Release | 公开、非 draft、prerelease，标签目标为 `7c20c55c12e26f31c1afefecbc75ef045dd70d6f` |
| release.json | 6649 bytes，SHA-256 `9e73203ccd52d2a6632cd9b7b2a78ffbf28f363fd5fbbb8454a75d885da0cf4f` |
| install.sh | 5186 bytes，SHA-256 `65a139a703977b47488b124f263d22f5af250afe6dcbf6d32ef7e64efdcef819` |
| Linux installer | 1656384 bytes，SHA-256 `be972775a6e7dab0e114007a3ca939dff431c61294c423b4c278fa3bae8ac686` |
| macOS installer | 1329600 bytes，SHA-256 `4df3d850358f12a1a76cea2bf69a0a31bc56a3e3c995479ca6768094c74fd795` |
| 安装 jobs | Linux `131947220782`、macOS `131947220668`，均成功 |

自动发布 run `43438440412` 的 Release 身份错配原件继续保留：它的标签为 `v0.1.4-dev.43434383969.1.g9591e93b2f73bc57fab812237aec9335083615d1`、标签目标为 `9591e93b2f73bc57fab812237aec9335083615d1`，而本轮 workflow head 为 `7c20c55c…`，与 landing run/source 不一致，因此不能用于下游迁移或补作本轮证据。

四个下游均从最新 `dev` 建立新的 feature 分支，采用上述固定 preview，并按 `feature → integration → dev` 实际合入。feature collection、integration collection、dev 落地 collection 和 CodeQL 如下；本地业务单元也均通过。

| 下游 | feature → integration | integration → dev | dev 落地 / collection / CodeQL | 本地单元 |
| --- | --- | --- | --- | --- |
| Go | PR `76`，head `7f95f246`，run `43445356522`；merge `72ea2f8f` | PR `77` | `1d77a4e6` / `43445898852` / `43445898916` | `harness`, `rates` |
| TypeScript | PR `71`，head `3ce8a5b0`，run `43445354737`；merge `de217b9c` | PR `72` | `62ce2234` / `43445899388` / `43445898591` | `harness`, `labels` |
| 混合语言 | PR `71`，head `bfa44f7a`，run `43445354017`；merge `1b9ba922` | PR `72` | `49ed1373` / `43445912876` / `43445913304` | `harness`, `labels`, `normalize`, `rates` |
| 首次接入 | PR `64`，head `4ec94ce0`，run `43445355617`；merge `1eaa2ce8` | PR `65` | `34a1ade4` / `43445901060` / `43445901084` | 宿主 canonical check |

四个 `dev` 的 `distribution.json` 已核对为同一 preview、manifest 摘要和两平台 installer 摘要；Go、TS、Mix 本地业务检查以及首次接入本地检查均为 `check: passed (exit 0)`。当前候选已完成 preview 采用，但正式晋升仍需先处理版本合同：上游 stable 已存在 `v0.1.4`，而该候选的 release plan core 仍为 `v0.1.4`，不能再次创建同名 stable。下一步在上游负责层将正式版本计划明确推进到 `v0.1.5`，随后重新产生唯一 preview、重新走四仓迁移，满足新版本的稳定门槛后再发布正式版。

收口时按登记入口执行 `/bin/bash .chrono-harness/ci/bootstrap-native.sh . .chrono-harness/ci/bootstrap-core.json`，退出码为 `2`，原始原因是 `retention policy migration required; original record preserved`。这是宿主生命周期的前置缺口，不是发布或下游采用失败；未重复启动该 bootstrap。由于该前置未完成，FILEMAP／登记 canonical judge 没有有效结果，本轮登记检查保持 `unknown`。不依赖 bootstrap 的文档检查（`git diff --check` 和登记 JSON 解析）已通过。

## 第十二轮 v0.1.5 preview 采用与下一候选（2026-11-13～14）

上一正式版 `v0.1.4` 之后确认上游存在真实净改动，版本计划已推进到 `v0.1.5`。最近一个完整成功的测试版固定为：

| 项目 | 身份 |
| --- | --- |
| preview 版本 | `v0.1.5-dev.43478399135.1.g07876e055ad618bf830788d37458377dcca6a9e6` |
| source / tree | `07876e055ad618bf830788d37458377dcca6a9e6` / `7c65e6737a49f98e33bbf3c685217d4a745d9af4` |
| landing collection / CodeQL | `43478399135` / `43478397347`，均 attempt `1` |
| 发布 workflow | run `43482547108`，attempt `1`；`.github/workflows/chrono-release.yml` blob `ce38fbe5b0ced74709141b9ee43f26f4865cd124` |
| Release | 公开、非 draft、prerelease，标签目标为 `07876e055ad618bf830788d37458377dcca6a9e6` |
| 公开资产 | 34 个；`release.json` 6649 bytes，`4eb1c64ba88b8bbfef2f9eb0a9e0cc10277a30f76cd3a66196845898c5cfa166` |
| 两平台资产 | Linux `1657696` bytes，`9091d00b8beb1546781c1031662fc171a14aa264ea24ff1e84c0996a045403c3`；macOS `1329600` bytes，`b3a21d966d19bf360daecaea1bdb1722b08e394a81c95648c337068ba7034837` |

该发布 run 的 Linux/macOS frontend、全部登记构建与验证、package、publish、Linux 安装和 macOS 安装均成功。四个下游的 `dev` 声明都锁定该 preview；当前落地和最新 CI 为：Go `4639d31dea633c935ebbc14a308a1bd58c186624`（collection `43490975370`、CodeQL `43490974524`），TypeScript `717e65192567c1890228a13aec442eae0c30cf7b`（`43490976822`、`43490974801`），混合语言 `8a9167496f85ebf5b4690f084e7ba781b926cce0`（`43490976918`、`43490976242`），首次接入 `47d117da4d6d774468614ec43da2294f13f07dd8`（宿主检查 `43490974834`、CodeQL `43490974883`）；这些落地运行均成功。

上游随后继续前进到 `00abeb0c559268f8ac762341fbcc00a0c1648071`，因此不能把上述 preview 晋升为 stable。由该变更触发的发布 run `43491845132` 已成功终态，但发布日志确认其真实候选不是 workflow 自身的 `head_sha=00abeb0c…`，而是固定的 `source=846af39b2372f607dfdd496bc26b41311c497348`，对应 landing collection `43489853160`（CodeQL `43489853379`），workflow blob `ce38fbe5b0ced74709141b9ee43f26f4865cd124`。公开 preview 为 `v0.1.5-dev.43489853160.1.g846af39b2372f607dfdd496bc26b41311c497348`，Release 公开、非 draft、prerelease，共 34 个资产；`release.json` 为 6649 bytes、SHA-256 `78a6a8578c2f02a2f4410d28a0800a9b8c75fae7f1c9a56acbd43ea733d53245`；Linux installer 为 1657696 bytes、SHA-256 `9091d00b8beb1546781c1031662fc171a14aa264ea24ff1e84c0996a045403c3`，macOS installer 为 1329600 bytes、SHA-256 `b3a21d966d19bf360daecaea1bdb1722b08e394a81c95648c337068ba7034837`。publish、Linux install、macOS install 均成功。

四个下游随后按 `feature → integration → dev` 完成该候选迁移；四仓本地安装、适用 bootstrap 和 canonical check 均通过。首次 adoption 默认入口因现有版本不同返回 `E_DISTRIBUTION: existing adoption differs`，保留原始 stderr 后使用登记的 `--replace` 完成替换。首次误用不存在的 `bootstrap-native.sh` 返回 `127`，原始 stdout/stderr 保留；Go、TypeScript、混合仓随后均改用其实际登记的 `python3 .chrono-harness/bootstrap.py .`，首次接入仓按登记直接执行 canonical check。

| 下游 | feature PR / merge | integration→dev PR / dev SHA | dev collection / CodeQL |
| --- | --- | --- | --- |
| Go | #88 / `44b65f676f6000c7e32a7d504c38bef3f9360c28` | #89 / `ab518ef23ea53e6a3c55cc6fbf768848dffa2aa4` | `43497591420` / `43497590275` |
| TypeScript | #83 / `a934b932311fae20901b63a54270db18609922a1` | #84 / `e9e484540a08397ac38283e9f7d6b5bd43836b0d` | `43497591062` / `43497590681` |
| 混合语言 | #83 / `6838467334517469656172a8237f66114db53c79` | #84 / `1c07f4b2d5ffcaff4807273720f352f8b0658045` | `43497596643` / `43497595416` |
| 首次接入 | #76 / `8dff12f6dc9226d49a6ebce5c82cb26de8916bf1` | #77 / `9512c3e9ff5376878ea78e331b346965de3bfa52` | `43497590892` / `43497590771` |

四个 dev push 的业务检查、汇总和 CodeQL 均成功，且 dev SHA 与上述落地提交一致。由于上游在该候选之后已经漂移到 `00abeb0…`，本候选完成 preview 采用但仍不得晋升 stable。

此前同一连续迭代中的失败 run `43487804581`（source `e83c53877c2b7fc87300f37d619b85861ac85280`）已终态为 failure：`linux_verify_runner_tests` 的 `producer_native_read_failure_keeps_transport_exit` 观察到 Linux `ETXTBSY (Text file busy)`，`linux_verify_worktree_tests` 的 `owner_deployment::owner_deployment_uses_the_declared_producer_entrypoint_for_stage_inputs` 失败，随后 `linux_package` 失败；`publish`、`linux_install`、`macos_install` 均 skipped。原 stdout、stderr、退出码及诊断 artifact 已保留，不能用后续运行覆盖这些失败证据。

收口时仍保留 bootstrap `retention policy migration required; original record preserved` 的退出码 `2` 和登记检查 `unknown`，不把它写成通过。

## 第十三轮 `v0.1.5` preview 采用（2026-11-13～14）

第十三轮固定候选为 `v0.1.5-dev.43490624514.1.g00abeb0c559268f8ac762341fbcc00a0c1648071`：source `00abeb0c559268f8ac762341fbcc00a0c1648071`，tree `8b8050dadd81151498dfc481dcf66495febf5976`，landing collection `43490624514`、CodeQL `43490623349`，均为 attempt `1`；发布 workflow 为 `43494090756`，workflow blob `ce38fbe5b0ced74709141b9ee43f26f4865cd124`。Release 公开、非 draft、prerelease，共 34 个资产，`release.json` 为 6649 bytes、SHA-256 `2c07aea6fc10e911eca01d71d227de131d0efaf44a4365cbf437e189d2db590c`；Linux installer 为 1660576 bytes、SHA-256 `006651f2331bae218bd00b47f2409a1759ca14034e318b75fc6ccdf702cfbd7d`，macOS installer 为 1329600 bytes、SHA-256 `301a6d248c7a08d97bd6401c7a32dfa89f4f49548e04b18808a7d0630271b32a`。发布 workflow、Linux/macOS 安装和上游检查均成功。

四个下游均完成该固定候选的 `feature → integration → dev` 晋升，且 feature/integration PR 已实际合入，落地 `dev` 的业务检查、汇总和 CodeQL 均成功：

| 下游 | feature → integration | integration → dev | dev 落地 / collection / CodeQL |
| --- | --- | --- | --- |
| Go | PR `90`，merge `091b1b240a8885c2749ee61437e2588e2272c6fd` | PR `91` | `3a285782a50e0465ad7325a05756144b5a975dcd` / `43500356053` / `43500355038` |
| TypeScript | PR `85`，merge `7b28fbba3247ef2bc86d033a5a4bfa8cfc5be102` | PR `86` | `f1af482f8392dbbced36b58037aa4880aaca6de8` / `43500354447` / `43500354532` |
| 混合语言 | PR `85`，merge `34216f16455c73e2116399c4c44ececb305b7f1d` | PR `86` | `494c59f961fab280b744c685033044bbcf0c5202` / `43500357757` / `43500357193` |
| 首次接入 | PR `78`，merge `d56e0eb4af434cfccef1f2f5b38e76d29ad8ce5d` | PR `79` | `fa1576fb4720f2ec9f6cde8153d08cc10dd41152` / `43500355976` / `43500356305` |

四个 `dev` 的声明均锁定上述 preview、manifest 和两平台资产摘要。候选之后上游已继续前进到 `577d82eaf816363e2bd565db4bf789103bc1f512`、`5e8bca29ed4cb0cb6aea197d5bbdea2079fda790`，最新 `dev` 为 `ebcba42683b8f1f7ed820806ffa37334a5b6e8a8`；对应的 landing、CodeQL 和发布任务仍沿原句柄等待。因此本候选完成 preview 采用，但稳定门槛因源码漂移不满足，不能晋升 stable。

## 第十四轮 `577d82e…` preview（2026-11-14）

测试版固定为 `v0.1.5-dev.43496586282.1.g577d82eaf816363e2bd565db4bf789103bc1f512`，source 为 `577d82eaf816363e2bd565db4bf789103bc1f512`，tree 为 `6e5420cad71ff7e181103152f25f2dd3e895f960`；landing collection `43496586282` 与 CodeQL `43496584712` 均成功。发布 run `43500113784/1` 的 workflow head 为 `ebcba42683b8f1f7ed820806ffa37334a5b6e8a8`，frontend 原件明确其实际 source 是 `577d82e…`，两端 workflow blob 都是 `ce38fbe5b0ced74709141b9ee43f26f4865cd124`。workflow head 不能替代 source，也不能将本次发布算作 `ebcba426…` 的采用证据。

Release 公开、非 draft、prerelease，共 34 个资产。`release.json` 为 6649 bytes、SHA-256 `a678187845367bbb33593a27de652d96f6639ae0843f7b22defa2c3d659ea4f8`；`install.sh` 为 5186 bytes、SHA-256 `92e80b24972ad31f673e2fd628a23a6605d01fd1f7835816e98504afeb954c1f`。Linux/macOS installer 分别为 1660576 bytes／`006651f2331bae218bd00b47f2409a1759ca14034e318b75fc6ccdf702cfbd7d` 和 1329600 bytes／`301a6d248c7a08d97bd6401c7a32dfa89f4f49548e04b18808a7d0630271b32a`。上游发布的全部登记验证及两平台安装均成功，Linux/macOS install job 分别为 `132155014843`、`132155014809`。

四仓均通过正式工作树入口从各自最新 `dev` 创建同基线的 feature 和 integration，再使用候选安装器显式 `adopt --replace`。公开安装、适用 SDK bootstrap、CI verify 和本地 canonical check 全部成功；原 stdout 摘要、完整性、退出码和候选 SHA 已核对。Go 实际运行 rates 与 bootstrap，TypeScript 运行 labels 类型检查／测试与 bootstrap，混合仓运行 rates、labels、normalize、invoice 和 bootstrap，首次接入执行文档宿主检查。四仓 feature → integration 和 integration → dev 的适用 PR 检查均成功，并已匹配固定 head 实际合入。已核对各仓远端 `dev` 为表中落地 SHA，且各自 `distribution.json` 均锁定本轮 preview 和 manifest 摘要 `a678187845367bbb33593a27de652d96f6639ae0843f7b22defa2c3d659ea4f8`。

| 下游 | feature → integration PR | integration → dev PR / dev SHA | dev 落地检查（本次观察） |
| --- | --- | --- | --- |
| Go | #92 | #93 / `d3fbab3e48a2973260e3ed6ec29c95ec3a3343c0` | collection `43505474427`、CodeQL `43505474213`，成功 |
| TypeScript | #87 | #88 / `4e541a833a1145859a4551902e82b52c0f84eb42` | collection `43505506351`、CodeQL `43505503897`，成功 |
| 混合语言 | #87 | #88 / `430a686e6c8f39198fbaf14313553d4b43301dc3` | collection `43505517258`、CodeQL `43505516662`，成功 |
| 首次接入 | #80 | #81 / `d454fcaaab96d76f25edae19b25c637445ea1980` | 宿主检查 `43505526615`、CodeQL `43505525633`，成功 |

四仓落地原件已收取并核对报告、原 stdout/stderr 摘要、完整性、退出码和候选身份。Go 实际执行 `ci.verify`、`test.bootstrap`、`build.rates`、`test.rates`；TypeScript 执行 `ci.verify`、`test.bootstrap`、两项 labels 类型检查和 `test.labels`；混合仓另执行 invoice、normalize 和 rates 的登记操作；首次接入是无业务单元的宿主检查。上述 collection／宿主检查及 CodeQL 均成功。

后续发布 run `43502804440` 已以 failure 结束：原件确认 source 为 `ebcba42683b8f1f7ed820806ffa37334a5b6e8a8`、landing CI 为 `43499450155`。`linux_verify_runner_tests`（job `132153400864`）的 `producer_native_read_failure_keeps_transport_exit` 在 `tests/protocol_v1.rs:3646` 执行 fixture 时得到 `ExecutableFileBusy`／`Text file busy`（OS code `26`），Cargo 原退出码为 `101`；`linux_package` 随后失败，publish 和两平台 install 均 skipped，不能作为已发布候选。

修复 PR `#1083` 从同基线的 feature／integration 开始，先以 `a3d43482bc971f6945e316087dab55811007b1d8` 去掉 producer 测试的临时可执行文件复制，直接调用 Cargo 已编译的 helper。该初始修复的源码 bootstrap、格式化和未过滤 runner 测试均成功，原读取失败用例通过，既有一个 ignored 用例保持原状。首次本地 canonical 退出 `1`，原件固定 base `ca923e075fcadab90336d70ab8dd64019fe098af`、candidate `a3d43482…`，原因为缺少前移 dev 的验证分类；首次 PR CI `43508508980` 的 detector 又报告两个受影响旧文件未分类，业务单元 skipped。保留两次真实失败后，正常合入所需的 `ca923e…`，补上 producer 模式判断的同源 Verus 实现和独立边界预期，候选为 `a2bcac276bd3db8b4ed2364b006814910d7472f8`。其 PR CI `43510398699` 的 runner job `132180316112` 明确失败：`E_VERUS_COVERAGE: declared file is absent from compiler sources: crates/runner-tests/tests/protocol_v1.rs`，不能将只有登记的根写成真实证明覆盖。

同次本地 canonical 也退出 `1`，并观察到独立的 retention 失败：`explicit_small_reservations_admit_concurrent_outputs_and_enforce_each_bound` 在原八轮回收后未达到零个剩余 fixture 输出。retention 二进制为 `115 passed / 1 failed`，隔离子测试原退出 `101`；逐轮报告保留 `remaining_outputs: null`、`scope.complete: false`，不能把未知当零。原报告为 `.chrono-harness/state/preparation/check-1d2b5f276150d709bf7b98a61e29a41eca9fc7728ef802c06532add8081aa299.json`，原进程流已解码并核摘要。远端本次只观察到 coverage 失败，不能将本地 retention 失败冒称远端失败。

随后提交并推送 `08e3caf3740667765d1b36348ae968c8e8b17a35`：将 producer 模式的独立边界预期移入实际 `protocol_v1.rs`，并从 `verified_fixture.rs` 真正包含该文件；原协议和 IO 测试保留原生执行，证明范围仍仅覆盖所声明的模式判断。正式格式化、源码 bootstrap 成功；本机 fixture 证明为 `143 verified / 0 errors`，未过滤 runner 测试通过，原读取失败用例和新增模式边界均成功，既有一个 ignored 用例保留。此前 retention 用例本次也通过，但没有针对它修改实现，旧失败原因继续未知。PR CI `43513711712` 已成功，`#1083` 由 GitHub 自动合入 integration，实际提交为 `5d56e5ba355346cd94eab4c5c8379451d54f896c`；继续以 PR `#1088` 交付到 dev，已匹配该 head 启用原生自动合并。新本地 canonical 退出 `1`，其实际 base 为前移的 `25c3d192687b1d416ab9fc7efc574c81311aac59`、candidate 为 `08e3caf3…`，报告 `.chrono-harness/ci/check.json` 的既有验证分类缺失；该输入差异与目标仍为旧 integration 基线的远端 PR 分别记录，不将其写成通过，也不为消除状态反复重跑。原报告为 `.chrono-harness/state/preparation/check-7888d4e027745c6ad5c4b6eabb9ce0cb9874347e16bdf53b3418ed9cb1a70ad4.json`。原生自动合并继续启用，Linux 原故障和 retention 失败尚未完成修复复验。

新修复工作树首次 bootstrap 的实际错误是 PATH 中缺少 `rustup`，内层退出 `127`、外层退出 `2`；原始流摘要已核对。仅在本次命令 PATH 中补入现有 `.cargo/bin` 后，正式入口成功。此前原检出的 retention migration 退出 `2` 和登记检查 unknown 继续保留，不能由另一工作树的成功覆盖。

## 第十五轮 `5e8bca29…` preview（2026-11-14）

既有发布 run `43501191193/1` 已成功，其 workflow head 是 `ebcba426…`，实际 source 是 `5e8bca29ed4cb0cb6aea197d5bbdea2079fda790`，tree 为 `aceae9ef1aaf23ec8a648ff4dd93fd5130825e3c`，landing collection 为 `43498572837`，CodeQL 为 `43498571788`，workflow blob 为 `ce38fbe5b0ced74709141b9ee43f26f4865cd124`。公开版本为 `v0.1.5-dev.43498572837.1.g5e8bca29ed4cb0cb6aea197d5bbdea2079fda790`；Release 公开、非 draft、prerelease，共 34 个资产。`release.json` 为 6649 bytes／`d5a08864f04f36c60ba9e89c3de227c8e288ad45a9e178e9883c9b9830ef7152`，`install.sh` 为 5186 bytes／`a5677505c3f9f4a638d6c5a59bc65808e333582426dfb69473852ab8b7cd8aa1`。Linux/macOS installer 分别为 1660576 bytes／`006651f2331bae218bd00b47f2409a1759ca14034e318b75fc6ccdf702cfbd7d` 和 1329600 bytes／`301a6d248c7a08d97bd6401c7a32dfa89f4f49548e04b18808a7d0630271b32a`；已下载核对原字节。两平台 install job `132161959496`、`132161959632` 均成功。

四仓从本轮各自最新 dev 创建同基线 feature／integration，通过候选安装器 `adopt --replace` 更新，仅改变安装声明和生成入口，并逐项核对保留原有 16 个工具。公开安装、适用 SDK bootstrap、CI verify 和本地 canonical 均退出 `0`；原始报告、stdout 摘要、完整性及候选 SHA 已核对。四个 feature PR 和 dev PR 的适用检查均成功，并已匹配固定 head 实际合入；远端 dev SHA、安装声明和落地检查均已核对。

| 下游 | feature head / PR | integration merge / dev PR |
| --- | --- | --- |
| Go | `074d8b619fc4e3bd61f650670639c81f4bf5038b` / #94 | `b0a36bfd18a0a0ac10dc647bc5dfa0c7c0567521` / #95 |
| TypeScript | `0227fcf3c60622d1a97cf65bf84d04d854817a5e` / #89 | `03bc9ace47b726dc744b5b197e5cb6a1b5cf0476` / #90 |
| 混合语言 | `af5d172ff3d7af28e1666d023863ca1b0c64dce1` / #89 | `1b4277fd658a4980fa95927f42975d4309a5720e` / #90 |
| 首次接入 | `f03da39c15040f59d9c5e2c2f0e9293a1b682607` / #82 | `6445ad169a0a159a5664cd770cde6939d4042d0b` / #83 |

| 下游 | dev 落地 SHA | dev 业务／宿主检查 | dev CodeQL |
| --- | --- | --- | --- |
| Go | `faa92ceb2011e692f8f4f7e3acdc1f02fa4de277` | `43513482160`，成功 | `43513480981`，成功 |
| TypeScript | `488f2095bd23979964a4e71dab267aaf36e6c51e` | `43513484956`，成功 | `43513483471`，成功 |
| 混合语言 | `3b8d81afc0c9c876e3f4c9c80806f3500170215b` | `43513486697`，成功 | `43513485678`，成功 |
| 首次接入 | `1cd609dcf658bcdbcbfdb1663dd2fce535e5ca2e` | `43513491790`，成功 | `43513491410`，成功 |

四仓安装声明锁定本轮 preview、相同 manifest 和两平台 installer 摘要／长度，均保留 16 个工具。落地原始报告、judge stdout/stderr 完整性和摘要、业务进程原流及退出码已核对；报告候选恰为表中 dev SHA。Go 执行 `ci.verify`、bootstrap、rates 构建／测试；TypeScript 执行 bootstrap、labels 两项类型检查及测试；混合仓另执行 invoice、normalize 和 rates；首次接入在 macOS 执行无业务单元的宿主检查，不宣称该宿主已完成 Linux 检查。

本地部分临时检出及 start 原件随后缺失；没有据此判断仍在运行的命令已终止。已核 Git 保留的分支／提交，并将上述固定 feature 和 integration 提交推送保全；远端 PR 使用这些已核对的分支，不重新发布同一候选。缺失原因尚未确定。上游在本候选之后仍有源码改动且 runner 修复未完成发布复验，因此本候选继续保持 preview。

## 稳定门槛与下一轮

下列条件全部满足时，候选才可 stable：上游 preview 公开资产和两平台安装通过；四个下游的 feature、integration、dev 链均落地；dev 上的真实单元、汇总和必要安全检查通过；候选相关失败已经在负责层修复并重新验证；stable Release 的标签、源码、资产和下游稳定版声明一致。任一条件缺失都保持 preview 或 unknown。

正式采用完成后，下一轮目标从稳定版源码的下一个真实改动开始。先审计当前使用方和 SPEC，确认改动的最小范围，再产生新的 feature、测试版和下游波次；不要因为流程本身生成版本。

