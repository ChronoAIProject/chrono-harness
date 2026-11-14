# 统一维护进程：独立预期与采用边界

状态：源码阶段，未运行构建、生成、测试、证明或宿主采用。既有完整清单源来自固定 f6ad4ac6cb3fd1f976d9c794a1ad58de9ee7dfe3，原验收义务不变。

**产品与宿主政策一起修改。** 普通入口由同步清理改为现有所有者持久交接后发送事件；维护由每个物理协调者的单一执行进程承担。政策、程序和 OS 实例必须一同采用，生成指南和实际部署由后续阶段负责。本文件先于实现提交，不声称已观察 red 或 green。

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

## 判定与登记

Rust 行为预期归现有 worktree-tests 与 runner-tests；所有具体外部执行边界用既有 Rust mock，不真实调用 OS 命令、Git 或编译器。状态决策放入现有 verified_lifecycle 与其独立 Verus 预期，正式入口复用 verify.worktree-lifecycle／verify.worktree-lifecycle-tests。FILEMAP 登记每个新增源、测试、数据和依赖，成本未知。后续仅按固定基线与候选两端 DELTA 运行登记的 bootstrap、generation、证明、专属测试和 canonical；本轮禁止执行这些操作。

## 工程选择与真实采用

复用 Unix datagram 和 OS socket activation：OS 持有 socket，缺席时启动维护进程；业务只发送非阻塞提示，不启动子进程，不等待回应。提示没有处置权，也不建立任务账本。负责恢复和删除的权威仍是现有 enrollment、producer 和 retention 状态。显式维护与服务使用同一内核 singleton 租约，业务不取得它。维护逐地址取得必要 metadata，仅同地址效果期间排除；完整 footprint、producer 和 retention IO 不保留 main 的排他。

宿主提出 idle 300 秒、定时 3600 秒；idle=null 代表常驻。定时由服务自身的阻塞等待期限支持常驻，OS 定时在服务缺席时启动；不提高现有节点、25 ms、FD、线程、stdout、报告或其他预算。定时错过／延迟和启动原因须原样报告，不从事件 ACK 推断完成。使用 macOS launchd；Linux 通过 systemd socket activation 的 LISTEN_FDS 交接，未采用 OS 配置时 wake 可失败且保持原责任。

后续真实采用必须排除旧程序、保全原状态，采用固定已交付程序、相容政策和 OS socket／timer 配置；核缺席事件启动、常驻唤醒／定时、idle 退出及再启动、单实例和未完成维护期间合法 main／linked 业务进展。调用方预先登记至少三次连续实际定时完成观察，包含错过机会、原回执、至少一个释放且到期对象的实际回收、中断恢复和保护。无合格对象时相应结论仍未建立。

原库存 300 地址／>64 KiB、完整 producer／retention 成员、IO／unknown、父恢复引用和有限保留成本全部保留。固定输入／范围／方法／单位／窗口比较增量 IO、FD、耗时、留存字节和实际占用；这些值目前 unknown。139、ordinary-complete 同输入 EMFILE、absent-terminal／身份与政策来源恢复等原义务各自继续，不由此机制替代，也不重置原额度。
