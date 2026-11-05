# CI 生成与当前检查范围

`crates/` 下的 `runner`、`judge-registration`、`judge-filemap`、`judge-routes`、`judge-projects`、`judge-ci`、`ci` 都是独立 Rust 生产项目，各有专属 `-tests` 项目、Cargo.lock 和 target。`instructions`／`instructions-tests` 也采用同样布局。容器目录不是 Cargo workspace，也不用于发现测试；每个 manifest、操作、依赖都须明确登记。

各模块分工如下：

| 模块 | 负责内容 |
| --- | --- |
| `runner` | 外部进程传输、请求身份、严格协议检查、结果发布 |
| `judge-registration` | 完整格式的登记检查 |
| `judge-filemap` | 完整类型化声明的影响范围，以及共享的图合并／传递计算 |
| `judge-routes` | 操作计划、工具绑定、回执比较 |
| `judge-projects` | 受影响的项目对及真实执行结果 |
| `judge-ci` | 按范围 CI 的登记、快照、起点和操作政策，通过明确适配器复用上述机制 |
| `ci` | GitHub 工作流生成及事件输入准备，不选测、不判断结果 |

`selection_explanation.extra_selections` 解释旧规则额外选中项，`legacy_only_selections` 指出不由 test-execution 边选中的部分。这些不变成完整政策边，见[影响范围说明](filemap-impact.md)。`instructions` 仍独立，catalog、布局和相对根别名不变。

按范围政策可明确选 `selection_evidence: "explicit-graph/v1"`：保留完整两端合并 `edges`、`seeds`、`seed_causes`、额外选择及原因；`closure.schema: "chrono-ci-reachability/v1"` 发布排序的 `reached` 节点和图选中的 `selected_tests`。由保留图和原因可重建前驱及贡献起点表，因此传输省去重复派生字符串，不改选择或执行。省略保留旧说明形状，null／未知格式在业务前失败。

本宿主因 2697 条私有声明使原响应超过未改动的 8 MiB 上限而采用此表示。Rust 测试保留全部 2697 个明确起点输入和输出限制。完整 FILEMAP 影响输出保留原版本格式。源码／bootstrap／标准／原生验收及总验证成本仍分别处理。

五份完整治理登记仍为 **proposed**。当前有限宿主 profile `chrono-ci-check/v3` 明确消费 FILEMAP／项目数据，独立单元／汇总使用 `chrono-ci-judge/v2`；不分单元的本地检查及旧 profile 保留各自说明的行为，见[独立 CI 单元](ci-units.md)。

此外，`check` 可通过登记 full profile 使用 `chrono-judge/v1` 及 registration／filemap／routes／projects 判官。schema4 自动产生上下文，旧 full profile 仍要求明确上下文。完整治理尚未完成，实际求值会非零退出；工作流已有[有限认证](workflow.md)，完整自动生命周期仍未实现。[混合修改](mixed.md)已有专属分类／警告判官和真实链测试，成本报告也有专属判官及登记消费者测试。共享操作计划、执行和回执比较见[执行说明](execution.md)。

<a id="this-repository"></a>

## 在本仓运行

核心 bootstrap 及所有者构建／验证 profile 把可写 HOME、CARGO_HOME、TMPDIR 放在 `.chrono-harness/cache/bootstrap-owner-tooling/`。可重建工具沿用已登记缓存生命周期；原报告、输出流、安装来源留在 `.chrono-harness/state/`。`host_environment` 只接受缓存路径，保留字面路径和符号链接检查。

这曾同时修改 bootstrap 合同和宿主登记。原发布单元产物的工具部分有 556,290,234 逻辑字节（压缩后 179,203,963），其他成员共 5,382,827 逻辑字节（压缩后 636,057）。把工具移出上传证据，去掉了这份重复内容；原生总量和耗时收益尚未验证，原 bootstrap 和标准检查仍须执行。

CI fixture 从 `.chrono-harness/tests/ci-tools.json` 的明确 OS／架构项选择 Git。准备命令和绑定 Git 消费者使用同一原生执行物，故障适配器在输入登记保留所选路径。平台缺登记就失败，不从环境 PATH 回退。这不改变消费宿主 Git 政策，也不证明 Git 输入完整。

从仓库根运行。本 macOS 宿主为 bootstrap／迁移明确绑定 `/usr/bin/python3` 和精确 `Python 3.9.6`，按范围工具表也登记同一合同。bootstrap 需要该解释器、Git、rustup；Rust 1.95.0 不可用时安装，并独立确保 rustfmt 存在，包括已装最小工具链的情况。

它按 `.chrono-harness/projects.json` 构建明确列出的 bootstrap 操作，再复制声明二进制，不复制 Cargo 配方或发现项目。工具链操作明确点名版本，不改全局默认。专属 CI 测试的复制宿主回归消费已安装候选工具，须先 bootstrap：

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

提交修改、确保候选检出干净后，本地和生成 CI 使用同一标准判断命令：

```sh
.chrono-harness/bin/chrono-harness check --unit ci
```

bootstrap 最后参数明确选择宿主 CI 配置，省略沿用 full bootstrap 默认。生成的单元／汇总工作流选择 runner、judge-ci、ci、本地 worktree 输入生产者；其余构建来自原完整测试计划。并发运行单元须用不同检出。

本地不带 `--unit` 时，按 FILEMAP 调度执行整个所选 DELTA；同一入口加无值 `--collect`，则准备声明清单、核单元证据，不执行业务。

### 短命令的输入准备

本宿主使用 config schema4。`canonical_check` 声明 `profile`、`inputs.local` 及可选 `inputs.ci`。生产者沿用 `{operation,tool,argv}` action、工具／环境／协议上限。runner 观察真实短入口，调用绑定生产者，保留原进程证据，再通过已有判官路径执行准备后的类型化检查，不重新分发虚构的长参数。

准备记录环境值摘要及原环境摘要。可选 `environment.credential_environment` 从 `inherit` 点名仅供获取的凭据：传给生产者和其声明的收集传输，不传给 Git 或判官。本仓声明 `GH_TOKEN`，与已有 gather 凭据政策一致。外层获取继承也将已声明 `SSH_AUTH_SOCK` 交给 worktree Git 所有者；缺失仍记为缺失。环境清空和各所有者明确白名单继续适用。

获取限 2400 秒，覆盖登记的 1800 秒原生汇总等待；冲突的更短获取上限明确失败。

`CHRONO_CHECK_SOURCE` 必须明确继承，不能固定值覆盖。未设或 `local` 选本地，`ci` 选原生；其他值、缺少原生绑定／变量都会失败。CI action 明确点名事件、载荷、工作流 revision 和汇总仓库变量。`chrono-github-ci/v4` 生成短检查步骤，含该选择器和工作流 revision；checkout、bootstrap、上传仍是原生步骤。

按范围 profile 将 registration_config、facts_config 都明确绑定到固定配置入口，短命令获取不使用旧未绑定 Git。push、PR、分支创建、明确手动端点的含义仍归已有事件生产者。准备只是获取输入，尚未作判断。

空业务宿主可用独立 v4 provider 和明确 `initial_inventory`，保留空 projects／scripts／plans／units。schema4 可选 `canonical_check.initial_profile` 点名单独初始 profile，`profile` 仍用于 DELTA。请求绑定两路径及字节摘要，provider 的清单路径和生成 profile 必须与初始绑定一致，两者都是候选输入。

生产者只在真实物理 Git 根或原生事件证据支持时选初始清单。准备和报告保留所选 profile、精确请求、原生产者／报告对应及真实裸入口。初始判官输出 `initial-inventory`、null base／DELTA、`governance: not-evaluated`，不证明 DELTA 治理。初始绑定缺失／错误会失败。

单元／汇总模式仍须有真实登记单元消费者，单元 provider 的非空要求不变。已有 scoped v3 的明确非空单元初始清单，保留自己的 profile 和首次事件含义，与独立初始判官不同。

### 固定端点与上下文

本地使用已声明 worktree Git／remote／workflow target，每次从干净、已提交、非根 HEAD 获取并固定目标。获取前关闭 replacement objects，用 `cat-file commit` 读取物理提交头。真正无父 HEAD 使用明确绑定的独立清单，无远端 base；浅历史边界或父对象缺失不能抹去物理父头。非根仍须登记远端目标，缺 fetch／base 证据就失败。

按范围检查不要求 origin 或分支年龄上下文。完整、不分单元的检查要求 worktree v2 声明 origin／context 路径和创建种类／运行角色映射。start／reconstruct 发布最终原 birth 报告和精确目标关联；生产者保留创建时间／分叉点，观察当前端点／时间。历史快照或交付证据交接缺失就明确失败。

full CI v1 保留精确上下文传输；v2 生成短检查，并在所选原生上传根保留相同原始 dispatch 上下文字节。原载荷／上下文／生产者／回执在该根下寻址、验证，也支持收窄的自定义根。full v2 当前 context／report 文件仍是可替换结果，其短命令原始证据不可变。

full v1 重复准备若只差进程所有权描述符编号，则返回原准备，在声明产物目录的 `preparation/native-attempt-<sha256>.json` 保留新真实报告。上下文字节及其他观察必须一致；Git 结果、环境或输入身份改变，仍报输出冲突。完整独立范围、固定轮次和到期复核尚未实现。

本仓单元／汇总工作流登记 `macos-26`，Git 字节／版本绑定当前 macOS 机器。开发工具目录尚未明确选择；托管原生 runner 的声明输入完整匹配仍未验证。其他文档中的有限原生成功不证明这个完整范围或同判。旧宿主保留原明确命令拼法；当前宿主只接受登记的短命令，拒绝部分覆盖。

### 从候选构建工具

检查前必须从候选检出 bootstrap。runner 报告实际 runner／judge／operation 执行物摘要。所采用 core、cache、default profile 还把 rustup 所选 Cargo／rustc 绑定到声明 SHA-256、版本，明确有效 `RUSTC`，在报告保留本地实际路径。

共享 startup 在安装、verify 前复核同一身份；字节／版本相等时允许等价迁移，安装报告保留生产路径和本地执行路径。绑定缺失、冲突或变化，在 bootstrap 验收前拒绝。这不证明可复现构建或编译器／后端／SDK／输入完整。

core、cache、detector、default 共用源码登记 `.chrono-harness/ci/bootstrap-toolchains.json`，明确声明 Darwin／arm64（macos-26 CI）及 Linux／x86_64（ubuntu-24.04 发布 cache-bootstrap）。未知／缺少组合在准备工具前失败。

每个执行物 SHA-256 都从官方 Rust 1.95.0 组件归档的精确 Cargo／rustc 成员取得，独立于本机安装。登记保留官方 manifest URL／摘要／日期及组件 URL、归档摘要、成员、大小、版本来源。归档摘要只说明来源，不能代替执行物字节预期。Cargo 上游版本实现解释 package 0.96.0／CLI 1.95.0 的区别。

获取这些预期是准备已提交输入；bootstrap 不下载或从当前安装学习预期，仍用已有 rustup 准备工具。profile 直接选 rustup，不写个人安装路径。调用方提供私有 Rustup／Cargo home 时，继承 PATH 须把所选工具链 `bin` 放在 Rustup 代理前。专属消费者清除无关 Rustup 变量，Cargo doctest 仍须从同一安装找到 `rustdoc`。

bootstrap 只改自身子进程环境，不改调用方之后的标准检查环境。记录真实路径／身份，不改全局默认。旧内联可迁移绑定中，重复的声明／配置原路径只是历史位置，即使已失效或缺失也不探测；另行明确配置的当前执行物必须存在并匹配。不可迁移绑定仍要求精确路径。

实际 Cargo、有效 rustc、继承编译器冲突、包装器及构建后变化都继续检查。源码报告含登记摘要，共享 produce／install／verify 在安装前核同一声明。config 输入清单、FILEMAP、标准 required_inputs、缓存兼容输入都明确登记这个新来源。之后的独立 Cargo action 仍有已说明的输入完整性缺口。

每次 bootstrap 在 profile 状态目录下不可变 `bootstrap-*/report.json` 旁保留操作 argv、cwd、执行物摘要、PID、退出及原 stdout／stderr 文件。输出流立即转发给已有、有上限的生命周期所有者。失败保留原流和失败报告，不替换上次成功报告、不安装工具。

共享 produce／install／verify 核源码登记、原操作选择、编译器环境及每份流字节。传入的生产路径只作来源；本地绑定按 profile 单独核。

### 真实消费者与证据保管测试

登记 action `host.bootstrap-real-consumer` 用 `--real-consumer` 调用已有 Python 测试入口。它从精确干净候选克隆主 fixture，执行 cache／core 构建、共享 produce／install 和 runner 单元登记 verify，再消费真实 runner 单元检查。

检查若选中 `test:runner-tests`，其成功独立 `test.runner-tests` Cargo 结果满足消费断言。单元无操作时（含候选已落地、与 dev 相同），fixture 在已有租约和剩余时间／输出上限内，用 scoped 政策工具链环境明确执行同一登记 Cargo action。

这是 fixture 验收操作，不改变原 DELTA 选择、报告或无工作成功。缺失／失败的已选执行不能替代。fixture 结果明确引用原检查或另行登记命令作为消费证据，因此只完成 startup 不足验收，成功所选操作也不重复执行。

原件保留在 `.chrono-harness/state/bootstrap-real/pending-attempt/`。生产者维护 `result.json`，记录源码、argv／cwd、原 stdout／stderr 路径、摘要、大小、继承所有权描述符，以及成功、非零失败或超时。超时保留原件，不编造退出码。执行前写结果，普通退出时更新；中断尝试继续受保护。该位置独占创建，旧尝试未解决时拒绝再克隆／构建。历史 `attempt-*` 目录保留，不迁移，也不声称已知其所有权。

构建任何输出前，生产者用调用方绑定源码的 `chrono-worktree` 管理新主 fixture。已有 `use` 为不存在的登记主缓存输出建立登记，运行内部 action `host.bootstrap-real-fixture`。同一租约覆盖 cache／core 生产、共享 install／verify、runner 检查和报告断言，嵌套采用命令继承保护。所有者从调用方保留的 bin 运行，fixture 安装 core 不会覆盖正在执行的所有者。

受管子进程全部等待结束后，生产者请求已有 `finish --artifacts-only`。它消费真实释放、cache intent、处置回执，核原摘要、精确声明输出及确实缺失，再记 `outputs-disposed-evidence-retained`。不创建 Python 清理账本，也不接管历史记录。

源码、已安装 bin、状态报告、原输出和描述符目标仍在原路径，供失败诊断及调用方验收／审读。主缓存释放不删除精确发布的 startup manifest：Python 生产者在发布前将原字节写到 `.chrono-harness/state/startup-originals/manifest-<binding>.json`，binding 是该字节 SHA-256，内容含源码身份和原报告关联。

发布或后续消费失败都保留该原件，真实 fixture 结果引用其路径、摘要和长度。它与 fixture 证据共享验收／审读期限，由调用方负责退役；传递副本可处置。释放主缓存不表示任务完成，也不删除检出。

所有权缺失、报告不完整、活租约或处置未完成时，保持 `protected-pending`，拒绝在同一位置再次尝试。外层超时可能留下活的继承租约；调用方等待或排除消费者，再调用已有所有者前，必须保持 pending。这不证明通用断电恢复。

直接 Python 保管测试用真实已有所有者管理小型、由生产者创建的主 fixture。源码和原所有者回执保存在 `.chrono-harness/state/bootstrap-custody-tests/`，用于当前检查诊断／审读；每次最多八个 fixture，不重试。证据消费者须先释放，调用方再通过测试生命周期退役 fixture。登记生成输出在测试中由所有者回收。

可处置内容断言在 Python fixture 内、继承所有者租约下执行，先于成功、失败或超时。尝试上下文先完成处置，父进程再读保留断言和处置回执；`use` 返回后不再需要那些内容。合成用例通过不证明原生／标准或真实 Cargo 验收，历史真实 fixture 不动。

活消费测试用本地 socket 等待 fixture 就绪，同时原生等待受管所有者退出，两个通知进入同一个阻塞 selector。就绪共用已有 40 秒尝试期限，含所有者准入；旧独立十秒文件轮询可能在准入时到期。

父进程先创建 fixture 的 ignored state 目录，再发布通道地址、启动所有者，新的原生 fixture 也如此。fixture 须收到真实释放消息，退出前生成释放标记。原 15 秒持有、20 秒操作、25 秒最终等待及登记测试集上限不变。所有者早期拒绝时保留原 stderr／退出，不等就绪文件。已有活消费者及拒绝所有者用例覆盖两条路径；原生验收及额外 IO／进程成本另行观察。

独立 fixture 绑定自己调用的 Git 路径、版本、字节，并为受管 action 选择正在运行测试的 Python。不能把父宿主预期当作原生 fixture 预期。回归要求普通成功、非零退出、超时、活消费都保留生产者输出及状态。

所有者报告缺失或活消费就绪失败，须保留先前失败、未解决保管状态并拒绝重入。只有验证释放／处置，才允许已结束成功或失败后重入。每条命令都须保留等待后的结果和原流；仅受管消费者允许失败，且保留失败所有者报告。准备／释放失败、中断、完成未知、原件缺失仍拒绝重入。准入不改变旧失败。

bootstrap 测试项目仍限 900 秒／1 MiB，FILEMAP cap 为 2。生产者从同一期限预留 15 秒给所有者释放，其余时间传给每条命令。普通命令原件仍限 1 MiB；所有者 CLI 报告含源码观察，用已有宿主协议 64 MiB 上限，紧凑结果引用原文件而不重复正文。

没有增加进程或输出预算，外层登记进程所有权仍负责命令和后代。本生产者不是进程回收器，也不是流式磁盘配额。新增原生耗时／存储和完整生命周期成本未测。

**登记与运行行为曾同步修改：**原生声明、标准／缓存输入、DELTA 边与 bootstrap／共享选择及真实测试证据处理一起改变。直接 Python 用例覆盖这些行为；干净已提交候选 bootstrap、真实共享／runner 消费、标准 DELTA 和两个原生消费者仍各需调用方检查。这不改完整 SPEC 范围，也不说明 CPU 加速。

### 初始检查、退出与报告

无父提交可用 `--candidate FULL_OID --initial`，不带 `--base`。报告明确为 `initial-inventory`，不编造历史 DELTA。事件准备和初始判官关闭 Git replacement overlays 后读原提交头；同树子提交不能通过 `git replace` 视图或浅历史边界冒充初始根。完整登记初始 profile 见[执行说明](execution.md#initial-inventory)，清单完成不启用完整治理。

检查只有收到有效 passing response 才退出 0；判官失败退出 1；用法、不支持配置、传输、协议错误或报告 IO 失败退出 2。`--help`／`--version` 不执行检查。有效的无项目检查结果仍核快照和登记清单，逐项报告未选绑定 `not-required`。

报告保留 `parity: unestablished`。当前有效输入生产者报告 `completeness_proven: false`，两份报告匹配不能证明输入完整或普遍确定性。独立 parity 比较／报告改写命令和固定 `spec status` 均已退役，调用返回用法错误；状态由真实检查生产者负责。

报告位于 ignored `.chrono-harness/state/`。`check.json` 含请求、runner／judge 身份、所选操作、执行退出／输出／执行物摘要、blocked／not-required 项、原因和范围。每个登记单元／汇总上下文分别记录固定事件输入、工作流源码 revision、candidate／base。`bootstrap.json` 记录所选配置和真实工具。

成本仍未测。同命令本身不证明本地／CI 同判：环境中的编译器后端、SDK、配置、环境、外部输入及确定求值尚未完整建模。已为声明的检查／发布消费者启用原生缓存，有限验收和输入／完整性／生命周期限制见[缓存说明](ci-cache.md)。

<a id="configuration-and-selection-contract"></a>

## 配置与选择要求

JSON 解码器拒绝重复成员；profile、provider、command、protocol 类型也拒绝未知字段。FILEMAP／projects 是范围较广的现有登记，此实现只验证消费字段，不声称验证所有提案字段含义。

`.chrono-harness/ci/check.json` 字段如下：

| 字段 | 要求 |
| --- | --- |
| schema | `chrono-ci-check/v1`、明确 Git 绑定的 `chrono-ci-check/v2`、独立单元 `chrono-ci-check/v3`，见[单元说明](ci-units.md) |
| judge | `{program, args, env?, timeout_seconds, output_limit_bytes}`；明确执行物／解释器及 argv，正上限，每条捕获流最多 64 MiB |
| report_path | `.chrono-harness/state/` 下安全相对文件，同时位于声明产物目录内 |
| policy.filemap / projects | 明确相对路径；FILEMAP v1 为历史／v2 为当前，projects v1 为历史／v2 明确语言；不支持在受检查范围两端迁移登记路径 |
| policy.tools | 工具 ID → 执行路径或 PATH 命令；旧 scoped 解析方式。当前 registration_config 提供真实版本声明，所选工具每计划绑定一次 |
| policy.bindings | 仅历史 FILEMAP v1；当前 v2 拒绝重复绑定，消费 FILEMAP.execution_plans |
| policy.registration_config | 可选完整配置路径；当前宿主用候选解码器解释历史数据及工具声明 |
| policy.facts_config | scoped v2 必需，`.chrono-harness/` 下字面路径，选择候选 full-v3 Git 绑定；v1 拒绝，包括 null |
| policy.environment | 明确传给操作的环境覆盖，本宿主设 `RUSTUP_TOOLCHAIN=1.95.0` |
| policy.artifacts | 以 `/` 结尾的相对目录前缀，允许 ignored 执行产物存在；已跟踪文件不得重叠 |
| policy.required_inputs | 此 profile 必需的明确普通已跟踪输入 |
| policy.adoption_base | null，或明确允许其缺少旧 CI profile 的精确完整 base OID |
| policy.operation_timeout_seconds / operation_output_limit_bytes | 旧操作进程的正数兜底上限 |

采用的 profile 设 `require_tool_bindings: true`，为 `cargo`、`rustc` 声明 `tool_bindings`，含预期路径、SHA-256、精确 `--version` 输出，以及可选 `allow_relocation`，允许等价本地 rustup 安装。登记操作前，生产者解析所选工具链文件，拒绝配置／PATH／编译器选择冲突，报告真实路径及身份。

projects v2 使用 registration 所有者的严格 schema。按范围 CI 对 DELTA 影响的所有者复用 projects 的双向专属配对／语言检查，含选中测试组和仅计划选中项。只改语言 metadata 也参与影响计算；改 projects schema 版本还将两端声明所有者作为起点。脚本使用明确 `script:` 节点。接受相同字面语言 ID，及生产 `shell`／测试 `python` 配对；不重判无关项目对。

这里只核声明语言，不核文件内容或隐藏辅助程序。projects v1 保留原 scoped 含义，采用 v2 不启用完整治理。

scoped v2 改变 Git 获取，保留下文选择、执行和操作环境合同。明确 `facts_config` 用 [full-v3 读取器](git-facts.md) 绑定 Git 执行物、文件摘要、精确版本、环境、逐进程上限。候选绑定读两端；绑定配置和检查 profile 须匹配固定候选 blob，无环境回退。

Git 绑定本身不要求完整登记；`registration_config` 独立选择已有完整解释和标准调用观察检查。v2 提供它时共用同一读取器和进程记录。

绑定建立后的 scoped v2 成功／失败响应都携带 `evidence.git_facts`，包括版本执行失败或版本不符，保留原 stdout／stderr 字节／摘要、退出、超时／输出限制失败。绑定前不能编造进程。`previous_enforcement` 写实际基线 profile 版本，初始清单／明确采用时为 `none`；读 v1 基线不执行其 Git。

本地／CI 标准命令和 scoped 协议不变，v1 保留旧获取方式，不自动升级宿主。可选 Git 绑定扩展从 beta.9 提供；本产品宿主当前使用独立单元 scoped v3，并在现有短命令路径明确选择 Git 配置，见前文。这不证明输入完整、完整治理或确定同判。

### 快照与变更传播

两端树以 NUL 分隔 `git ls-tree -rz` 记录读取，保留路径、blob、mode。比较实现新增／修改／删除，改名按删除加新增处理，不用改名猜测。两端全部已跟踪路径须有 FILEMAP 项，每个登记文件须存在。

相对文件符号链接须精确声明目标，目标为普通已登记文件；拒绝目录链接、submodule、不支持 mode、非 UTF-8 路径。candidate 必须为 HEAD，且是完整 40 或 64 位小写提交 OID，两对象都须存在。

暂存／未暂存修改和全部未跟踪非产物文件（含 otherwise ignored）都失败。索引 `assume-unchanged`／`skip-worktree` 不支持，即使 Git diff 隐藏修改也明确失败。判官在执行前后读 NUL 分隔索引事实，不清标志或修索引。支持的干净快照要求完整检出且无这些标志。

profile 字节须同时匹配请求摘要和候选树，操作后再核快照；操作修改已跟踪输入会失败。任意提交对只是明确比较，此实现不证明新鲜度、祖先关系或 integration 来源。

选择起点包括：变更文件、FILEMAP 记录、project／script 记录、操作、绑定、工具／环境／上限设置，以及变更图边两端。沿旧新合并图传播。只有明确 `test-execution` 边选择测试绑定；compile／build-input／runtime-input 传播到声明节点。`judge-trigger` metadata 不属此实现。

节点／绑定缺失、操作重复、action 无效、未知变更路径都会失败。删除边仍选择原来存活的消费者；删除受影响绑定或绑定操作会失败，不隐含退役、不回退全测。候选操作按 FILEMAP 调度，缺政策时串行；保留绑定顺序，共享操作去重。不执行历史命令，每条实际失败留在结果。

本宿主为受影响生产／测试对绑定格式化、生产二进制构建、专属测试执行。生产构建覆盖入口点，独立测试项目仅把库作为依赖编译不足以覆盖它；不另排冗余 Cargo check 或测试项目构建。bootstrap 在四项执行工具尚不存在时先构建它们；标准构建结果仍明确检查所选候选执行物源码。

首次采用的精确 `adoption_base` 只允许基线缺 CI profile，仍保留并验证真实基线 FILEMAP／projects。候选绑定投射到已有基线登记，新绑定不编造基线节点，报告 `previous_enforcement: none`。本仓明确采用 `c4536e5c35d06302cdec655b3f11f9c257919155`。

旧仓库未登记时，须先建立如实登记，不能用空树或候选替代基线。全新无父宿主用 `--initial`。

## 外部判官与脚本

runner 在 stdin 发送一份 JSON `chrono-ci-judge/v1` 请求：`{protocol, request_id, host_root, config_path, config_sha256, base, candidate, initial}`。响应为一份 JSON 对象：`{protocol, request_id, status, results, evidence}`。

`status` 为 `passed`、`failed`、`blocked` 或 `not-required`。`results` 非空，每项 `{id, status, cause, exit_code}`，ID 非空且唯一、cause 非空；非进程检查或基础设施失败可用 null exit_code。`not-required` 必须为 null，已执行操作不能标未执行。进程通过要求 0，进程失败摘要不能为 0。

子进程成功后，回执／清理验证若失败，失败摘要退出为 null；原回执仍保留真实退出 0、诊断、argv、环境和字节。汇总保留失败原报告，不补跑替代业务。恰好在全部结果为 `not-required` 时，总状态也为 `not-required`。

evidence 须为非空对象。ID／协议、总状态／逐项状态、实际子进程退出须一致。输出空、多余、格式错误、结果缺失、超时、溢出或崩溃都失败。进程使用有限捕获和 Unix 进程组，实现已在 macOS 验证。报告保留真实 stdout／stderr；脚本可通过明确登记解释器参加，不隐含插件登记或其他平台支持。

<a id="owned-github-projection"></a>

## GitHub 工作流的生成与所有权

完整治理有单独明确 dispatch schema [`chrono-github-full-ci/v1`](full-ci.md)，保留调用方上下文字节，本地／CI 使用同一完整 runner 命令。下面事件 profile 保留按范围含义，采用时不会静默升级。

<a id="registered-event-git"></a>

### 为事件读取登记 Git

源码实现接受 `chrono-github-ci/v3` 及明确 `facts_config: ".chrono-harness/<registered-config>.json"`。该文件须为 full config v3，声明 [Git 工具／输入绑定](git-facts.md)，作为产物目录外的政策输入，并与生成初始 profile、执行物／上下文输出分开。文件和真实消费者都须登记到 FILEMAP，生成器不推断依赖或生成 Git 声明。公开 beta.8 不包含此能力。

v3 保留事件和标准命令构造方式，可选 `initial_inventory` 保留原生成含义，省略则用单一检查 profile。v1／v2 拒绝 `facts_config`，包括 null，保留各自 Git 和初始 profile 合同。采用须明确；`init` 保留已有来源的定制和版本。

事件进行网络观察前，所选 Git 核绑定配置原字节匹配固定候选，并确认 HEAD 就是该候选。v3 事件读取、远端基线观察、对象探测、fetch、初始父读取共用同一候选绑定、环境和逐进程上限。

事件库仍消费调用方提供的 `Config` 和载荷，不证明 provider 来源或 CI 源码与候选一致。生成工作流和事件身份须由真实交付消费者验证。

批量对象探测在 stdin 发送一个完整 OID。只有精确成功响应 `OID commit` 证明存在，只有 `OID missing` 允许从 `origin` 明确 fetch。非零、超时、溢出、类型错误、格式错误或多余输出都报错。fetch 成功后再次探测并核提交身份，不把诊断文本或任意探测失败解释成历史缺失。旧版本保留原探测路径。

成功 `chrono-ci-inputs/v1` 上下文新增 `git_facts`。绑定后错误在 `E_CI_GIT` 保留观察；绑定期间失败则尽可能保留读取器 `E_GIT_FACTS` 诊断。每个实际进程保留 argv、stdin 摘要、原输出／摘要、退出及限制失败，不编造未启动进程或成功上下文。失败不覆盖旧上下文文件，调用方须读本次真实退出。

专属测试用真实本地 remote 和所选 Git 入口，包括从其他 cwd、遮蔽环境 Git 的真实 CLI 调用。覆盖历史缺失、探测失败／错误、fetch 失败、配置 integration 基线、initial／manual／PR、源码／检出变化及旧版本对照。fixture 原生测试不证明宿主采用生成 v3 事件路径。

本产品宿主当前事件配置见前文；网络、凭据、Git 配置、helper、OS、委托输入仍未完整验证。绑定不会关闭仓库配置的隐含 Git 行为，也不证明输入完整。完整启用、宿主版本采用和确定同判仍分别验收。

### provider 数据与首次清单

`.chrono-harness/ci/github.json` 的 `schema: chrono-github-ci/v1` 明确配置 `workflow_path`、`name`、`runs_on`、`push_branches`、`pull_request_branches`、`branch_creation_base_ref`、SHA 固定的 `checkout_action`／`upload_artifact_action`、`timeout_minutes`、bootstrap argv、runner、generator、check_config、context_path、artifact_directory。

路径相对宿主；此 provider 支持单 job、GitHub Actions。脚本、工具 bootstrap、runner 标签、事件分支、超时都是数据，所选测试命令不出现在 YAML。runner 命令渲染与事件证据共用标准 argv 构造器。

新登记应为 check／full／release provider 设 `include_hidden_files: true`；单元 provider 用 `collection.include_hidden_files`，也管理 detector 上下文上传。它包含明确产物根下隐藏后代，不扩根、不证明回执内容。省略或 false 保留旧生成方式及序列化来源身份。本仓检查／发布已明确开启。

要用独立根清单，在同一来源选 `chrono-github-ci/v2` 并增加下列声明，填入真实所选判官摘要，其余字段含义沿用 v1：

```json
"initial_inventory": {
  "path": ".chrono-harness/ci/initial.json",
  "profile": {
    "schema": "chrono-initial-check/v1",
    "host_config": ".chrono-harness/config.json",
    "timeout_seconds": 30,
    "stdout_limit_bytes": 1048576,
    "judges": [{
      "id": "initial-registration",
      "executable": ".chrono-harness/bin/chrono-judge-registration",
      "version": "0.1.0",
      "sha256": "<actual installed binary SHA-256>",
      "argv": ["--protocol", "chrono-initial-judge/v1"],
      "selector": "every-initial",
      "modes": ["inventory"],
      "after": []
    }]
  }
}
```

`host_config` 可为严格一层原生选择器。初始请求保留入口路径，按共享登记快照合同解析每个固定端点的直接目标；选择器和目标仍是两个独立登记文件。

`init`／`generate` 将声明生成登记 JSON profile 和工作流，`verify` 不修改文件地核对两者。来源声明拥有 profile 输出：改来源再生成，不独立维护第二份政策。初始事件准备和工作流初始分支，通过同一标准 argv 构造器选择此精确 profile；普通 DELTA 继续用 `check_config`。

v1 拒绝扩展、保留单 profile 行为，v2 必须有此声明；版本或 null 不会静默选择回退。profile 复用 runner 的真实 schema、绑定、摘要、DAG 验证器，不发现 judge ID、二进制身份、宿主语言或布局。

输出须在 `.chrono-harness/ci/` 独立路径，位于产物路径外，区别于来源、普通检查、宿主配置、执行物。来源和输出都须明确登记 FILEMAP。生成器不检查／合成五份宿主登记，也不证明内容正确。

新采用先检查两个输出，已有 profile 不同就拒绝，完全相同可复用。已有来源定制保留。登记后按来源重新生成可替换 profile 漂移，工作流标记和符号链接检查继续适用。改名／移除输出声明不自动删旧文件，AI 须明确退役旧登记和文件；不自动升级 v1 来源。

专属测试在已提交无父宿主上，以真实 runner／registration 进程消费生成 profile，覆盖未登记文件拒绝、空格路径、其他 cwd，以及旧明确存储／stdout 报告身份。短检查显示状态、有限诊断、不可变原报告位置，消费者读取已有完整文件。

公开[初始宿主示例](https://github.com/ChronoAIProject/chrono-harness-examples-initial)安装 beta.5，使用生成 v2 CI 验证真实无父首次 push，之后在 integration／PR／dev 执行普通文档 DELTA。它明确登记 macOS arm64／`macos-14` 和该平台初始判官摘要。真实证据及范围见[示例索引](examples.md)，不证明跨平台初始 profile、完整 bootstrap 来源／输入、治理启用或确定同判。

### push 基线与生成流程

可选 `push_baselines` 明确把字面分支 ref 前缀连到 `origin` 基线分支。本宿主和可复制示例采用：

```json
"push_baselines": [
  {"ref_prefix": "refs/heads/integration/", "base_ref": "refs/heads/dev"}
]
```

匹配的创建和后续 push 都把完整候选与观察到的基线顶端比较，包括 integration 分支早先提交。前缀须为以 `/` 结尾的字面字符串，不是 glob；重复／重叠前缀、基线位于自身前缀内都报错。配置了行时，push 载荷须有有效完整分支 ref。列表缺失／空保留旧 push 行为，未匹配分支用普通事件映射。不从名称、布局或语言推断分支角色。

准备器只读一次精确远端 ref，绑定完整提交 OID；需要时 fetch 该 OID，不随后从共享 `FETCH_HEAD` 取身份。ref／对象缺失、事件错误都失败，不退回载荷前提交。上下文 source 为 `push-configured-baseline-ref`。准备只绑定输入；新鲜度、稳定性、integration 认证仍归工作流判官。

`chrono-ci init --host-root H --config INPUT` 可从任意目录接受明确来源 JSON，将其采用到 `H/.chrono-harness/ci/github.json` 并生成工作流。已有采用配置保留，采用新数据前先核输出所有权。

`generate` 确定生成，不重写相同输出，拒绝无所有权／符号链接冲突。`verify` 用同一渲染器，只报漂移不修复。v1 发布单个拥有的普通工作流文件，保留无关工作流。不承诺并发写入、多文件崩溃事务或广泛 provider 框架。IO 失败可能发生在部分输出已发布后，仍是错误，后续生成须协调声明输出。

事件映射只准备输入，不是政策：

| 事件 | 固定输入 |
| --- | --- |
| 匹配 `push_baselines` 的 push | 创建及后续 push 均用观察到的配置基线 ref／载荷 `after` |
| 未匹配的已有分支 push | 载荷 `before`／`after` |
| 创建配置基线分支（`created`、before 全零、ref 匹配） | 无父 `after` 用同一 check 加 `--initial`，无 base，模式 `initial-inventory`；有父候选要求明确范围 |
| 创建其他未匹配分支（`created`、before 全零、ref 不同） | 观察一次配置创建基线 ref，固定提交，与载荷 `after` 比较；候选等于已有基线就如实无操作 |
| 指向 dev 的 PR | 事件 `pull_request.base.sha`／`head.sha`；检出 head，不用默认 merge ref |
| workflow_dispatch | 明确完整 base／candidate，或明确 initial 加无父 candidate |
| 删除分支 | 无候选检查，不能生成通过的候选报告 |

这里的单工作流 provider 检查 dev、integration/** push，指向 dev 的 PR 和手动 dispatch。已采用独立单元 provider 复用事件准备，collector 没有手动触发，见[单元说明](ci-units.md)。

工作流检出事件候选、bootstrap 候选登记工具、调用 `chrono-ci prepare`、运行标准检查，失败也上传证据。checkout 临时读取凭据保留给必要对象获取，由 post-job 清理移除。对象缺失须明确 fetch 或失败。手动 dispatch 可用性可能取决于默认分支是否登记工作流。源码 revision 用 `github.workflow_sha` 独立记录。

创建事件须有有效 `refs/heads/...` 载荷 ref。创建基线本身绝不把新候选当作自己的旧基线。无父首次 push 记 source `baseline-creation-initial-inventory`、null base、`initial-inventory`；其他未匹配创建记 `branch-creation-baseline-ref`、`delta`。

有父首次基线 push 须通过手动 dispatch 或本地标准命令提供真实完整 base／candidate OID；普通创建路径失败，不编造范围。

登记操作 `ci.verify` 核漂移，FILEMAP 明确连到 generator／config／workflow／bootstrap 源。CI 不在比较前重新生成。工作流绿只证明实际所选检查，不保证分支保护／ruleset；不增加人审门。

## 在其他宿主采用

[`examples/ci-host`](../examples/ci-host/README.md) 是完整小型宿主包，含明确 FILEMAP、projects、检查 profile、provider 配置、安装 bootstrap、Python 检查和输入。运行只需已安装／复制二进制及 Python，无需 harness 源码，也不推断语言／项目路径。

GitHub bootstrap 要求自托管 runner 上明确配置的工具目录；托管 runner 须明确登记固定版本安装／下载步骤。不假定不存在的预构建分发或隐含联网安装。

复制后安装三个二进制、配置 bootstrap 来源、执行 init，再提交初始化清单。复制的 `chrono-ci` 可从其他 cwd、含空格宿主路径使用。加检查时登记真实操作、文件／边及 profile 绑定；改 runner／bootstrap／事件数据时，编辑 provider 并 generate，YAML 只作生成结果。

专属行为测试覆盖真实成功／失败子进程、协议错误／超时、依赖范围内外 DELTA、边／操作变化及删除、未知登记、精确快照、采用、初始清单、无变化生成、漂移／所有权冲突、保留、扩展、事件映射。原指令行为仍由 61 项测试覆盖。本地测试不证明 GitHub 原生路由；宣称外部结果前，交付须核实际候选运行、选择、成功／失败及 PR head 行为。

首次登记执行的原生 [36292159644](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36292159644) 在 macos-14-arm64 bootstrap 成功后，标准 harness 步骤失败。base `4f08aef7ab40d7b0a3fb6ba42af2620600f18d98`，candidate／工作流源码 `556fdb73b6f93275d18bd210b9b01be1173daec9`，原错误 `E_TOOL_BINDING: migration version mismatch`，未执行受治理操作。报告当时没保留解释器真实版本输出。

修复后宿主使用明确系统解释器路径，避免 PATH 优先级影响，保留精确版本要求。本地 Darwin arm64 的 `/usr/bin/python3 --version` 返回 `Python 3.9.6`，修复后的 integration／PR／dev 也观察到该路径／版本。

已验证 [dev 36295299904](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36295299904) 从同一 base 检查到落地 `f8338fd9496878914216e2c070d8320ea50fbb6e`，34 项操作、227 项 Rust 测试通过。树与独立审读候选相同，完整同判仍未建立。迁移绑定失败现在保留预期／观察版本诊断。

## 按需运行的独立 job

当前宿主在一个父工作流内生成明确 detector、按条件运行的独立单元 job、始终运行的汇总。分支保护只要求汇总。选择来自完整 Git 事件 DELTA，不用工作流 paths 过滤器或 API 文件列表。

真实原生检查保持固定短入口 `check`、`check --unit ID`、`check --collect`。provider 未明确采用 `job_gating` 时，保留原独立工作流。登记、原件收集、重跑、迁移及完整调度边界见 [CI 单元](ci-units.md)。

<a id="standalone-initial-host-transition"></a>

## 迁移独立初始宿主

CI 所有者已有 `migrate`，可把明确保留的独立 v2／v3／v4 清单 provider 迁到同址或新址的独立 v4。保留旧来源，用 `--previous-config` 声明原地址，再以新 provider 调用登记迁移入口。

写入前核旧工作流和生成清单，保留精确清单声明及输出；拒绝改动／删除清单，或迁入单元汇总。宿主配置、facts／平台政策、FILEMAP、profile 绑定都须明确编辑，迁移不推断选择。

之后 `generate` 幂等，`verify` 只检测漂移不修复。这是候选源码行为；公开安装、原生首次提交事件和兼容发行锁仍须各自交付证据。
