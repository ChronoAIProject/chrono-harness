# 统一维护进程：独立预期与采用边界

状态：本地联合验证阶段；已运行候选 bootstrap 和正式生成，测试、证明与 canonical 以本轮原始报告为准，尚未采用真实服务。既有完整清单源来自固定 f6ad4ac6cb3fd1f976d9c794a1ad58de9ee7dfe3，原验收义务不变。

**产品与宿主政策一起修改。** 普通入口由同步清理改为现有所有者持久交接后发送事件；维护由每个物理协调者的单一执行进程承担。政策、程序和 OS 实例必须一同采用；指南通过正式入口生成，实际部署由调用方负责。本文件的独立预期先于实现提交，源码阶段未观察 red 或 green。

## 实现前的独立预期

| 输入或边界 | 成功预期 | 可证伪失败 |
| --- | --- | --- |
| 合法 main 与另一个已登记目标；无关处置或完整 footprint IO 被阻塞 | 两者都到达原业务终点，原 argv、stdin、stdout、stderr、退出保留；维护另报 | 等待扫描、返回 busy／timeout／retry 或改写业务退出 |
| start／reconstruct／use／check／bootstrap／finish 和 producer finish／release | 先保存现有 birth、使用、代际、终态或生产者状态；业务返回只需要自身封存 | 事件收据冒充清理完成；异步跳过必要封存或同对象保护 |
| 无执行进程的事件 | 由 OS 已采用 socket 激活同一服务；发送者不等待启动或维护 | 自行脱离子进程、等待 ACK、丢失原恢复责任 |
| event／timer／explicit 竞争 | 同一物理协调者只有一个实际执行者；其他显式请求准确报告已请求 | 两个实际扫描／处置者；维护锁成为业务准入锁 |
| 重复／合并事件、轮次中新代际 | 下一次实际轮次读取原权威状态；已处理事件不表示对象已清 | 以事件身份替代对象代际；删除新代际／活使用方 |
| 轮次结束后仍有 busy／保护／延迟对象 | 保留原因；等待下一事件或定时机会，正常达到 idle 退出 | 内部发布或保留对象自己制造无限空轮次 |
| idle 决策／退出交界到达事件 | OS 仍拥有激活 socket；后续服务读取事件并恢复 | 退出 unlink 掉新事件；把无事件当业务完成 |
| 中断／重新启动 | 原件、原账本和生产者责任仍在，启动轮次继续原恢复 | 释放未知使用方、删除未知锁、伪造原结果 |
| socket 缺失／满／IO 错误 | 保留原责任，单独报告 wake failed；业务原结果不变 | 声称 OS 已接受或以 wake 失败覆盖业务结果 |
| 每个实际轮次 | 沿现有完整 JSONL 报告记录完整成员、窗口、错误、真实动作和原回执；父报告接住恢复引用 | ACK 当作完成；topN／64 KiB 抽样替代成员；报告 IO 失败被隐藏 |
| 退出 | 所有本进程启动的工作均已同步加入；idle 不持 enrollment metadata | 后台工作未加入就退出或发布最终成功 |
| OS 保留的激活 socket 位于已登记 state | 完整 footprint 通过 lstat 测出条目并单列 socket 数；实际处置仍拒绝 special file | 每轮因自身 socket 固定失败，或观察能力反过来授予 socket 删除权 |

## 判定与登记

Rust 行为预期归现有 worktree-tests 与 runner-tests；所有具体外部执行边界用既有 Rust mock，不真实调用 OS 命令、Git 或编译器。idle 退出的布尔判定放入现有 verified_lifecycle 与其独立 Verus 预期，正式入口复用 verify.worktree-lifecycle／verify.worktree-lifecycle-tests；这只证明该判定，不覆盖 IPC、IO、锁或完整进程。FILEMAP 登记每个新增源、测试、数据和依赖，成本未知。后续按固定基线与候选两端 DELTA 运行登记的 bootstrap、generation、证明、专属测试和 canonical；实际结果由各阶段原报告负责。

## 工程选择与真实采用

复用 Unix datagram 和 OS socket activation：OS 持有 socket，缺席时启动维护进程；业务只发送非阻塞提示，不启动子进程，不等待回应。提示没有处置权，也不建立任务账本。负责恢复和删除的权威仍是现有 enrollment、producer 和 retention 状态。显式维护与服务使用同一内核 singleton 租约，业务不取得它。维护逐地址取得必要 metadata，仅同地址效果期间排除；完整 footprint、producer 和 retention IO 不保留 main 的排他。

宿主提出 idle 300 秒、定时 3600 秒；idle=null 代表常驻。定时由服务自身的阻塞等待期限支持常驻，OS 定时在服务缺席时启动；不提高现有节点、25 ms、FD、线程、stdout、报告或其他预算。定时错过／延迟和启动原因须原样报告，不从事件 ACK 推断完成。使用 macOS launchd；Linux 通过 systemd socket activation 的 LISTEN_FDS 交接，未采用 OS 配置时 wake 可失败且保持原责任。

后续真实采用必须排除旧程序、保全原状态，采用固定已交付程序、相容政策和 OS socket／timer 配置；核缺席事件启动、常驻唤醒／定时、idle 退出及再启动、单实例和未完成维护期间合法 main／linked 业务进展。调用方预先登记至少三次连续实际定时完成观察，包含错过机会、原回执、至少一个释放且到期对象的实际回收、中断恢复和保护。无合格对象时相应结论仍未建立。

原库存 300 地址／>64 KiB、完整 producer／retention 成员、IO／unknown、父恢复引用和有限保留成本全部保留。固定输入／范围／方法／单位／窗口比较增量 IO、FD、耗时、留存字节和实际占用；这些值目前 unknown。139、ordinary-complete 同输入 EMFILE、absent-terminal／身份与政策来源恢复等原义务各自继续，不由此机制替代，也不重置原额度。

## 负责文件、实际使用端和差额

| 负责源码／政策 | 输入与当前使用方 | 必要输出／现有能力为何不足 | 成本与判法 |
| --- | --- | --- | --- |
| `crates/worktree/src/automatic/service.rs` | 原协调政策、enrollment、producer、retention；显式 maintain 与 OS 服务 | 共用原轮次／恢复和独立 singleton；旧定时单次命令不具备事件驻留与 idle 退出 | 原预算不变；逐地址锁、事件／timer／退出和真实业务边界 Rust/mock；IO、FD、耗时 unknown |
| `crates/runner/src/maintenance_signal.rs` | 已采用维护政策与既有所有者的完成／释放；worktree、runner retention 与 producer | 非阻塞 OS 提示及独立实际收据；无需任务表、ACK 或子进程 | 一个临时 socket FD 和小政策读取；缺失／满队列／线程抑制预期未运行；成本 unknown |
| `crates/worktree/src/automatic.rs` | 原 birth／use／finish 和身份／代际；现役生命周期调用者 | 保存原责任，移除同步 drain；必要同对象排除和原件封存保留 | 现役生命周期证明与原 Rust/mock 断言迁移；收益、成本 unknown |
| `crates/worktree/src/start.rs` | 原报告和父消费者；start／reconstruct／producer finish 及显式维护 | 原发布完成后唤醒；竞争的 request 与实际 maintained 分开 | 不另设完成权威；原出版／恢复预期待验 |
| `crates/worktree/src/automatic/inventory.rs` | f6ad 完整声明／原回执；每个实际轮次 | 原 JSONL、非原子时间窗及快照；完整 footprint 在 metadata 排除外 | 原 300 地址、>64 KiB、IO、unknown、父恢复引用与留存成本全部待验 |
| `crates/runner/src/artifact_disposal.rs` | 原 footprint 遍历与登记 state 中的 OS socket；完整库存调用方 | 同一遍历在观察模式只 lstat socket 并单列数量；处置模式仍拒绝所有 special file | 不重复遍历或放宽处置；原字节／分配块口径及 strict／observed 边界 Rust 预期未验 |
| `crates/worktree/src/automatic/retirement.rs` | 原退休声明和逐对象排除；统一维护与同地址负责方 | 提取同一恢复实现，避免借主地址 Manager 包住无关 IO | 无新删除资格；原中断／身份／unknown 回归及证明待验 |
| `crates/worktree/src/automatic/producer.rs` | 原 store、精确 handle 与显式协调来源；ci-command-result 现役消费者 | release 后提示；显式 bind 保持原实际 owner，不推断目录 | 原消费者与生产者 Rust/mock；绑定读取／报告成本 unknown |
| `crates/worktree/src/automatic/producer/rolling.rs` | 原 fixture 终态／活能力；现役 producer finish | 持久交接后返回，独立维护保留原处置断言 | 终态、活／unknown 和诊断保护未验；不把 finish 当已退休 |
| `crates/runner/src/retained_artifacts/retention.rs` | 原引用／释放／当前输出；现役报告和 producer | 自身必要封存后发送提示；维护内部 publication 不自唤醒 | 原节点／25 ms／FD／字节限制；普通 complete 同输入 EMFILE 仍是独立未完成义务 |
| `crates/worktree/src/verified_lifecycle.rs` | event／timer／idle 三个实际观察；service 退出决策 | 同源 idle 判定，不从空闲推断完成 | 八种独立真值预期；证明／Rust 均未运行 |
| `crates/worktree-tests/tests/unified_maintenance.rs` | 原 Git／业务 mock、可控真实 IO 边界、OS socket 替身 | main 与 linked 到达业务终点后才释放后台边界；原错误与 eventual 清理分别断言 | 新线程无 detachment，异常先解除屏障并 join；无观察 red／green |
| `crates/runner-tests/tests/maintenance_signal.rs` | 受控私有 socket、原件和线程 | 缺席／满队列／收据与内部抑制，不调用 launchctl／systemctl | 复用 runner-tests 未过滤入口；未执行 |
| `.chrono-harness/maintenance.json` | repository 提出的唯一配置；服务与唤醒调用者 | 原 state 下的 singleton、短 socket 路径和 idle／timer 参数 | 无新可变账本；配置／OS 路径相符与成本待验 |
| `.chrono-harness/maintenance.launchd.plist` | 固定已交付程序、协调者、socket；当前用户 launchd | OS 持有 socket，缺席事件启动；原定时与优先级保留 | 不追加日志副本或新监督程序；真实采用未运行 |

上述新文件及既有消费文件由 FILEMAP 逐文件登记；项目、测试和同源证明的选择仍沿前后两端 DELTA，不用本表猜测选测。现有 Rust 标准库、libc、kernel lease 与 launchd 已足以提供执行和激活，不加依赖或工具链。旧同步轮次无法提供调用者不等待和缺席事件激活，因此差额限于执行归属、事件边界及必要消费迁移。

socket 必须是协调者已登记 state 下的准确物理路径；默认 `.chrono-harness/state/maintenance.sock` 避免附加长 namespace 名，绑定仍受原 OS Unix 路径上限约束。路径太长／未采用／队列满如实 wake-failed，不放大 OS 限制或建立临时路径别名。实际 OS activation 初次启动可能来自加载、事件或 timer，产品只报 `os-activation`（已读事件时报 event）；不能凭它冒充真实 timer 来源，宿主须以原生定时证据核对。常驻期间已到期 timer 优先标记为 timer，并保留错过机会；周期按原时间基准推进，长轮次不会重设并掩盖机会。

`maintenance_request` 的 handoff 表示既有所有者状态已经承担责任，不表示 OS 接受。发送后的实际激活收据单独出现在 stderr 或 release 返回值，`accepted:true` 只证明 OS socket 接收提示，`round_completion` 仍为 not-established。`maintain` 竞争成功仅返回 maintenance-requested、round_started=false；实际轮次才有完整清单与原报告。必要当前输出 publication 和父消费者交接继续同步；内部报告的成功不代替业务、维护或宿主持续稳定验收。

每次轮次开始前只合并有限批 OS 提示，剩余提示继续留在 OS socket，不产生应用任务队列。提示数量不决定成员选择；每个实际轮次仍展开全部原权威声明。内部 publication 不制造新提示；受保护、繁忙或延迟对象不会被自身报告反复唤醒。

## 因果消费者验收差额

本次有限批次只迁移已定位的同步消费者：业务成功后另取实际维护回执，原源码、未知成员、
活使用方、独有提交、失败原件及最终回收断言保留。显式维护的 metadata busy／unknown
导致完整库存不完整时仍返回失败，普通业务结果单独核对；失败进度属于真实状态。
打开所有者前的失败没有子清理回执，消费者读取并保留已发布父原件，不补造子报告。
库存投影沿父报告传入当前 agent，继续复用现役创建者提醒判断。

**程序与验证登记同步修改：**原 idle 判断移到实际 service 源，由原生命周期证明根直接编译；
发送长度判断也由实际 maintenance_signal 源进入原 limits 根。库存的动作索引追加保留每一条
匹配动作的原索引与顺序，既有两个生命周期根编译同一实现；原独立预期文件分别进入现役
测试根。分类为 program，局部证明不覆盖 IPC、JSON、锁、IO、OS 或完整事务。
未增加证明入口、依赖、线程、等待期限或报告预算。构建和验证总成本以本批原报告为准；
可比增量 IO、峰值 FD、有限留存成本和候选真实磁盘收益尚未测得，保持 unknown。

完整 footprint 用例在同一生产观察器的首个实际完整计量调用处暂停，维持原五秒准备期限。
暂停期间 main 和另一已登记 linked 目标必须达到原业务终点、保留原输入输出与退出；
解除后仍须验最终清理。该用例修正准备边界，不据旧超时修改产品锁或性能机制。

本批完整行为运行进一步暴露 finish 交接报告漏绑自身 terminal 原件：旧同步处置曾由
子清理补上该输入，异步交接必须自己登记同一 retained_input，并复用 bind_report_inputs。
仍由原 retention 引用维护寿命；原消费、释放和最终回收断言保留，没有第二份状态。
正式本地入口采用固定新上游时，还需保留其已采用的 PR 测试分类。直接复用固定
83ef6518 的 lifecycle／verified_snapshot 两份测试源及其既有根声明；没有追平分支，
没有删除分类门，也没有虚指编译源。相关原登记证明、格式与 mock 行为仍须实际验收。
