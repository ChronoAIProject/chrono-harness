# AI 的 PR 工具

目标是让 AI 用一次命令打开 PR、查询固定提交的检查结果，或等待检查完成。独立 Rust 项目 `pr` 维护 `chrono-pr`，`pr-tests` 验证真实 CLI 和子进程。进程执行、时限、输出上限和子孙进程回收复用 runner；GitHub 认证和等待使用现有 `gh`，不另建凭据代理、后台任务或集中状态账本。每次操作的原件由现有输出保留所有者保护。

设计参考固定版本 [trureturing 的 PR 入口](https://github.com/the-omega-institute/trureturing/blob/481b9d7c46c8ef94b34842eb888d1ac254b2349f/tools/scripts/pr.sh)：显式远端分支、head SHA、结构化结果、有界请求和等待。参考仓库只读，其政策、门数、语言工具链和认证代理不属于本产品。

## 构建与调用

从本仓根目录执行登记的 `build.pr`：

```sh
cargo build --locked --manifest-path crates/pr/Cargo.toml
```

当前源码入口为 `crates/pr/target/debug/chrono-pr`；尚未加入公开发布包。使用方须已有能访问目标仓库的 GitHub CLI。工具沿用 gh 的原生认证，不申请额外权限。`.chrono-harness/pr.json` 使用 `chrono-pr-config/v2`，显式设置仓库、目标分支、检查名称、gh 程序、资源上限，以及 `state_directory`／`state_producer`。本宿主采用 `.chrono-harness/state/pr-operations` 和 retention.json 登记的 `pr-original`。宿主必须采用现役 retention/v2 并登记该生产者的精确范围；缺失时不派发 gh。配置与结果均只接受当前格式。其他宿主提供自己的同格式文件；默认读取 `--host-root` 下的 `.chrono-harness/pr.json`，也可用 `--config` 指定。

```sh
crates/pr/target/debug/chrono-pr pr-open \
  --head integration/my-change --head-sha <40位提交SHA> \
  --title '添加功能' --body-file /path/to/body.md
crates/pr/target/debug/chrono-pr pr-status --pr 123 --head-sha <40位提交SHA>
crates/pr/target/debug/chrono-pr pr-watch --pr 123 --head-sha <40位提交SHA>
```

所有命令支持 `--host-root`、`--config`；`pr-open` 可加 `--draft`。打开前，调用方先保存、检查并推送候选。工具只支持同仓库分支，先核远端分支 SHA，再查找相同 head／base 的现存打开 PR；唯一匹配则复用，无匹配才创建。标题和正文按字面值传给 gh，正文通过 stdin 传输。创建后重新读取 PR，核对仓库、编号、base、分支和 SHA。并发调用或网络中断不保证事务；创建失败或回读失败保留未知结果和原回执。恢复时先核原操作、远端 PR 与固定身份；再次打开虽然会先查询现存打开 PR，但空列表不证明旧创建没有发生，不能据此盲目重复创建。跨中断的自动恢复尚未实现。

`pr-status` 一次读取 PR 和指定提交的检查。`pr-watch` 先读取一次；仍待完成时，只启动一个原生 `gh pr checks --watch --interval 60` 等待通道，结束后回读同一 PR／提交。等待程序退出本身不产生通过状态。等待覆盖 gh 看到的全部检查，最终判定只消费配置列出的必需检查；原生通道未见到检查、提前返回或中断时，不额外轮询，结果保持 pending／unknown。超时停止并加入本工具的本地子进程，远端 CI 仍可继续；报告保留 PR、SHA 和原始回执，供后续查询。

## JSON 和退出码

stdout 恰为一份 `chrono-pr-result/v2` JSON，帮助信息除外。结果含 command、repository、base、pr、expected_head、observed_head、outcome、checks、error 和 receipts，并给出 `operation`、`operation_state`、`command_exit_code`、`pr_state`、`merge_commit` 和更新时间。每条回执保存实际 argv、执行物摘要、PID、退出／signal、失败、流完整性和 stdout／stderr 原件的路径、长度及摘要；正常结果不再内嵌两套流字节。消费者读取原件并核摘要。原件写入失败时，stdout 退回实际已取得的内嵌字节并明确报告 `storage_error`，退出 2；该输出不声称已持久保存。不保存子进程环境值或凭据，调用意图只保留环境摘要。

| 退出码 | outcome | 含义 |
| --- | --- | --- |
| 0 | opened、existing | PR 已创建或已复用，身份回读通过；不代表检查通过 |
| 0 | passed | 所有登记必需检查成功，PR head 与期望 SHA 相同 |
| 1 | failed、closed | 必需检查失败，或 PR 已关闭且未合并 |
| 2 | unknown | 参数、配置、API、协议、输出上限或本地等待错误 |
| 3 | pending | 必需检查缺失、尚在运行或等待超时，不能判定成功 |
| 4 | stale | PR／远端分支 head 已变化，当前结果不能用于期望 SHA |

必需检查集合须非空。只将 `SUCCESS` 视作成功；取消、失败、超时、需人工操作、过期和启动失败均失败，neutral／skipped 也不满足必需成功。缺项保持 pending。未知状态、同名歧义、错误仓库／编号／提交和截断的检查集合明确报错。API 的分页上限为 100 项，超出时保持 unknown，不把可见子集当完整集合。合并后的检查成功仍只表示该提交通过检查，不能证明落地提交、发布或工作树清理。

## 本地操作原件与当前边界

每次调用在登记目录下取得独立操作地址。原件保留所有者先登记输出 intent、绑定物理目录并保护恢复使用方；工具再写不可变的 `config.json`、`intent.json`。每次 gh 调用前保存实际 argv、输入原件、执行物／环境摘要、时限、输出上限和时间；这些写入失败就不启动 gh。没有实际句柄或结果时字段为 null。进程返回后保存 `call-N.stdout`、`call-N.stderr` 和 `call-N.result.json`，最后发布 `result.json`。文件及所在目录同步后才继续派发。缺失、部分写入或未完成保留发布均不算终态。

`operation_state=completed` 只表示本地调用已返回并收取本次子进程结果，实际命令退出码另存；`pr_state=OPEN` 仍是活跃 PR。MERGED 必须带真实有效的 `mergeCommit.oid`，它只是远端观察，不证明实际落地树／发布已验收。原结果、PR 状态和存储发布状态分开；最后的保留封存或引用释放失败也退出 2，先前已写出的业务结果继续作为原件保护，不把它当完整成功。

每次操作只有自身输出与恢复引用，没有总清单准入或全库扫描。结果无错误且对象已确认为 CLOSED／MERGED，或明确尚无 PR 的正常返回，可以释放本次恢复引用；原件再按现有 diagnostic 保留窗口滚出。本宿主窗口为 86400 秒，单输出、每轮节点／字节／时间及元数据上限沿 retention.json；沿既有 retention 维护入口回收，不另造清理器。

**这一批是 §7.5 的局部实现。** gh 尚未返回时，实时启动句柄及部分流还没有接入这份 PR 原件；中断后的结果必须保持 unknown，不能从 PID 缺失或旧 intent 推定结束。跨调用的对象接续和旧恢复引用释放尚未实现，因此 OPEN／unknown 的原件仍保护，重复查询会增加受保护记录；不能用当前实现宣称活跃历史已有有界轮转。保留政策迁移、真实中断恢复和全部工具采用也未完成。参数／配置／留存入口在有效 state 建立前失败时，`operation=null` 明确表示没有可声称已保存的操作。不得手改这些状态制造成功或释放。

**产品与宿主政策同改：**产品新增上述原件和结果格式，宿主把现役 PR 配置迁到 v2，并在 retention／FILEMAP 登记生产者、输入和消费者。旧 v1 不再支持，不迁移或改写旧原件。现有共享状态须沿其所有者的正式政策迁移和部署边界采用；本候选不改协调者或现有共享安装。新增磁盘 IO、验证与存储成本未测。

## 验收与范围

登记的 `test:pr-tests` 执行两项目格式检查、生产构建及 Rust 行为测试。测试用独立 Rust gh 替身核实际 argv、stdin、输出和退出状态，覆盖创建／复用、失败恢复、字面文本、身份变化、缺检查、失败检查、截断和协议错误，以及原生等待后的重新核对。gh 替身在执行模拟副作用前核真实 intent 与输入原件；另测持久结果／流、未登记时零派发、结果发布失败和真实合并身份。外部接口样本独立登记。

产品与宿主登记同时变化：新增 PR 产品、配置、配对、依赖和检查计划，复用 worktree CI 单元运行 PR 测试，不增加主检查工作流或必需检查。发布验证沿用“覆盖所有现役测试项目”的合同，新增 Linux／macOS 各一个 `verify_pr_tests`，执行原未过滤测试，无发布资产前置；发布包不新增二进制。两项目的 target 进入现有生成物和清理白名单，沿用原有身份、使用保护与清理条件。原有检查判法不变，新增构建、测试和 target 存储成本未测。新项目锁定 runner 当前使用的 `libc 0.2.189`；初次解析的 `0.2.191` 缺少 runner 在 macOS 使用的 `NOTE_REAP`，不采用该组合。

Rust 行为测试覆盖上述输入、状态和失败边界。真实 GitHub 固定提交已验证查询、终态读取，以及单个原生等待进程结束后的重新核对；等待退出码不替代必需检查的实际结果。远端写入、公开分发、自动合并、落地核验仍分别需要实际验收，这些结果不能由本地替身或检查通过推导。
