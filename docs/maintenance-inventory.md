# 显式维护的完整清单

使用方是每轮清理后核对磁盘占用、保留原因和真实效果的维护者。唯一操作仍为
`chrono-worktree maintain`；普通 use／check／bootstrap 不承担完整清单。
本次只准备源码、Rust/mock 预期和登记，未运行编译、测试、证明或生产维护；
修改前的 red 尚未观察。源码提交不代表可运行、已接受、已交付或稳定。

## 实现前独立定义的预期

1. Git 与 enrollment 的联合地址全部出现，包括只有一端、物理缺失、空地址、损坏、
   unknown、活跃、繁忙、保留、未选择和已处置项。开始观察到、随后删除的成员仍在本轮原件。
   无法枚举时明确标记不完整，不能把已知数量当成总量。
2. 按各负责方实际采用的政策列出产物；生产者和 retention 声明全部可展开。
   64 KiB 摘要装不下的成员继续保存在完整原件中，不以 topN 或样本代替。
3. 占用记录单位、逻辑长度／分配块方法、精确路径范围、时间窗口和新鲜度。
   缺失观察与测量错误分开；错误不得成为零。sealed manifest 是历史测量，
   没有观察时间不得补造。重叠路径、硬链接和卷共享不累加成物理释放量。
4. 实际动作只引用本轮负责方的原回执；未选择、未执行、部分失败、保护和完成各自保留。
   报告不产生删除资格，不把 caches-released 当作任务结束或整个编译树已删除。
5. 枚举／测量／报告 IO 失败保持原始失败，独立清理沿现有入口继续；
   报告不能发布完整或成功。测试在 Rust 中模拟 Git 等外部程序边界。
6. 完整清单沿现役 lifecycle-original 的引用和有限保留合同发布；
   中断的未完成原件继续保护，最终报告接住引用后释放临时恢复责任。
   不建立权威库存账本、后台任务或新调度器，不提高现有预算。

## 必要性和成本

| 对象／负责方 | 输入和实际使用方 | 输出用途／替代方案 | 验收与成本 |
| --- | --- | --- | --- |
| worktree 显式维护 | Git inventory、enrollment、各地址采用政策；维护者 | 连接完整成员、占用和原 drain，复用 status 分类；现有摘要不足以解释所有留存 | Rust/mock 联合集合与失败回归；每轮 IO、耗时和保留字节待实测 |
| producer 原件 | 原 Store 声明、生成路径、body／rolling 状态；同一维护报告 | 完整成员与原处置引用，不另设所有权 | 正常、繁忙、损坏与缺失回归；成本未知 |
| runner retention 观察适配 | 现役逐地址声明、sealed manifest 和原 bounded round；同一维护报告 | 流式交出未抽样原观察；删除判断仍归 retention | 超过 64 KiB、错误、未执行和 IO 失败回归；每轮原限制不变，增量成本待测 |
| 直接测试和登记 | 合成文件与 Rust/mock 输入；第 2 次 assignment 的 canonical／证明入口 | 先固定可观察预期；本次不执行 | 沿前后 FILEMAP 的 DELTA 选中，不跑全仓兜底 |

第 2 次 assignment 才能在调用方交接容量后 bootstrap、运行全部适用检查并修正真实失败。
独立评审、两段真实 PR、落地树核对、实际采用、同范围占用比较、合格对象真实回收、
报告增量成本、ENOSPC／中断恢复和连续定时稳定性均未验收。
原 EMFILE、139 创建政策恢复、frontier 和 absent-terminal 义务不由本报告代替。

## 候选原件接口

父维护报告的 `complete_inventory` 引用同一报告目录中的 `.inventory.jsonl`，
提供 SHA256、实际字节数、行数、起止毫秒时间、耗时、发布结果，以及成员枚举和测量
各自的完整性。JSONL 每行是一条记录：`worktree` 保存前后状态，
`worktree-artifact` 保存已采用政策中的精确路径与当前 footprint，
`worktree-action` 指向父报告的原 drain 项；没有原 drain 时明确未执行。
`producer-policy`、`producer-allocation`、`producer-artifact` 展开原生产者声明，
`producer-result` 引用原生产者回执。`retention` 保存完整声明地址、现役 bounded round
已经读取的逐对象状态及未抽样原结果；不增加第二次关系展开或元数据读取。
状态在原 round 前观察，路径 footprint 在该 round 后测量，两个窗口不冒充同时快照。
封存 manifest 的长度和分配块仍标作历史值，时间未知时保留未知。

`member_enumeration_complete` 表示本次观察窗口内的声明迭代已穷尽且没有未知声明；
它不表示原子快照、物理路径全部存在或全部动作完成。无已采用政策的 Git-only 地址、
缺 owner 的 metadata 和未登记 producer 仍有明确行与不完整标记，不猜其声明。
`measurements_complete` 单独核全部可声明范围；旧值、未知和报告发布完成不能替代新鲜测量。
处置资格与效果必须沿原结果核对；`observed` 也不证明清理成功或完整目标达成。

原件由现役 `lifecycle-original` 发布，最终父报告接住依赖后释放原件临时 recovery 根；
继承父报告与现役政策的保留窗口，不另设永久消费方。IO 中断保留原部分字节和恢复责任。
保留现有 stdout 报告格式与 retention 的 64 KiB 样本上限，完整清单在引用原件展开；
不提高节点、时间、FD、线程或报告字节限制。显式维护多一个原件写描述符，
封存摘要时短暂读取同一文件；完整 footprint 的遍历、读 IO、增量耗时和保留成本尚未实测。

## 文件责任与后续判法

| 文件 | 负责输入 → 输出和使用方 | 依赖／成本／操作 |
| --- | --- | --- |
| `crates/worktree/src/automatic/inventory.rs` | 已有状态、采用政策和原结果 → 显式维护原件 | worktree、footprint、retention；成本未知；只由 maintain 调用 |
| `crates/worktree/src/automatic.rs` | Git／enrollment／采用政策 → 同一 status 分类、维护与报告连接 | worktree；原生命周期证明与直接 Rust/mock 测试待执行；普通入口不生成全量原件 |
| `crates/worktree/src/automatic/producer.rs` | 原 Store 和精确成员 → producer 声明投影 | worktree；沿现役生产者回执，不创造处置权；成本未知 |
| `crates/worktree/src/recovery.rs` | 父报告原件引用 → 原有 retention 依赖和 recovery 交接 | worktree／runner；中断与保留回归待执行 |
| `crates/runner/src/retained_artifacts/partition.rs` | 原 bounded round 已读声明 → 未抽样观察回调 | runner；原 retention 证明与行为测试待执行，不改变每轮限制 |
| `crates/worktree/src/lib.rs` | 原写入边界 → 既有测试 feature 下的 Rust IO mock 接口 | worktree-tests 的实际调用；纯转发不另添形状测试 |
| `crates/worktree-tests/tests/complete_inventory.rs` | 合成联合成员、缺失、损坏、链接和 IO mock → 独立可观察预期 | worktree-tests；包含于原 creation 测试目标；未执行 |
| `crates/worktree-tests/tests/automatic.rs` | 原 mock fixture → 上述直接测试模块 | worktree-tests；未新增测试入口或资源额度 |
| `crates/worktree-tests/tests/support/git_fault.rs` | 明确合成 inventory／错误 → 原 Git 执行边界回执 | worktree-tests；不执行真实 Git |
| `crates/worktree-tests/tests/fixture_body.rs` | 实际声明的活 allocation 和复制成员 → 完整 producer 报告预期 | worktree-tests；已有命令边界 mock；未执行 |
| `crates/runner-tests/tests/retention.rs` | 超过样本容量的声明、活／损坏／保留项和失败回调 → 完整成员及错误预期 | runner-tests；复用原大样本，未提高预算，未执行 |
| `.chrono-harness/FILEMAP.json` | 上述新增源码、测试与本文 → compile／文档登记 | 沿前后登记的 DELTA 选择，未运行规划或判官 |
| `docs/worktree.md`、本文 | 当前维护入口和源码候选状态 → 维护者交接说明 | 无独立运行任务；编译／IO／存储成本仍未知 |

操作登记仍复用宿主唯一 canonical check，由前后 FILEMAP 与实际 DELTA 决定全部适用操作；
本阶段没有执行选测或判官，也不手填最终选中列表。源码改动不表示相关证明已经覆盖
JSON、文件系统、流式写入或整个维护事务。后续须核原 stdout／stderr、退出和 owned work
收束，再由调用方完成其余已批准的独立评审、交付及真实宿主验收。
