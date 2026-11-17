# 程序输出、使用方与滚动清理审计

本次审计的目标是：每项输出都有实际用途和负责方；使用结束后，可再生成的文件及其保管元数据能在有限时间内回收，不能让报告、失败现场或清理本身持续积累垃圾。使用方是本仓维护者、CI／发布调用方和安装本产品的宿主。

**当前结论：不能证明所有最终产物都能按时间滚动清理。** 有确定的覆盖缺口和反例，也缺少维护进度、外部交接的证明。当前仍有用途的源码、安装程序、固定报告和公开发行资产不能直接当作垃圾。下文区分有界存储、到期资格、实际删除和仍未知的使用方。

2026-11-17 的需求补充明确：所有临时产物都有有限窗口，有价值的内容应在窗口内
转存到其他持久存储；消费者、当前指针和恢复角色不能无限延长临时原件寿命。
规范性定义归 [SPEC §10.1](../SPEC.md#101-报告证据与临时产物的自动清理目标合同)。
本文件的固定基线反例和有条件定理仍是审计事实；其中“所有根最终释放”尚未由程序保证，
不能作为免除到期义务的前提。实现须进一步证明窗口不可续长、转存真实完成、消费者改读
持久地址、临时正文／目录／元数据最终退休，以及维护可公平推进。当前尚未达到这一目标。

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

O9 给出独立的目录反例：调用方在此前不存在的 `state/output-N/` 中发布一个文件，
随后释放该文件的全部使用方，维护删除文件及其元数据。令 `d` 为此次新建的父目录；
在继续运行现役 maintain、没有其他目录所有者接管的轨迹中，`d` 不在文件声明里，
没有针对它的 Delete 转移，故仍有 `∀t≥t₀.P(d,t)`。这个反例不依赖文件回收失败；
重复分配不同父目录可增加目录数。固定目录复用的有界结论与此不冲突。

可以陈述一个**有前提的回收定理 L**。对有限的一批对象，若存在有限 `T₀`，使此后每项都满足：

1. 已纳管、身份稳定，拥有有限的交接／中断保留期；所有真实使用方及根最终持久释放，不重新引用该代际。
2. 真实维护机会无限继续，已合格对象不会饥饿；每个所需文件／声明 IO 最终可成功，不永远卡在预算准入之前。
3. 存在自然数进度量 `r`，包含剩余清单节点、依赖脱离、删除和元数据退休步骤。无动作不增加 `r`；每个有效步骤使 `r` 至少减少 1；中断恢复从已保存的状态继续。
4. 删除正文后仍会触发有限元数据期限，且相同公平推进条件继续适用于元数据；新维护报告属于新代际，不反向重新保护已释放的这一批。

则每项正文及元数据最终删除。

证明：在 `T₀` 后对这批对象的进度量求和 `R=Σr(o)`，`R` 是有限自然数。只要 `R>0`，公平性及最终可成功 IO 给出有限时间内的下一有效步骤，且 `R` 严格下降。自然数不存在无限严格下降链，至多 `R(T₀)` 个有效步骤后所有正文删除。有限元数据期限过后，对元数据步骤应用同一论证，得到 `¬M`。这证明 L；它没有证明程序已经满足 L 的全部前提。

要进一步给出墙钟上界，还须有每个有效步骤最大等待 `D` 和维护／IO 完成上界，才能写出类似 `T_delete≤T₀+D·R(T₀)` 的界。当前睡眠／注销、长轮次、活使用方、失败、预算停顿均没有这样的已验上界，不能承诺“每小时清完”。

“每个对象最终删除”也不推出“总占用有界”：若第 n 个对象存活 n 秒且每秒新增一个，所有对象最终删除，总量仍能持续增长。总存储界另需生产速率上界、有效回收延迟上界和活使用容量界；当前这些量未知。50 MiB 的正常日志家族容量是独立的局部界。

现有 Verus 根证明局部选择、轮转、游标交接、引用判断等，见 [形式证明范围](verus.md)。本文件没有新增或执行 Verus 证明，不把上述数学定理、静态审阅或既有局部证明报告成全仓机器验证。

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

同日还只读核对四个示例仓库的固定 dev。它们的 `.chrono-harness/distribution.json`
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
其余资产的安装后用途、另外三个示例的原安装、其他外部用户和全部最终释放仍为 unknown。
本次没有重跑下游 CI 或改变其采用版本；后续版本不能沿用这份固定采用结论。

本次源码检出开始时没有构建输出，`du -sk .` 为 14,116 KiB；卷可用空间为 178,987,352 KiB。主库 bin／state／cache 的另一次计量没有收取到完整退出和输出，保持 unknown，未将它当成零或空间收益。本次没有运行新的全库维护，也没有声称已完成三次定时验收。

## 当前候选迁移与验证（2026-11-17）

本次将当前 policy `39c88afc330a2542ec0b768e1362fc2e21d61d75be6ca7cc7629b61ce9a5433f` 与旧 policy `c56d8a9449df752c5c095c9b0ad66962d8d0b165a901c3b5a4fda2c8937d3321` 做了 19 个具名原件的定向迁移。程序实际报告 `completion:not-established`、`effects:metadata-binding-only`、`outputs_preserved:142`、`roots_preserved:63`；18 个对象已经进入可退休边界，两个对象仍受真实关系保护。迁移不删除文件、不改写 role／ledger／record，也不把缺失原件变成新的读者准入。

两个未完成对象的原始记录如下：

- `output-84690-1794738198375302000-1` 的终态为 `disposing`，原报告已经缺失，但仍有报告／引用关系。记录保留原有 `expires`，不会用当前时间重建 `created_at`，也不会因物理原件缺失删除记录。
- `output-73145-1794735628774966000-4` 的终态为 `sealing`，原件缺失，且仍有真实 active worktree enrollment 角色 `worktree-enrollment:fcc16ca706d6390b0c1cfe02ffa11b2383d35c9f3758290739750e06d5de637e`。该使用方对应 `/Users/macstudio2/chrono-spec-current-dev`、分支 `feature/ci-generation-state-20261113`、HEAD `1412c62f2818546c2a833d2937c4b6d2aa276ea1`，下一动作是 `complete-work-and-finish`。在该使用方交接前不能退休。

候选 runner 的验证原件为 `.chrono-harness/state/retention-migration/runner-tests-current-20261117-v2.*` 和 `runner-cargo-verus-current-20261117-v2.*`。登记的 runner 测试退出码为 `0`，其中一个测试进程报告 `202 passed`，另一个报告 `75 passed`；直接执行登记 Cargo-Verus 参数的退出码为 `0`，输出报告 Verus `0.2026.04.24.f8e1704`、`verified:42`、`errors:0`。这些结果证明缺失终态迁移、重复迁移、活引用保护和相应局部 Verus 条件已通过候选检查，不证明全仓输出已经纳管或宿主已采用候选。

正式宿主采用仍被阻断。把 `runner-build.json` 当作 bootstrap profile 被 `host-bootstrap` 拒绝（`unknown field binary`）；`bootstrap-core.json` 直接运行报告 `owner stage original report differs`；旧已部署 `chrono-worktree` 无法解析 `max_lifetime_seconds`；当前候选 bootstrap 又被 active root 引用过期输出阻断（典型错误 `output creation deadline closed: output-32354-1794196581936968000-4`），维护 socket 也曾连接拒绝。伪造 `CHRONO_PROCESS_FDS` 得到 `invalid or duplicate ownership descriptor`，因此没有绕过 owner 保护。上述原 stdout／stderr／退出码均保存在 `.chrono-harness/state/retention-migration/`，当前不能声称候选已经部署。

从当前新分支执行宿主登记的 `.chrono-harness/bin/chrono-harness check` 也以退出码 `2` 失败：旧参与程序在读取已采用 policy 时拒绝 `max_lifetime_seconds`（`unknown field max_lifetime_seconds`），并报告无法保留原 participation。该失败原文的 stdout／stderr 摘要由程序输出，不能把本地文档变更误报为通过；候选 runner 的局部测试与 Verus 结果仍只覆盖上段所列源码候选。

正式维护的最近结果仍不是完成证据：迁移后维护原件报告 `scope.complete:false`、`accounting_errors:5`、`disposed:80`、`removed_logical_bytes:1942179`、`physical_bytes_reclaimed:null`。两个未完成对象和 active consumer 仍由维护器保护；未知、损坏、过期引用和报告交接未完成时继续保留。必须先完成 owner adoption、解决真实使用方交接，再用同一候选程序连续取得至少三轮真实定时完成结果，并核对报告元数据、实际物理回收、恢复和同范围占用，才可重新评估全称目标。

## 达成目标还需要什么

先处理 O1、O2、O5、O9 的真实覆盖缺口；O9 要由目录的实际分配者明确所有权和退休，不能从文件删除推断整条目录路径已回收。再由各负责方结清 O3、O6 的已结束使用，不能跳过未知和失败义务。O4 若继续要求所有临时文件按时间到期，须在原日志所有者实现并验证。O7 要补实际状态机的进度及公平性证据；O8 要验证清理自身的报告和元数据不形成无限保留链。外部 provider 和下游须给出各自有效保留／释放合同与实际采用证据。

最终验收必须包括：固定候选全部受影响的登记检查、成功／失败／边界／中断行为、实际消费者读取及交接、同范围占用前后测量，以及至少三次连续真实定时完成轮次。上述任一依赖仍未知，只暂停该依赖的完成结论；不能把静态清单齐全、退出 0、event accepted 或局部 Verus 证明写成目标已完成。
