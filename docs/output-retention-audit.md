# 程序输出、使用方与滚动清理审计

本次审计的目标是：每项程序输出及其保管元数据从创建时起都有有限清理窗口；有价值的内容在窗口内转存到独立持久存储，实际消费者改用持久地址，临时原件到期清除。报告、失败现场和清理自身的输出也须遵守这个合同。使用方是本仓维护者、CI／发布调用方和安装本产品的宿主。

**当前结论：不能证明所有最终产物都能按时间滚动清理。** 有确定的覆盖缺口和反例，也缺少维护进度、外部交接的证明。当前仍有用途的源码、安装程序、固定报告和公开发行资产不能直接当作垃圾。下文区分有界存储、到期资格、实际删除和仍未知的使用方。

2026-11-17 的需求补充明确：所有临时产物都有有限窗口，有价值的内容应在窗口内
转存到其他持久存储；消费者、当前指针和恢复角色不能无限延长临时原件寿命。
规范性定义归 [SPEC §10.1](../SPEC.md#101-报告证据与临时产物的自动清理目标合同)。
本文件的固定基线反例和有条件定理仍是审计事实；其中“所有根最终释放”尚未由程序保证，
不能作为免除到期义务的前提。实现须进一步证明窗口不可续长、转存真实完成、消费者改读
持久地址、临时正文／目录／元数据最终退休，以及维护可公平推进。当前尚未达到这一目标。

## 11 月 19 日在运行中的宿主上核对的结果

以下状态对应 `dev` 的固定提交 `7872b5a42a059c8588f4d393ce471a5940dca108`。下文的全项目清单保留 11 月 16 日的固定基线。24 小时是本次验收采用的窗口。

| 产物及实际读者 | 已观察的行为 | 尚缺的验收 |
| --- | --- | --- |
| detector／单元工件供同一运行的 collection 取回，汇总报告供维护者核验实际判官 | 一天保留已入 `dev`。落地运行 `44411290710` 的汇总工件 `14479687303` 通过 ZIP、原报告、判官两流与退出状态核对，覆盖 21 个检查、146 项执行 | 首个已跟踪的 startup 工件 `14448903049` 的 provider 到期时间为 11 月 19 日 17:00 UTC。到期后还须实读元数据和下载地址 |
| 输出树内成员的读取器需要父对象租约 | 查询修复已入 `dev`。PR 运行 `44413424380` 核 21 个检查、146 项执行，受验与落地树相同。空查询无需创建子地址 | 历史检出六个原索引已由下述显式回收验收；统一定时服务采用仍待核实 |
| 维护者读取每轮处置及失败原件 | 减少重复无处置报告的落地运行 `44412447672` 核 13 个检查、98 项执行 | 宿主报告 `maintain-59544-1795039354841907000-5678.json` 仍记录 timer、failed、子工作已等待、31 项失败。逐行核对清单 145,302 行、167,919,617 字节，SHA-256 为 `6026174a6a0760b6bdb2946a292df14473c26ed396da764e4b49f856c29eaa8d`。连续三轮成功验收为 0/3 |
| 旧运行日志供运维诊断，判官另存原 stdout／stderr | `swift-blossom` 的同一文件包含从 11 月 16 日至 18 日的 2,355 行诊断记录，共 1,858,072 字节。它与协调者的四个日志槽位都没有期限头，两处宿主均未启用日志时间窗口 | 旧日志迁移候选的 diagnostics 单元先在截止时刻观察到 0 份回收，改后观察到 1 份回收，活锁、未知行和截断行得到保留。还需采用兼容写入器、维护程序和宿主配置 |

历史路径回收候选 `c7c0b59db31e8808f57395dc037ad1e09d02e833` 的 runner 单元通过，`verify.retention-release` 验证 128 项、0 错误。一个固定代际从原地址到待删地址、空目录再到消失，对应秩 3、2、1、0。证明以真实身份对应这些状态、每个持续合格阶段最终有一次成功推进为前提。定时服务的原结果和旧宿主上的实物处置仍是独立验收项。[路径回收候选](https://github.com/ChronoAIProject/chrono-harness/pull/1405)

失败恢复仍有具名消费者。254 份历史 `lifecycle-recheck` 原件共 925,856,103 字节。按原报告的操作字段统计，cleanup 占 86 份、producer-cleanup 占 71 份、maintain 占 47 份，其余 50 份涉及导入、迁移、检查和部署等操作。13 个报缺终态发布凭据的路径仍能按账本 SHA-256 读回原终态文件，当前路径声明均无原件 ID。该轮状态显示其中 9 个为 disposed、4 个为 retained。原终态的完整字节、当时的发布事实、调试义务是否已结清是三个不同的证据问题。

## 树成员读取留下路径索引

在 2026-11-19 的 `swift-blossom` 历史检出中，正式政策迁移给 500 份正文声明绑定了创建后 86,400 秒的期限。另有 54 份声明只剩元数据。对三份已到期的 `chrono-judge-projects` 结果执行定向维护前，原结果均记录已返回、零操作、零测试。三个对象均无登记角色、入向引用或出向引用。维护后逐项核实，正文目录、对象声明目录与能力文件均已消失。三个正文共 56,022 逻辑字节，文件系统实际释放量未知。

| 对象 | 正文退休后仍存在的索引 |
| --- | --- |
| `output-1356-1794840280744950000-0` | 结果目录与其中的 `result.json` |
| `output-67192-1794835670922933000-0` | 结果目录与其中的 `result.json` |
| `output-86179-1794845625260888000-0` | 结果目录与其中的 `result.json` |

六个地址各自仍有 `address.json`，其 `output` 均为空。旧版读取器逐级建立地址以查找保管租约，即使实际提供租约的是上层的输出树。对象退休时，路径游标遇到子地址即停止。显式维护只枚举对象、角色与事务地址，对象声明消失后，遗留地址没有独立的维护入口。由此可构造有限批次反例：此后没有发布者或读取者，再运行任意多轮这种维护，六份地址声明仍在。正文的退休判据不能单独证明保管元数据的进度前提。

读取器的使用方需要父输出租约来保护所读字节。成员无需独立的空声明。候选查询只锁定现存前缀，缺席成员通过上层声明查找。历史地址仍需由保管所有者在身份和独占租约核实后回收。历史外部读取者身份未知。

11 月 19 日随后对 `swift-blossom` 执行了真实显式回收。先复核没有角色和入向引用的候选：9 份已进入处置的成功 bootstrap，3 组共 226 项已对照原文摘要的通过操作，以及 4 份已定位后续验收的失败检查。其中 3 份原失败由 PR #1328 同链集成的 inventory、Verus 和格式检查通过记录覆盖，另 1 份在同一提交成功重跑。运行来自集成受验树 `84a422d2f0acbbfa0a089510bf0535994241a529` 的正式 `chrono-harness retention`，执行文件 SHA-256 为 `43abee923cf51eaf54b3ffe6ac4259005f08bfecb1de9d375f7f49f6ce00e49f`。

程序退出 0，stderr 为空，stdout SHA-256 为 `0937025296db587d4310c5105e4b1868729909a52e48b61a55777f7e7824786e`。原报告记 12,699 毫秒、1,121,366 个逻辑删除字节、0 项记账错误，物理回收量未知。运行前后逐项核对，567 个空映射中有 563 个原址消失，剩余 4 个仍承载子路径。上表三个对象的六个原地址全部缺失，按逻辑路径摘要核对的待删地址也全部缺失，待删目录总数为零。工作树的分支、提交和 Git 工作状态经复核。该操作只调用这一个历史检出的输出保管者，定时服务的连续成功数仍须单独验收。[集成受验检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/44415322435)

## 固定范围和判法

源码基线为 `e8eb9a71cfd28b2b28680908fde4b6724138bd40`，2026-11-16 从明确的 `origin/dev` 获取。审计不运行历史判官。范围是该提交的全部 40 个 Rust 项目（20 个生产项目、20 个专属测试项目）、2 个登记脚本项目、分发脚本、CI／发布工作流和 Python 示例。测试二进制对生产入口的源码别名，仍归原生产者负责。

入口清单来自 [projects.json](../.chrono-harness/projects.json)、[FILEMAP](../.chrono-harness/FILEMAP.json) 和 Cargo manifest；输出合同来自现役 action、bootstrap／release 配方、[产物登记](../.chrono-harness/git/macos-27.json)、[cleanup.json](../.chrono-harness/cleanup.json)、[retention.json](../.chrono-harness/retention.json) 和 [logging.json](../.chrono-harness/logging.json)。同时核对 Linux 产物登记、外部命令边界、文件写入／重命名／复制／链接、临时文件转持久、stdout／stderr 和 provider 上传。搜索用于发现可能遗漏的生产者，不替代执行计划或完整性证明。

验收必须同时回答以下问题：

1. 谁创建什么字节；失败、中断和成功分别留下什么？
2. 哪个具名使用方需要原文件；没有该文件，哪个已接入行为或明确在研目标失败？
3. 使用方什么时候交接；谁把交接持久化？
4. 谁枚举、获得删除资格、推进删除和回收元数据；是否有实际定时调用？
5. 有限预算、活使用方、损坏、失去通知或 IO 失败是否会使对象永久不前进？

本文件是维护者处理本次缺口的审计输入，负责方为 repository。当前结论保留在 Git；原运行数据仍由原保管者按自己的期限维护，不复制整套原件。缺口修复并完成验收后，在负责文档更新现状，本文件作为固定基线的审计结果，不作为运行时清理清单或政策。

## 全部生产项目的输出与使用链

表中的“接入”表示能定位到当前代码或配方中的读取端，不表示每个外部宿主已实际采用。全部命令还可能写运行日志；全部构建／证明的输出另见后表。

| 生产项目 | 文件／通道输出及负责源码 | 使用链与必要终点 | 回收结论 |
| --- | --- | --- | --- |
| identifiers | 库返回值，无独立运行文件；[lib.rs](../crates/identifiers/src/lib.rs) | runner、worktree、发布和判官的标识／路径判断 | 运行时无独立文件寿命；构建产物有真实编译使用方 |
| diagnostics | JSONL、`runtime.lock`、stderr；[rolling.rs](../crates/diagnostics/src/rolling.rs)、[runtime.rs](../crates/diagnostics/src/runtime.rs) | CLI 故障诊断；CI 原 stderr 另由报告采集 | 正常日志按容量滚动，缺少时间到期；锁文件是固定状态 |
| runner | 准备输入、快照／blob、原进程流／launch、检查原件、固定报告、full 导出、保留声明和租约；[prepared_artifacts.rs](../crates/runner/src/prepared_artifacts.rs)、[lib.rs](../crates/runner/src/lib.rs)、[full.rs](../crates/runner/src/full.rs)、[initial.rs](../crates/runner/src/initial.rs) | 输入→判官→单元／汇总→CI 上传／PR 准入；固定指针保护当前检查，原流用于核真实退出和失败 | 已采用 retention 的路径有条件回收；initial 报告及未采用时的直接写入不能据此认定已纳管 |
| instructions | 指南、别名和宿主输入；事务 staging／backup／intent／结果；[lib.rs](../crates/instructions/src/lib.rs)、[transaction.rs](../crates/instructions/src/transaction.rs) | 人与 AI 读取生成指南；原事务支持中断恢复、结果查询 | 当前指南是产品结果；事务原件有有限终态政策，领域恢复根和活使用方继续保护 |
| judge-ci | stdout 判定／选择说明、诊断流；通过 routes／projects 保存所选操作原件；[main.rs](../crates/judge-ci/src/main.rs)、[lib.rs](../crates/judge-ci/src/lib.rs)、[collect.rs](../crates/judge-ci/src/collect.rs) | runner 消费判定→本地退出／CI 准入；collection 读取各单元原报告并核原流，避免把缺失或失败写成通过 | 判定通道归 runner；操作文件归 projects 的 publication；日志归 diagnostics。判官自身没有另建一套报告保管目录，不能把委托执行产生的文件漏出审计 |
| judge-registration | stdout 响应；[main.rs](../crates/judge-registration/src/main.rs) | runner 消费登记判定→拒绝错误／缺失输入的执行 | 通道由调用方采集；无独立领域文件输出 |
| judge-filemap | stdout 影响图响应；[main.rs](../crates/judge-filemap/src/main.rs) | routes／runner 按明确 DELTA 选中工作 | 同上 |
| judge-routes | stdout 计划／执行响应；[lib.rs](../crates/judge-routes/src/lib.rs) | projects 执行当前登记方法，runner 汇总 | 同上；实际子进程文件归所执行 action 的所有者 |
| judge-projects | 操作目录、`result.json`、stdout／stderr／launch 原件；[state.rs](../crates/judge-projects/src/state.rs) | workflow／runner／离线汇总核对真实操作、证明源码及退出 | 经现有 publication 交接；未知结果仍保留恢复责任 |
| judge-cost | stdout 成本判定；[main.rs](../crates/judge-cost/src/main.rs) | runner 拒绝不满足当前声明的结果 | 通道由调用方负责；不是另建文件保留根 |
| judge-mixed | stdout 合并判定；[main.rs](../crates/judge-mixed/src/main.rs) | runner 组合语义／机械判定 | 同上 |
| judge-workflow | stdout、integration certificate；[certificate.rs](../crates/judge-workflow/src/certificate.rs) | 后续 workflow 核 integration 证据和原 producer 报告 | 证书直接写入登记位置；更新位置有界，但不能由顶层 state 的 evidence-retain 推出 TTL 或原报告引用已释放 |
| ci | 生成 workflow／profile、事件／本地输入、detector、gather intent／下载原件、inventory 报告；[lib.rs](../crates/ci/src/lib.rs)、[gather.rs](../crates/ci/src/gather.rs)、[inventory.rs](../crates/ci/src/inventory.rs) | GitHub 运行生成工作流；detector→单元→collection→准入；inventory 核未过滤测试的真实分组 | gather 已有原件责任；生成物是源码产品；inventory 固定名字限制数量，但不提供按时间退休；生成临时文件中断见缺口 |
| inputs | capture／pair／compose 快照、来源回执、内容寻址 blob；[lib.rs](../crates/inputs/src/lib.rs)、[composition.rs](../crates/inputs/src/composition.rs) | 明确输入→full／workflow／离线检查；来源回执避免执行历史判官 | `publish`／`publish_blob` 直接落盘；没有 publication 的创建／依赖／交接，不能证明最后使用后进入维护集合 |
| distribution | 平台包、组合包、manifest、安装器、adoption lock／launcher、安装文件、临时 stage／备份；[lib.rs](../crates/distribution/src/lib.rs)、[bootstrap.sh](../assets/distribution/bootstrap.sh) | release→下载者→宿主安装→实际 CLI；adoption 文件用于后续明确版本安装 | 当前安装／发行包有真实合同；RAII／trap 只覆盖正常退出，强制中断没有独立 TTL 保证；任意输出目录归调用方 |
| cache | plan、attempt、provider outcome、封存 manifest、恢复 intent／结果、下载和编译缓存；[attempt.rs](../crates/cache/src/attempt.rs)、[content.rs](../crates/cache/src/content.rs)、[recovery.rs](../crates/cache/src/recovery.rs) | CI plan→restore→bootstrap／test→seal／save→provider；验证失败不能掩盖构建失败 | plan／probe 已有一天、32 项、64 MiB 的终态选择；实际缓存及 provider 数据是不同保管链；旧恢复原件仍需真实交接 |
| worktree | checkout、Git refs／元数据、birth／ledger、迁移／保全、intent／result／process、producer 目录、完整维护 JSONL、socket／service 状态；[automatic.rs](../crates/worktree/src/automatic.rs)、[start.rs](../crates/worktree/src/start.rs)、[inventory.rs](../crates/worktree/src/automatic/inventory.rs) | 开发／构建使用检出；原件供恢复与清理；维护者读取完整清单核回收和保护原因 | cache、临时 producer、原件各有负责入口；`bin`／`state` 保留会使整个检出不能退休；分支／源码保全不是缓存，不能直接 TTL 删除 |
| pr | 操作目录／intent／命令流／结果、对象观察成员和响应；[state.rs](../crates/pr/src/state.rs) | 查询／create／watch→真实 PR 终态和 head→完成交接 | OPEN 和未返回命令有真实用途；永久不终结的外部对象没有有限释放保证；不能凭本地退出猜外部完成 |
| host-bootstrap-native | 编译缓存、安装、owner stage／指针、bootstrap 原件、共享 transport、工具链下载、证明构建原件；[construction.rs](../crates/host-bootstrap/src/construction.rs)、[shared.rs](../crates/host-bootstrap/src/shared.rs)、[proved_build.rs](../crates/host-bootstrap/src/proved_build.rs) | 安装→本地／CI 命令；共享 producer→单元；固定成功构建供来源核验和升级 | 当前安装及必要 stage 有使用方；失败 proof build 创建持久诊断使用方，须实际结清；下载／target 按登记生命周期释放 |
| host-release-native | frontend transport／源码与命令原件、平台／汇总包、失败证据、公开和安装报告；[execution.rs](../crates/host-release/src/execution.rs)、[delivery.rs](../crates/host-release/src/delivery.rs)、[local.rs](../crates/host-release/src/local.rs) | 固定源码→平台验证→collector→公开 Release→两平台安装；原报告用于晋升正式版及恢复 | 本地 cache 根按 dispose／finish-dispose；Actions artifact、公开资产、安装宿主各自有保管者，缺少全链最终释放证明 |

## 构建、测试、脚本和外部存储

| 输出族／生产者 | 实际使用方、用途与替代方案 | 有限保留证据／剩余义务 |
| --- | --- | --- |
| 全部 40 个项目的 Cargo target、`.rlib`／`.rmeta`／dep-info、增量缓存 | 安装、链接、测试启动；可按 lock 和登记工具重建 | 按各 action 的真实 target-dir 对应 cleanup 根；大部分 finish-dispose。main、活／未知使用、政策未迁移或源码保全失败继续保护；不是任意创建年龄删除 |
| 登记的 Verus `verify.*.rlib` 与 `.d` | 验证 action 需要实际编译；`.d` 被 `verified_sources::bind_compiler_sources` 消费以核固定源码，结果再交给 CI；不能把证明编译与仅执行 SMT 混同 | 固定文件名不随每次调用新增，但直接输出到 state，未发现独立时间退休交接。是否需要在结果封存后保留 rlib 原文件仍为 unknown；未据此删掉编译／源码绑定合同 |
| runner-tests、instructions-tests、judge-ci-tests、ci-tests、judge-registration-tests、judge-filemap-tests、judge-routes-tests、judge-projects-tests、judge-cost-tests、judge-mixed-tests、judge-workflow-tests、inputs-tests、distribution-tests、worktree-tests、diagnostics-tests、cache-tests、host-bootstrap-native-tests、pr-tests、host-release-native-tests、identifiers-tests | 每个项目消费自己的测试二进制、样本、替身、捕获和断言；别名 CLI 仍消费真实生产逻辑。失败现场用于调试当前失败，成功现场通常不需保留 | 宿主登记的 worktree／ci／projects／workflow-tools producer 采用目录级滚动，成功 0 秒，失败／中断 86400 秒，已删除元数据 3600 秒。隔离测试协调者采用的实际政策把这四类 producer 的已删除元数据期限改为 0 秒，失败／中断期限不变。其余 `TempDir` 正常 Drop 不证明 SIGKILL 后回收；显式 `.keep()` 另有缺口 |
| `host-bootstrap-launcher`、`host-bootstrap-launcher-tests` | 薄 shell 转交生命周期／Rust bootstrap；shell 测试消费自己创建的替身和调用捕获 | 测试成功 trap 删除，失败保留 `mktemp` 目录；当前 cleanup 的 producer 没有 `operations` 映射为该 action 包住全部 TMPDIR，不能援引文档里的运行级滚动代替实际配置 |
| `.chrono-harness/release/native.sh`、`consume.sh` | 前者转交已安装 Rust；后者从 transport 解包执行物给真实 release job | frontend／cache 生命周期归原 Rust 及配方；shell 不另建保管权威 |
| 分发生成的 launcher／installer | 用户执行固定版本安装，消费 manifest、lock、安装器和二进制 | 当前版本和安装是产品；临时目录正常 trap 清理，强制中断仍需宿主保管，不能把 trap 叫时间滚动 |
| Python 示例 `message.py`、`check.py` 及 install／verify shell | 示例程序打印 `message.txt` 内容，测试断言 greeting；install 复制四个实际 CLI，verify 消费 CI 配置 | stdout 归调用方；CPython 可能创建 `__pycache__`，安装副本和 bytecode 没有示例自己的 TTL；不能从 Rust target 的登记推及它们 |
| Git branches、临时 fetch ref、保全分支、rebind backup | 创建／恢复、保存源码／索引／独有提交；清理需要实际保全证据 | 临时 ref／rebind 有明确恢复入口；`remove_branch:false` 与保全目标不承诺自动删除永久 Git 历史，不应以清缓存替代源码保管 |
| GitHub Actions artifacts、cache、日志和 `$GITHUB_OUTPUT`／临时 runner 文件 | 单元／collection、发布 collector、晋升和安装 job；原生输出传递选择和 artifact ID。startup 也由始终运行的 aggregate 下载、核验和安装，不依赖业务单元是否被选中 | workflow 没有显式 `retention-days`；后续精确运行 API 已证实 12 项 artifact 有有限 `expires_at`，见下文。其余 artifact、cache 淘汰、日志与 runner 文件仍由 provider 负责；未取得全局有效设置及实际释放证据，不能泛化为全链回收 |
| 公开 GitHub Release／tag／发行资产及下游安装 | 下载者、四个示例宿主、正式版晋升和固定版本恢复；需求见[发布合同](release-ci.md)与[示例](examples.md) | 已核四个固定 dev 的版本选择，以及首次接入示例的 macOS 安装和实际检查执行物，见下文；其余采用、用途与最终释放保持 unknown。没有本仓时间滚动退役公开版本的合同 |
| 固定报告、当前安装指针、OS plist／socket、租约／owner 文件 | 当前位置查询、当前执行物／启动、互斥与恢复；删除它们可能使仍在使用的工具失效 | 固定数量与内容到期是不同性质。卸载、换代和释放后才可退休；不得泛化为按目录年龄删 state／bin |

隔离测试还存在一层独立保管链：[`isolated_capability.rs`](../crates/runner-tests/tests/support/isolated_capability.rs) 先分配外层临时目录，复制 Git／Cargo 替身，再把其 `custody/` 交给子测试。[`fixture_coordinator.rs`](../crates/worktree-tests/tests/support/fixture_coordinator.rs) 只在该子目录采用上述四类 producer，并覆盖已删除元数据的期限。外层实际持有目录租约、等待最初启动的子测试进程结束，再调用退休观察；只有子测试成功且观察确认退休，才删除整个外层目录。复制的替身由该子测试实际执行，失败原件由调试和回归验收消费；子测试失败或退休未确认时，外层 `.keep()` 保留原件。内层登记没有把整个外层目录交给定期维护者，打印路径及“按已登记政策处理”的提示也不构成持久交接。因此，迁移某个内层分配不能单独关闭外层失败／中断后的回收缺口。

## 可复核的缺口和反例

| 编号 | 事实／反例 | 为什么现有能力不能证明最终回收 | 修复与验收负责方 |
| --- | --- | --- | --- |
| O1 | `inputs::publish` 在调用方新路径发布快照；`publish_blob` 在 `state/inputs/blobs/SHA256` 新建不同内容的 blob；两者没有 retention publication | 产生者可连续生成互不相同的文件。retention 枚举自己的 objects／roles，不扫描 state 取得未知文件删除权；顶层 evidence-retain 保留它们 | inputs 与 runner：复用现有 publication，登记快照→blob 及最终报告依赖；成功／失败／中断交接，历史精确接管；先核 full／compose 使用链，再验证末项使用结束后原件和元数据实际退休 |
| O2 | 基线的 `ci-tests/tests/initial_short.rs::Host::drop` 在 panic 时 `.keep()`；`ci-tests/tests/gating.rs` 失败时新建 `chrono-gating-failure-*` 并 `.keep()`；`runner-tests/tests/support/isolated_capability.rs` 在子测试失败或退休未确认时也 `.keep()` 外层目录 | 这些分配没有完整的定期保管交接；内层 producer 的滚动声明不覆盖外层目录。正常使用的 TMPDIR、OS 一般清理或打印交接提示不能补出产品合同 | ci-tests 与隔离测试负责方：复用既有 TemporaryHost／rolling producer，分别覆盖内外层实际分配；保存原失败到有限诊断窗口，验证 panic／强制中断／活使用保护及到期后整个目录与元数据回收 |
| O3 | host-bootstrap 的 `proved_build::build_attempt` 创建 `proved-build-repair:<目录>`，正文声明“调试／验收后释放”，不提供最终释放时间 | 期限和真实消费者保护并存；永久不结清消费者是合法执行，TTL 不会越过它 | bootstrap 调用方：绑定真实失败／修复活动的终点及恢复责任；结束消费后正式 release-consumer，不能在调试义务未完时由维护者猜释放 |
| O4 | diagnostics 仅在新记录会超容量时轮转，`open` 只处理过量槽位 | 写入停止后，任意旧日志可无限保留。按当前政策写入的正常家族最多 50 MiB；这不包含外部或旧政策留下的超大文件，`open` 不会自动把它们缩至当前上限。不能把“非时间轮转”说成正常家族无界增长 | diagnostics／宿主：若要求全部临时输出时间到期，需在现有所有者接入有限年龄；必须保留与不可变 stderr 原件的分离，不让日志轮换破坏报告 |
| O5 | `ci::write_file` 使用 `.chrono-ci-PID.tmp`，错误返回尝试 remove；分发、inputs、bootstrap 使用 RAII 临时文件；shell 使用 trap | SIGKILL／断电不执行清理代码；PID 文件名、年龄或空闲不足以取得历史未知临时文件的删除权 | 各写入者：让实际新分配进入现有目录级生命周期或对应恢复事务；注册发生在写入前，不能另建扫描任意 `/tmp` 的删除器 |
| O6 | 当前 `bin/` retain、`state/` evidence-retain；OPEN PR、current-report、native-delivery、领域恢复根可持续存在 | 有效保留根能永远阻止其依赖删除。缓存 TTL 不能证明安装／证据／整个检出最终退休 | 实际安装、报告、PR、上传和恢复负责方：分别核真实终点与换代交接。unknown 继续保护；不能为了全绿把引用删掉 |
| O7 | 显式维护每地址有 25 ms、256 节点、64 MiB 的预算；损坏、忙、预算耗尽会继续保护 | 定时存在不证明单对象每隔有限轮一定推进；对象大小／图准备和 IO 可能让每次都停在有效动作之前 | runner retention／worktree：给实际状态机定义可下降的进度量，并证明每个持续合格对象公平取得有效步骤；行为验收覆盖大于预算的清单、引用环、并发新增、独立损坏和重启 |
| O8 | 维护的完整 JSONL／父报告本身也是新原件；一次真实清单已达 593,413,179 字节 | 完整清单有维护者核对用途，但“清理成功”不意味着清单的引用、父报告和元数据已最终释放；每轮再写报告可以形成新的积累来源 | inventory／生命周期报告负责方：核父引用、当前／失败窗口和实际读者交接；至少三次真实定时完成并观察上一轮已释放证据到期退役；测原件、元数据和净新增量 |
| O9 | 在后续固定提交 `9a085abc882988f0253b9704be23c9858cf0ca4a`，[`publish_owned_with`](../crates/runner/src/retained_artifacts/retention.rs) 先 `create_dir_all` 创建目标父目录，只登记目标文件；[`dispose_entry`](../crates/runner/src/artifact_disposal.rs) 只删除指定条目，不退休其祖先 | 调用方选择新的嵌套路径时，文件及其元数据退休仍可留下未登记的空父目录；不断变换父目录可积累目录／inode。固定的 `inputs/blobs` 等父目录数量有界，不能据此称标准路径无限增长 | 发布器与调用方的目录所有者：核实际目录分配、共享使用方和用途，明确交接及退休责任；复用现有目录生命周期，验证末个使用方释放后新分配目录退休，不能按空目录或年龄盲删祖先 |

这些缺口没有取消当前输入、调试、恢复、源码保全或公开发行需求。本次没有证据把 unknown 文件认定为冗余，因此不直接删除它们。

O1 后续修复先补实际读者：pair 保护源快照／blob，compose 保护清单／运输原件，
registration 保护本地 blob；作用域沿已有原件所有者保留到调用方交接。
两个 Rust 回归先复现“读取成功后，外层仍使用时维护已删除原件”，再核修复后的
保护和释放；具体边界见[原件读取](artifact-retention.md)。这不纳管 inputs 的发布，
也不证明 full 的快照读取、持久依赖或跨主机交接已补齐，O1 继续未关闭。

## 形式化结论

令 `O` 为程序产生的文件及目录代际集合；同一路径的新 inode／内容视为新对象。对对象 `o` 和时间 `t` 定义：

- `P(o,t)`：该代际仍存在；`M(o,t)`：原件保管元数据仍存在。
- `A(o)`：写入前已由实际所有者登记；`I(o,t)`：身份／政策／源码排除检查有效。
- `C(o,t)`：活读取凭据、真实使用方、当前指针或领域恢复根，及从这些根可达的引用闭包仍保护它。
- `E(o)`：真实完成／交接时间加有限保留期；`Q(o,t)`：该对象在这一时刻得到有效维护步骤。

安全要求为 `Delete(o,t) ⇒ A(o) ∧ I(o,t) ∧ t≥E(o) ∧ ¬C(o,t)`。这要求删除效果前再次核对，不是“年龄够了就删”。实际内核租约、文件系统观察及交接 IO 仍有各自证明义务，不能由下列数学式替代。

用户希望的全称活性命题为：

```text
U ≜ ∀o∈O. ∃T<∞. ∀t≥T. ¬P(o,t) ∧ ¬M(o,t)
```

**U 在当前程序／政策下为假。** 取 O1 中一个已成功发布、后来不再需要的独立快照 `s`，不再执行 inputs，持续只调用现役 maintain。`s` 没有输出声明；maintenance 只枚举声明，顶层 state 保留；不存在该对象的 Delete 转移。因此可构造无限轨迹 `∀t≥t₀.P(s,t)`，直接否定 U。即使把 O1 排除，取 O4 在最后一次成功写日志后不再写入的轨迹，同样没有按时间删除的转移。取始终保留的有效消费者，还可同时满足安全规则且使 U 为假。

24 小时不可续长窗口也没有消除最后一个反例。在候选 `a60c3681` 中，令已封存对象
`o` 在到期前取得具名消费者根 `c`，随后只运行维护，不执行该消费者的释放或持久转存。
`reference_locked` 仅对新增引用检查准入；维护仍把已有根及其闭包加入 `rooted`，
到期时只记录 `deadline_missed:true`。以维护轮次归纳：初态 `c→o`；每轮不删除这个
消费者根，且有根对象不进入删除，故下一轮仍有 `c→o ∧ P(o)`。这给出时间持续前进、
维护持续运行、对象仍永久存在的合法无限轨迹。它不依赖 IO 失败或未登记文件，
说明局部窗口证明不能替代消费者在窗口内交接及最终退休的进度证明。

O9 给出独立的目录反例：调用方在此前不存在的 `state/output-N/` 中发布一个文件，
随后释放该文件的全部使用方，维护删除文件及其元数据。令 `d` 为此次新建的父目录；
在继续运行现役 maintain、没有其他目录所有者接管的轨迹中，`d` 不在文件声明里，
没有针对它的 Delete 转移，故仍有 `∀t≥t₀.P(d,t)`。这个反例不依赖文件回收失败；
重复分配不同父目录可增加目录数。固定目录复用的有界结论与此不冲突。

运行日志还有一条持续写入的反例。只要当前文件持续接收小记录且未超容量，
最早记录就仍处于同一文件代际。每条新记录都可刷新修改时间，因此以末次写入时间判定
到期会延长同一代际的窗口。本机观察到主仓的当前日志在创建 80 小时后仍在写入，
最老的轮换文件已超过 224 小时。文件日志服务于维护者的运行诊断，判定报告的原始
stderr 由报告生产者另行保存。日志生产者须在写入前为每代声明固定期限，持续写入时
也须换代。同一维护入口应在没有新写入时回收合格旧代际。读者如需长期保存诊断，
应在窗口内将必要内容交给实际故障处理方并改用其保存地址。

可以陈述一个**有前提的回收定理 L**。对有限的一批对象，若存在有限 `T₀`，使此后每项都满足：

1. 已纳管、身份稳定，拥有有限的交接／中断保留期；所有真实使用方及根最终持久释放，不重新引用该代际。
2. 真实维护机会无限继续，已合格对象不会饥饿；每个所需文件／声明 IO 最终可成功，不永远卡在预算准入之前。
3. 存在自然数进度量 `r`，包含剩余清单节点、依赖脱离、删除和元数据退休步骤。无动作不增加 `r`；每个有效步骤使 `r` 至少减少 1；中断恢复从已保存的状态继续。
4. 删除正文后仍会触发有限元数据期限，且相同公平推进条件继续适用于元数据；新维护报告属于新代际，不反向重新保护已释放的这一批。

则每项正文及元数据最终删除。

证明：在 `T₀` 后对这批对象的进度量求和 `R=Σr(o)`，`R` 是有限自然数。只要 `R>0`，公平性及最终可成功 IO 给出有限时间内的下一有效步骤，且 `R` 严格下降。自然数不存在无限严格下降链，至多 `R(T₀)` 个有效步骤后所有正文删除。有限元数据期限过后，对元数据步骤应用同一论证，得到 `¬M`。这证明 L；它没有证明程序已经满足 L 的全部前提。

要进一步给出墙钟上界，还须有每个有效步骤最大等待 `D` 和维护／IO 完成上界，才能写出类似 `T_delete≤T₀+D·R(T₀)` 的界。当前睡眠／注销、长轮次、活使用方、失败、预算停顿均没有这样的已验上界，不能承诺“每小时清完”。

“每个对象最终删除”也不推出“总占用有界”：若第 n 个对象存活 n 秒且每秒新增一个，所有对象最终删除，总量仍能持续增长。总存储界另需生产速率上界、有效回收延迟上界和活使用容量界；当前这些量未知。50 MiB 的正常日志家族容量是独立的局部界。

`verify.retention-release` 对有限批次检查上述自然数下降论证。`retirement_rank`
汇总每个对象的正文和元数据剩余步数。`retirement_stays_complete` 在进度量单调且
每个正值最终下降的前提下得到一个归零时刻。`zero_rank_finishes_each_member` 将
归零结论分别落实到正文和元数据。实际执行器能否为每个对象持续取得有效步骤，仍须
按目录、引用、排除凭据、重启恢复和生产预算核验。其他局部保证见 [形式证明范围](verus.md)。

2026-11-19 复核本机连续两次真实定时轮次
`maintain-59544-1795015300039536000-1476.json` 与
`maintain-59544-1795018417110007000-1984.json`。两份 JSONL 完整读取后，其
SHA-256 分别与各自父报告吻合。按保管根、轮次起始地址和推迟处置的原件地址去重，
两轮分别有 3,307 和 2,665 项因预算耗尽而未尝试处置，重合 2,033 项。
其中分别有 3,191 和 2,601 项的处置节点为零，且已读元数据低于 64 MiB 上限。
两轮主保留库分别报告删除 34,359,723 和 27,201,255 个逻辑字节，但整体轮次均失败。
这组观察说明即使轮次继续且有其他处置生效，同一地址仍可重复停在准入前。
原件是否满足全部删除条件，以及这种停顿能否无限重复，仍须按具体对象核验。

## 后续输入发布修复的范围

O1 的新分配复用现役 retention 发布、依赖和读取保护，接口与验收见
[输入产物保管](inputs.md#输入产物的保管与验收)。库调用返回 `Produced`，保护实际
读取和发布的原件；跨进程导出须由具名调用方接收消费者声明，使用结束后释放。
快照依赖共享 blob，组合来源回执依赖组合结果及保留的来源原字节，暂存也在写前登记。
没有新增保留期限或清理服务。行为回归分别检查活结果、两个独立使用方共享 blob、
先释放其中一个及最后释放后的正文／元数据退休，以及 CLI 缺失交接时零业务写入。
这些用例的实际结果以候选 canonical check 原件为准，不是全仓最终回收证明。

已有未知原件没有被这条新发布路径接管；未采用宿主、历史精确迁移、维护公平推进、
外部调用方最终释放和真实定时验收仍各有独立义务。因此固定基线的反例保持有效，
也不能把新分配获得有限到期资格写成 O1 的全部历史产物已经清理。

## 真实维护原件与采用边界

2026-11-16 的只读观察表明，macOS 已加载 `org.chrono-harness.maintenance`，以 `maintenance-service` 服务 `/Users/macstudio2/chrono-harness`，间隔 3600 秒，控制台两流均为 `/dev/null`，有 OS datagram socket。所选程序地址来自 `rooted-wide-report-20261116`，其观察 HEAD 为 `73860e7617c8edd07c2415bf48804d7ff0eca779`。这与审计基线不同，不能把其运行说成本候选部署验收。

读取的已完成父原件为 `maintain-19816-1794827069531814000-0.json`，原状态 `failed`。另一个较新报告仍为空，保持未完成／未知，没有重启维护或改写旧结果。

| 原件观察 | 实际值与限制 |
| --- | --- |
| 父报告大小 | 57,736,587 字节；不重复存储其正文 |
| 完整 JSONL | `.chrono-harness/state/worktrees/maintain-19816-1794827069531814000-0.json.inventory.jsonl`；593,413,179 字节，563,943 行 |
| JSONL 字节核验 | 完整流式读取并计算 SHA-256 `37df064db46ccf7749a57aa38398ad4d9194972e4a740d39f66747b78c6a8540`，与父报告相同 |
| 观察时间窗 | `1794827070031` 至 `1794831614078` Unix ms；非原子库存；报告 elapsed 为 4,544,022 ms，不能当成单次目录清理耗时 |
| 完整性 | `measurements_complete:false`、`member_enumeration_complete:false`、`physical_release_bytes:null`；12 条 enumeration-error、3 条 unknown-declarations |
| 全流内容 | 468 条 worktree、12,423 条 worktree-artifact、74 条 producer-allocation、550,408 条 retention；数量含同一对象多次观察，不是文件／操作数 |
| 去重观察 | 按 `(source_root, namespace, address)` 得到 32,576 份有正文输出的不同声明：lifecycle-original 10,874、check-original 20,807、pr-original 860、bootstrap-owner-stage 11、ci-generation-operation 20、instructions-operation 4；不是全库当前存活总数 |
| 主保留库结果 | 原报告 `removed_logical_bytes:295214810`、`physical_bytes_reclaimed:null`、`remaining_outputs:null`；不能与完整清单体积相减声称实际磁盘净释放 |
| 原错误 | 8 条 cleanup_failures；另有缺失原件、政策／项目格式不符、Git 未解决冲突和身份／所有权不符。保持原失败，未借此清理其他人的源码／元数据 |

一个实际 `check-original` 声明仍有 `consumer:launchd-maintenance-program-202de6ec-20261115` 与 current-report 根；所读父报告来自 main 的源码观察 `202de6ec…`。它证明存在真实引用，不证明该旧程序当前仍被这一个 launchd 服务使用，也不证明全部外部读者已结束，必须由原安装负责方结清。

后续只读核验取得同一服务的四份已完成原报告：上述首轮，以及
`maintain-19816-1794831618082676000-409.json`、
`maintain-19816-1794835908281788000-821.json`、
`maintain-19816-1794840761273716000-1232.json`。
四轮均为 `failed`，各有 8 条 `cleanup_failures`；第四轮明确为 `timer`，
`owned_work_joined:true`。其完整库存为 584,967,436 字节、559,663 行，
SHA-256 `a341e3a55313142df09c02ebeade9bbed307ea8346262d03b81d40158c9249dd`，
已完整流式核对；父报告内 9,171 份进程记录的两流也均核对解压长度和摘要。
该轮观察窗为 `1794840761971` 至 `1794845129311` Unix ms，
报告 elapsed 为 4,367,311 ms；两项完整性标志仍为 false，
`physical_release_bytes:null`。这更新的是实际失败证据，不能计为三次连续成功定时验收，
也不能把非原子库存体积变化当作物理释放量。本次没有重启或改配该服务。

第四轮这个固定库存还包含前三轮父报告及各自 JSONL 的六份声明。三份父报告均为
`phase:released`，持久 `roles` 和 `incoming` 都为空；各自 JSONL 也为 released，
只被自己的父报告引用，没有其他持久角色。父报告的 `expires` 分别为
`1794918017`、`1794922306`、`1794927159` Unix 秒，直接依赖数分别为 202、204、204；
未观察到这三份父报告直接互相引用。JSONL 的原期限虽已到达，父引用仍保护它。
这为 O8 补上了具体的有限期限和父子交接观察；实际内核使用凭据、后续完整依赖关系、
公平维护和到期删除仍须核验，不能由六份声明推出全库报告都能退休。

本次候选 `bc4d037b…` 的本地 canonical check 实际通过，原件为 `preparation/check-a8cf1a68df21d2ced9cf2e0304b60023f32b17e527aee736f854a781924ecd7c.json`。原 judge 退出 0、`failure:null`、`owned_work_joined:true`，两流完整且摘要与原字节相符。但控制台同时报告维护请求 `wake-failed`，原错误为 `No buffer space available (os error 55)`；它没有宣称轮次完成或删除。这是 O7 的实际激活缺口，不能用业务检查通过覆盖它，也没有据此重启或重复启动维护。

外部只读 API 在同日返回 Actions cache 29 项、10,412,706,640 字节；这是 provider 的逻辑大小观察，不是本机占用或永久容量界。公开版本 `v0.1.5-dev.43667091970.1.g92022124cdf26b4bab71b135ec78f13a6318de4e` 的安装器和 manifest 各有 3 次下载、两平台 worktree 程序各有 2 次下载。这支持“确有下载消费”，不能识别下载者、证明当前安装或区分内部安装验收与外部用户。首次读取仓库级 artifacts 列表失败；补取 HTTP 原响应为 500、空正文、请求号 `F953:1CD415:70E10A4:B2233F8:6AFB0357`，当时有效过期信息保持 unknown，没有用默认天数补值。

2026-11-17 后续按精确运行读取
`repos/ChronoAIProject/chrono-harness/actions/runs/RUN_ID/artifacts`，取得以下实际 metadata。
两次均返回完整的运行内列表；12 项均为 `expired:false`，合计 API 大小为 92,471,479 字节。

| RUN_ID／固定源码 | artifact 数量与当前用途 | API 字节数 | 全部条目的 `expires_at` |
| --- | --- | --- | --- |
| `43891490817`／`7a45d8d2c1e49658f8fb248f9b42569b9bc51cc4` | 3 项：startup、detector evidence、collection | 41,480,323 | `2027-02-14T14:58:48Z` |
| `43891364614`／`5fb22791e9eb4351f69ad2cf909e856db68d8658` | 9 项：startup、detector evidence、collection，以及 judge-registration、ci、judge-projects、runner、judge-routes、cache 单元原件 | 50,991,156 | `2027-02-14T14:58:06Z` |

这证明这 12 项实际产物具有有限过期 metadata，不证明未来删除已经发生、全仓 artifact
均有相同政策，或释放了本机空间。仓库级列表在缩至一项后仍返回空／非 JSON 结果，
全局覆盖继续 unknown。startup 的实际读取端由
[units.json](../.chrono-harness/ci/units.json) 的 `job_gating.startup.consumers.aggregate`
登记；[生成工作流](../.github/workflows/chrono-ci.yml) 的 aggregate 以
`needs.detect.outputs.startup_artifact` 这个精确 ID 下载，按 aggregate profile 安装，
随后执行 `check --collect`。文档 DELTA 没有业务单元也不取消这条使用链，不能据此退役 startup。

后续在已落地的 dev 运行中再次读到约 90 天的工件保留期，超过本任务暂定的
24 小时窗口。仓库级工件和日志保留设置接口对当前集成凭据返回 HTTP 403，
因此由已核验的逐运行元数据判断该批工件，仓库级日志、缓存及实际到期处置仍待核实。

发布晋升需要跨运行读取测试版发布回执及其绑定的平台工件。晋升没有 24 小时内完成的
合同。若测试版发布于 `t₀`，晋升在 `t₀+25h` 开始，而原下载地址在 `t₀+24h`
失效，则当前晋升会在核验成功来源时失败。应先由既有发行存储接收有实际晋升用途的证据，
核对原回执、平台包、来源身份和摘要，再用实际晚于窗口的晋升验证新地址。单元工件和
startup 的运行内消费者仍须用各自的真实 job 和重跑合同验收。两条链的负责位置分别见
[发布晋升](release-ci.md#正式版复用最终测试版)与[单元和汇总](ci-units.md#conditional-jobs-and-the-required-aggregate)。

2026-11-19 按完整归档字节核对了一次成功测试版。发布工作流运行 `44385842612` 的第 1 次尝试选择 dev CI `44381529770`，发布回执绑定 `b486e5e541b423cd656d070e7d454ef51fea212e` 及 GitHub Release `450516532`。以下三份 ZIP 的 SHA-256 和字节数均与 GitHub 工件记录一致。发布回执及两个平台回执均记录成功，并各自绑定原作业和同一运行。正式版晋升以发布回执选择两个平台 collector，随后重验原始进程流、平台包、公开资产与固定源码。

| 工件与实际作业 | ZIP 字节 | 文件数 | 解包逻辑字节 | 内容已与公开发行资产匹配的逻辑字节 |
| --- | ---: | ---: | ---: | ---: |
| `14467886605`，`publish` | 63,875,545 | 141 | 148,490,027 | 148,045,027 |
| `14467781183`，`linux_package` | 34,894,982 | 1,147 | 83,557,117 | 80,944,904 |
| `14467104169`，`macos_package` | 30,418,092 | 1,147 | 69,778,236 | 67,088,288 |

以完整文件的 SHA-256 和长度计算，2,435 个路径合计 301,825,380 逻辑字节，共 275 种不同内容。34 种与公开发行资产一致；其余不同内容合计 4,450,275 字节。这个数字未计路径清单和传输编码，也未证明每个剩余成员都需要跨运行保留。临时 ZIP 的实际到期时间为 UTC 2027-02-16 19:57:16。当前晋升仍通过 Actions 工件取回原件。发布者还会拒绝已公开版本上未登记的额外资产，迁移必须配对更新真实写入端、读取端和发行资产判法。[发布运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/44385842612)

对同一运行按晋升代码的实际输入展开，发布者核对 139/141 个路径，两平台 collector 各核对 1,140/1,147 个路径。2,419 个路径共 301,763,875 逻辑字节，覆盖 3 份主回执、136 份进程流、34 个公开资产、2 份平台清单、32 个平台二进制和 2,212 个 collector 已声明输入。逐项以原 ZIP 字节核对声明长度及摘要，扣除公开资产的相同内容后，晋升仍需在受限保管方保存 **234 种不同内容，合计 4,421,812 字节**。余下 16 个路径的 61,505 逻辑字节包含两个发布 API 请求和两平台依赖／frontend 文件。它们有生成阶段用途，当前晋升校验没有读取这些路径，其他跨窗口使用方未知。稳定版仍依赖三个 Actions 工件存续。[消费者读取范围与原运行](https://github.com/ChronoAIProject/chrono-harness/pull/1409)

这两类地址的实际访问范围也不同。未携带凭据请求工件 `14467886605` 的 ZIP 接口返回 HTTP 401，同条件读取公开 `install.sh` 的第一个字节返回 HTTP 206。一个现有草稿的固定 Release ID 在已认证请求中返回 200，在匿名请求中返回 404；用其标签查询得到另一个公开 Release 的 ID。选择受限 Release 作为保管方时，晋升须绑定并重验固定 ID 与草稿状态。仓库当前的发布配置尚未指定保管构建进程原件的持久位置。

2026-11-17 还只读核对四个示例仓库的固定 dev。它们的 `.chrono-harness/distribution.json`
都选择 `v0.1.5-dev.43590532131.1.ge307951ea6ebe94c7f91300a315249079fccbe6b`，
每个固定提交均有成功的 dev push CI：

| 示例仓库 | 固定 dev 提交 | 同提交的成功运行 |
| --- | --- | --- |
| chrono-harness-examples-go | `bac97bf72bcfa4596da685911d570d6aacb00aab` | [43594251829](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/43594251829) |
| chrono-harness-examples-ts | `b1eb024a2f1340208857ea34c6350d3b92a04a03` | [43594250908](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/43594250908) |
| chrono-harness-examples-mix | `313773ecb309e9c73617a4db30232a6237d0d4bb` | [43594252987](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/43594252987) |
| chrono-harness-examples-initial | `0dacf9660af0791c40e650cdad37c0634a40ce29` | [43594251814](https://github.com/ChronoAIProject/chrono-harness-examples-initial/actions/runs/43594251814) |

首次接入示例的 workflow 在该固定提交先调用 `install.sh`，再运行标准 `check`。
原 artifact `14113503015` 的 ZIP 为 179,853 字节，SHA-256
`9806ec085cc905bb56ba2f34ed0e07f68d10d22821908d3ac1cfd6880d68d0a5`，已完整核对。
其中 `distribution.json` 为 `chrono-install-result/v1`，选择 macos-aarch64，
16 条安装记录的摘要／长度均匹配该公开版本的原 `release.json`；清单为 6,649 字节，
摘要 `9ec8035524d16f8e13bb9088c3ede78a0d457ae27bfcaa9d28e6cf88940a240d`。
原检查候选匹配该示例提交，judge 退出 0、子任务已等待，两流完整且摘要已核，原响应 passed。
runner 的实际摘要 `d9891811d8248f6eb3fe290b6d9e38812d9c9177aacf8c8777bcd57bf782d2b5`
和 judge 的实际摘要 `96e94b2b37ea951b2d3c348e575d3b76b3783c741d9aec5dcc3bef4ed1abd751`
分别匹配公开的 `chrono-harness`、`chrono-judge-ci` 资产。这建立了这两个文件的具名实际使用关系；
其余资产的安装后用途、其他外部用户和全部最终释放仍为 unknown。
本次没有重跑下游 CI 或改变其采用版本；后续版本不能沿用这份固定采用结论。

### 三个 Linux 示例的原安装与实际执行

2026-11-18 补核上表 Go、TypeScript 和混合示例的同一固定提交及运行，未重新执行业务。
原运行日志中的 `chrono-install-result/v1` 分别来自 4、4、6 个作业；每份均安装
16 个成员，其名称集合、摘要和长度逐项匹配上面的公开 `release.json`，平台为
`linux-x86_64`，源码为 `e307951ea6ebe94c7f91300a315249079fccbe6b`。
安装记录只证明这些作业取得了文件，实际业务使用另从检查原件核对。

| 示例 | collection artifact／ZIP SHA-256 | 核对的检查报告／不同进程观察／流引用 |
| --- | --- | --- |
| Go | `14113827020`／`248e39ccf9194c8065700078fc06ee6530f62c752fc4a17377479c9bffe5004a` | 6／142／620 |
| TypeScript | `14113448313`／`daf383cb745724cb12be8338208f779ba83404c719016eaacd0a89607cf0f112` | 6／131／570 |
| 混合 | `14113822012`／`adb90a2d88600f8d1685ec89c4c75e2be3e927f7cd4b3201fbd6eebaf8f4754b` | 10／218／946 |

表中报告数含单元、汇总及其下载副本，流引用也含重复引用，均不是业务重执行次数。
逐份检查的候选匹配上表固定提交，原响应 passed，judge 退出 0、子任务已等待、
两流完整且摘要正确。不同进程观察按原 launcher／child／argv／执行物摘要去重，
不等同于操作数；所有有序列化原流均核摘要，已提供长度和完整性字段的也逐项核验。
Go、TypeScript、混合各有 28、28、36 条 `gh` 流引用未序列化完整性字段，
因此不对这些引用补称“完整”。

三者都建立了 `chrono-harness` → `chrono-judge-ci` → `chrono-ci` 的实际业务使用关系，
原执行物摘要逐项匹配该公开版本的 Linux 资产；`chrono-ci` 的版本探测另有观察，
没有把它单独当成业务使用。其余 13 个安装成员在这些原件中的业务用途仍为 unknown，
不能用安装、下载次数或专属测试替它们证明必要，也不能据此认定可删除。

日志 ZIP 摘要依次为
`137cad3ce65db36e29befd0b25036571bbc042e5f7e4dc1a75a112d1093b63aa`、
`90f9ebdfa423149cdaff6c2ef41fc37fd1fa46f45b959ed1b3ed3391b8c62878`、
`c6244bc69bdd826ad7bcc8500ac610dd0f60981a144e1000c95e8a2d95d8e67c`。
provider API 为上述 collection 返回的 `expires_at` 分别为
`2027-02-12T12:19:34Z`、`2027-02-12T12:19:33Z`、`2027-02-12T12:19:34Z`；
这只是外部保管者声明的未来期限，不证明已经删除，也没有满足本任务暂定的 24 小时窗口。
本次 ZIP 和日志只在内存中核验，没有新增持久下载副本。

本次源码检出开始时没有构建输出，`du -sk .` 为 14,116 KiB；卷可用空间为 178,987,352 KiB。主库 bin／state／cache 的另一次计量没有收取到完整退出和输出，保持 unknown，未将它当成零或空间收益。本次没有运行新的全库维护，也没有声称已完成三次定时验收。

## 当前候选迁移与验证（2026-11-17）

以下原件和相对路径均归协调宿主 `/Users/macstudio2/chrono-harness`，不是本次关联检出自己的 state。19 项定向请求为 `.chrono-harness/state/retention-migration/migration-retryreq-20261117.json`；当前 policy 为 `39c88afc330a2542ec0b768e1362fc2e21d61d75be6ca7cc7629b61ce9a5433f`，旧 policy 为 `c56d8a9449df752c5c095c9b0ad66962d8d0b165a901c3b5a4fda2c8937d3321`。原迁移回执 `retry-19-targeted-v2.stdout` 报告 `completion:not-established`、`effects:metadata-binding-only`、`outputs_preserved:142`、`roots_preserved:63`；这些数量包含实际访问的依赖，不是请求的原件数。2026-11-17 后续逐项观察该请求：17 项为 `output:null`，另有一项 `disposing`、一项 `sealing`，共 19 项。`output:null` 只表示正文输出已退休，不证明对象地址和全部保管元数据已清除。迁移由程序写入政策绑定及适用的期限；本次没有手工改写声明，也没有由迁移删除原文件、解除恢复根或补写业务成功。

两个未完成对象的原始记录如下：

- `output-84690-1794738198375302000-1` 处于 `disposing`，原报告缺失、清单封存且为空，仍有 51 条待解除的向外依赖，入向引用和角色均为空。这是未完成的元数据退休，不能写成有外部读者保护。现存记录已有 `lifetime.created_at:1794739181`、`deadline:1794825581`，`expires` 等于该期限；重复迁移须保留它们。
- `output-73145-1794735628774966000-4` 处于 `sealing`，原件缺失，业务完成仍为 unknown；该阶段只说明待续接封存，不能作终态成功。记录仍有 worktree enrollment 角色 `worktree-enrollment:fcc16ca706d6390b0c1cfe02ffa11b2383d35c9f3758290739750e06d5de637e`。已有使用方观察对应 `/Users/macstudio2/chrono-spec-current-dev`、分支 `feature/ci-generation-state-20261113`、HEAD `1412c62f2818546c2a833d2937c4b6d2aa276ea1`，下一动作为 `complete-work-and-finish`。现存 `created_at:1794649228`、`deadline:1794735628`，没有因本次观察刷新。领域负责方尚未交接，通用迁移不能代为释放；这仍是有限窗口目标的缺口。

候选 runner 的已有验证原件为 `.chrono-harness/state/retention-migration/runner-tests-current-20261117-v2.*` 和 `runner-cargo-verus-current-20261117-v2.*`。前者退出码为 `0`，16 个测试进程合计 761 项通过、0 失败、1 忽略；三项新增回归覆盖缺失原件迁移、未知恢复根与活读者保护，以及缺失非退休阶段／替换原件的拒绝。它是行为测试结果，不是形式化证明或宿主 canonical check。后者退出码为 `0`，Verus `0.2026.04.24.f8e1704` 报告 `verified:42`、`errors:0`，但实际证明缓存 `.chrono-verus-proof.json` 绑定的是 `chrono_identifiers` 依赖，不能据此声称 retention 或新增 IO／可选出生时间逻辑已获证明。保留期限谓词的登记证明入口为 `verify.retention-release`；完整清理进度、文件系统及消费者交接仍须独立验证。

正式宿主采用仍被阻断。把 `runner-build.json` 当作 bootstrap profile 被 `host-bootstrap` 拒绝（`unknown field binary`）；`bootstrap-core.json` 直接运行报告 `owner stage original report differs`；旧已部署 `chrono-worktree` 无法解析 `max_lifetime_seconds`；当前候选 bootstrap 又被 active root 引用过期输出阻断（典型错误 `output creation deadline closed: output-32354-1794196581936968000-4`），维护 socket 也曾连接拒绝。伪造 `CHRONO_PROCESS_FDS` 得到 `invalid or duplicate ownership descriptor`，因此没有绕过 owner 保护。上述原 stdout／stderr／退出码均保存在 `.chrono-harness/state/retention-migration/`，当前不能声称候选已经部署。

从当前新分支执行宿主登记的 `.chrono-harness/bin/chrono-harness check` 也以退出码 `2` 失败：旧参与程序在读取已采用 policy 时拒绝 `max_lifetime_seconds`（`unknown field max_lifetime_seconds`），并报告无法保留原 participation。该失败原文的 stdout／stderr 摘要由程序输出，不能把本地文档变更误报为通过；候选 runner 的局部测试与 Verus 结果仍只覆盖上段所列源码候选。

正式维护的最近已读原件仍不是完成证据：`maintain-after-19-migration-20261117.stdout` 报告 `scope.complete:false`、`accounting_errors:5`、`disposed:80`、`removed_logical_bytes:1942179`、`physical_bytes_reclaimed:null`。上述两项分别有未解依赖和未交接领域角色；不能都归因为活读者。未知、损坏、过期引用和报告交接未完成时继续保留。必须先完成 owner adoption、解决真实使用方交接，再用同一候选程序连续取得至少三轮真实定时完成结果，并核对报告元数据、实际物理回收、恢复和同范围占用，才可重新评估全称目标。

## 政策输入副本的后续修复与验证（2026-11-17）

协调宿主的真实 `migrate` 曾因 `output-66572-1794263173320876000-2` 超期退出 2；
该对象是固定 Git 政策字节的临时副本。候选
`a60c3681e6300819176654e95d32d7cb1073ff77` 改用 Git 持久保存的原输入，
新迁移回执只记录提交、路径、存在性、摘要及长度，不再复制或新引用旧副本。
旧登记仍消费的迁移回执和输入副本继续完整校验；本次没有刷新期限、手改声明或补写完成。

该候选的成功干净 bootstrap 原件在
`/Users/macstudio2/ec91280d/expired-policy-dependencies-feature-20261117/.chrono-harness/state/bootstrap-77059-1794923142515563000-0/report.json`。
同根的正式检查原件为
`.chrono-harness/state/preparation/check-3948d01986583fb3f25bdcced8a0ca7241d9b077936a534c88070959be555129.json`，
基线 `a9e5f98a17dc6455dec5a8dc8977ae5595d4afa4`，实际执行 87 项登记操作，退出 0，
原响应 passed；265 份报告、两流及启动引用的实际字节和摘要已核。
`test.worktree.core` 的 447 项通过，包含超期后再次改变政策的新增回归。
登记证明 `verify.retention-release` 和 `verify.retention-release-tests` 分别报告
`verified:122`、`verified:177`，均为 `errors:0`；它们覆盖所选保留判断及边界，
不证明全部文件系统事务、真实消费者转存、维护公平性或全仓最终回收。

另外已找回当前安装所需的不可变 stage 原件：协调宿主的
`.chrono-harness/state/bootstrap-98102-1794405976516615000-0/report.json`，
摘要 `49c0ab61a1922d0467f957938204f9e5dee611dbad69ed3d1791ab21df9926f5`。
它与原 adoption 的摘要、stage 及干净来源 `c827dad6827a50d142803331854507ee6a211faa`
匹配，两项操作退出 0，原输出流已核。adoption 仍指向被后来构建覆盖的可变
`bootstrap-owner-stage.json`；找回原件不等于正式解析路径已经修复或新程序已经部署。
上述代码已通过两段真实 PR：[#1359](https://github.com/ChronoAIProject/chrono-harness/pull/1359)
合入 integration，[#1360](https://github.com/ChronoAIProject/chrono-harness/pull/1360)
合入 dev，落地提交为 `758c209a27c579141a1409495d9255fe124423df`。
两段 CI 均成功；落地树与受验候选的树 `eb4ac11c625b19a443de15e550957e8ca17fdf17`
相同，dev push 检查仍在运行，不能预报其结果。

使用该候选成功 bootstrap 的已核程序定向重试协调宿主 `migrate`，实际退出 0，
原件 `.chrono-harness/state/worktrees/migrate-84434-1794926836581249000-0.json`
摘要为 `dd0fb4a58d4a5f71801ff1235d4efa84cef9e53f3f3893a7fa87f7c299d27a85`，
stdout 与存盘原件一致。旧 10 项迁移链已由程序核验并交接为当前政策绑定，
`policy_migrations` 已退休；原超期输入的完整输出声明保持不变。
迁移未执行删除，也未声明任务终态。OS 采用和三次连续定时回收仍未完成。

另一次协调宿主元数据的只读观察，在 Unix 时间 `1794925516.182251` 至
`1794925517.515738` 间读取到 5,139 份消费者角色声明，其中 4,173 份的 `ids` 为空。
这是约 1.33 秒的非原子盘点；空成员不等于已过元数据期限或可以立即删除。
另有 3 个角色地址缺少 `record.json`：
`cbc04ee2903c7579d49d56c60ccf87290c20eb3df3dda9cc097d22b6b4d8c082`
保留了 275 字节完整 `record.pending`，原角色为
`recovery:.chrono-harness/state/worktrees/producer-finish-73510-1794472297849584000-54.json`，
`ids`／`attached` 为空，原元数据期限为 `1794558698`；
`bb52c4f8b16fae30b62caf6f2014c3addbc53943763b60e3b90405a5537ddbdc`
只剩零字节 pending；`3f0635da9cef4a0d6fff1c64276f51fcb2af9e6c6c815bda25a288fe10a0d803`
为空目录。现役 `prune_role` 只接受正式记录，因而继续报
`unattached consumer birth; preserve metadata`。完整 pending 的恢复与后两项未知状态
须分别处理；本次未补造正式记录、删目录或把这些错误改为已回收。

Unix 时间 `1794925983` 的另一次只读观察发现 303 个非空 `consumer:` 角色，
310 条成员引用所指声明均已超过创建期限；引用未按输出去重，不证明仍有真实消费者。
按角色名称分组为 bootstrap 40 个、lifecycle 254 个、disk 6 个、maintenance 1 个、
launchd 1 个、native 1 个。`lifecycle-recheck` 的生产代码已在
`1218464d46769b005bfac1f48b23d2f7bee83e9b` 被有限诊断窗口替代，该改动已在当前 dev；
它不再新建这个永久消费者，但已有角色仍在。为限制本次读取成本，只解码不超过
1 MiB 的普通原文件，其中 152 个均报告 failed；另 102 个未解码，结果保持未知。
后续已完整解码全部 254 份原件，均为 failed，正文合计 925,856,103 字节；
这补齐了上次 102 份未知的状态观察，不证明原调试、恢复及验收义务已经结束。
仍需由原生命周期所有者核原报告、实际完成和当前需求后交接这些旧角色；代码已退役
和到期本身不授予直接删除原件的资格，其余消费者也须沿各自实际使用链核实。

### 完整空声明的真实恢复与维护剩余项

固定候选 `7f3d2a620ef5fcf235388aa1ba3e45e4200472ff` 的显式维护原件为协调宿主的
`.chrono-harness/state/worktrees/maintain-88387-1794949637766914000-0.json`，
SHA-256 `8884b16201c6ce4a6c51f58f238f49083bccdbf3c3730243fb11c9992066f774`，
80,830,573 字节，原退出码 2、status failed、全部自有工作已等待。
本地正式检查已通过 135 项登记操作，但这不覆盖本轮发现的全部旧状态问题。

本轮沿原所有者恢复上文 `cbc04ee…` 的完整初始空 pending，保留其原期限，随后退休该
角色地址；完整清单记录了 5 个节点、6,700 字节 metadata 观察和 `verified-absent`，
结束后精确地址确实缺失。零字节 `bb52c4f8…` 和空目录 `3f0635da…` 仍存在并报告未知，
没有补造记录或删除它们。该恢复只适用于严格验证过的初始空声明。

已逐份核验 232 份直接清理子原件的摘要、输入长度和状态，以及共 14,188 个不同进程的
28,376 条原流引用；实际 Git 非零观察仍保留，不把它们改成成功。
完整清单为同名 `.json.inventory.jsonl`，184,001,903 字节、163,055 行，
SHA-256 `0f746139f6816fe170d1505b39d474b5a67f88560b08e72569b4b1bae911c0c5`，
已全流解码并核对字节、摘要和行数。它含 34 条 enumeration-error、22 条
unknown-declarations；`measurements_complete` 和 `member_enumeration_complete` 均为 false。

直接 drain 观察为 cleaned 221、failed 28、preserved 12，另有 5 个 producer drain cleaned。
产物子动作中 20 项报告 `verified-absent`，结束后 20 个精确路径均缺失；这些是两个工作树的
编译／工具缓存，不代表两个任务已经完成。主保留库另外报告 1,499 次处置观察、
1,899,008,289 字节逻辑删除，含正文及元数据；不是不同原件数，物理释放量仍为 null。
同方法 `du -sk` 结束观察中，`retention-state-handoff-feature-20261117` 为 250,928 KiB，
`terminal-receipt-handoff-feature-20261118` 为 231,732 KiB；后者在此前手动清理后为
1,700,956 KiB，两窗口差为 1,469,224 KiB。并发写入和不同测量窗口仍限制其归因。

本轮仍有 31 条 cleanup_failures，包括未封存的旧终态原件、不能代表完成的缺席采用回执、
未接收的清理结果、旧项目格式、身份／锁变化及未合并索引。两个旧
`retention-tree-progress` 登记各保留 57 次尝试，最后 receipt 仍为 null；对应原报告实际存在，
错误是 `retained commit does not preserve the exact worktree tree`。
当前代码没有恢复这类封存结果的账本交接，且会把已有完整历史原件继续加入后续报告依赖。
这是真实消费责任和 O8 的剩余缺口，不能用本轮局部回收或退出状态替它关闭。
本轮是显式调用；真实 OS 定时采用和三次连续成功定时维护仍未完成。

### 导入政策的持久使用方交接

后续候选修复 `worktree-enrollment` 对临时导入报告的永久政策依赖。实际使用方是
维护时恢复已采用政策的 `restore_adopted_policy`，需要调用方的固定 Git 提交、配置摘要
及政策摘要；目标原提交中的输入仍是独立的原始观察。现有 `adopted_policy.policy_source`
足以保存这些后续必要事实，不需要复制政策文件或归档整份报告。新导入在登记时核实
并保存来源；旧导入完整核原件、输入、Git 字节、namespace 和登记后，沿已有
`save_result_handoff` 先持久保存，再由 `protect_originals` 缩减原引用。

固定旧实现 `e4b3e4fcf92167f87e596787552e9b64474dcc64` 的真实登记测试在
`.chrono-harness/state/worktrees/use-5691-1794929503059147000-0.json`
保留失败原件（协调主库，SHA256
`7bff951cf5c2aae635e0974673541016801b40ca181df5bf7b7421ce83028c42`）。
新回归的有限窗口在创建时采用，已有领域引用及独立读取保护；旧实现仍以
`output creation deadline closed` 拒绝维护。此前一次 fixture 政策变更拒绝和
缺少安装工具的失败属于准备问题，不作为这个行为反例。

局部判法复用同源 Verus 的 `imported_policy_required`：已有当前政策绑定的 import
不再需要原导入报告。释放旧引用前须先验证绑定格式及其与迁移链的互斥；字段存在
不能单独证明交接有效。超期原件的所有嵌套读取也须沿同一个已有领域使用方完成。
实际写入仍须先完成全部核验，并由现有原子持久交接及缩减检查
保全中断边界；这个布尔分类证明不覆盖文件系统原子性、所有读者终结或维护公平性。
适用检查、真实部署与超期原件退休须分别验收；局部通过不关闭 O6 或全链目标。

2026-11-18 补充回归的固定候选 `ea5c7252` 通过登记的 `test.worktree-tests`
实际观察到 451 项通过、2 项失败：超期 import 仍被嵌套普通读取拒绝；损坏的
当前绑定在报错前已把旧保留角色清空。协调宿主原件为
`.chrono-harness/state/worktrees/use-10713-1794931718866290000-0.json`，SHA256
`f6b88ffb7eba54182c62dd0149189297c1a21d9be59a7fa430eaaf9a61107a59`。
原 Cargo 退出 101，`failure:null`、`owned_work_joined:true`，两流完整且长度及摘要已核。
完整清单和满容量结果交接的预期已按新语义迁移，保留其全部成员、失败保存与独立读者要求。

## 2026-11-18 的消费与定期维护观察

正式版 v0.1.3 的晋升回执记录测试版运行 `43317169856`，实际读取发布原件
`13990766095`、Linux 包 `13990910236`、macOS 包 `13989233388`，共
124,137,330 字节。正式版 v0.1.4 的回执也记录独立的测试版运行 `43416115837`
及两个平台原件。这两条链以 Actions 的精确 artifact ID、原始 attempt 和字节为输入。
当前可下载的公开版本承担发行文件的持久保存。晋升还需要可核验的原发布和平台回执，
须由发布负责方接收并迁移实际读取入口，之后才可将这一类临时工件的窗口收至 24 小时。
[v0.1.3 晋升运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/43326911672) · [v0.1.4 晋升运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/43422705025)

本机 launchd 进程 `59544` 的首轮由 OS 激活，下一轮由事件触发。两轮真实原件的
成员枚举和占用清单都报告 incomplete，各有 31 条失败。首轮有 228 项处置记录为
`cleaned`，事件轮有 230 项。核对了两轮原始清单的摘要、行数和引用的原报告。
事件轮还记录同一旧部署 bootstrap 目录已开始有界处置，其 stage 分配本轮消耗完
处置预算时尚未执行。两个目录仍可观察到，连续成功的定时轮次尚为零。

同一服务对已换代的 11 月 15 日部署原件完成消费者释放。维护程序及必要构建原件已
写入当前用户的持久应用目录，并逐份复核本地副本的长度、摘要与文件身份。launchd
仍从原临时检出读取。实际换址、剩余消费者释放及目录和元数据的到期回收各有独立的验收结果。

## 2026-11-19 的到期实物、定时维护和消费观测

协调主库在 UTC 2026-11-18 21:41:15.684 至 21:41:16.420 枚举了 14,631 个对象地址，读取 21,046,207 字节声明。逐件按实际生产者使用的设备号、inode 和文件类型核对路径，得到以下非原子读数。一次读取期间地址消失，该项状态未知。

| 观察对象 | 数量或逻辑字节 |
| --- | ---: |
| 仍声明正文的对象 | 10,373 |
| 已过创建期限的正文声明 | 6,450 |
| 到期且仍能核实原文件或目录身份 | 6,447 |
| 上述对象中有入向引用或保留角色 | 6,437 |
| 上述对象中两者均为空 | 10 |
| 仅剩元数据的声明 | 4,257 |

仍在的 6,447 项分别属于生命周期原件 6,430 项、检查原件 15 项、bootstrap 所有者 stage 2 项。其中普通文件的逻辑长度按声明相加为 6,841,860,548 字节，目录只核对身份，物理释放量未知。可复核的例子是 `output-62678-1793866053361016000-1`，声明处于 `disposing`，期限为 Unix `1793952453`，本次仍能通过文件身份核对。6,437 项的登记关系只表明有程序引用；真实使用须继续追到读者。余下 10 项也须由所有者核租约、原件身份和清理进度。

在另一个观测窗口（UTC 21:43:37.666 至 21:43:40.502），从 1,013 个非空角色的 `ids` 出发，沿 10,447 条输出引用计算可达集合。这次有 6,465 个到期实物，其中 6,179 个从至少一个角色可达，286 个在已读角色的闭包以外。恢复、工作树登记、显式使用方和当前报告四类角色分别可达 2,410、2,086、2,157、20 项，集合有交叉。一个对象地址和两个角色地址缺少可读取的正式声明，内核活读者另有独立保护。这两次观测的起止时间和统计口径不同。

### 清理自身产生的文件

同一 launchd 进程 59544 的三份相邻定时轮次原件均报告 `failed`、31 条失败和 `owned_work_joined: true`。表内原件位于协调主库 `.chrono-harness/state/worktrees/`。逐字节核实每份 `.inventory.jsonl` 的 SHA-256、长度、行数和每条可读取的直接清理结果引用。

| 原件 | 清单字节／行数 | 只观察到“无产物可处置”的独立子报告 | 这些报告的逻辑字节 |
| --- | ---: | ---: | ---: |
| `maintain-59544-1795026954007663000-3533.json` | 163,251,216／142,118 | 242 | 32,972,116 |
| `maintain-59544-1795029898796972000-4069.json` | 164,126,753／143,258 | 245 | 33,277,314 |
| `maintain-59544-1795033100128323000-4602.json` | 165,694,176／144,796 | 247 | 33,465,206 |

三轮共有 242 个相同工作树得到上述观察。直接读者是当轮维护汇总，它读取子报告并保存其长度、摘要和引用。三个主报告分别有 263、263、265 个向外依赖，本次各自的入向引用和保留角色均为空；每份 JSONL 清单只有其主报告这一个入向引用。在这个样本中，主报告、清单的创建期限都是 86,400 秒，正文和元数据退休尚待到期后逐件验收。需要保留的事实是当轮所检查的目标身份、政策、无处置原因及真实错误。只有上次已完成、再次核实时条件仍相同的观察，才可直接记入当轮汇总。

### 独立持久转存的真实读取

`dev` 的固定提交 `b486e5e541b423cd656d070e7d454ef51fea212e` 的启动工件 `14464666858` 将于 UTC 2026-11-19 19:39:44 到期。其 macOS arm64 维护程序为 9,468,240 字节，SHA-256 为 `443755032672731a4cceea905b6e06dd60fa0e4ab82bd22c5d807b59ec5634d7`。在窗口内，宿主把程序和实际所需的 20 份来源、构建报告和进程流（232,787 字节）写入当前用户 Application Support 的版本目录，重读核对每项摘要。从该目录执行的 `status` 和 `start` 分别留下 `status-72125-1795037688582595000-0.json` 和 `start-73556-1795037825820998000-0.json` 的真实业务结果。launchd 仍运行旧临时检出的程序。持久安装的接管、临时产物实际不可读取、旧使用方释放及三轮成功定时维护仍需独立验收。[已核对的落地运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/44381529770)

## 达成目标还需要什么

先处理 O1、O2、O5、O9 的真实覆盖缺口；O9 要由目录的实际分配者明确所有权和退休，不能从文件删除推断整条目录路径已回收。再由各负责方结清 O3、O6 的已结束使用，不能跳过未知和失败义务。O4 若继续要求所有临时文件按时间到期，须在原日志所有者实现并验证。O7 要补实际状态机的进度及公平性证据；O8 要验证清理自身的报告和元数据不形成无限保留链。外部 provider 和下游须给出各自有效保留／释放合同与实际采用证据。

最终验收必须包括：固定候选全部受影响的登记检查、成功／失败／边界／中断行为、实际消费者读取及交接、同范围占用前后测量，以及至少三次连续真实定时完成轮次。上述任一依赖仍未知，只暂停该依赖的完成结论；不能把静态清单齐全、退出 0、event accepted 或局部 Verus 证明写成目标已完成。
