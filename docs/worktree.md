# 按登记创建和维护工作树

`chrono-worktree` 是独立 Rust 程序，有专属 `worktree-tests` 项目。它从刚获取的明确远端目标创建 feature 或 integration 工作树。工作树是同一 Git 仓库的独立检出目录；下文 OID 指完整 Git 对象标识。

程序不从目录名选项目，不恢复语言依赖，不重置旧分支，不开 PR，也不作治理判断。

宿主将 `.chrono-harness/worktree.json` 登记到 FILEMAP。原 `chrono-worktree-config/v1` 形状如下：

```json
{
  "schema": "chrono-worktree-config/v1",
  "host_config": ".chrono-harness/config.json",
  "remote": "origin",
  "git": {"program": "git", "expected_version": null, "sha256": null},
  "environment": {
    "inherit": ["PATH", "HOME", "SSH_AUTH_SOCK"],
    "values": {"GIT_TERMINAL_PROMPT": "0"}
  },
  "timeout_seconds": 120,
  "output_limit_bytes": 1048576,
  "report_directory": ".chrono-harness/state/worktrees/"
}
```

目标分支、feature／integration 前缀、FILEMAP 成员及产物所有权仍以五份宿主登记为准，程序复用已有严格加载器。加载或创建成功不证明完整引用准入、输入完整、新鲜度或启用治理；交付上下文和 DELTA 仍由普通 workflow 判官判断。

`host_config` 可为直接配置，也可为与其他完整消费者相同的稳定 `chrono-git-configs/v1`／v2 平台入口。共享登记快照读取器分别从固定来源和获取提交选择直接 full-v3 目标；有效配置／workflow 路径可不同。平台项／目标缺失或所选政策不支持就失败，不回退。

全部获取仍使用工作树政策绑定 Git、声明环境和进程限制。宿主配置只是数据，不会选择历史判官执行物。

从宿主登记安装器安装所选发布的 `chrono-worktree`；产品仓库也可 bootstrap 候选。创建命令为：

```sh
.chrono-harness/bin/chrono-worktree start \
  --host-root . --config .chrono-harness/worktree.json \
  --kind integration --name task --path ../new-task-checkout
```

五个参数都必需，未知、重复、空参数失败。目标须明确，其父目录须存在，目标本身不得存在或嵌套于已有登记工作树。路径相对 `host-root`，不相对调用者 cwd；空格／UTF-8 名称按字面参数处理。此版本支持已测试的 Unix Git 接口。

## Git 身份与固定输入

程序只解析配置 Git 一次，观察并绑定实际字节，之后每次 Git 操作都使用它。可选预期摘要／版本不符就拒绝；null 不作预期身份保证。只继承声明环境，明确值覆盖继承值。

本宿主采用 `chrono-worktree-config/v2`，保留必需 `check_inputs`，Git 预期摘要／版本为 null。创建、恢复和清理需要在一次运行中使用同一个实际 Git；既有观察与逐命令摘要检查已满足这个要求。没有当前使用方要求工作树按主机摘要白名单准入，因此退役额外的 v3 多身份选择、专属测试和宿主摘要副本。

自有回执保留实际 Git、版本及原政策摘要，未建立预期身份或跨环境同判保证。可选单一摘要仍在版本探测前核对，不符拒绝执行。源码／获取端／协调者的已提交政策相等、v2 自动清理和显式 migrate 合同保留；候选测试不能代替存活协调者的部署验收。独立的 [Git 事实选择器](git-facts.md#exact-host-executable-selection)仍按其现役合同准入，本批未改变它。

读取未跟踪清单时，子进程环境明确关闭 Git 全局 literal／glob／noglob／大小写折叠 pathspec 模式，排除参数自身是区分大小写的字面值。原进程记录这些范围内覆盖。拒绝仓库重定向变量，不证明 Git 配置、凭据 helper、hook 输入完整。不改全局配置，新分支明确关闭自动 tracking。

本地政策须匹配源码提交。程序读取 workflow 目标，将该精确远端分支 fetch 到唯一临时 ref，记录 commit／tree，再用观察 OID 只删该 ref。创建前读取获取端登记，要求政策字节及目标分支兼容。不保证 fetch 后远端不再移动，回执只固定观察顶端。不重置已前进源码检出或无关脏文件。

创建与本地输入准备的 fetch 显式使用空 `--refmap=`，只采用命令行指定的临时目标，不更新共享的远端跟踪引用。这样获取新基线无需额外写入 `origin/dev` 等共享引用，也不会因这次无关更新与其他 fetch 争用。原 Git 失败、临时 ref 恢复和进程限制保持原合同。

源码登记在 fetch 前获取一次，复用于政策检查及摘要。两端登记摘要覆盖完整解析快照，含稳定选择器及有效目标；原字节和失败保留在固定 Git 进程观察。回执／intent 的 `config_path`／`config_sha256` 继续指工作树政策。重建、恢复、rebind、清理都共用该快照获取模块。

### 有限读取与复用

登记列表使用 runner 共享、有上限的 Git batch 获取。metadata 固定 blob 身份／大小，内容请求按原输出上限分批并与 metadata 核对。小列表或框架空间不足时仍直接读。batch 错误／失败在检出效果前保留原进程字节和退出。只有完整验证的不可变字节进入单次操作缓存；HEAD、检出、政策、附件、所有权仍实时观察。

为超过普通诊断上限的明确不可变输入，v1／v2 政策还可声明 `immutable_input_limits`：精确仓库相对路径 → 正字节上限，最多 67,108,864。省略保留原传输合同。本仓为 `.chrono-harness/config.json` 声明 2,097,152 字节，`output_limit_bytes` 仍 1,048,576。这只增加获取容量，不登记文件、输出、消费者或删除资格。

blob 模块先在原时间／输出上限内获取固定提交对象 metadata。过大输入在读内容前拒绝。允许读取时，只有 stdout 上限改为精确实测长度，空 blob 用一字节；stderr 和其他 Git 命令不变。数据命令点名观察 OID，Rust 核完整 Git blob hash 和长度。

两进程、字节、摘要、退出及部分失败都保留，数据进程另记 stdout／stderr 传输上限。失败／不符字节不进缓存，进程所有权和租约交接不变。

**产品与政策曾同步采用：**政策提交该字段前先装兼容工具，旧严格解码器会拒绝。增大输入容量不豁免已提交政策相等、实时检出检查、产物登记或迁移。传输完整不证明输入完整或同判。

Rust 获取回归使用自含的 2,022,211 字节输入，核对普通输出限制、完整获取、声明容量和内容漂移；无需外部历史文件。

协调维护可用此数据合同读取已有登记项，同时保留目标已提交配置／HEAD。目标原政策输入须仍匹配登记绑定，cleanup hash 须匹配协调者。只改协调者传输声明，不授权改目标或重写旧尝试；pending 缓存尝试若绑定旧协调配置，仍须按原规则协调。

固定 OID 树成功解析后，在同一操作／根内共用原获取。清理各保护复用不可变树，索引、物理源码、政策、锁、使用状态仍实时读取；失败／错误树不进缓存，下一操作重新取证。

实时检出身份在一次有限 Git 进程读取根、共同仓库、metadata 目录、HEAD、完整分支名。维护结合新 inventory、物理身份、政策和协调者检查使用它。每个处置保护重新观察，不跨删除效果复用。字段错误、身份不符、Git 失败都保留 pending 工作及原失败。

本地准备在 fetch base 前后，都将索引和物理文件与固定候选比较。最后 HEAD 观察不能为之后文件比较换快照；干净的替换提交不能替原候选过验，未跟踪检查保持实时。这不是原子 Git 事务。

## 创建结果与失败保护

创建使用新分支及本次调用拥有的 worktree 锁，解锁前核真实 Git inventory、HEAD、分支、根、干净状态。复用 registration 产物分类器，允许并保留声明未跟踪产物；已跟踪修改单独检查，产物根下也不例外。

一次未跟踪观察同时含 ignored／非 ignored 文件，不依赖 ignore 规则变化；返回前只排除明确产物目录。准入只需判断是否存在未登记工作，复用 Git 的目录列举，省去非空未跟踪目录内的逐文件输出；空目录不构成文件变化。其余路径用原分类器。未知邻居、字面／大小写近似项、用文件或链接冒充登记目录都错误，上限不变。年龄复用的源码保全仍使用完整文件清单，逐文件保存未登记工作。

已有路径／分支保留。hook 失败、超时、身份变化、新检出脏都会使创建失败，保留新分支／工作树供恢复；不自动删除，不从部分 Git 完成推断成功。fetch 失败可能留下具名临时 ref，恢复前须看报告。

每次已观察尝试在配置 state 目录写唯一 `chrono-worktree-report/v1`，stdout 返回相同 JSON，包含源码身份、基线／tree、分支／目标、开始时间、配置摘要、继承／有效环境、Git 版本及真实子进程 argv、字节、摘要、退出、限制失败。

Git 观察、版本探测、托管命令及其 use 回执复用 runner 的无损
`chrono-retained-process` 编码：每条流只保存压缩十六进制、原长度及 SHA-256，
容量边界上的不可压缩流使用现有 v1 十六进制回退，不再同时保存文本和数字字节数组。
Rust 消费者先用 `full::expand_process` 展开；宿主 bootstrap 用
`bootstrap.retained_process` 核编码、原长度及摘要后转发原 stdout／stderr。
没有新增共享缓存或总清单，原进程身份、退出、失败和每流 64 MiB 上限不变。
这只减少报告中的重复字节表示，不证明总生命周期时间或计费节省。

托管的 `use`、`check` 和 `bootstrap` 在派发前为本次报告创建独立的
`<report_path>.process/` 目录。它由同一个 `lifecycle-original` 保留所有者维护，
借现有目录发布能力保存 `intent.json`、`launch.json`、原 `stdout`／`stderr`，
返回后再保存 `result.json`。命令在目标工作树运行，原件存于报告所属协调宿主；
环境只保存摘要，结果引用原流，不在目录内再复制流正文。

最终报告和 use 意图显式引用这个目录。正常加入全部自有工作并核对启动与流原件后，
已发布报告接收保留责任；报告及其真实使用方释放后，目录沿现有有限保留政策回收。
外层中断时，空报告、缺失结果和已有启动记录仍分别保留，不据此宣称命令完成。
现有独占租约协调确认所有登记使用方已退出后，把原目录交给中断回执保管，
解除该次发布恢复引用；原命令结果仍为 unknown。此路径不新增等待通道，
也不提供丢失原等待句柄后自动收取进程终态的保证。

工具观察／配置接纳前失败，非零诊断且不编造进程回执；观察后 status 为 `created` 或 `failed`，分别退出 0、2。`governance: not-evaluated`、`parity: unestablished`，部分 context 只是后续输入，不是完整检查上下文或集成凭据。

新报告和对应 stdout 用紧凑 JSON，末尾一个换行；历史原件保留原表示和字节。完整进程捕获、字节数组、摘要、退出、上限仍在，避免缩进放大 Git 字节，同时保持存储／stdout 完全一致。

专属测试使用真实 bare remote、前进远端、任意布局、空格／Unicode 路径、脏／锁工作树、两端政策变化、remote 缺失、工具不符、真实 hook。采用政策有真实子进程消费者，宿主明确登记二进制及测试计划。

`worktree-tests`、`ci-tests`、`judge-workflow-tests` 将生产 `src/main.rs` 编译为
`chrono-worktree-test-cli`，通过 Cargo 的 `CARGO_BIN_EXE_chrono-worktree-test-cli`
调用或复制真实 CLI；FILEMAP 明确登记三处源码别名。六个 CI 测试单元不再额外执行
`build.worktree`。worktree 专属测试的只读副本、所有者阶段发布和中断用例均使用该执行物；
bootstrap、正式 owner-stage 和生产发布继续消费各自登记的生产构建。

**测试编译路径与宿主政策同时变更：**六个测试单元同步退役重复构建和
`check.worktree` 缓存订阅；该缓存已无使用方，声明一并删除。测试项目自身的缓存及生产发布缓存继续保留，
工作流由现役生成入口更新。原测试、断言、分组、线程数和时限不变，各测试根沿用既有开发配置，
不宣称与独立生产构建严格同输入。真实 CLI、部署和中断行为仍由原有测试验收；
完整构建、CI、存储及维护的净成本收益未测。

[示例宿主](examples.md)明确采用工具和政策，实际版本与采用范围见示例索引；不靠语言或目录推断。

下文增加恢复及明确清理。AI 语义协调、PR provider 操作、合并／落地编排仍未完成。后续治理仍用本地／CI 同一 `chrono-harness check`。此增量曾同改产品和采用政策，验证增加新项目／测试对及 bootstrap／发布登记；成本未知，不增加确认或人审。

<a id="explicit-reconstruction"></a>

## 明确重建工作树

沿用同一宿主政策，在登记未跟踪 `.chrono-harness/state/` 产物下提供本次计划：

```sh
.chrono-harness/bin/chrono-worktree reconstruct \
  --host-root . --config .chrono-harness/worktree.json \
  --kind integration --name task-r2 --path ../task-r2 \
  --plan .chrono-harness/state/reconstruction.json
```

严格 `chrono-worktree-reconstruction/v1` 点名完整固定提交和原完整净 DELTA，改名按删加。下列占位符须换真实完整 OID：

```json
{
  "schema": "chrono-worktree-reconstruction/v1",
  "base": "OLD_FULL_BASE_OID",
  "candidate": "OLD_FULL_CANDIDATE_OID",
  "changes": [
    {"path": "arbitrary/source.ext", "action": "carry"},
    {"path": "obsolete-change.data", "action": "retire", "reason": "Superseded requirement"}
  ]
}
```

每个变更路径恰好一次，无无关项；retire 必须有非空理由。AI 判断是否仍需保留，工具只核覆盖并应用选择，不推断语言／选测。旧 base 须为候选祖先，候选等于源码 HEAD；源码须已提交、除声明产物外干净。计划自身须有产物所有者，创建前仍须满足政策采用要求。

程序以字面路径、无改名猜测／外部 diff／文本转换，提取原 binary／full-index patch，再复用 start 获取目标、创建新分支，不合并旧分支。Git 用 `--3way --index` 应用所选 patch，保留原 diff／apply 及补丁输入摘要。补丁不能在上限内保留时，创建前失败；空或全退役 DELTA 不 apply。

成功为 `status: reconstructed`、退出 0，含新目标 parent、实际暂存路径集及 `reconstruction.index_tree`。暂存尚未提交，所以部分 context 的 candidate 为 null。不删原工作树／分支。

失败退出 2，保留冲突、原进程失败、调用锁供 AI 协调。源码修改或目标身份变化也失败，不删工作。成功后 AI 审读暂存、协调语义／登记、提交，再用新证据跑标准检查。重建不证明新鲜度、检查或集成；新鲜度归 workflow，Git／config／hook 输入完整、自动解冲突、provider 编排仍不保证。恢复及保存工作清理见下文。

<a id="registered-recovery-and-cleanup"></a>

## 登记式恢复与清理

维护沿用已提交 `.chrono-harness/worktree.json` 的 Git、进程限制和报告位置，计划放登记 `.chrono-harness/state/`。不按分支年龄、目录名、语言选工作：

```sh
.chrono-harness/bin/chrono-worktree recover \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/recover.json
.chrono-harness/bin/chrono-worktree cleanup \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/cleanup.json
```

recover 完成调用方对失败 start／reconstruct／cleanup 检出的协调。严格计划为：

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "recover",
  "receipt": {
    "path": ".chrono-harness/state/worktrees/FAILED_REPORT.json",
    "sha256": "SHA256_OF_ORIGINAL_REPORT_BYTES"
  },
  "head": "EXPECTED_FULL_HEAD_OID",
  "index_tree": "EXPECTED_RESOLVED_INDEX_TREE_OID"
}
```

原报告须绑定本源码、当前已提交政策、失败创建／重建／清理、精确目标／分支及自有锁。调用方明确给出解决后的 HEAD／index tree，失败清理留下的自有锁也可先恢复再重试。

要求原获取 base 仍为祖先、索引已解决、无未暂存修改／未登记文件、同一物理 Git 仓库。解锁前复核身份。允许已提交协调，暂存仍保留暂存。新报告保留原字节、摘要、失败，状态 `recovered`，不重跑 hook、不改旧失败为成功、不发检查／集成凭据。调用方提交剩余暂存后跑标准检查。

cleanup 点名精确工作树，以及另一个保留其提交的本地分支。以下字段全必需，须换真实路径／OID：

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup",
  "path": "../finished-checkout",
  "branch": "integration/finished-task",
  "head": "EXPECTED_FULL_HEAD_OID",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_FULL_RETAINED_COMMIT_OID",
  "retention": "same-tree",
  "discard_artifacts": [".chrono-harness/bin/", ".chrono-harness/state/"],
  "remove_branch": true,
  "allow_absent_worktree": false
}
```

`retention: ancestor` 证明 HEAD 是固定 retained commit 祖先；`same-tree` 用精确树相等支持 squash 交付。两者都保留位于声明提交的具名本地分支，不证明远端合并、测试或不同树语义等价。

工作分支须匹配明确 feature／integration 前缀。主／源码工作树、带锁、嵌套、外来、身份不符、脏或未知目标都失败。每个可丢目录须是源码／目标 HEAD 两端登记的未跟踪产物；未列产物也阻止删除，登记本身不授权丢弃。产物路径下已跟踪变化仍错误。清理使用有限字面清单查询，不逐个列构建缓存文件。

每次核对保留条件时，用一次 Git 调用读取当前保留 ref、HEAD 的提交身份，以及 `same-tree` 所需的两端 tree。结果须逐项为完整 OID 且数量准确；不存在的 revision 即使返回退出码 0，也必须拒绝。`ancestor` 继续核真实祖先关系，各效果前的检查点照常重新读取。

程序核精确检出、HEAD、分支、保留 ref，取得调用锁再核，只释放自有锁，再核保存工作／内容后移除工作树。可选删分支前先验证路径／inventory 缺失；按预期 OID 删分支，核无登记工作树仍使用它。

失败保留实际进程及明确部分效果，不承诺回滚。`worktree_removal`、`branch_removal` 区分 `not-attempted`、`attempted-unverified`、`verified-absent`、`already-absent`，计划保留分支为 `not-requested`。只有本次移除且核缺失后，`*_removed` 才 true。false 不证明失败 Git 无效果，应读状态和原进程。

`allow_absent_worktree: true` 明确允许部分移除后重试，含剩余分支；已缺失只报告观察，不算新移除，保留分支仍须匹配。配置、hook、并发文件写入不是原子事务，不宣称排除一切并发变化。

维护返回同一存储／stdout 报告及原进程字节，`governance: not-evaluated`、`parity: unestablished`；成功 0、失败 2，保全源码。中断且有 intent 走下文独立合同；metadata 损坏／缺失、旧回执丢失走明确 rebind，不自动删 metadata 或解语义。PR／合并／落地生产者未完成。上述命令从公开 beta.8 随固定宿主安装入口提供。

### 清理临时 fetch ref

fetch 或临时 ref 删除失败，可能留 ref 而未建工作树。原失败报告存在时，沿同一维护配置用 `cleanup-fetch`：

```sh
.chrono-harness/bin/chrono-worktree cleanup-fetch \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/cleanup-fetch.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-fetch",
  "receipt": {
    "path": ".chrono-harness/state/worktrees/FAILED_REPORT.json",
    "sha256": "FULL_ORIGINAL_REPORT_SHA256"
  },
  "head": "FULL_EXPECTED_FETCH_COMMIT",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "FULL_EXPECTED_RETAINED_COMMIT",
  "allow_absent_ref": false
}
```

原报告须绑定本源码／配置和失败 start／reconstruct，且未报告临时 ref 已移除。`fetch_ref` 须为 `refs/chrono-harness/fetch/` 加调用 token；若有 base，须等于计划 head。预期提交须为明确本地保留分支 retained_commit 的祖先。符号 ref、OID 变化、保留分支移动均失败。不删工作树、其他 ref 或远端分支。

程序复核保留及原报告，按预期 OID 删除，观察缺失，原失败字节不变。`fetch_ref_removal` 使用同样四状态；`fetch_ref_removed` 只在本次删并核缺失后 true。失败命令可能已改 ref。只有明确允许缺失重试才设 `allow_absent_ref: true`，保留条件仍须成立。不重建缺失／中断报告，不保证并发 ref／配置原子性。退出／报告沿用维护合同，公开 beta.8 已含此入口。

## 恢复中断的检出操作

公开 beta.8 还提供 `recover-interrupted`，沿宿主固定分发锁安装。start／reconstruct 创建检出前、cleanup 取得检出锁前，先在结果旁发布后缀 `.intent.json` 的 `chrono-worktree-recovery-intent/v1`。先写并同步临时文件，再以不覆盖已有路径的方式发布；失败就不执行之后检出／加锁，但更早的 fetch 等准备可能已发生。

intent 保留源码／配置／登记身份、所选 base、目标、分支、调用锁理由和最终结果路径，没有结果或虚构退出。完整结果引用其路径／摘要。最终发布前进程停止时，调用方可按观察到的调用／锁身份明确选 intent；工具不扫描或猜所有者。

恢复完成前保留 intent 和不完整结果。intent 存在不证明旧调用已停止，AI 调用方必须先确认这一点。

```sh
.chrono-harness/bin/chrono-worktree recover-interrupted \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/interrupted-recovery.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "recover-interrupted",
  "intent": {
    "path": ".chrono-harness/state/worktrees/start-INVOCATION.json.intent.json",
    "sha256": "ACTUAL_INTENT_SHA256"
  },
  "result": {"presence": "absent"},
  "head": "CURRENT_RECONCILED_HEAD_OID",
  "index_tree": "CURRENT_RECONCILED_INDEX_TREE_OID"
}
```

空或截断结果文件存在时，用 `"result": {"presence": "present", "sha256": "ACTUAL_RESULT_SHA256"}`。缺失与空不同。absent 不接受内容字段，present 要求字符串 sha256。未知字段、presence 缺失／未知、内容类型错误，在 Git 观察／报告创建前就解析失败。

读取器拒绝存在状态／字节变化、非普通或链接输入、intent／配置不符，以及已声明工作树终态的原 JSON 结果。已保留终态须用普通维护，不能删它来满足此入口条件。

恢复复用原 recover 对真实检出、共同仓库、分支、自有锁、base 祖先、解决后 HEAD／index、未暂存修改、声明产物的检查。解锁前后复核所选证据，新报告保留原 intent／结果字节，写 `original_outcome: unknown`，只报告本次恢复结果。不补完中断命令、不恢复丢失原索引。暂存仍须提交并标准检查。

cleanup 已删检出时，使用明确允许缺失的 cleanup 计划，不能声称恢复检出。真实 Git 测试在创建、补丁应用、清理锁定后终止 CLI，覆盖缺失／空／截断、终态拒绝、身份变化、intent 路径被占。这是有限进程中断验证，不保证断电或并发事务。fetch 中断见下文；intent 缺失、metadata rebind、PR／合并编排不属此合同，也不增加治理／同判保证。

## 清理中断的 fetch

公开 beta.8 的 `cleanup-fetch-interrupted` 使用独立 intent。start／reconstruct fetch 前先核源码政策／报告产物登记，在 `REPORT_PATH.fetch-intent.json` 发布不可变 `chrono-worktree-fetch-intent/v1`，沿用同步、不覆盖发布，失败阻止 fetch。完整结果分别引用它和后续检出 intent。

fetch intent 绑定源码提交／登记摘要、源码／配置身份、remote、target ref、调用 token、精确临时 ref、最终结果路径。发布时还没有观察到 fetch base 或结果，因此不写。停止的调用可能留 ref，也可能已删 ref，但最终报告仍缺失／空／不完整。调用方必须先确认旧调用停止。

```sh
.chrono-harness/bin/chrono-worktree cleanup-fetch-interrupted \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/interrupted-fetch-cleanup.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-fetch-interrupted",
  "intent": {
    "path": ".chrono-harness/state/worktrees/start-INVOCATION.json.fetch-intent.json",
    "sha256": "ACTUAL_FETCH_INTENT_SHA256"
  },
  "result": {"presence": "present", "sha256": "ACTUAL_PARTIAL_RESULT_SHA256"},
  "head": "CURRENT_EXPECTED_FETCH_COMMIT_OID",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_RETAINED_COMMIT_OID",
  "allow_absent_ref": false
}
```

result 严格形式与检出中断恢复相同，只有实测缺失才用 `{"presence":"absent"}`。保留终态拒绝此入口，须用普通回执维护。不按目录／ref 前缀发现 intent 或所有者。

复用普通 cleanup-fetch：要求直接临时 ref、明确当前预期 OID、另一个保留该提交的本地分支，按预期 OID 删并核缺失。只有明确 `allow_absent_ref: true` 才接受已缺失。

保留原 intent／结果字节，在处置前和最终保留检查后复核，报告 `original_outcome: unknown`。效果后失败保留真实删除状态和进程，不回滚或编造原 fetch 结果，也不解锁／删除原调用后来可能创建的检出。

真实 CLI 测试覆盖 fetch 后／ref 删除后终止、start／reconstruct、缺失／空／截断、明确重试、身份／保留变化、符号 ref、终态、发布路径占用、删除期间证据变化。可用回执和 intent 都缺失、PR／合并编排仍不支持。不保证并发原子、断电持久、治理或同判。

<a id="remote-branch-retirement"></a>

## 退役远端工作分支

历史公开二进制与示例宿主的退役验收观察见[原 PR](https://github.com/ChronoAIProject/chrono-harness/pull/61)及其发布记录；这些观察不证明当前候选已经验收。

`cleanup-remote` 沿同一登记配置和进程报告，使用明确计划：

```sh
.chrono-harness/bin/chrono-worktree cleanup-remote \
  --host-root . --config .chrono-harness/worktree.json \
  --plan .chrono-harness/state/remote-cleanup.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "cleanup-remote",
  "branch": "integration/finished-task",
  "head": "EXPECTED_FULL_REMOTE_BRANCH_OID",
  "expected_url": "https://example.org/owner/project.git",
  "retained_ref": "refs/heads/dev",
  "retained_commit": "EXPECTED_FULL_TARGET_OID",
  "retention": "same-tree",
  "allow_absent_ref": false
}
```

源码登记须声明工作前缀和目标分支，retained_ref 必须是另一个目标分支。配置 remote 须解析出唯一 push URL，且与计划相等。两远端 ref 都在该 URL 实测，不读 remote-tracking ref。目标在本地／远端均须等于固定 retained commit，满足 ancestor 或 same-tree 保留。

对象须已在本地，此入口不猜或 fetch 缺历史，不推断落地、PR 状态或测试。push 只点名该 URL 和完整分支 ref，关闭跟随 tag 和 submodule push，要求 `--force-with-lease=REF:EXPECTED_OID`；分支已变时 Git lease 保留它。本地工作树／分支、其他远端 ref、Git 配置不作处置目标。

操作前后核计划／已提交政策、端点和保存工作；符号、意外、重复、格式错误的远端 ref 宣告失败。成功要求 push 成功、实测分支缺失、保留／计划／端点复核都通过。

报告 `remote_branch_removal` 四状态与本地同义，`remote_branch_removed` 只对本次核实删除为 true。失败命令可能已删分支，原字节／退出仍保留。明确允许缺失重试时重新核保留和缺失，不重写旧结果；治理／同判仍未建立。

lease 只保护被删 ref 的预期 OID。目标／URL 观察和本地检查不与服务器构成原子事务，不排除全部并发／瞬时变化，也不补齐 Git config、hook、transport、credential、OS 输入。目标在 push 中变化，可能删除后最终检查失败。

中断结果未知，调用方确认已停止后可用允许缺失计划；不推断远端恢复 intent、服务器锁、PR 授权或合并凭据。配置传输仍可能需凭据，不把含秘密 URL 写入保留计划／报告。

真实 bare remote 消费者覆盖保全本地脏工作、无视配置 push refspec 而明确选端点、squash 保留／缺失重试、端点／分支／保留拒绝、符号 ref、并发分支更新、删除后原失败、删除后目标／计划变化。

<a id="explicit-metadata-rebind"></a>

## 明确重建 Git 元数据关联

公开 beta.11 提供 `inspect-rebind`、`rebind`，在 linked 检出 metadata 缺失或不可用时修复 Git 关联，包括旧回执全丢。AI 提供已有分支、精确当前提交、希望采用的 index tree 和每个路径，不推断旧索引／原操作结果。健康关联使用普通维护。

```sh
chrono-worktree inspect-rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/rebind-inspection.json
chrono-worktree rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/rebind-plan.json
```

检查计划是明确数据，OID 换真实 commit／tree；index_tree 声明要安装的索引，可表示所选暂存工作，不必等于 HEAD。

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "inspect-rebind",
  "head": "<existing branch commit OID>",
  "binding": {
    "path": "../orphan checkout",
    "branch": "feature/recovered-work",
    "index_tree": "<chosen Git tree OID>",
    "metadata_id": "orphan-checkout",
    "backup": ".chrono-harness/state/original-metadata",
    "donor": "../fresh-donor",
    "expected": null
  }
}
```

inspect 报 `observed` 和 `operation: rebind` 的 `proposed_plan`，含实测 expected 身份。执行前将精确 proposed_plan 存到登记 state。expected 含可空 gitfile 身份（sha256、size、权限 mode）、可空 metadata 摘要、可见子树摘要。报告保留范围项原路径字节、普通文件内容摘要／大小／mode、字面链接目标；只排除目标根 `.git`。

此文件系统观察只核明确保全范围，不登记文件或推断测试依赖。inspect 写状态报告，不改目标检出。

目标须是物理目录，在源码／共同 Git 目录及其他检出外。分支使用登记工作前缀，在声明 HEAD 存在，不能归其他检出。旧 metadata 成员由同一共同仓库 `worktrees` 下明确选择，父目录自身可缺失。

可用旧 gitfile 指针和 metadata backlink 须与所选目标／成员一致。相对指针按所在目录词法解析，不把符号链接别名视为同一物理所有权。metadata 存在时，至少一条可用链接须识别它。旧回执丢失不授权其他检出 metadata。拒绝根 `.git` 目录／链接及保全范围内特殊文件。

backup 须是登记 `.chrono-harness/state/` 下尚不存在的路径，物理父目录已存在。donor 为父目录已存在的新目录，不重叠保护路径；其 basename 不能是 metadata_id 前缀，避免 Git 复用旧分配。

执行前复核源码／配置／分支／计划和实测身份，发布不可变 `chrono-worktree-rebind-intent/v1`，含精确计划 hash、预期输入摘要。复制旧 gitfile 字节／mode，把残留 metadata rename 到 `backup/metadata`，创建 locked detached donor，使用 `--no-checkout`。

之后设声明分支／索引、发布 donor 指针、Git repair、核关联、删空 donor、解锁，不通过 checkout／reset 写原工作文件。最终还核可见子树和保留备份。

`rebound` 只表示声明关联／索引已核且观察工作保全，报告 `index_origin: explicit-plan`、`original_outcome: unknown`、`governance: not-evaluated`、`parity: unestablished`。暂存／未暂存仍由调用方在标准检查前处理。不证明历史索引恢复、输入完整或候选通过。

普通失败保留阶段、原进程／退出、不可变 intent、backup、部分效果；不要对非空 backup／donor 盲重跑。Git repair 已附着目标且原锁仍在时，可由 AI 协调后用回执绑定 recover 解锁，保留失败报告、backup、剩余 donor。下文 resume-rebind 还支持附着前失败和结果缺失。

AI 须先确认旧进程停止，协调证据冲突。backup rename 要同文件系统，跨设备失败不删旧 metadata。不保证并发隔离／崩溃／断电原子性；身份观察不保留时间戳、ACL、xattr、硬链接结构。

<a id="continue-an-interrupted-or-failed-rebind"></a>

### 继续中断或失败的 rebind

`resume-rebind` 从公开 beta.15 提供，消费已有 `chrono-worktree-rebind-intent/v1` 及其 plan_path 的精确原计划。继续计划须有自己的登记 state 路径，不猜新索引、backup、donor 或所有权。

```sh
chrono-worktree resume-rebind --host-root ROOT --config .chrono-harness/worktree.json --plan .chrono-harness/state/resume-rebind.json
```

```json
{
  "schema": "chrono-worktree-maintenance/v1",
  "operation": "resume-rebind",
  "head": "<original branch HEAD>",
  "intent": {
    "path": ".chrono-harness/state/worktrees/rebind-INVOCATION.json.rebind-intent.json",
    "sha256": "<observed original intent digest>"
  },
  "result": {"presence": "present", "sha256": "<observed original result digest>"}
}
```

只有实测结果缺失才用 `result: {"presence": "absent"}`，空／部分字节用 present。完整原失败须匹配 intent，保留 `original_outcome: failed`；缺失／部分为 unknown，已完成成功拒绝。原 intent、plan、result、metadata 不改，新报告保留输入身份及原进程。

初次／继续共用保全和附着代码，核源码提交、登记、政策、计划字节、分支 HEAD、明确 index tree；可见文件字节／mode／字面链接仍须匹配原观察。

完成缺少的指针保留或 metadata 迁移，只复用原 locked donor，从声明树安装缺失索引、附着目标、删空 donor、释放自有锁。已有索引必须已匹配，不覆盖索引／可见工作强行凑一致。

目标已附着且解锁时，只有精确分支／索引／原 backup、donor 缺失才接受。继续调用自身中断后，确认其进程停止，可对未改原 intent／result 再试。

冲突 backup、可见工作变化、donor 文件、其他锁／分支、外来 metadata、过时计划／结果／配置、分配歧义，都保留并在继续效果前拒绝。中断留下的 `index.lock`／`HEAD.lock`、不完整 donor metadata、临时文件、写了一半的旧 backup，须 AI 明确协调，工具不丢字节或猜含义。

保留指针发布使用原子文件替换，backup 迁移仍须同文件系统。intent 或命令不证明旧进程停止、并发排除、断电原子或历史索引恢复。

真实 CLI 测试在保全前和 donor 创建、分支／索引准备、指针附着、repair、解锁周围终止生产者，覆盖缺失／部分／失败结果、重复中断、身份变化和完成结果拒绝。rebound 只核当前关联与保全，不是候选检查或治理通过。

### 释放或删除前核真实文件

源码检出保护复用 runner [字面文件／索引比较](git-facts.md#literal-checkout-identity)。创建、重建、协调恢复、清理都拒绝字节、链接表示、所有者执行位变化，即使 Git 配置报告干净。

清理保全未保存工作，恢复保留锁直到真实文件匹配明确 index tree。原产物排除、已保存提交检查继续适用。此变化从 beta.11 分发，不证明 Git 配置完整或并发删除安全。

<a id="declared-artifact-disposal-before-checkout-removal"></a>

## 先处置声明产物，再移除检出

cleanup 把明确生成目录处置与有限时 Git 检出移除分开。删任何目录前先核全部所选项：两端登记未跟踪产物、不含候选已跟踪路径、缺失或无符号链接祖先的物理目录。原保存工作、分支、检出、未知文件、嵌套工作树检查继续适用。

产物处置明确区分时机：`dispose` 在消费者释放后的普通缓存回收中可删除；
`finish-dispose` 用于跨操作复用的缓存，普通 bootstrap／check／use 返回及 maintain 保留，
在明确 finish 或实际检出处置中才选择；检出继续复用时不因创建年龄回收。`retain` 保留，
`evidence-retain` 仍须符合原证据释放条件。所有时机都须核原所有权和活消费者。
主检出的 finish 持久记录缓存释放代际，中断后由同一维护入口恢复；新消费开启新代际，
旧释放不会授权删除后来复用的缓存。

处置在自有 worktree 锁下运行，解锁／Git 移除前再核字面检出和保留。计划不发现可删目录。`artifact_disposals` 逐项记 `already-absent`、`attempted-unverified`、`verified-absent`。

文件系统阶段归调用方宿主 job 生命周期，不继承 Git 子进程期限。失败可能已部分删除授权产物但保留锁；解决原因后，用已有回执／intent 恢复，再明确重试。之后 Git 失败仍保留原进程及已完成产物效果，均不改写为检出移除成功。

这解决构建输出耗尽 Git 移除期限、留下部分删除检出的问题，不保证并发／断电原子，不恢复旧 Git 移除已部分删的检出，也不保证任意已跟踪源码量的删除时限。此变化晚于 beta.11。专属真实 Git 消费者不靠耗时而核删除顺序、处置前拒绝已跟踪内容、保全内部链接目标、后续 Git 失败及重试。

公开 beta.11 匿名 macOS 二进制曾安装到升级混合宿主克隆及其自有检出。旧回执移除、索引损坏后，明确 inspect／rebind 保全暂存／未暂存、原二进制字节、执行位、字面链接、所选索引、原 metadata，原结果仍未知。保留工作提交后，登记 cleanup 核检出／分支移除。此证据不含中断 rebind 继续，见[示例采用](examples.md)。

<a id="opt-in-automatic-lifecycle-cleanup"></a>

## 按需采用自动生命周期清理

源码在原 v1／v2 工作树配置增加 `automatic_cleanup`，值为 `.chrono-harness/` 下已跟踪、FILEMAP 登记的政策路径。省略仍走明确维护。不提供 daemon、定期空闲触发、年龄到期、兄弟目录扫描或语言推断。主宿主调用登记 `.chrono-harness/bin/chrono-worktree`，使用新配置前先部署兼容候选工具。

本仓选 `.chrono-harness/cleanup.json`，其 v2 保留这些 v1 字段：

| 字段 | 要求 |
| --- | --- |
| `coordinator_root` | 精确绝对物理 Git 检出根，或交给已有 Git inventory 的明确 `git-main-worktree` 选择器；协调者须在处置后存活，属于同一共同仓库 |
| `state_directory` | 协调者 `.chrono-harness/state/` 下登记未跟踪目录 |
| `retained_ref`、`retention` | 一个直接本地分支，使用原 ancestor 或精确 same-tree 保留含义 |
| `remove_branch` | 明确选择；本仓保留分支 |
| `allow_evidence_disposal` | 是否允许终态所有者另选证据处置；本仓禁用 |
| `artifacts` | 精确 config.artifacts 目录，各选 dispose／finish-dispose／retain／evidence-retain，不按 kind 名或布局解释；未列路径继续保留 |

调用宿主和协调者分别核对自己的已提交政策与配置，不要求两份文件逐字相同。共享状态只要求所有权协议、物理协调者和状态目录相同；每个工作树的清理选择、保留条件和实际产物登记归自身负责。共享保留生产者仍由其物理存储所在的协调者政策负责，linked 工作树不能改写该权限。本仓保留 bin 和 state，因此普通 finish 回收缓存、保留检出。整检出移除前，证据所有者须明确改政策并迁移登记项；改政策不会静默重释旧项。

成功 start／reconstruct 自动登记精确 linked 物理关联，在存活协调者保留 birth 观察。创建失败、状态缺失、所有权未知都受保护。v2 已登记但 birth 未封存时，普通入口先完成准入／关联排除、核原政策，再协调；原结果缺失／部分仍未知，不追认成功创建或终态交接。

创建前两入口核政策／身份，只维护点名目标；跨目标清理由明确 maintain 负责，无关工作树／fixture 不作为创建前置。目标身份或状态发布失败仍阻断目标。

固定命令默认 `--host-root .`、`--config .chrono-harness/worktree.json`：

```sh
chrono-worktree use --path /exact/worktree --operation test.example
chrono-worktree finish --path /exact/worktree
chrono-worktree maintain
chrono-worktree status
chrono-worktree status --path /exact/worktree
chrono-worktree import --path /exact/existing-worktree
chrono-worktree migrate --path /exact/enrolled-worktree
```

显式 `import` 按调用宿主当前已提交的政策接管保管责任，同时逐字核对并绑定目标原提交中的配置与政策输入；不解析目标的退休配置格式，也不改源码、HEAD 或索引。未提交的政策输入仍拒绝导入。目标自己的政策采用归 `migrate`，普通 use／finish／清理继续严格解释现役格式并核对原绑定；导入不授予绕过升级或迁移的权限。`status --path` 同样只观察本地状态，不采用目标政策。

复用同一物理工作树切换分支后，也使用上面的 `migrate`。创建时的 `branch` 和 birth 原件保持不变；当前采用的分支单独由 `checkout_branch` 保存。普通 use／check／finish 不自动跟随分支变化。迁移持有该工作树原来的独占租约，核当前 HEAD、Git inventory、物理附件、Git 锁及已提交政策。候选资格核对通过后，沿现有原件所有者核验并交接本工作树已封存的完成结果，再协调已静止的中断 use，最后核无待处理项目；不要求先维护所有登记工作树。中断 use 沿原 intent 核物理附件及租约绑定，保留原结果或其缺席，命令结果保持 unknown，任务完成和终态交接均为 not-claimed。活跃租约、缺失／漂移 intent、未通过原件核验的完成结果和终态工作继续阻止分支采用。birth 工作树仍只采用登记的工作分支；显式 import 保留原有本地分支／detached 合同。

`cleanup` 保存上次清理的观察，不单独表示仍有任务运行。取得原独占租约并通过上述检查后，先前的等待或缓存回收记录不阻止分支采用；迁移保留这些记录、缓存回执和原件，不补写任务完成，也不清理缓存。use 协调和已封存结果交接各自保存已证实的结果；后续采用失败不撤销这些观察，也不改写旧报告。

采用仅更新当前绑定和一次缓存代际，不切换分支、不写 index／源码／ref，也不声明任务完成或清理资格。只换分支不追加政策迁移历史，重复采用不增加代际；政策同时变化时沿现有政策回执保留旧新输入。升级前须排除旧所有者和全部实际使用方，正式部署理解 `checkout_branch` 的工具后再采用，不能让旧二进制读取已采用的新状态。创建／续接新任务仍由调用方保证最新 dev 和原检查义务；完整语义读写范围、index／ref 冲突协调和分项交接尚未实现。

### 本地状态与 AI 提醒

`status` 从协调者已有的逐工作树账本和 Git inventory 观察本地状态。默认查看该协调者全部登记地址，包括物理检出已缺失的地址；`--path` 只查看指定地址。查询不创建协调者状态或登记目录，不导入、清理、恢复结果交接或补写任务终态。结果沿用生命周期报告及其现役保留政策，不另建成员清单或永久快照。

查询持有共享部署租约，读取时核对已有记录的身份和原字节。维护持有元数据的目录或文件独占租约时，仍可观察已有账本。命名空间身份或政策输入变化时查询失败；单条 `owner.json`／`ledger.json` 缺失、损坏、身份不符或读取期间变化时，保留错误并报告未知，不等待维护结束或制造空账本。

报告的 `worktree_status.worktrees` 分别给出 `work_state`、`cleanup_state`、`attention_required`、`next_action`，并引用已有终态和清理原件。`active` 表示任务尚未交接，不能由空闲、缓存已清或构建成功推断完成。`terminal` 配合 `retained` 仍有保留事项；只有已有终态交接、账本记为 `disposed`，且物理路径与 Git 登记都已核实缺失，才停止该对象的清理提醒。长驻协调主检出正常时无需终结。

无当前登记、记录损坏、附件变化和记录与检出不一致分别显示 `unregistered` 或 `unknown`，保留原件并提醒检查。单个损坏账本不会遮住其他可观察工作树；namespace 本身无法完整枚举时，结果另列未知项，不能据此声称清单完整。观察是本次快照，不授予之后删除或清理的资格。

采用 automatic_cleanup 后，start／reconstruct、use／check／bootstrap、import／migrate／finish 在原报告中附当前目标状态；maintain 附协调者范围状态。未完成或清理待办以 `W_WORKTREE_ATTENTION` 写 stderr，stdout 仍为与存盘原件相同的完整 JSON，提醒本身不改变退出码。普通入口不读取其他工作树账本。AI 开始及交接工作时运行 `status`，根据下一步提示保全工作、解决保留条件，并在实际完成及加入消费者后执行 finish；提醒不代替这些义务。

提醒分类由 `verified_attention.rs` 同源 Verus 判断，真实 Git／文件系统／JSON 适配与行为测试另验；证明范围见 [Verus 说明](verus.md#本地工作树提醒)。

### 按创建年龄复用和回收

v2 政策可显式选择 `age_policy`，例如
`{"reuse_after_seconds":7200,"delete_after_seconds":14400,"preserved_branch_prefix":"cleanup-preserved/automatic/"}`。
年龄取物理检出的创建时间，严格大于阈值才具备资格；边界相等、创建时间不可读或在未来时保留。
维护在选择和执行前采用目标检出自己的已提交政策；协调者的年龄阈值或未选择年龄政策不替代目标选择。
主检出、当前调用者、活跃或未知使用方仍受原租约和身份检查保护。

普通 use／check／bootstrap 保留正在复用的工作树与缓存，不因创建年龄先保存源码或回收缓存。
显式 maintain 或明确 finish 后的维护在达到复用年龄时，先把当前源码、原索引和 checkout reflog 的提交保全到真实分支，
再回收明确登记的普通 dispose 产物；检出继续复用时，finish-dispose 缓存保留，
仅当前代际的明确 finish 释放可以选择它们。达到删除年龄后，同一维护入口可回收检出及其 finish-dispose 产物；
保留的 bin、必要证据或其它未解决条件仍会延期删除。源码保全包含未跟踪和忽略的源码，
生成目录只按已有产物登记排除，不按目录名猜测。保留分支已存在时必须核对原提交，不覆盖。

年龄回收不声明任务成功。原保全、处置意图和中断结果继续保留，重入核原身份并发布独立结果。
配置须由能理解该字段的所有者处理，已有登记按原 migrate 入口采用已提交政策；
源码具备能力不代表其他协调宿主已经升级或采用。

### use 与 finish

use 只消费一项已登记项目／脚本操作，沿用 tool、argv、环境、进程限制。在与 finish／处置共用的目标 metadata 租约下记受管使用，调用已有有限进程模块，保留原字节／退出，子进程等待结束后释放使用。子进程非零仍失败，即使用途已安全释放。

生命周期结果在存活协调者发布；即使 wrapper 从待处置检出调用，释放后也无结果写入者留在那里。其他自有 gate 忙时，准入／释放在登记进程上限内等待；中断 gate 保留，不按时间过期。

活 use 阻止 finish。wrapper 终止、结果缺失、所有权未知保留使用记录，不按年龄清除。已有进程模块终止自有进程组内残留后代；逃出所有权的 detached／后台写入不支持，调用方须等待。Git 锁是额外检查，不替代 use。

finish 是调用方等待全部自有 job、确认完成／落地后的明确终态交接。运行时观察登记物理关联、当前 HEAD／分支、允许保留 ref，发布不可变终态回执，立即执行同一清理。不创建源码 HEAD 保留 ref，不判断 PR／合并成功。

可选 `--retained-commit OID` 在 ref 移动前核调用方真实 squash 落地身份；观察 OID 总会固定到回执。后续 ref 变化明确失败，不猜替代落地。

`--artifacts-only` 只请求清缓存，保留检出／ref。未解决 evidence-retain、未保留提交、脏源码或其他保留产物，也会给出明确保留理由，只允许处置所选、无活消费者的输出。成功缓存清理项变 `retained`，后续每次 start 不反复扫描。

后续明确 finish 只有政策允许时可用 `--dispose-evidence` 解决证据，旧终态回执留历史。终态交接后拒绝新 use。finish 若从被登记检出自身运行，清理排除该源码，之后从协调者明确 maintain 才处置。

整检出移除委托已有 Cleanup，含字面干净检查、两端产物白名单、精确保留、物理身份、worktree／Git 锁、嵌套检查、可选预期 OID 删分支。共享产物阶段还保护产物前缀内暂存源码，拒绝符号链接根／祖先，内部链接只删链接、不跟目标。

自动报告记录处置前字面项长度及之后核实零字节，这是逻辑字节，不是分配块节省。旧明确维护保留原报告形状。

### 主检出只清缓存

可选 v2 `main_cache_only: true` 通过同一所有者，在明确 import、普通 use、标准 check、已采用 bootstrap 为物理 Git main 登记。main import 只建立当前缓存保管，不执行受管命令或宣称历史生产。之后 finish 取得原独占登记租约，保留真实主缓存释放，包括尚无输出时。未选此项仍只允许 linked import。

登记、use intent、共享内核租约、原进程结果、释放、缓存尝试都使用已有状态。main finish 释放缓存并保持 active，绝不授权删检出、分支、索引、源码、bin、证据或提交；拒绝证据处置／落地 pin 标志。

已有登记输出若没有已采用消费者释放，仍明确作为历史对象保护。在受管生产前，登记可取得逐项声明的缺失输出所有权；这不迁移当前宿主历史 target／生产者。

历史输出可由同一所有者明确接管当前保管：`migrate --path /exact/main --adopt-cache STATE_PATH`。登记状态下严格 `chrono-main-cache-custody/v1` 声明含物理 main path、完整当前 head、精确 artifacts、`current_consumers_released: true`、非空 reason。

当前操作者须真实等待或排除旧未受管消费者，再声明当前保管决定。这是工程切换，不声称过去生产者已完成、成功、封存或释放。只有 active 已采用 main 项及登记 dispose／finish-dispose 输出可接管；源码／index、bin／证据、未知路径、活的已采用消费者都拒绝。未选历史对象继续保护，不否决合格兄弟项。

切换由准入和原独占登记租约保护，独立不可变回执保留原声明／关联、记录所有权变化，历史生产结果仍 unknown。历史生产／处置结果保留。已有 reconcile_uses 可追加中断历史、去掉失效 use token，因此可变登记／uses 不保证逐字不变。

回执发布但账本未更新，可按同绑定重试。重复接管已受管路径不增历史、不处置。原 pending 缓存代际和普通清理在排除后回收新受管输出，不设 finish、不加清理器。后续 bootstrap／check／use 共用租约及中断恢复。采用前部署兼容所有者；旧读取器拒绝非空 `main_cache_adoptions`。

### 各地址独立维护

main 再进入时，先清已释放的 pending 缓存代际，再启新消费者。正在进入的 linked 检出保持排除，直到新消费者取得保护；释放后其他普通入口可恢复该检出。安装、测试、缓存保存和其他登记操作都须持同一 use 保护，真实共享描述符保护活后代及重叠消费者。不增加检查后 Cargo clean。

每次删除前，处置模块重新核 HEAD、物理关联、政策、实时索引。分别报告逻辑长度和分配块（`st_blocks * 512`，含目录块），物理释放未知，块差不证明文件系统节省。失败保留原错误和实测残留，重试保留原尝试。

每个工作树在 cleanup 状态目录拥有 `enrollments/SHA256(absolute-path)/ledger.json` 和绑定身份的 `metadata.lease`，只记录该精确地址历次物理关联。普通更新不写其他工作树记录。清理一直持本地 metadata 排除和目标独占能力，贯穿物理效果及原结果发布。

没有总 ledger.json、全局准入租约或围绕独立 IO 的释放／重载合并。无关所有者忙／损坏只保留并报告，不拒绝有效目标；明确 maintain 报失败并继续独立可用项。

普通 start／reconstruct／use／check／bootstrap／finish 只维护目标，诊断 inventory 限实际来源／目标，不先扫独立工作树或 fixture。明确 maintain 分别访问声明登记和生产者，不是无关工作前置。这去掉普通入口随工作树数量增长的遍历，失败／缺结果留在责任所有者记录。

不可变 `namespace.json` 绑定物理 Git 所有者和执行物 `deployment.lease`，无可变成员表。普通所有者共享该租约；只有替换共用执行物时才独占，并在发布前排除真实受影响项，保护共享工具而不串行普通更新。

部署核协调者当前政策／缓存所有权，其他被排除项绑定物理关联、HEAD、原租约。各自独立政策改变不阻止工具发布，但下一次普通 use 仍核政策／配置，需要时明确采用。部署不改它们的政策／账本。

维护只枚举声明登记 namespace，并核各明确地址；未知数据不授权处置。普通入口拒绝旧总状态，无兼容读取或回退。当前保管者须在原排除下保留、拆分活状态，再切换工具代际。

**产品与宿主采用曾同步修改：**登记存储及现役消费／测试绑定迁移，原生命周期／bootstrap 命令、保留窗口、处置权限不变。受影响核心、证据保管、政策迁移、所有者部署行为须过登记检查；真实结果前构建／迁移／总运行成本未测。

### 保留生产者与明确释放

可选 v2 `retained_producers` 点名精确生产者 ID、证据目录，以及相对每份明确回执的生成输出。已采用 `ci-command-result` 归 `ci-tests::behavior::retain_command_result`。

Rust `RetainedArtifact` 接纳精确保留对象并发布 intent。采用 wrapper 先复制原 stdout、stderr、binding、configuration、state，再 begin，封存生成目录身份及原流／binding 摘要。begin 前中断或准入失败留下的未登记副本作为未知证据保护；已准入未封存回执也保护。

消费者各有独立明确 acquire／release 回执，丢 handle 或进程消失不释放。已有清理只读该家族明确 `producer-records/` 下生产者写的登记，每项声明精确 artifact、owner、引用，不扫产物目录发现成员。

发布通过 `RetainedArtifact::registered_store` 解析生产者，与清理共用已提交 worktree 配置、cleanup 政策、登记和物理 Git main 协调者检查。声明查找只读政策，不取得维护准入、不初始化账本，因此活维护租约不阻止查找。最终 HEAD／配置／政策稳定检查仍适用，实际发布／处置各有原登记和所有权保护。

命令 fixture 使用自己的已提交 receipt owner 政策，生产检出与被测命令根分开传；linked 生产者仍在自己的 main 协调者声明目录发布。`CHRONO_CI_TEST_RECEIPTS` 与 judge-ci 诊断共用调用方证据目录语义：向该目录写原件引用及路径、摘要和长度，不通过私有 Git mock 对源码检出作生产者准入，也不复制原件到第二存储。引用写明实际 producer、store、artifact 和释放责任；独立 diagnostic 引用及其原 owner 保留到调用方完成调试／验收。底层物理 store 仍须精确匹配已提交声明，未声明、重叠、不匹配、符号链接位置继续拒绝。解析或导出不释放消费者，也不授权删 main／分支。旧 linked 存储没有已采用转发路径，继续作未知证据保护。

本宿主只接纳两个复制的独立 Cargo 输出 `original-state/cargo/producer/target/`、`original-state/cargo/tests/target/`，生产者还要求精确源码产物声明。所选 expected-exit 断言消费原结果后发布 release；意外退出、发布不完整、活诊断引用都保留副本。

全部引用明确释放后，所有者可回收生成副本；stdout、stderr、binding、配置、源码、恢复原件、其他 state 仍是证据。无 expected-exit 合同的旧调用方保留未释放引用。未知旧回执及未登记对象全部保护，整个 state 不因此可删。

兼容已提交政策／工具部署、真实宿主迁移、原生 CI 缓存保存参与、完整 SPEC 采用仍不在本实现验证内。

协调账本在效果前记录原身份、终态回执、尝试报告路径。失败保留原字节／摘要及部分效果。普通重试只可在精确未变检出协调记录自有锁，再走允许缺失的 cleanup。

结果缺失／中断、关联／HEAD／政策／ref 变化、其他锁或源码损坏，均保留供明确协调。重试不覆盖原失败。存储失败、未登记并发写入、断电不保证事务；失效准入／use 记录须调用方等待原 job 后协调，v2 内核恢复的明确范围见后文。

import 为一个明确已有 linked 工作树登记，核当前登记／关联并绑定原政策输入，含明确缺失。它接受其他客户端已有的本地分支名，也接受 detached HEAD；后者以 Git 保留名 `HEAD` 登记，持续核 inventory 的 detached 标志、当前提交和原物理关联，不获得删除任何本地分支的权限。feature／integration 前缀约束仍用于新建 birth。已导入对象的自动清理消费同一 enrollment 及其物理身份保护；直接 cleanup 计划仍要求登记前缀，不接管未登记的 detached 检出。可导入旧检出而不编辑它，也不假装它采用新自动政策。未解决索引、活使用方及必要证据继续保留；年龄源码保存只更新该 detached 检出的 HEAD，先保全原索引和 reflog 提交，不改其他分支。处置路径须已在两端声明。观察为 import，不声称历史 birth；已有空地址记录也不能充当历史登记或完成证据。

观察登记要求目标原政策输入已提交并有登记；目标无需先采用调用宿主的配置格式或全部产物范围。后续迁移、消费和处置各自重验当前输入，实际处置仍要求路径在两端明确登记。

调用方 Git／GitHub、发布、立即回收旧宿主、证据结算仍不属本生产者。worktree fixture 用已有 `resolve_program` 从继承的 PATH 解析 Git，找不到程序就失败；原绑定在本轮 fixture 内记录实际路径、身份和版本，不在仓库维护主机路径清单。产品宿主的 Git 政策独立；剩余真实 Git fixture 的命令边界 mock 迁移仍未完成。

本地真实 Git fixture 覆盖整检出移除；真实 main finish／落地仍是调用方采用后的验收事件。

### 明确迁移政策

`migrate --path /exact/enrolled-worktree` 为一个已有 v2 内核登记项采用该目标自己已提交的 cleanup 政策／worktree 配置，可从目标或存活协调者运行。要求原准入及独占登记租约、同一物理 Git 关联、已核 birth 证据、无外来 Git 锁和目标产物登记。协调者无需先复制目标的清理选择。

政策输入地址、协调者、状态目录、所有权协议保持固定，不支持在此迁移它们。旧版／缺内核所有权、待终态处置保持保护，政策改变不隐含迁移。

两端宿主配置不同时，各自保留登记来源／输入。迁移绑定目标的 cleanup 政策、worktree 配置和固定提交；普通维护也按该目标的登记选择产物。协调者的登记只约束其自身对象和共享生产者，不替独立工作树登记产物。

没有真实生产者的输出不造登记。目标未采用新绑定时保持原保护，不能由协调者的新政策静默改判。此命令不改 Git；原提交／结果继续保留。

原登记、birth policy hash、birth receipt、uses、旧清理尝试保留身份。独立不可变迁移记录旧新绑定、原字节、观察提交，再次核新绑定后才向账本追加回执。普通 use／cleanup／finish 核整条链，使用最后绑定；回执或原输入缺失／改变，拒绝后续 use／处置。

中断移除后物理检出已消失时，候选程序从共同 Git 仓库读取该对象最后采用的固定输入，核配置及政策的原摘要，只从配置读取 `host_config` 和 `automatic_cleanup` 地址，再核登记和所有权命名空间后恢复政策。政策中的 `retained_producers` 不参与该对象恢复；共享生产者由现役协调者的声明负责，其余清理字段继续严格解析。未消费的执行配置不解析，也不执行历史程序。协调者后来采用的不同清理政策不能重释这一待完成处置；原件保护和删除条件继续适用。

尚未显式迁移的 import 采用调用宿主的政策，目标原提交和原始输入绑定仍单独保留。缺失路径恢复时，程序持有导入原报告的读取保护，核对保留所有者的封存、实际摘要及物理身份，再核报告类型、成功状态、协调者、原报告路径及配置地址。只从该原件的 `policy_source_commit` 读取采用来源，以 `config_sha256` 核配置，以登记的政策摘要核 cleanup 字节；缺来源、未封存、被替换或摘要漂移均保留并报错，不猜历史来源。显式迁移后的 import 与 birth 继续使用最后的目标绑定，不改写原导入证据。

采用前部署兼容工具，旧读取器拒绝新增非空 migration 字段，不误作旧绑定。迁移不处置、不更新 Git、不作终态交接，保留源码、输出、任务状态。

active 项开新缓存代际，retained 项仍 retained，后续终态处置需新明确 finish。已静止中断 use 保留协调记录。绑定未变重复请求不增历史；迁移已发布但账本未采用时，重试复用原件，其存在不证明采用成功。unknown／未封存 birth 须先按原政策协调，再迁移。

<a id="kernel-ownership-and-unfinished-cache-recovery-v2"></a>

### v2 内核所有权与未完成缓存恢复

v2 使用 `chrono-worktree-automatic-cleanup/v2`，保留原政策字段，另登记新状态目录。本仓选择 `.chrono-harness/state/automatic-cleanup-v2/`。切换时调用方须部署兼容协调者并排除不兼容生产者，不能把 v1 gate、token 或未知记录追认为内核租约。

#### 先在独立主克隆验证，再切协调者

调用方负责分阶段验证和切换。先在独立、干净 Git main clone 用正式 bootstrap／短检查验证精确已提交候选。当前 scoped 本地生产者为 `full_context: None`，无需伪造 linked origin 回执。main-only bootstrap 可初始化缺失所有者工具；这只验候选，不验存活协调者或完整／原生义务。

存活协调者的 Git 合同阻止候选准备时，也只用这一登记适配路径：独立 main clone 从最新 dev 建 fresh integration，在负责处审计并修正实际合同，提交干净候选后运行正式 bootstrap／短检查。

若仅为修复前失败探测在此临时克隆初始化过所有者二进制，须先等待全部自有任务，核工具身份及无消费者，再保留到安装位置外，让同一 main-only bootstrap 初始化候选所有者。

这不允许替换活的／外来协调者、验证中改预期摘要、伪造 linked 回执或跳过政策相等。仍须用原生 macOS-26 验精确候选，核实际 Git 观察，再部署匹配的已提交政策／参与声明，之后通过普通 start 创建三份真实 linked 工作树。

真实切换时，等待／排除不兼容生产者、保留 v1 状态，在存活 main 协调者装兼容工具，提交并部署匹配 v2 worktree／cleanup 政策、所选宿主配置和 participation，再开始 v2 linked 工作。

之后才通过普通 start／reconstruct 自动登记 v2，并在新 linked 检出用原公开 bootstrap／check。协调者工作文件和已提交政策／配置须与调用检出一致；只升工具或只改 linked 政策，故意在构建／清理前拒绝。不把 v1 token 转租约。真实协调切换及原证据仍由调用方验收。

#### 内核租约如何保护消费者

每个点名登记项有稳定、绑定身份的 metadata flock inode；每代物理关联有稳定共享／独占（SH／EX）使用／处置租约。普通 use 在 intent／spawn 前取 SH，持有到结果发布；业务执行前释放准入，Git 修改子进程继承准入。

清理持准入并尝试非阻塞 EX，有活持有者就保全。最后一次 close 才释放所有权，不对共享继承 open description 发 `LOCK_UN`。身份缺失／替换、政策变化、外来 Git 锁都保全工作。

birth 在登记和回执封存期间持关联租约。生产者中断后，普通 use／maintain／finish 可在 EX 下发布不可变、身份绑定协调。发布后再次中断，先复用原字节，再把指针接到账本。恢复保留原报告及未引用封存回执为证据，`sealed_receipt` 仍缺失，写 `original_outcome: not-established`。

清理拒绝只影响自身登记项。普通 start／reconstruct／use／check／bootstrap 不尝试无关项或 fixture 清理，目标准入／所有权／状态发布失败仍阻断。明确 maintain 在 drain、不可变失败回执、cleanup_failures 保留无关失败，继续独立项；只要本轮尝试有失败仍非零，不丢旧回执。

#### 创建 agent 与待维护提醒

工作树配置的可选 `agent_identity_env` 点名当前调用方身份的环境变量；本宿主采用 `HECA_AGENT_ID`，并在工作树和各平台检查配置的 `environment.inherit` 转交该值；公开 `check` 和现役 Rust bootstrap 同时保留所有者的 stderr 提醒。普通 agent 不需要另加 CLI 参数。没有配置、缺值、空值、控制字符或超过 256 字节时，原报告的 `agent.status` 为 `unknown`，不猜用户名、PID 或分支所有者。它是调用方声明，用于归属和提醒，不授予清理权限。

start／reconstruct 的原报告和创建前 intent 保存 agent；同一地址的 birth 登记保存 `creator_agent`。后续 use、finish、maintain 和其他 agent 的操作保持该创建者。显式 import 仅保存 `custodian_agent`；历史创建者未知时仍为 null。旧记录也保持 unknown，不根据当前调用方补写历史。

相关自动生命周期入口复用 `worktree_status`。已知 agent 的普通操作读取同一协调宿主的显式 enrollment 元数据，观察目标及该 agent 创建的地址；不为提醒检查其他 agent 的物理检出。status／maintain 仍提供全部状态。每行保存 `creator_agent`、`custodian_agent`、分支、工作／清理状态、清理详情、后续动作及 `agent_attention_required`；stderr 只提醒当前已知 agent 自己的未完对象，最多显示 64 项，超出时报实际项数，完整清单保存在原报告。完成工作仍须加入自有任务并执行 finish；已交接但 retained 的对象继续提醒。

`worktree_status.agent`、`unknown_creator_records`、`unreadable_records`、`namespace_error` 和 `agent_observation_complete` 明确观察范围。身份未知时保留通用目标／全量状态提醒，并报 `W_WORKTREE_AGENT_SCOPE`；不能把这些对象认作当前 agent 创建，或把未知范围解释为没有待办。损坏记录保留错误，独立合法操作继续。提醒不设置终态、不授权删除，也不增加后台扫描器或人审门。

**产品与宿主配置同改：**这批新增身份输入和即时提醒，宿主显式选择身份来源，使用现有 ledger 和原报告，不另建 agent 状态索引。相关读数和提示的总成本待实测；旧协调宿主仍须按原配套部署／迁移边界切换。它不补齐历史创建者，也不证明终态历史、租约及空地址目录已全部有限回收。

#### 保留后的接续与清理阶段

每项登记可带 `cleanup`，分别保留 `stage`、原因 `detail` 和原 `report_path`。阶段为 `pending`、`in-progress`、`waiting-for-consumers`、`retained`、`failed`、`caches-released`、`disposed`。它与工作状态 `status`／`terminal` 分开：缓存释放不表示工作完成；原进程或清理中断仍保留原尝试，不能仅凭阶段补写成功。旧记录缺少该字段时，表示尚无此阶段观察。

v2 政策可登记 `terminal_history` 的 `keep_seconds`、`max_entries`、`max_bytes`；后两项须为正，作用于每个地址 ledger 的 disposed 历史。字节口径为各条目紧凑 JSON 的字节和。本宿主采用 24 小时、64 条、4 MiB。成功整树处置的生产者保存实际 `disposed_at`；后续同地址操作或显式 maintain 在达到保留时间、条数或字节上限后，按原登记顺序移出合格记录，不在 finish 返回前立即丢失新结果。

移出前重新核对原终态和清理回执、附件、实际物理路径及 Git 登记，并持有原关联的独占租约直到 ledger 更新；仍活跃、保留、缺时刻、未知、忙碌、回执损坏或地址已复用的条目继续保护。它只释放 ledger 的历史使用方引用，不额外删除工作树或分支。`history_rotation` 报告实际移出量、剩余条数／字节和是否仍超限。

退出代际的租约先转交同一 ledger 的 `retired_leases`，再重新取得独占能力，核精确身份和空文件后删除；删除中断可按原身份接续，未知内容或活使用方继续保留。未完成责任保存原创建 agent、分支和真实错误，相关操作仍显示 `maintain-retired-lease`。它是当前清理责任，不是另一份永久历史。

普通 v2 metadata 准入复用现有 `DirectoryLease`，再取得原 `metadata.lease`。显式 maintain 可回收无条目、无待交接／租约退休责任且物理／Git 地址均已退出的登记目录；先写 `retirement.json`，绑定协调者、目录身份和三个已知 metadata 文件的原身份、摘要及长度，再逐项删除。中断时先恢复该声明；原字节变化、未知成员或忙碌时继续保护。删除声明后中断留下的真正空目录，在同一目录排除下回收。过程不额外删除检出、分支或未知文件，也不扫描认领已失去原登记的旧租约。

所有者须配套升级后再写这些当前字段，旧严格解码器会拒绝。新增 IO 和验证成本尚未实测；本地／CI 回归与真实宿主迁移仍须分别核验，不能把登记目录退出扩大为所有历史文件已回收。

已采用原件保留时，每个地址的 ledger 也是实际使用方。打开登记时保护其明确引用的已登记原件；保存前先保护旧、新记录所需原件的并集，ledger 发布成功后才释放不再引用的部分。消费范围由登记、终态、清理／缓存尝试、迁移及未完成 use 的字段明确列出，不扫描目录或遍历任意 JSON 猜引用。这样，一次调用返回后，独立 retention 维护仍须保护后续恢复所需的清理结果。尚未接入原件保留的历史文件保持原语义；此保护本身不证明终态历史已有有限轮转。

原件保护在核对登记身份后、恢复已缺失工作树的采用政策前建立；旧政策读取失败仍报告原错误，并保留账本所需原件。显式 maintain 在处理各登记项和生产者后才执行全局原件维护。某地址的元数据忙碌或无法验证时，`output_retention` 记为 `preserved` 并列出未能保护的地址，保留这一依赖完整使用方信息的维护步骤；其他合法工作树与生产者继续处理。

自动所有者发布的不可变回执和保留字节，现也使用 `lifecycle-original`。物理字节出现前，将原件加入同一地址的 `worktree-enrollment` 引用；随后按 typed 输入字段登记依赖并封存发布。调用方实际读取／产生的原件能力持续保护报告交接，use 释放令牌前先把原结果依赖接到原报告。账本收窄或终态轮转后，仍被报告或其他使用方引用的原件继续保留，其余沿 retention 的有限维护回收。中断留下的现有登记按原身份恢复，不据新政策扫描、认领或删除未登记的历史文件。

use 结果及 main-cache release 在最终报告交接前，暂存在所属 enrollment 的 `completed_uses`／`main_cache_releases`。后续清理准入核对精确原回执、最终报告的绑定、retention 生产者封存的摘要／长度和物理身份，再登记报告依赖并释放临时引用；部署与尚未通过候选资格核对的迁移拒绝，不改写这项登记。分支采用在资格核对通过后可先完成本工作树已封存结果的交接；后续采用失败仍保留已证实的交接结果，不反写旧报告。待交接结果本身也引用最终报告，避免独立原件维护先回收它；已有保留图会回收失去全部根的循环引用。报告缺失、部分写入、字节变化或尚未封存时继续保留，并阻止该条目滚出历史；即使工作树已移除，仍用 `report_handoff_pending` 和 `recover-result-handoff` 提醒后续处理。这不是永久操作历史，也不改变工作树的工作终态。

**迁移合同与产品同时变化：**分支采用现在可以独立交接已核验的完成结果；纯政策迁移和部署不增加这项恢复。沿用原发布、摘要、物理身份和使用保护判法，无新入口或全局维护前置；真实迁移、拒绝边界、缓存与无关工作树保留须同次验证，完整成本未测。

**产品与宿主政策同改：**宿主 `retention.json` 为 `lifecycle-original` 增加已采用的 automatic state 地址，只覆盖该生产者实际发布的文件。新增共享引用的原件发布行为及工作树回归；DELTA 沿 runner 和 worktree 的明确依赖验证。额外 IO、维护耗时和存储收益尚未实测。旧协调宿主仍须配套迁移，不能据候选提交宣称已部署。

`cache_history` 使用相同三个限额管理活跃地址的缓存尝试索引，包括主检出。
成功清理的生产者记录实际 `completed_at`；同地址后续操作或显式维护只移出早于当前代际的已完成尝试。
轮换前核精确 intent／result、附件和代际，并持有地址的独占使用租约直到新 ledger 发布。
当前代际、活跃使用、失败、缺失或变化的原件及未知完成时间继续保护，超限时报告 `over_limit`。
字节限额只计索引 JSON，实际原件由原件保留模块负责；新的辅助回执纳管和历史边界见[原件保留](artifact-retention.md)。

显式 `maintain` 会重试等待整树回收的 `retained`，重新取得原关联的独占租约并核政策、提交、源码和原回执。普通启动不重试无关保留项。`evidence-retain` 仅在实际登记路径存在且未获释放时阻止整树移除；访问错误或链接仍拒绝。证据使用方释放自己的路径后，下次维护可继续删除整树，无需伪造新的工作终态。`artifacts-only` 完成本轮缓存处置后仍保留检出，不由维护重复消费其旧交接；政策迁移后的新责任需重新 `finish`。

**产品行为与验证登记同改：**以前只要政策声明证据目录，即使目录不存在也会保留整树；现在核实际路径。清理选择、证据阻挡及结果阶段由 `verified_lifecycle.rs` 统一判断，工作树计划增加两项整 crate Verus 证明；真实 Git 回归核整树移除、保留后接续和原件不变。额外验证及运行总成本未测。这一批不证明漏登接管、终态历史滚动清理或现有全部工作树已回收。

#### 用引用保留原结果

自动生命周期用 `chrono-worktree-retained-input/v1` 引用旧输入，包含物理协调者 `source_root`、相对 state `path`、sha256、byte_length、format（receipt 或不透明 bytes）。完整不可变回执原位引用；部分／未封存字节在登记 state 发布一次，不把缺失／未知升级为成功，不改旧原件。未变操作重试不内嵌旧报告字节或解析历史。

引用字段为 `terminal_input`、`prior_report.input`、`prior_cache_attempt.intent_input`、`prior_cache_attempt.result.input`、`original_report.result.input`、`unreferenced_original_receipt.input`、`original_result.input`、`drain[].input`。

当前 drain 行用描述符及原 receipt 保留精确子报告，父报告可见子 status／error，不内嵌解析后的整报告。要读处置效果或原进程，消费者寻址子报告并核长度／摘要。标准 check／bootstrap 仍消费原父 `managed_process` 流和状态。

回执消费按明确引用传递核验，同次检查去重读取。原件缺失、变化、错绑或符号链接拒绝清理；不透明部分结果可保留，不要求有效 JSON。拒绝内联 drain 和 `prior_report` 正文，不提供旧表示的兼容读取。

显式 `recover`／`cleanup-fetch` 的 `prior_report` 也只保留 `receipt` 和 `input`。程序核原失败报告的摘要、身份和明确输入，在本次操作内读取解析结果；新报告不再复制原数字字节数组或完整解析副本。原报告仍在原地址，父报告通过现有逐输出依赖保护它；实际调用方释放后，再按原保留政策回收。调用方读取 `input` 时须核 `source_root`、路径、长度和摘要，再从原件取得失败及进程结果。此变更不退役仍有使用方的原件，也不新增共享总清单。

这限制历史重复内嵌，不限制必要原观察数量／总体积，保留和处置仍归政策。只有当前 `drain[].input` 描述符参与引用核验。stdout 协议上限不变，父发布仍可能因自身 IO／传输失败，保留子失败仍是清理失败。

#### 任务未 finish 时回收缓存

后续点名 start／reconstruct、受管准入或明确跨目标 maintain，可处置未完成但已无活消费者的缓存，包括成功 use 且无孤儿 token 的情况。EX 允许把孤儿 use 协调为 result-unavailable，同时保留原结果字节。

缓存处置仍保持 `status: active`、`terminal: null`，保留检出、ref、脏／暂存源码、未保留提交、bin、证据。只选两端登记 dispose，拒绝已跟踪／index 路径和符号链接祖先，不创建持久自有 Git worktree 锁。

成功一轮清 pending 代际，后续 use 重新启用。任何已发布缓存尝试后再准入消费，也开新代际，即使旧尝试中断／失败，仍保留旧 intent／result，允许当前 HEAD／登记绑定用于重建和再回收。

效果前，不可变缓存 intent 绑定关联、租约、HEAD、政策、登记、路径、代际。结果缺失／失败可在重新排除、绑定未变时幂等重试，生成新真实回执，原件保持。

可选标准配置声明为 `"participation":{"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]}`。公开命令仍 check、check --unit ID、check --collect。参与路径转发原 stdout／stderr／退出，含成功警告和非零诊断。所有者另标已完成命令失败；传输、所有权、结果发布失败仍是生命周期错误，带原报告引用。

CLI 字符串保留 UTF-8 控制台字节，所有者报告保留原字节数组。worktree 所有者在获取前准入原 runner，再转发原控制台／退出。bootstrap 另声明 entrypoint tool／script 和 participation coordinator／program／Git／config；独立 bootstrap 在构建效果前委托已部署协调工具，新 linked 无本地 bin 也如此。所有权缺失明确失败。

引擎以 CLOEXEC 所有者副本及 pre-exec 可继承子副本传明确不透明描述符。`CHRONO_PROCESS_FDS` 穿过嵌套 runner 的清空环境，现役 Rust bootstrap 和 CI 清单消费者使用登记的进程引擎转交方法。旧 Python 转交模块随其消费者退役。这些是已测路径，不是通用继承保证。

真实 Cargo 回归区分 launcher 和原生 child，杀 Cargo／受管 wrapper，核活 child 继续持租约直到完成。Cargo run 可能 exec 替换自己，杀同 PID 不证明子进程仍存活。不能由 PID 或父消失推断子退出。完整采用仍须已提交组合及支持原生平台。

整检出 finish／保留和旧自有锁恢复合同不变。不提供 daemon、全局进程／文件扫描、年龄过期、空闲调度或断电事务。调用方已提交标准检查、支持原生平台、进程模块组合验收、main 部署／切换仍为必需。

没有已部署所有者的 Git main 可初始化自身 bootstrap 工具，不能作为可丢 linked 关联登记。linked 缺协调所有者就在构建前失败；这样干净原生 CI 可 bootstrap，而不改工作流拓扑或允许不安全 linked 回退。

### 明确处置证据副本的当前保管

`chrono-worktree retire-evidence --host-root ROOT --config POLICY --plan STATE_PATH` 是维护模块的有限文件切换。计划为 `chrono-worktree-maintenance/v1`，含 `operation: "retire-evidence"`、精确 head、schema `chrono-evidence-custody/v1` 的 custody。

custody 声明物理 root、完整 branch ref、`current_consumers_released: true`、工程 reason、authority 原件（path／SHA256／length）及精确 objects。各对象含 original 身份、独立 survivor 保留路径、生产 owner、释放 reason。相等字节证明内容保留，当前保管者明确释放才提供处置权限。

声明前，调用方须等待／排除旧未登记消费者，结束副本路径断言或 manifest 重放。历史 manifest 仍是原证据，此切换不声称已退役路径仍能重放。

exclusion 声明已有 coordinator_root、已跟踪 config_path、已采用 state_directory。命令将当前已提交内核政策作数据读取，持原准入／登记租约，核一个当前 active 项、物理关联、未释放 use token、外来 Git 锁及干净固定检出。不清其他项、不迁移旧源码政策、不改 Git、不释放检出、不造历史 seal。历史宿主可用候选工具及自身未改已提交政策／产物登记调用。

只有明确列出的登记 evidence state 下未跟踪普通文件可处置。源码／index、所选 survivor、authority、尝试输入／输出都拒绝退役。unlink 前流式核别名和 survivor 的声明身份，含稳定物理文件身份。独立身份／保留失败保全该文件，其他合格对象继续。

独立 `effects.jsonl` 同步声明引用及每次效果前 intent／result，报告只含计数／引用，不复制旧正文。重复同一有限计划复核 survivor，将已缺别名记实测缺失，不猜哪个中断尝试删除。原失败、登记、源码、ref 不变。

退出 0 只表示精确有限处置完成，未解决成员／绑定失败退出 2。removed 只计完整核实完成。unlink 后核验失败可在效果已发生后增加 failed；removed 为零不证明没删。精确 custody 声明和 authority 仍须保留，报告不保证自含或独立不可变声明副本。逻辑删除计数不证明共享卷物理空间释放。

worktree fixture 只在 `CHRONO_WORKTREE_TEST_RECEIPTS` 明确请求或 panic 时保留；默认成功不复制全部 state。副本断言／失败诊断仍是消费者，直到当前保管者明确结束路径义务。

### 准备本地初始输入

登记 check-inputs action 支持 schema4 独立可选 `canonical_check.initial_profile`，将两个 profile 摘要绑定已提交 HEAD，经已有 Git 观察记录物理父提交。真正无父候选准备独立清单，无 base、fetch、origin、full context；非根保留目标 fetch 和普通 DELTA／上下文。目标缺失、浅历史不变成根。

裸 runner 入口先核原准备，再运行已有初始判官，生产者自身不发布清单成功。

CI 复制宿主断言复用专属测试独立 receipt 所有者及明确平台 Git 输入，不把源码宿主已部署 Git 摘要／版本当 fixture 政策，不改源码绑定。原错误和成功对照均由真实命令产生，未保留 commit-tree fixture 明确提供作者名／邮箱。

真实 Cargo 保留重建的握手在冷构建后发生，等待现役受管进程真实结果，沿用原 30 秒进程上限，不另用十秒就绪时钟杀编译。普通参与式 check 的本地输入准备也等原有限生产者结果，不另提前杀它；其他就绪等待仍十秒。

缺 marker 时消费、保留真实退出和进程原件，退出前没完成就不产生成功。专属负例用缺失 manifest 核失败并等待子进程。旧原生失败只留下空外层流和 use intent，不能据此判断编译阶段或停滞原因；修复后的支持平台原生验证仍归调用方。

### 在生成前登记 fixture 的保管范围

**产品与宿主政策曾同步修改：**已有保留生产者模块增加 `TemporaryHost`，cleanup.json 增加五个有限 fixture 生产者。工具及协调者／生产者已提交政策须一起部署。Git facts 的身份选择和工作树的运行内字节绑定各归其当前合同。历史存储和未确认释放迁移保持保护；实现、验证、存储、迁移成本未知。

`TemporaryHost::registered_store` 与 RetainedArtifact 共用只读已提交政策／Git main 协调者解析，另要求所选生产者明确采用生成前临时保管。分配在产物生成前发布 intent，列精确生成目录和声明 Git 源码根。

丢 handle 不表示任务完成或生产成功。最后一个受支持持有者及明确诊断消费者释放后，普通重入可恢复合格输出，不依赖记得 finish。分配／租约缺失、未知对象、部分复制、身份变化保留原结果，独立合格兄弟可继续。inventory 不登记／移除 Git 关联。

原生适配器在唯一 `bind_command` 前设 argv／环境，将已有内核能力传给真实原生／Cargo 子进程。命令跨范围释放／线程交接保留自有描述符副本，禁止重绑。

`capture_output` 绑定尚未绑定命令，`capture_bound_output` 捕获已绑定路径，两者在 spawn 前保留 intent 和原 stdout／stderr 文件。`capture_bound_input` 另保留 stdin 原字节并提供相同字节及 EOF。只有真实 wait 才发布已等待退出；spawn 失败／中断保留空／部分原件，不造结果。这些捕获不进入生成成员集合。

`register_copy` 在真实复制前保留验证后的源码原件、登记精确未来目标；`relocate_copies` 在分配内真实 rename 前只迁已登记成员。复制准备在持 allocation 租约时计算摘要、复制、同步，不占 allocation metadata 事务。

发布只重开当前登记，核不可变 namespace、allocation、intent 绑定，保留期间新增分配、body 登记、诊断引用。并发复制只有核已发布胜出者字节后才共用内容寻址原件，其物理指纹直到登记都须不变。当前保管采用沿用同一所有者、独占 allocation 和消费者已释放检查。

原准入上限及默认测试并发不变，早期登记超时的确切持有者仍未知。`fixture_bodies` 明确采用成员恢复。Git 身份／对象、已提交／index 源码、原捕获及未知成员保护；Git metadata 内只允许明确登记的复制 hook 叶文件成为成员。每次效果前核源码／index 和物理身份。

复制成员与原件生命周期独立，最终原件保留／发布及兼容共享效果组合另行完成，此 API 不释放原件。

每个 allocation 拥有一份当前 `producer-records/` 登记。不可变 `objects.json` 只含 namespace 身份，无对象表或可变结果。普通准入读小身份和点名 allocation 记录。allocation、复制内容、诊断引用更新持该 allocation 身份绑定目录锁，无关分配不共享修改锁、解码缓存或目录表重写。只有初始 namespace 创建排除并发创建，当前存储不建 registry.lease。

清理枚举明确登记 namespace，核各严格 schema、owner 摘要、规范地址，再核真实 allocation／use 租约。这读生产者声明，不扫产物树或按文件名猜所有权。错误登记保留错误／原件，独立项可继续。发布在登记 namespace 外准备，再原子替换自身记录，无全局解码快照或可变成员索引。

原源码、消费者、嵌套保管、处置检查仍适用，性能和完整有限 metadata 保留仍需实际验收。普通清理不阻塞地尝试 allocation metadata 排除；正在更新者保持保护及原状态，独立项继续。真实发布／引用更新仍受三十秒生产准入限制。

明确 retained-producer 维护只持调用登记的 metadata 排除；独立项各有记录／锁，不为物理 IO 作释放／重载合并。实际回归、协调者／原生验收仍必需，报告发布模块另有自身并发限制。

`migrate --path COORDINATOR --partition-producer ID` 是已采用保留生产者唯一数据切换入口。在原生产者独占租约下消费精确 v1 字节，保留原件及独立迁移回执，写逐对象登记，原子启用不可变当前所有者。只复制结果／消费者，不释放／处置。

该入口也转换实际保管中的旧 `sha256-nibble/v1` 分页登记。调用方须加入原生产者；操作持协调者、namespace、原 metadata／allocation 排除及原 cache 租约，核 owner、严格记录形状、规范地址、原回执及持续不变的原字节。旧 owner 和逐项记录进入绑定原 namespace 的原件目录，当前登记逐项保留相同消费者、领域状态和输入；捕获、旧保管字段及原删除声明保留在原件中，不增加整目录删除资格。准备与归档中断可沿同一绑定恢复；未知、部分、漂移或活消费者具体拒绝，原结果继续保留。新产物随后走现役目录滚动路径，普通运行不解码旧分页形状。

中断准备可复用绑定身份 namespace，未知 metadata 创建保持保护。旧写入者拒绝启用后的严格形状，普通运行无历史解码／格式回退。部署、等待／排除旧写入者、真实迁移、完整 SPEC 验收归调用方，仅改源码不表示已切换。

#### 复制内容的删除与原件引用

复制内容准入只在共享效果模块的紧邻 unlink 前回调执行一次，核当前成员内容／身份、保留原件身份、实时 Git source／index、协调政策／HEAD、allocation、消费者租约、最终原件排除，再由效果模块复核物理身份后删。

intent 先于准入，因此拒绝成员仍有 intent／失败结果。准入只在实际效果前读取，不在 intent 前重复读取，执行仍受登记上限约束。

source／index 回归覆盖同长度内容漂移和成员保管；不证明原生端到端耗时或完整 SPEC。旧迁移的部分结果和失败直接查原报告，不在本文维护读数。

受管 use／check／bootstrap 命令真实等待成功后，所有者返回前清理点名工作树。fixture 生产者维护只属明确 maintain，不先于无关操作。独立 `post_use_cleanup` 保留本轮，不替换准入 drain、原进程或 use 回执。

调用源码／目标沿用排除和活租约，生产者恢复仍须精确成员、原字节、持有者／消费者释放。失败命令保留原失败、跳过成功后清理，保留部分诊断。清理不释放原证据或宣称生命周期完成；原生产物体积／汇总耗时收益须观察候选。

每轮在 `producer_drains` 列清理原件，含精确 receipt 和 `chrono-worktree-retained-input/v1` 引用。消费者核引用字节，从原生产报告读 `producer_objects`，父报告不复制行。

生命周期发布向已有 retention 所有者登记准入／post-use 依赖，让当前父消费者保留生产原件。释放后，明确有限维护可回收无根引用范围；仅历史引用不永久保活。各原报告保留状态、错误、进程。其他重复内容、物理收益、完整 SPEC §10.1 仍未验证。

普通 linked 生命周期、创建、重建也保护协调者报告家族直到父报告发布，包括未给 `--report-consumer` 的调用。只保护报告和明确依赖，不遍历两检出无关输出。发布不建持久消费者根，后续明确维护可回收无根报告，普通重入保留它们。

`migrate --path WORKTREE --adopt-fixtures STATE_PATH` 通过已有迁移模块消费有限 `chrono-fixture-body-custody/v1`，绑定当前 HEAD、allocation／intent／物理身份、精确成员字节、配方、Git 根、嵌套 registry／lease 引用及真实当前消费者释放。保留历史结果，不处置。

外层成员回收前，嵌套诊断引用及真实持有者也须释放。不扫描成员或根。真实 worktree Cargo／hook／full-native、CI short／full fixture 和共享 projects Host 使用此模块。共享 Host 的 FILEMAP、inputs、workflow 消费者都依赖同一 worktree 库。

workflow 原生工具、历史迁移各有有限声明，源码、外部输入、manifest／lock、离线原件仍保留。复制／硬链接／原生捕获适配器保持真实操作。panic／明确请求诊断在 allocation 保管释放前取得持久 `fixture-diagnostic` 引用，真实诊断保管者消费后经 RetainedArtifact 释放，不加整范围 TMPDIR 或并行登记／清理服务。

共享 drain 还从 Git 明确 inventory 发布 `worktree_inventory`：当前 HEAD／分支、locked／prunable、物理关联、当前登记匹配和保留理由。inventory 进程失败在效果／准入前致命；不可读行只是被阻断诊断。没有账本、已变化、处置路径被复用的关联，不靠观察登记。

可变 HEAD 与 birth 关联身份分开。这些行不授权处置，效果保护仍取新事实。共享测试所有者在各自 update_lockfile 登记 `lock.<owner>`，委托 Cargo 对精确本地测试包 offline update，保留独立 manifest／lock／target；实际集合归[项目登记](../.chrono-harness/projects.json)。

本地标准验证、真实兼容协调者部署、精确原生汇总、独立审读、历史迁移、落地都须分别验收，源码适配本身不证明任何一项。

## 受保护地构建普通 bootstrap 所有者

公开 bootstrap-core 选择 `.chrono-harness/ci/bootstrap-owner-upgrade.json`，使用 `chrono-bootstrap-owner-upgrade/v1`。仅输入 selected_original（普通 bootstrap 报告）和 stage_config（当前构建 profile）；执行物来自已有 participation program，不用固定历史签发者、外部启动包或执行物 hash 准入表。

Git main 工具缺失保留原 seed 行为。linked 已安装本树工具时，普通 bootstrap 使用本树程序保护构建，仍核实际协调状态和所有权；本树尚无工具才由协调者保护首次构建。两处工具都缺失或实际所有权不兼容，在构建前拒绝；本树程序拒绝不触发改用协调程序重试。

构建前，候选从真实保留生产者核已安装所有者资格。普通安装读取原干净 bootstrap 报告及生产者固定 commit／tree／登记为数据，核执行、原流、全部安装内容、所选工具物理身份。生产者须属于本物理宿主且唯一安装该目标，变化的现用候选不能替换原生产者配置／事实。

独占发布成功而后续 core 安装失败时，已安装所有者的生产者是保留 stage／adoption。核固定源码、工具链执行／流、stage／旧 inode、发布 intent／result、原 Rust adoption 回执，不执行历史 bootstrap、不改旧 core 报告。原件缺失、改变、不完整、未生产都在原生效果前拒绝。

候选把合格已安装字节复制到 `.chrono-harness/state/bootstrap-owner-upgrade/` 的保留构建尝试，附生产报告、资格确认、当前升级声明。启动前复核原摘要、执行身份、副本字节。已有 migrate／bootstrap 携原准入／登记保护。

内部 stage profile 构建 build.worktree，把独立执行物留在 `.chrono-harness/bin/owner-stages/`。构建失败不请求部署，原命令／后代全部等待结束后，stage 才请求 EX。

core 用 `reuse_owner_operations` 明确列所有者 fetch／build。当前候选部署后，core 消费原操作记录／流，以保留 stage 工具作为安装输入。复用前核当前 adoption、源码、声明、工具链、实测环境及这批操作实际使用的工具选择；core 其他操作新增工具槽不使原构建失效。原 PID／退出／流描述符不改，`reused_from` 点名 stage 报告，不假称第二次执行。首次 seed 或独立所有 linked 工具仍运行声明构建。

**产品与宿主政策曾同步修改：**去掉已采用 main 的重复 owner 构建，避免临时重建替换部署字节。登记 bootstrap 测试覆盖复用、选择、原绑定拒绝，再验源码 bootstrap 及受影响标准／原生范围；总节时未测。

Git main 的普通 bootstrap 复用相同协调者字节，改变须走现有 owner 部署入口。linked 在原登记保护下安装并复用自己的本地工具，后续构建不要求先升级共享程序。Rust bootstrap 解析 Git main inventory，区分状态协调根和本次调用程序；本树工具的选择由现有 `verified_bindings` 根定义，并随同一 DELTA 的独立边界预期和真实 CLI mock 验收。安装继续拒绝符号链接，对共享 inode 的本地硬链接也应用协调者字节门。

候选回归以明确的 Git mock 覆盖初装、本地替换、协调者字节／身份不变、直接／祖先符号链接和 inode 别名；文件系统观察仍由真实 Rust 实现完成。本宿主的阶段、core 和 shared 入口均通过薄 shell 选择 Rust 实现；旧 Python 实现和专属测试随入口退役。Rust 的现有部署所有者直接核构建报告、登记摘要、工具链绑定、原操作与两流，并负责原 inode 保全、原子替换、中断发布重入、原准入／登记 EX 和采用检查。发布不再调用构建解释器，不生成不存在的 managed_process；不可变 intent／result 继续由实际发布者产生。

发布的身份／内容比较、发布意图的完整阶段绑定、原件／意图／替换状态选择由同源 Verus 实现，
阶段报告值匹配、操作／原流唯一性、进程退出与 PID、摘要形状、执行物绝对路径和搬迁许可使用同一证明根的有限条件序列，保留首个失败的具体信息。
JSON／工具链平台选择／路径／文件观察仍是明确的适配接口，不把逐值判断证明当作整个报告构造或文件事务已证明。
worktree core／adoption 计划都先执行生产根和独立边界根的整 crate 证明。
原件存在时仍需核其完整旧绑定；孤立准备文件、错绑意图或意外所选 inode 拒绝。
局部证明与真实文件系统测试的边界见 [Verus 实施范围](verus.md#所有者可执行文件发布)。

部署从逐工作树记录和 Git inventory 产生真实排除集合。独占发布阻止新登记／use，在改所选字节前，为该物理协调者每个未解决登记（含 linked）取得原 EX，贯穿发布／采用，并复核准入、账本字节、inventory、关联、协调政策、租约身份。

已存在的 linked 检出切换本地分支或进入 detached HEAD 后，部署使用本次固定 inventory 的引用和 HEAD 核物理附件，仍须取得并保持原登记租约。报告分别保留原登记分支和当前引用；发布期间引用、HEAD、附件或账本变化仍拒绝。部署不迁移该工作树的身份、政策或任务状态，不改源码、索引和 refs；普通消费及清理继续按原登记身份判定。回归同时检查活子孙阻止部署、退出后可升级，以及暂存／未暂存工作与原清理限制保留。

被排除 linked 无需先采用政策。已登记对象的原路径和原 Git metadata 均缺失、Git inventory 不再关联其路径或分支时，可以在持有原独占租约、无未加入 use、适用的原 birth／政策回执及账本保持有效的条件下升级所有者。部署报告把该对象的 `availability` 记为 `absent`、当前 `head` 记为 `null`；不退役记录、不删除分支，也不宣称任务或清理完成，status 继续提示未知状态。路径或 metadata 部分存在、活后代、未知关联、缺所有权或未等待 use 输入仍具体拒绝，保留安装字节／原件。已处置代际不变活消费者，另一物理协调者不在集合。真正绑定的协议外生产者仍须其原保管者等待／排除，未知历史任务不能证明该绑定。不可变发布结果不独自证明采用或任务完成。 Rust 部署回归覆盖构建解释器不可用、保留原 inode、替换前后精确 intent 恢复、同字节异 inode／符号链接／原件漂移拒绝，以及发布后原准入变化时保留结果。原源码安装、标准 DELTA 与原生验收分别记录；源码存在不证明这些验收已通过。

**产品与宿主政策同时变更：**Rust bootstrap 确认并复用真实已安装所有者，core／FILEMAP 用当前升级声明替代固定历史供应，移除专属供应验证及只用外部包测试。共享 startup 保留真实 CI 消费者及测试。

Rust 专属回归覆盖普通／已发布生产者、原件丢失／损坏、资格确认后变化和构建失败不发布。外部工具调用使用 mock；真实协调 bootstrap、两端标准 DELTA、原生采用和落地另验。摘要、尝试存储、验证成本未测，原期限／操作上限／排除不变。

构建载体、stage 候选、保留旧 owner inode、原报告／流用于当前工程／PR 验收。待验收／调试和依赖消费结束，走登记生命周期回收；保全独有工作和失败原件，不按年龄删或宣称完成。真实生产者数据仍是必要证据，不构成历史格式／接口兼容义务。

宿主可为 `bootstrap-owner-stage` 显式选择 `.chrono-harness/bin/owner-stages/` 保留范围。
它按完整 stage 目录登记，原构建报告引用真实 stage；当前 adoption 经原报告保留同一目录。
构建阶段不提前声明可变发布目录完成。部署取得该目录原 EX 能力，核原 producer、
物理身份及未完成状态，有界统计现有节点／逻辑字节后再写旧 owner、intent 和 result，
实际发布后才封口。成功完成的原件不可重新打开；重试只消费并核对原发布结果。
替换当前 adoption、报告及其所有真实消费者结束后，按原有限期限整目录回收。
未采用的目录和失败原件沿用原保护，不从文件名认领。

**能力与宿主采用分步交付：**retention 允许显式 bin 子目录，仍拒绝整个 bin、源码和
保留元数据；源码／index、活消费者及每轮节点／字节／时间上限继续适用。
先用原保留政策部署能理解该范围的所有者，再提交并采用新范围；旧所有者不能读取新
政策，不能只先改 retention.json。新增依赖为零，目录内容资格及完整构建／CI／存储
成本未测；使用现有保留和发布所有者，不另建清理清单。

所选 profile 通过原 bootstrap 宿主路径模块解析可写 HOME／CARGO_HOME／TMPDIR，位置归当前登记。解析在取得授权前无效果，普通参与或 main 缺工具 seed 后才建目录。锁定 Cargo fetch 明确允许在线，填所选私有 home。macOS 环境为标准消费者选同一私有 home，原生 profile 保留自己的输入；这不证明同判。原离线／缺包失败仍保留，产品限制不变。

### 原生子进程的输出与完成

Cargo wrapper 存活回归在原 wrapper 中断和维护检查后，用阻塞 socket 等真实原生 child。child 仍发布原释放／完成文件，测试同时核 socket 响应和完成文件。关闭／超时通道以原 IO 失败，不能文件轮询代替完成，原十秒完成上限不变。

发就绪前，fixture 将自身 stdout／stderr 绑定保留原文件，使 Cargo／受管 launcher 退出后目的仍可用。两 wrapper 退出后，父请求真实进度／诊断写入，在维护及释放／等待断言前核原字节。写错误保留 OS 错误并使回归失败。

完成消息只表示 fixture 函数完成，不独立证明整 libtest 进程终态或全部调用方义务完成。

该回归只维护自己的样本宿主，核对目标因真实存活消费者而保留。最后一次源码断言结束后，
样本已有的 `TemporaryHost::finish` 定向回收自己的整个分配；测试不再调用共享宿主的全范围
`maintain` 来检查一个分配。活子进程期间的缓存保留、原输出、完成消息和源码保全断言继续执行，
原操作时限和测试线程不变；测试不依赖无关工作树的维护进度。

## 明确登记生命周期报告消费者

当前 finish、maintain、import、受管入口接受 `--report-consumer ID`，点名调用方真实原件消费义务，独立于缓存 use／release。已有 retention 生产者在关闭发布能力前传精确消费者根，在 report_consumer 返回 `chrono-retention-consumer/v1` 声明，要求已采用生命周期保留。

生命周期命令先只读核对输入及协调者归属，在该协调根预留唯一报告后，才进入管理和业务派发；linked 调用根不再留下同名空报告或未结算预留。输入校验失败时，仍在调用根发布真实失败原件；协调根预留失败则不派发业务。普通创建、重建和检查输入报告继续由调用根负责。报告声明的 recovery、fetch、cache、main-release、drain 是明确保留依赖，不从 JSON 发现边；调用方消费者绑定实际发布的原件。

bootstrap fixture 保管者消费该声明，核真实生产路径、物理身份、字节、类型化回执依赖范围，在维护、缓存处置、准备恢复、再次准入、后续断言期间保持保护。

成功断言所有者通过已有 runner retention 命令及自身 release 回执，释放精确声明。终态断言回执绑定未改尝试和精确领域／所有者释放原件；原消费结束后，之后准入消费该回执，其他原件断言仍须真实字节，不用报告正文替代。断言失败或保管未解决保留原件。

独立真实 bootstrap 消费者保留调用方验收／审读保护，直到真实交接。缓存处置、源码／index／活消费者排除归原所有者；缓存释放或存下 stdout 不代替原件，也不证明之后已释放。

**产品／fixture 同改说明：**发布新增可选调用方消费者，本仓普通政策和零保留窗口不变。验证使用专属 runner／worktree Rust 及登记未过滤 bootstrap 测试。直接脏源码工程调用不是标准源码绑定验收。

临时进程原件由实际生产者在 state 中发布；失败原件保留到调试／验收结束，再释放给已有生命周期所有者。各 action 沿用登记的执行限制和测试线程；完整构建／CI／存储成本未知。

临时分配的 acquire／release 回执存入该对象已有的 `rolling-metadata`，与原意图、使用方状态和处置回执共用登记寿命。活跃、unknown 和未释放使用方继续保护；确认处置并到达元数据期限后，由原维护入口整体退役。Rust 回归核对回执归属、原字节保护及最后使用方释放后的实际退役，不以测试退出成功代替这些事实。产品实现与宿主测试登记同时变化，新增验证和存储成本未知；旧失败原件仍由调用方保管。
