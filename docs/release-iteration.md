# 测试版、下游采用与正式版

本文供发布维护者和执行迭代的 AI 使用，负责从测试版反馈推进到正式版的验收。开发和合并沿用[开发流程](development.md)，发布参数与恢复由[发布 CI](release-ci.md#持续交付与恢复)负责，安装采用由[分发说明](distribution.md)负责。可复用的执行提示见[发布迭代 skill](../skills/iterate-releases/SKILL.md)。不另建发布器、常驻循环或第二套检查入口。

## 一轮迭代

1. **选定候选。** 从实际成功的 dev push 检查取得完整源码 SHA、原 run ID 和 attempt，核工作流 ID／路径、事件、分支与结论。先检查这个原 CI 身份是否已有自动发布；有则接续原运行，没有才通过同一工作流显式选择 preview。Actions 列表中的发布工作流 head 可能是工作流自身的修订，实际发行源码须从原 CI、发布回执和清单共同核对。
2. **先发测试版。** 等待原发布运行的真实结果，核全部登记平台、验证和 collector，以及公开 Release、标签源码、清单和资产。Linux／macOS 安装作业都须通过。失败先读原件，身份相同的 draft 按现有入口恢复；新修复通过新的源码候选发布。不能覆盖旧公开资产。
3. **迁移真实下游。** 当前范围是[示例索引](examples.md)列出的 Go、TypeScript、混合语言和首次接入四个仓库。分别读取其规范、定制、安装与检查登记，从其最新 dev 建立隔离分支；规则、CI 或稳定性迁移先验 integration。按固定测试版的 release.json 重新 adopt 并生成安装入口，核摘要、长度和所选工具。安装新版不自动迁移旧 schema、命令、工具绑定或指南默认；逐项比较当前合同，仅修改实际受影响的采用数据，并运行下游登记的生成和检查入口。
4. **核实际使用。** 下游须通过自身适用 CI，完成真实合并及落地检查。保留业务项目及其专属测试、文档宿主的初始清单与普通 DELTA 区别，以及各自明确的平台范围。只升级版本字符串、安装成功或 PR 绿，都不能代替实际采用。某个下游受阻时继续独立迁移；受阻项未通过前不作正式发布。
5. **反馈到负责处。** 将失败归到产品实现、发布链或下游定制的实际产生处，保留原输入、输出和退出状态。修复关键逻辑时补能检出该失败的行为回归；外部程序在测试边界使用 mock。产品修复交付到 dev 后选新候选，重新发布测试版并补验受影响下游。纯文字或转发不增加重复测试。

迁移时核对几个已暴露的边界：短命令 scoped profile 的 `registration_config` 与 `facts_config` 都绑定根 config 入口；需要多平台精确 Git 约束时，让根入口选择完整平台政策。provider v4 还须明确声明同一 `facts_config`，并把隐藏原件放进声明上传根。旧独立单元拓扑已退役，先保留并核原所有权，再由宿主负责方迁移旧输出，不能把旧标记改成新标记来绕过冲突。runner 标签不可调度须读实际注释，在 provider 源选择受支持镜像并重新生成、验收。


每轮在对应 PR／Release 中引用原检查、发布与下游 PR／落地 SHA，说明尚未验收的范围和恢复条件；原日志留在现有有限保留产物中，不在本仓复制长期运行表。等待使用一个原生阻塞通道，失去结果时保留句柄与 unknown，不重复启动来制造新结果。长时间 `gh run watch` 若因凭据过期返回 401，只说明观察通道失败；用现有权限重新读取同一 run，取得原状态后接续只读等待，不重新派发发布或申请更多权限。

更新 Release 采用说明后，回读核对正文与指定文件一致，并逐项比较标签、公开状态及资产身份与内容字段；资产列表按唯一名称匹配。GitHub 的 `downloadCount` 会随真实下载变化，不属于资产身份或内容，不参与前后相等比较，其余字段继续核对。第 25 轮更新说明已成功，却因单个资产计数从 1 变为 2 误报失败；保留原 before／after 后核定只有计数变化，没有重复编辑。该例只支持排除下载计数，不豁免缺失、重复、身份或内容差异。

自动发布还要核两平台 frontend 的工作流观察：`frontend.json.workflow` 与原 `workflow.json` 绑定实际工作流及候选提交；同一路径的 blob 必须相同。不要因默认分支发生无关前移就重建候选；工作流字节确实变化或观察失败时，保留原件，再按[固定 ref 恢复](release-ci.md#持续交付与恢复)。旧源码没有该观察时，仍只按它实际执行的验证范围说明结果。

收尾先核每个宿主实际采用的生命周期。只有已采用 `automatic_cleanup` 的宿主才使用 `finish`；未采用者沿其登记的显式 cleanup 计划，逐项绑定工作树、分支、head、保留引用和已释放产物。`automatic cleanup is not opted in` 是入口不适用，不能报告清理成功，也不为收尾启用新政策。失败原件仍有调试或验收使用方时继续保留；等使用方结束再核删除资格。

发布来源 CI 与安全扫描各按其登记身份验收。发布的 `verified_run` 仍须是本仓规定的成功 dev push 检查；GitHub 默认 CodeQL 的原运行可能是 `event=dynamic`、`path=dynamic/github-code-scanning/codeql`，列表名称也可能显示 `Push on dev`。核对固定源码、工作流路径、实际扫描检查及结论，不把发布来源的 push 条件套给 CodeQL，也不因名称不同跳过必需扫描。

## 何时发布正式版

“稳定”按当前固定候选的可观察结果判定，不按发布次数、经过天数或暂时无人反馈判定：

- 选定候选前，核对仍有效的已知失败及其源码、输入和依赖。scoped CI 只覆盖实际选中的义务；后续无关检查通过，不能冲销候选仍继承的失败。相关修复须纳入候选并通过适用验收；没有修复或原件不足时，保留失败或 unknown，不判为稳定。普通独立 PR 仍按现有规则自动合并，不因此重跑全仓。
- 该完整源码的 dev 落地 CI 成功，全部登记发布构建、验证、两平台打包及安装验收通过。
- 四个下游都已采用同一个固定测试版，实际合并并通过各自的适用落地检查；已知发布或采用故障均已修复并验收。缺原件、未完成和 unknown 均未通过此条件。
- 正式版本目标已明确，且该源码的发布计划已是匹配的正式版本。若须改计划，按发布链改动先经过 integration 和 dev，再为这份新源码发最终测试版；此前候选只可作为固定基线数据。
- Release 说明准确描述已验收能力、升级所需迁移及已知限制。当前 SPEC 尚未全部实现；正式版不把有限采用包装成全部 SPEC、所有平台或跨环境同判。

条件满足后，先按下文推进 `main`，再在同一发布工作流选择 stable，显式传入完整 source、verified_run、verified_attempt、匹配计划的 version，以及最终测试版发布的 preview_run／preview_attempt。发布者核对 `main` 指向该源码，并按[晋升合同](release-ci.md#正式版复用最终测试版)核原运行、实际选中工件和原件，复用测试版的工具字节；缺原件保持未验收，不改写回执或回退重建。正式版本仍须核公开 Release 的非 prerelease 状态、标签源码、全部资产和 Linux／macOS 安装结果。随后让四个下游锁定该正式版并验收落地，才报告正式版已被下游采用。若正式版已公开后发现故障，保留其身份和原件，修复后发布新版本，不修改原版本资产或标签。

手动正式发布的 `--ref` 选择已经核验、指向同一源码的最终测试版标签，并核实际 run 的工作流 head。`source` 不控制工作流本身；dev 前移后，新的 needs／downloads 可能与旧源码的构建配方不匹配。核对和恢复由[发布 CI](release-ci.md#持续交付与恢复)负责，不为通过而补假依赖、改原回执或放宽 collector。

## 推进正式版分支

`dev` 是默认开发分支和测试版来源；`main` 是正式发布快照。满足上述全部稳定条件后，固定最终测试版源码 OID，再使用以下唯一晋升入口。将占位符替换为已验收的完整 OID：

```sh
gh api --method PATCH repos/ChronoAIProject/chrono-harness/git/refs/heads/main \
  -f sha=<已验收的完整源码OID> -F force=false
```

`force=false` 只允许快进，保留测试版源码与原 CI 的提交身份。读取实际 `refs/heads/main` 并核对其 commit OID 后，才执行现有 stable 发布入口。更新被拒绝、发生分叉或实际 OID 不符时保留原错误，解决真实依赖；不强推、不制造新的合并提交冒充已验收候选。现役 CI 仍由 `dev` 的固定落地检查提供，`main` 推进本身不触发新测试版。

`main` 首次从已公开正式版 `v0.1.2` 的源码 `5e179a6f9f4ff84b776a6e31bbc85d165c3bab07` 建立，原发布及两平台安装见[正式版运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/43161336225)。既有公开版本和标签保持原身份；分支晋升不代替正式版公开或部署验收。

## 如何持续改进流程

计划总耗时不足以说明慢在探测还是交接。采用[缓存阶段日志](ci-cache.md#命令探测的连续原件)的候选，按相同 CLI PID／尝试核原进程、输入返回、结果封存、原件核验、计划发布及依赖绑定的实际事件。累计墙钟秒数只在同一函数起点内作差，不跨起点相加；文件 mtime 只能辅助定位，不能替代阶段时钟或证明 CPU 原因。原任务终结后保留其输出与有限日志，再修负责处的已证实重复工作。日志补齐不表示旧超时已修复。

每次真实执行中发现入口变化、缺失判据或可重复故障，就在负责程序、采用登记或本说明修正，并同步更新 skill 的实际操作提示。skill 引用现役合同，不复制命令实现，不将一次宿主差异扩张成所有仓库的要求。对外写入范围沿用当前任务授权；用户取消、授权范围改变或必要外部输入不可取得时，保留具体状态和恢复条件，停止依赖该缺口的发布，其余独立工作继续。

会合等待失败时，先核驱动进程的原返回：控制线程断言前后，工作者仍须等待自有进程并保留真实结果，不能只留下 socket 超时。`poll` 返回 0 表示到期，只有负值才读取原系统错误；旧 errno 不能证明进程消失。内层 CI 判官没有阶段原件时，在测试宿主启用现有 `chrono-log-stderr/v1`，沿原期限和输出上限保留阶段，不能从静默推断停在何处。诊断补齐不等于修复原超时，也不以其他环境绿灯冲销它。

断言失败也须保留实际输入、返回内容和状态。只有“未包含预期文字”而没有实际诊断时，先在原断言补输出，保留旧原因未知；后续候选通过只能证明当前输入的结果，不能补写旧失败的根因。继续沿原未过滤操作验证原成功、失败及边界预期。

发布运行 `43582025992/1` 的实际源码为 `4ce46760b8df5b5d6ba4e287b3b3c137b0c0dd97`。Linux PR 测试 job `132408921442` 中，`timeout_and_output_limits_preserve_the_original_local_receipt` 因空回执数组被直接索引，在 `lifecycle.rs:698` 越界；原操作退出 `101`，生命周期测试为 `43 passed / 1 failed`。先在原断言保留 `initial_timeout`、CLI 退出码及完整返回，再判断没有回执的原因；不能把越界本身归因为 API 超时，也不能放宽原回执、期限或状态预期。诊断补齐不代表原失败已修复。

PR 等待超时先核首次 API 是否真实成功。它尚未返回时的 `unknown` 与取得 pending 观察后的到期属于不同阶段，按 [PR 合同](pr.md)分别核状态、head 和原调用回执；不能将任意退出 2 当作通过，也不要求尚无远端观察的操作报告 pending。替身日志的部分写入不是另一份完整派发记录，原期限和原生等待验收继续保留。

启动失败与子进程退出失败分别记录。调用启动前保存 argv、cwd 和意图，启动失败保留原错误及系统错误码，不虚构退出码或两条子进程输出。遇到 `ETXTBSY`，先核可执行文件的写入和描述符生命周期，不能盲重试、降低线程数或提高期限。并发测试的可执行输入使用已封口且不会被测试修改的编译产物；需改写内容的校验样本另作独立副本。真实复制或解包后的生产执行仍由原生发布验收，mock 通过不能替代它。 对“最后关闭后立即回收”的断言，原件若显示元数据繁忙，先核消费者和实际进度；复用已有单用例隔离入口，避免无关测试 fork 继承短暂租约。保留父套件默认并发、原维护轮数和每轮预算；缺少具体持锁者证据时仍保留未知。

命令替身也是实际执行输入。原回执显示时间窗耗尽时，核对实际落地源码中的替身依赖、体积、重复身份校验及各进程实际读数；并行夹具迁移可能改变已验候选的工具闭包，不能只读原 feature 或 integration。高频的简单接口用已有标准库小替身即可满足时直接复用，不引入整个发布程序或进程引擎，也不保留只为较大替身所需的仓库初始化。保留当前外部命令 mock 边界、原构建配置、断言、默认线程、未过滤操作和期限；按相同输入与单位验证新的执行物，不能靠增大预算或忽略模拟成本通过。

fetch 超时须核实际执行物及原进程阶段，检查是否在替身中重复复制共享对象或重写已有不可变对象；对象导入的行为由[执行说明](execution.md)负责。用回归先观察冗余写入，再验缺失对象与浅历史合同，实际时间改善仍须同口径测量。输入准备完成后才超时的参与进程，不能仅凭同样的超时字样归为同一原因。

本地操作超时需要进一步诊断时，使用同一 canonical 入口的 `--unit ID` 收窄到受影响单元，并核实际启动记录中的参数、环境和期限。`chrono-worktree use --operation` 使用宿主协议的期限，同名操作不代表它与 canonical profile 的操作期限相同。采样绑定本次实际 PID 和源码，原失败现场继续保护；诊断结果不替代修复后按 DELTA 选中的验收义务。需要另建固定失败候选的检出时，先等待 Git 创建真正完成，再 import；初始化中的 inventory 锁不能当作附件损坏。

中断用例的握手须等到受管使用方真正启动。若原捕获显示 wrapper 仍在准备输入，先核原生进程及登记期限；使用已有受管等待入口收取提前退出或握手，不让另一条较短的就绪期限主动制造中断。保留原进程期限和握手后中断、未知使用保护断言；新结果不反推旧输入准备的完整耗时。

已经取得握手后再超时，要核对同一进程期限内的输入准备、等待和维护。缓存消失的断言须保留实际维护返回、退出码及阶段耗时，再与原参与回执、活后代和清理记录对应；不能仅凭缓存已删判定提前清理活租约，也不能继续套用“尚未就绪”的修法。阶段耗时是该次观察，不是 CPU 时间或旧失败的补证。按当前登记选中真正包含失败用例的单元；单元通过但没有执行该用例，不能算回归已验收。

缓存独立地址用例取得握手后，原探测的期限仍在计时。中断前要核独立计划的准备、探测、封存和发布，以及随后维护的实际返回；驱动等待完成后保留完整 stdout／stderr 与各阶段累计墙钟耗时。`kill` 返回进程不存在只能证明发信号时的状态，不能单独说明它为什么结束；不能加期限、减线程或删原断言取得绿色。

握手前超时而已有运行日志时，分别核 `process.preparing`、`process.launched` 和 `process.completed` 的原时间，区分启动前准备与子进程运行。前置阶段慢不能直接归因为哈希或死锁；沿实际调用审计重复读取和校验。复用进程引擎已有的启动前身份检查时，仍须保留缓存命中及进程返回后的完整漂移检查，并验证变化的执行物没有被启动。原期限、线程及发布验证继续适用，性能收益由新原件实测。

年龄回归须分清业务到期条件与执行时限。复用阶段先采用在登记执行窗口内不会到期的政策，进入到期阶段时通过原 `migrate` 显式采用要验证的删除阈值，并核文件系统创建时间没有重置。保留源码、索引、reflog、缓存复用和原阈值下删除的全部断言；不改时钟、账本或执行期限。实际原件若已经报告到期删除，测试不能继续将该检出当作尚未到期。

“文件已生成”与“结果已完整封存并解除恢复引用”是不同状态。遇到封存绑定错误，先核实际 `phase`、结果绑定和恢复根；单轮预算结束不是业务完成。修复要覆盖预算边界下同一原件的续接及真实内容错误，不能靠提高预算、放宽成功条件或重复发布消除症状。旧失败未保留临时状态时，其具体触发仍保持未知。报告字节删除、元数据边解绑与最终地址退役也须分别核状态和能力；续接验证保留替换条目拒绝、原结果与活使用方保护，不能由路径缺失推断地址已经安全退役。

业务检查通过后仍可能在缓存阶段失败。先核实际 DELTA 选中的操作和缓存原错误；原生步骤成功不代表静态缓存清单中的每个目录都已生成。按[缓存合同](ci-cache.md#可选的生产者内容核验)记录缺失产物并停止保存该缓存，保留类型、权限和内容错误。不能补跑无关测试、创建空产物或关闭内容核验来取得绿色；整轮原生结果通过后才能作为发布输入。

新增证明源码除实际包含外，还须通过当前证明编译器的语法解析；原生专属 cfg 不能屏蔽不兼容的 edition 语法。在负责源码作等价调整，保留现有证明入口和原生 IO 验收；局部状态证明不能代证全部路径操作。

**测试源码、宿主验证登记与迭代提示同步修改：**PR 超时测试的首次查询分类由 `lifecycle.rs` 中同源函数维护，原测试直接调用；四种布尔边界同时作为 Rust 测试和 Verus 独立预期执行。FILEMAP 将该文件实际接入已有 `verify.pr-snapshot-tests` 根，并补齐其原生测试依赖。证明只覆盖这个分类，JSON 解码、回执读取、进程与超时 IO 仍由原未过滤测试验收；保留全部原期限、线程和断言，不宣称空回执根因已修复。宿主 catalog 同时补入上述 Release 下载计数的判法，由正式入口生成 skill；不改变发布程序的资产合同。额外证明、构建、生成、CI 和存储总成本尚未测量。

本文和 skill 维护宿主发布流程与 AI 提示；发布程序的行为和运行输入归[发布 CI](release-ci.md)及其登记负责。验收为现有按 DELTA 选择的 canonical check、skill 格式校验和真实发布／下游采用；文字检查不证明后两项已完成。新增文档和执行过程的总维护、CI 与存储成本尚未测量。

检查开始至结果收取期间保持该检出的 HEAD 和已跟踪输入不变。推送固定提交可以继续，新增修复使用其他隔离候选，或等待原任务结束再改；`checkout HEAD does not equal candidate` 是实际输入漂移失败，旧结果不能赋给新 HEAD。

Git pin 漂移时，从本候选 bootstrap 的原始回执取得实际路径、摘要、版本和进程状态，AI 显式更新宿主登记并重验；观察回执须留在原日志或声明上传根。不要根据另一仓库、相邻运行或本地平台推定值。GitHub 未开放自动合并且现有权限无法启用时，使用下游既有的匹配 head 合并入口，等适用检查通过后合并并核落地；不申请新权限，不新增用户确认。

退役安装器也要核当前调用方及外部扫描配置。删除 Python 安装器后，默认 CodeQL 若仍固定 Python，可能以“没有源代码”失败。核实际树、原扫描日志和是否还有当前 Python 使用方；按现有权限更新已无使用方的配置。权限不足则保留具体远端限制，不增加空源文件、关闭仍适用的扫描或把失败写成通过。

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

## 本轮技能维护来源

`iterate-releases` 的原有诊断经验保留，维护源迁入宿主 catalog 的 `core.release-iteration`，由同一 manifest 生成仓库 skill；个人目录只安装验收后的生成结果。FILEMAP 声明输入、生成者和输出依赖。新增提示来自本轮实际采用：保留完整工具映射、证明根真实包含源码、分别核对本地和 PR 基线、临时检出缺失时保全原身份。沿现有指令生成与 DELTA 检查验收，不新增专属测试、发布器或审批步骤；本轮总验证与维护成本仍未测。
