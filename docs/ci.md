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

`selection_explanation.extra_selections` 解释操作、绑定、工具、环境或限制变化带来的额外选择。这些不变成完整政策边，见[影响范围说明](filemap-impact.md)。`instructions` 仍独立，catalog、布局和相对根别名不变。

按范围政策可明确选 `selection_evidence: "explicit-graph/v1"`：保留完整两端合并 `edges`、`seed_causes`、额外选择及原因；`closure.schema: "chrono-ci-reachability/v1"` 发布排序的 `reached` 节点和图选中的 `selected_tests`。由保留图和原因可重建前驱及贡献起点表，因此传输省去重复派生字符串，不改选择或执行。省略时发布完整传播结果，null／未知格式在业务前失败。

按范围 CI 的起点由 `seed_causes[].node` 维护，不再额外发布重复的 `seeds` 列表；`extra_selections` 与图选测结果已能区分额外选择，也不再发布两者差集 `legacy_only_selections`。现役读取方已迁移，完整 FILEMAP 的独立 impact 合同保持原格式。输出及计算成本收益尚未实测。

本宿主因 2697 条私有声明使原响应超过未改动的 8 MiB 上限而采用此表示。Rust 测试保留全部 2697 个明确起点输入和输出限制。完整 FILEMAP 影响输出保留原版本格式。源码／bootstrap／标准／原生验收及总验证成本仍分别处理。

五份完整治理登记仍为 **proposed**。当前有限宿主 profile `chrono-ci-check/v3` 明确消费 FILEMAP／项目数据，独立单元／汇总使用 `chrono-ci-judge/v2`；不分单元的本地检查及旧 profile 保留各自说明的行为，见[独立 CI 单元](ci-units.md)。

此外，`check` 可通过登记 full profile 使用 `chrono-judge/v1` 及 registration／filemap／routes／projects 判官。schema4 自动产生上下文，旧 full profile 仍要求明确上下文。完整治理尚未完成，实际求值会非零退出；工作流已有[有限认证](workflow.md)，完整自动生命周期仍未实现。[混合修改](mixed.md)已有专属分类／警告判官和真实链测试，成本报告也有专属判官及登记消费者测试。共享操作计划、执行和回执比较见[执行说明](execution.md)。

<a id="this-repository"></a>

## 在本仓运行

核心 bootstrap 及所有者构建／验证 profile 把可写 HOME、CARGO_HOME、TMPDIR 放在 `.chrono-harness/cache/bootstrap-owner-tooling/`。可重建工具沿用已登记缓存生命周期；原报告、输出流、安装来源留在 `.chrono-harness/state/`。`host_environment` 只接受缓存路径，保留字面路径和符号链接检查。

可重建工具不随证据重复上传；原生总量和耗时收益尚未验证，原 bootstrap 和标准检查仍须执行。

CI fixture 用已有 `resolve_program` 从继承的 PATH 解析 Git，不在仓库维护主机路径清单；找不到程序就失败。隔离测试在遮蔽 PATH 前解析真实 Git，并用 `CHRONO_CI_FIXTURE_GIT` 传给自己的子进程；生产调用仍只使用已绑定的工具。准备命令和绑定 Git 消费者使用所选执行物，故障适配器在本轮输入登记保留实际路径、版本和摘要。这不改变消费宿主的 Git 政策，也不证明 Git 输入完整；剩余真实 Git fixture 的命令边界 mock 迁移仍未完成。

从仓库根运行。core、缓存、detector、单元准备、共享启动、Verus 准备和所有者阶段构建由 Rust 执行，统一通过薄 shell 入口启动。首次前端编译需要 Git、rustup 和固定 Rust 1.95.0；已有工具后按 profile 选择并核验 SDK，独立确保 rustfmt 存在。所有者资格读取和构建也由该 Rust 实现维护。发布编排同样使用 Rust 与薄 shell；Python 解释器只由独立 Python 业务示例及其测试按自身登记使用。

没有现役 profile 使用的私有工具链安装分支及专属测试已退役。bootstrap 沿当前工具链选择：Cargo 不可用时安装指定版本的 minimal 加 rustfmt，缺格式化组件时只补该组件；可写构建目录仍由 `host_environment` 的缓存路径声明。工具身份、编译器选择及失败原件继续由原入口核验和保留，不为主机盘点另装完整 SDK。

FILEMAP 按实际读取者登记这些源配置：Rust 前端读取工具链登记、阶段 profile 和升级声明；worktree 在前端启动和阶段原件核验中读取相应 profile、工具链登记与 projects。关系分别指向生产项目，已有 shell／测试边不能替代它们；每次构建的观察和原件仍归 state／线上 artifact。

Rust 前端按 `.chrono-harness/projects.json` 构建明确列出的 bootstrap 操作，再复制声明二进制，不复制 Cargo 配方或发现项目。工具链操作明确点名版本，不改全局默认。专属 CI 测试的复制宿主回归消费已安装候选工具，须先 bootstrap：

```sh
/bin/bash .chrono-harness/ci/bootstrap-native.sh . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

所有者升级的阶段构建由 `bootstrap-owner-stage.json` 选择薄 shell 入口 `bootstrap-native.sh`，
在已有 bootstrap 租约内用固定 Rust 1.95.0 构建并调用本宿主的 `chrono-host-bootstrap`。
Rust 生产者执行登记的前置、SDK 选择和构建，保存原报告及两流，再交出现有 stage 描述符；
阶段校验读取同一生产者的固定原件。shell 只转交参数，不复制构建配方或解释报告。
升级的 `bootstrap-owner-upgrade/attempt-*` 按整目录交给现有 retention：
真实调用结束后，阶段报告的使用方保护原子交接为该目录的依赖，成功后可滚动回收。
失败或中断沿宿主有限诊断期限保留；没有有效 JSON 时保留原始 stdout、stderr 和退出码。
活的目录发布租约保护这些依赖，不另建可能在中断后永久遗留的父使用方根。
普通入口先通过已部署所有者构建并安装前端到 `.chrono-harness/bin/chrono-host-bootstrap`，再经同一固定版本的 `rustup run` 执行该保留文件，保持首次编译与后续调用的 Rustup 目录选择一致；不在释放租约后执行可回收缓存中的二进制。关联检出缺少协调所有者时在编译前拒绝。core 的升级编排也由 Rust 执行，Verus 准备沿同一入口委托登记操作。

项目操作的 `bootstrap-native` 工具槽也指向上述保留文件。首次执行这些操作前，先走同一候选安装入口；完成缓存释放后，构建前置继续消费已安装前端，不依赖 Cargo target 中的临时执行物。

Verus 下载、摘要校验和失败原件见[固定执行入口](verus.md#固定执行入口)，缓存复用见[Verus 固定下载包](ci-cache.md#verus-archive)。

本宿主只维护当前采用的安装 profile。薄 shell 入口必须明确传入 `HOST CONFIG`；普通构建的 Rust 前端使用相同参数，不另设 `bootstrap` 子命令。
旧的全工具 `bootstrap.json` 与未被执行计划调用的 `host.bootstrap` 操作已退役。
detector、缓存和各单元仍消费各自显式登记的 profile，不因默认安装而取得额外工具。
共享启动的 produce／install／verify 已登记到同一个 Rust 前端；produce 按所选 profile 的 `entrypoint` 和工具声明调用真实生产者。缓存、detector 和单元 profile 同样选择原生入口，操作 ID、原始回执、产物绑定及失败合同保留。消费者首次从源码准备前端的构建与存储成本仍须实测，不能把语言迁移算作性能收益。旧 Python 启动实现、伴生测试及导入专属断言已随入口退役。发布编排归现役 Rust 发布项目，详见[发布说明](release-ci.md)；外部宿主不运行本仓这项源码构建。
**实现与宿主政策同时变更：**本批修改 core／Verus 的正式方法及所属登记，增加前端安装和共享生产入口的行为验证；原生构建、真实升级及受影响标准／CI 消费分别验收。新增前端构建、验证和存储总成本未测。

提交修改、确保候选检出干净后，本地和生成 CI 使用同一标准判断命令：

```sh
.chrono-harness/bin/chrono-harness check --unit ci
```

生成的单元／汇总工作流选择 runner、judge-ci、ci、本地 worktree 输入生产者；其余构建来自原完整测试计划。并发运行单元须用不同检出。

本地不带 `--unit` 时，按 FILEMAP 调度执行整个所选 DELTA；同一入口加无值 `--collect`，则准备声明清单、核单元证据，不执行业务。

### 短命令的输入准备

本宿主使用 config schema4。`canonical_check` 声明 `profile`、`inputs.local` 及可选 `inputs.ci`。生产者沿用 `{operation,tool,argv}` action、工具／环境／协议上限。runner 观察真实短入口，调用绑定生产者，保留原进程证据，再通过已有判官路径执行准备后的类型化检查，不重新分发虚构的长参数。

准备记录环境值摘要及原环境摘要。可选 `environment.credential_environment` 从 `inherit` 点名仅供获取的凭据：传给生产者和其声明的收集传输，不传给 Git 或判官。本仓声明 `GH_TOKEN`，与已有 gather 凭据政策一致。外层获取继承也将已声明 `SSH_AUTH_SOCK` 交给 worktree Git 所有者；缺失仍记为缺失。环境清空和各所有者明确白名单继续适用。

获取限 2400 秒，覆盖登记的 1800 秒原生汇总等待；冲突的更短获取上限明确失败。
独立汇总的输入生产者在准备 Git 事件端点前拒绝这种静态时限冲突；条件父汇总不读取这项独立轮询检查所用的政策。

`CHRONO_CHECK_SOURCE` 必须明确继承，不能固定值覆盖。未设或 `local` 选本地，`ci` 选原生；其他值、缺少原生绑定／变量都会失败。CI action 明确点名事件、载荷、工作流 revision 和汇总仓库变量。`chrono-github-ci/v4` 生成短检查步骤，含该选择器和工作流 revision；checkout、bootstrap、上传仍是原生步骤。生成器直接生成所选步骤，不再先构造 v1/v2/v3 使用的显式端点准备和初始／DELTA 步骤，再把它们删掉。

按范围 profile 将 registration_config、facts_config 都明确绑定到固定配置入口，短命令获取不使用旧未绑定 Git。push、PR、分支创建、明确手动端点的含义仍归已有事件生产者。准备只是获取输入，尚未作判断。

空业务宿主可用独立 v4 provider 和明确 `initial_inventory`，保留空 projects／scripts／plans／units。schema4 可选 `canonical_check.initial_profile` 点名单独初始 profile，`profile` 仍用于 DELTA。请求绑定两路径及字节摘要，provider 的清单路径和生成 profile 必须与初始绑定一致，两者都是候选输入。

生产者只在真实物理 Git 根或原生事件证据支持时选初始清单。准备和报告保留所选 profile、精确请求、原生产者／报告对应及真实裸入口。初始判官输出 `initial-inventory`、null base／DELTA、`governance: not-evaluated`，不证明 DELTA 治理。初始绑定缺失／错误会失败。

单元／汇总模式仍须有真实登记单元消费者，单元 provider 的非空要求不变。已有 scoped v3 的明确非空单元初始清单，保留自己的 profile 和首次事件含义，与独立初始判官不同。

### 固定端点与上下文

本地使用已声明 worktree Git／remote／workflow target，每次从干净、已提交、非根 HEAD 获取并固定目标。获取前关闭 replacement objects，用 `cat-file commit` 读取物理提交头。真正无父 HEAD 使用明确绑定的独立清单，无远端 base；浅历史边界或父对象缺失不能抹去物理父头。非根仍须登记远端目标，缺 fetch／base 证据就失败。

按范围检查不要求 origin 或分支年龄上下文。完整、不分单元的检查要求 worktree v2 声明 origin／context 路径和创建种类／运行角色映射。start／reconstruct 发布最终原 birth 报告和精确目标关联；生产者保留创建时间／分叉点，观察当前端点／时间。历史快照或交付证据交接缺失就明确失败。

full CI v1 保留精确上下文传输；v2 生成短检查，并在所选原生上传根保留相同原始 dispatch 上下文字节。原载荷／上下文／生产者／回执在该根下寻址、验证，也支持收窄的自定义根。full v2 当前 context／report 文件仍是可替换结果，其短命令原始证据不可变。

full v1 重复准备若只差进程所有权描述符编号，则返回原准备，在声明产物目录的 `preparation/native-attempt-<sha256>.json` 保留新真实报告。上下文字节及其他观察必须一致；Git 结果、环境或输入身份改变，仍报输出冲突。完整独立范围、固定轮次和到期复核尚未实现。

本仓单元／汇总工作流登记 `macos-26`，scoped 检查按 OS／架构选择政策，在本次运行内观察并固定同一实际 Git，不维护各台主机的 Git 摘要／版本白名单。实际路径和身份归本轮报告；full v3 的显式预期合同保留。开发工具目录尚未明确选择，托管原生 runner 的声明输入完整匹配仍未验证；有限原生成功不证明这个完整范围或同判。当前宿主只接受登记的短命令，拒绝部分覆盖。

### 从候选构建工具

检查前必须从候选检出 bootstrap。runner 报告实际 runner／judge／operation 执行物摘要。所采用 core、cache profile 还把 rustup 所选 Cargo／rustc 绑定到声明 SHA-256、版本，明确有效 `RUSTC`，在报告保留本地实际路径。

共享 startup 在安装、verify 前复核同一身份；字节／版本相等时允许等价迁移，安装报告保留生产路径和本地执行路径。绑定缺失、冲突或变化，在 bootstrap 验收前拒绝。这不证明可复现构建或编译器／后端／SDK／输入完整。

core、cache、detector 共用源码登记 `.chrono-harness/ci/bootstrap-toolchains.json`，明确声明 Darwin／arm64（macos-26 CI）及 Linux／x86_64（ubuntu-24.04 发布 cache-bootstrap）。未知／缺少组合在准备工具前失败。

每个执行物 SHA-256 都从官方 Rust 1.95.0 组件归档的精确 Cargo／rustc 成员取得，独立于本机安装。登记保留官方 manifest URL／摘要／日期及组件 URL、归档摘要、成员、大小、版本来源。归档摘要只说明来源，不能代替执行物字节预期。Cargo 上游版本实现解释 package 0.96.0／CLI 1.95.0 的区别。

获取这些预期是准备已提交输入；bootstrap 不下载或从当前安装学习预期，仍用已有 rustup 准备工具。profile 直接选 rustup，不写个人安装路径。调用方提供私有 Rustup／Cargo home 时，继承 PATH 须把所选工具链 `bin` 放在 Rustup 代理前。专属消费者清除无关 Rustup 变量，Cargo doctest 仍须从同一安装找到 `rustdoc`。

bootstrap 只改自身子进程环境，不改调用方之后的标准检查环境。记录真实路径／身份，不改全局默认。旧内联可迁移绑定中，重复的声明／配置原路径只是历史位置，即使已失效或缺失也不探测；另行明确配置的当前执行物必须存在并匹配。不可迁移绑定仍要求精确路径。

实际 Cargo、有效 rustc、继承编译器冲突、包装器及构建后变化都继续检查。源码报告含登记摘要，共享 produce／install／verify 在安装前核同一声明。config 输入清单、FILEMAP、标准 required_inputs、缓存兼容输入都明确登记这个新来源。之后的独立 Cargo action 仍有已说明的输入完整性缺口。

每次 bootstrap 在 profile 状态目录下不可变 `bootstrap-*/report.json` 旁保留操作 argv、cwd、执行物摘要、PID、退出及原 stdout／stderr 文件。输出流立即转发给已有、有上限的生命周期所有者。失败保留原流和失败报告，不替换上次成功报告、不安装工具。

原生生产者用现有 `check-original` 登记整个新建目录，创建前持久发布意图，原进程及子孙共用目录租约。当前报告先保护新旧目录及实际复用的原件，再替换报告并释放旧引用；当前安装对 stage 原件的既有引用继续有效。成功且不再使用的旧目录可滚动回收，失败和中断目录使用宿主的 `diagnostic_keep_seconds` 窗口，真实活跃租约始终阻止删除。一次目录纳管不需要逐文件登记，也不接管历史未知目录。构建／安装及旧宿主升级尝试目录的其他生产者仍须各自采用，不据此声明所有产物已经纳管。

共享 produce／install／verify 核源码登记、原操作选择、编译器环境及每份流字节。传入的生产路径只作来源；本地绑定按 profile 单独核。

### bootstrap 命令 mock 与证据保管

登记操作 `test.host-bootstrap-native-tests` 直接运行 Rust 专属测试项目。外部 Git、Cargo、curl、rustup、编译器及工作树所有者 CLI 都使用明确配置的 Rust mock；测试核对程序选择、字面参数、cwd、输入、原 stdout／stderr、退出状态和描述符转交。公开入口回归覆盖已有所有者委托、未产生管理报告前拒绝，以及带原报告引用的失败；委托后不得发生本地构建效果。

mock 只证明候选调用和结果处理，不证明真实外部工具行为。真实源码构建、所有者部署、共享产物生产／安装和后续 canonical 检查仍由正式生产步骤执行，调用方分别核对其原报告与两条输出流。

全部现役 bootstrap profile 只登记 Rust 入口。Python `bootstrap.py`、所有者／共享模块、`process_fds.py`、专属测试及假工具均退役。固定生产者源码和登记仍可按原 commit 读取为数据；候选不会执行旧入口，也不因退役清除仍有诊断或验收用途的失败原件。

主宿主、scoped 检查和发布配方也移除没有现役操作使用的 Python 工具绑定。启动仍使用明确登记的固定 Bash；其 PATH 遮蔽回归由 Rust 执行。共享测试宿主及动态 Python 行为已迁到 Rust，Python 业务示例及其测试按自己的登记使用解释器。

Python 导入／字节码、Python 夹具保管实现及其专属断言随实现删除，不宣称逐项转移。当前生命周期边界分别由 Rust bootstrap 的调用测试、worktree 的真实内核租约／文件事务测试和正式宿主生产验证承担。脱离父进程的消费者由 Rust helper 启动，继续核对父调用返回后缓存仍受保护、子进程释放后才可回收。

仅供旧 Python 测试操作使用的 `bootstrap-test-run` 保留生产者随操作退役，不再分配新运行目录。已有失败／中断原件继续按原保管关系保护；删除登记不表示原消费者已释放，也不授权删除历史目录。

受管子进程全部结束后，Rust 生产者才请求已有 `finish --artifacts-only`。所选工作树所有者核对真实释放、精确输出和处置回执；保留原状态、原流及仍被消费的 startup manifest。活租约、缺失报告或处置未完成继续拒绝，释放缓存不表示任务完成，也不删除源码检出。

**产品与宿主政策同时变更：**本次移除旧 Python 实现和测试定义，将 causal consumption 的证据操作改为现役 Rust 专属测试，并同步 CI、发布、FILEMAP、工作树测试分组和原件读取边界。退役不会产生历史兼容义务，也不能用 mock 通过替代真实生产验收；验证、构建、存储和总成本仍未测。

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
| policy.adoption_base | 省略／null，或明确允许其缺少旧 CI profile 的精确完整 base OID |
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

首次采用的精确 `adoption_base` 只允许基线缺 CI profile，仍保留并验证真实基线 FILEMAP／projects。候选绑定投射到已有基线登记，新绑定不编造基线节点，报告 `previous_enforcement: none`。本宿主已完成首次接入，现役 PR／dev 的两端均有检查 profile，因此移除宿主中的旧基线例外；首次接入记录归 Git／PR。

旧仓库未登记时，须先建立如实登记，不能用空树或候选替代基线。全新无父宿主用 `--initial`。

## 外部判官与脚本

runner 在 stdin 发送一份 JSON `chrono-ci-judge/v1` 请求：`{protocol, request_id, host_root, config_path, config_sha256, base, candidate, initial}`。响应为一份 JSON 对象：`{protocol, request_id, status, results, evidence}`。

`status` 为 `passed`、`failed`、`blocked` 或 `not-required`。`results` 非空，每项 `{id, status, cause, exit_code}`，ID 非空且唯一、cause 非空；非进程检查或基础设施失败可用 null exit_code。`not-required` 必须为 null，已执行操作不能标未执行。进程通过要求 0，进程失败摘要不能为 0。

子进程成功后，回执／清理验证若失败，失败摘要退出为 null；原回执仍保留真实退出 0、诊断、argv、环境和字节。汇总保留失败原报告，不补跑替代业务。恰好在全部结果为 `not-required` 时，总状态也为 `not-required`。

evidence 须为非空对象。ID／协议、总状态／逐项状态、实际子进程退出须一致。输出空、多余、格式错误、结果缺失、超时、溢出或崩溃都失败。进程使用有限捕获和 Unix 进程组，实现已在 macOS 验证。报告保留真实 stdout／stderr；脚本可通过明确登记解释器参加，不隐含插件登记或其他平台支持。

<a id="owned-github-projection"></a>

## GitHub 工作流的生成与所有权

完整治理有单独明确 dispatch schema [`chrono-github-full-ci/v2`](full-ci.md)，保留调用方上下文字节，本地／CI 使用同一 schema4 短 runner 入口。下面事件 profile 保留按范围含义，采用时不会静默升级。

<a id="registered-event-git"></a>

### 为事件读取登记 Git

当前 scoped 来源只接受 `chrono-github-ci/v4` 及明确 `facts_config: ".chrono-harness/<registered-config>.json"`。该文件须提供 full config v3 的显式 Git 绑定，声明 [Git 工具／输入绑定](git-facts.md)，作为产物目录外的政策输入，并与生成初始 profile、执行物／上下文输出分开。文件和真实消费者都须登记到 FILEMAP，生成器不推断依赖或生成 Git 声明。公开 beta.8 不包含此能力。

v4 的工作流只调用 `check`、`check --unit ID` 或 `check --collect`，输入准备由登记的标准入口负责。可选 `initial_inventory` 生成独立初始 profile，省略则用单一 DELTA profile。旧 scoped v1／v2／v3 已退役，在发布输出前拒绝；没有未绑定 Git 的回退。`init` 保留已有当前来源的宿主定制，不自动迁移旧来源。

事件进行网络观察前，所选 Git 核绑定配置原字节匹配固定候选，并确认 HEAD 就是该候选。事件读取、远端基线观察、对象探测、fetch、初始父读取共用同一候选绑定、环境和逐进程上限。

事件库仍消费调用方提供的 `Config` 和载荷，不证明 provider 来源或 CI 源码与候选一致。生成工作流和事件身份须由真实交付消费者验证。

批量对象探测在 stdin 发送一个完整 OID。只有精确成功响应 `OID commit` 证明存在，只有 `OID missing` 允许从 `origin` 明确 fetch。非零、超时、溢出、类型错误、格式错误或多余输出都报错。fetch 成功后再次探测并核提交身份，不把诊断文本或任意探测失败解释成历史缺失。

成功 `chrono-ci-inputs/v1` 上下文新增 `git_facts`。绑定后错误在 `E_CI_GIT` 保留观察；绑定期间失败则尽可能保留读取器 `E_GIT_FACTS` 诊断。每个实际进程保留 argv、stdin 摘要、原输出／摘要、退出及限制失败，不编造未启动进程或成功上下文。失败不覆盖旧上下文文件，调用方须读本次真实退出。

专属测试用真实本地 remote 和所选 Git 入口，包括从其他 cwd、遮蔽环境 Git 的真实 CLI 调用。覆盖历史缺失、探测失败／错误、fetch 失败、配置 integration 基线、initial／manual／PR、源码／检出变化和旧格式拒绝。fixture 测试不证明当前候选的原生采用。

本产品宿主当前事件配置见前文；网络、凭据、Git 配置、helper、OS、委托输入仍未完整验证。绑定不会关闭仓库配置的隐含 Git 行为，也不证明输入完整。完整启用、宿主版本采用和确定同判仍分别验收。

### provider 数据与首次清单

`.chrono-harness/ci/github.json` 的 `schema: chrono-github-ci/v4` 明确配置 `workflow_path`、`name`、`runs_on`、`push_branches`、`pull_request_branches`、`branch_creation_base_ref`、SHA 固定的 `checkout_action`／`upload_artifact_action`、`timeout_minutes`、bootstrap argv、runner、generator、check_config、facts_config、context_path、artifact_directory。

路径相对宿主；此 provider 支持单 job、GitHub Actions。脚本、工具 bootstrap、runner 标签、事件分支、超时都是数据，所选测试命令不出现在 YAML。工作流与事件证据都记录对应短命令。

新登记应为 check／full／release provider 设 `include_hidden_files: true`；单元 provider 用 `collection.include_hidden_files`，也管理 detector 上下文上传。它包含明确产物根下隐藏后代，不扩根、不证明回执内容。省略或 false 保留旧生成方式及序列化来源身份。本仓检查／发布已明确开启。

要用独立根清单，在同一 v4 来源增加下列声明，并在 schema4 `canonical_check.initial_profile` 绑定相同路径，填入真实所选判官摘要：

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

`init`／`generate` 将声明生成登记 JSON profile 和工作流，`verify` 不修改文件地核对两者。来源声明拥有 profile 输出：改来源再生成，不独立维护第二份政策。标准输入生产者按真实拓扑和事件选择此精确 profile；普通 DELTA 继续用 `check_config`，工作流仍调用同一短命令。

`initial_inventory` 可省略，显式 null 和无效声明均失败。profile 复用 runner 的真实 schema、绑定、摘要、DAG 验证器，不发现 judge ID、二进制身份、宿主语言或布局。

输出须在 `.chrono-harness/ci/` 独立路径，位于产物路径外，区别于来源、普通检查、宿主配置、执行物。来源和输出都须明确登记 FILEMAP。生成器不检查／合成五份宿主登记，也不证明内容正确。

新采用先检查两个输出，已有 profile 不同就拒绝，完全相同可复用。已有来源定制保留。登记后按来源重新生成可替换 profile 漂移，工作流标记和符号链接检查继续适用。改名／移除输出声明不自动删旧文件，AI 须明确退役旧登记和文件。

专属测试在已提交无父宿主上，以真实 runner／registration 进程消费生成 profile，覆盖未登记文件拒绝、空格路径、其他 cwd，以及旧明确存储／stdout 报告身份。短检查显示状态、有限诊断、不可变原报告位置，消费者读取已有完整文件。

公开[初始宿主示例](https://github.com/ChronoAIProject/chrono-harness-examples-initial)锁定独立 beta.20 发布，保留该发布的生成 v2 CI；历史采用曾验证真实无父首次 push，之后在 integration／PR／dev 执行普通文档 DELTA。它明确登记 macOS arm64／`macos-14` 和该平台初始判官摘要。真实证据及范围见[示例索引](examples.md)，不证明跨平台初始 profile、完整 bootstrap 来源／输入、治理启用或确定同判。

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

生成独立工作流和条件父工作流时，共用同一渲染器的头部和任务片段。父工作流直接组合所需片段，不再先生成整份工作流再截取。任务渲染仍核对政策、来源路径和初始清单的路径冲突。v4 独立工作流也不再构造未使用的旧准备参数和收集步骤。生成内容保持不变；运行时间和总成本收益未测。

内部配置读取函数 `units::profile` 不再复制整份单元表后一并返回。准备检查和汇总报告的使用方直接读取或移出配置中的原表，继续执行原有的输入和路径验证。

事件映射只准备输入，不是政策：

| 事件 | 固定输入 |
| --- | --- |
| 匹配 `push_baselines` 的 push | 创建及后续 push 均用观察到的配置基线 ref／载荷 `after` |
| 未匹配的已有分支 push | 载荷 `before`／`after` |
| 创建配置基线分支（`created`、before 全零、ref 匹配） | 无父 `after` 由同一短命令选择显式初始 profile，无 base，模式 `initial-inventory`；有父候选要求明确范围 |
| 创建其他未匹配分支（`created`、before 全零、ref 不同） | 观察一次配置创建基线 ref，固定提交，与载荷 `after` 比较；候选等于已有基线就如实无操作 |
| 指向 dev 或 integration/** 的 PR | 固定 workflow merge commit 的第一父提交／该合并提交；须恰有两个父提交且第二父提交等于事件 head；检出合并提交 |
| workflow_dispatch | 明确完整 base／candidate，或明确 initial 加无父 candidate |
| 删除分支 | 无候选检查，不能生成通过的候选报告 |

这里的单工作流 provider 检查 dev、integration/** push，指向 dev 的 PR 和手动 dispatch。已采用独立单元 provider 复用事件准备，collector 没有手动触发，见[单元说明](ci-units.md)。

**产品与事件基线合同同时改变。** PR 的 `pr-workflow-merge-tree` 来源由现有绑定 Git 读取固定 workflow 对象及父身份，再由同源 Verus 身份谓词验证父数量和候选对应。缺对象、非双亲或第二父不符保留 `E_CI_GIT` 和原进程；旧事件 base 不作为替代路径。候选源码为固定合并树，detector 固定后各单元沿用同一对端点。`event_head` 独立记录事件 head，用来核对 GitHub 运行、job 和 artifact API 的 `head_sha`，不能替代被验候选。宿主默认关闭 strict 并保留落地检查，见[默认合并方案](merging.md)。新增成本是现有不可变对象／父读取，完整 CI 与存储成本未知。

工作流检出事件候选、bootstrap 候选登记工具、运行登记的短检查，失败也上传证据。标准输入程序委托事件准备，不生成额外长命令步骤。checkout 临时读取凭据保留给必要对象获取，由 post-job 清理移除。对象缺失须明确 fetch 或失败。手动 dispatch 可用性可能取决于默认分支是否登记工作流。源码 revision 用 `github.workflow_sha` 独立记录。

创建事件须有有效 `refs/heads/...` 载荷 ref。创建基线本身绝不把新候选当作自己的旧基线。无父首次 push 记 source `baseline-creation-initial-inventory`、null base、`initial-inventory`；其他未匹配创建记 `branch-creation-baseline-ref`、`delta`。

有父首次基线 push 须通过手动 dispatch 或本地标准命令提供真实完整 base／candidate OID；普通创建路径失败，不编造范围。

登记操作 `ci.verify` 核漂移，FILEMAP 明确连到 generator／config／workflow／bootstrap 源。CI 不在比较前重新生成。工作流绿只证明实际所选检查，不保证分支保护／ruleset；不增加人审门。

## 在其他宿主采用

[`examples/ci-host-job-gating`](../examples/ci-host-job-gating/README.md) 是当前可复制的宿主包，含明确 FILEMAP、projects、检查 profile、provider 配置、安装脚本、Python 检查和输入。运行使用已安装／复制的四个工具及 Python，无需 harness 源码，也不推断语言或项目路径。

GitHub bootstrap 要求自托管 runner 上明确配置的工具目录；托管 runner 须明确登记固定版本安装／下载步骤。不假定不存在的预构建分发或隐含联网安装。

按示例说明复制文件、安装工具，绑定实际 Git 和 Python 身份，配置 bootstrap 来源，执行 init／verify，再提交完整登记。日常使用同一短入口 `check`、`check --unit ID` 和 `check --collect`。加检查时登记真实操作、文件、依赖边和 profile 绑定；改 runner、bootstrap 或事件数据时，编辑 provider 并 generate，YAML 只作生成结果。

当前示例的真实执行测试覆盖首次单元清单、短命令、独立单元、汇总和失败原件；首次清单与空 DELTA 另有现役行为回归。旧 v1 示例包及三个仅服务该包的测试已退役，共享执行和回执辅助函数继续由当前单元测试使用。其他成功／失败子进程、协议错误／超时、范围选择、生成、漂移、归属和事件映射测试保留。本地测试不证明 GitHub 原生路由；宣称外部结果前，交付须核实际候选运行、选择、成功／失败及 PR head 行为。

本仓现役项目操作使用 Rust 与薄 shell，检查政策不再声明没有对应操作的 Python 工具或主机版本。Python 业务示例在自己的配置中声明实际解释器；语言配对和脚本登记测试中的 Python 标识只作配置样本，不启动解释器。工具绑定失败仍须保留预期、观察值和原进程；bootstrap 完成不代表受治理操作已执行。

## 按需运行的独立 job

当前宿主在一个父工作流内生成明确 detector、按条件运行的独立单元 job、始终运行的汇总。分支保护只要求汇总。选择来自完整 Git 事件 DELTA，不用工作流 paths 过滤器或 API 文件列表。

真实原生检查保持固定短入口 `check`、`check --unit ID`、`check --collect`。单元 provider 必须明确登记 `job_gating`，生成一个父工作流。登记、原件收集、重跑、迁移及完整调度边界见 [CI 单元](ci-units.md)。

<a id="standalone-initial-host-transition"></a>

## 迁移独立初始宿主

CI 所有者已有 `migrate`，可把明确保留的当前独立 v4 清单 provider 迁到同址或新址的独立 v4。保留旧来源，用 `--previous-config` 声明原地址，再以新 provider 调用登记迁移入口。

写入前核旧工作流和生成清单，保留精确清单声明及输出；拒绝改动／删除清单，或迁入单元汇总。宿主配置、facts／平台政策、FILEMAP、profile 绑定都须明确编辑，迁移不推断选择。

之后 `generate` 幂等，`verify` 只检测漂移不修复。这是候选源码行为；公开安装、原生首次提交事件和兼容发行锁仍须各自交付证据。
