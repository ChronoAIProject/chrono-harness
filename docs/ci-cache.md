# 登记式 CI 缓存

`chrono-cache` 是独立 Rust 产品，拥有专属 `cache-tests` 项目、CI 单元和中央发行资产，不与 CI 生成器合建。

`chrono-cache plan --host-root H --config .chrono-harness/cache.json --consumer ID` 为一个明确登记的消费者准备键。它是 provider 内部入口；日常检查仍使用 `chrono-harness check`。

本机固定 Verus 归档按现役 checksum 复用，命中时不再次下载；失配时重新下载并核验后替换。
runner 的编译缓存和原证明也沿同一个登记构建入口复用，损坏时只恢复一次，
成功调用不累积临时报告目录，具体合同见 [runner 的证明库与缓存](verus.md#runner-构建中的证明库与缓存)。
这两项本机复用不代表下文全部 CI 缓存或其他使用方已经验收。

<a id="bounded-native-acceptance"></a>

## 真实 CI 已验证的范围

本宿主的 `cache` 单元只消费 `check.cache-tests`；测试项目直接编译生产 CLI，不再恢复或保存生产 target。此前两项 `producer-manifest/v1` 缓存的有限原生观察覆盖冷启动保存、健康精确命中、兼容源码变化后的重建，以及不兼容环境分域／重建。原候选、输入、恢复键、原件和结果直接查[冷启动运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39688684172)、[源码变化及精确命中运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39690377913)和[环境分域运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39695762671)，不在仓库维护运行表。

这些观察只覆盖原候选和登记消费者，不能证明当前候选、其他消费者或完整缓存行为正确。不可用内容恢复、持久修复及修复后在原兼容范围健康精确命中的验收仍未完成；访问失败及部分观察见[退役说明](#retired-remote-acceptance-controls)。实验中的临时优化设置已移除，当前参数以现役登记为准；历史摘要、后端条目、artifact ID 和耗时归原运行。并发、linked 所有权、完整编译器／SDK／输入范围，以及传输、存储和总生命周期成本仍待验收或测量。

planner 准备身份；GitHub provider 在原 bootstrap 和检查／发布命令两侧生成固定版本的原生 restore／save action。本宿主在检查、发布登记中明确采用。计划或工作流生成成功，不代表真实恢复／保存、编译器复用或验收通过。SPEC §4.2 仍要求后端证据、冷／热／源码变化／失败／并发验收和当前执行物验证。

`worktree` 和 `worktree-adoption` 两组分别为自己的测试编译缓存采用内容核验和失败后封存。兼容输入各自绑定现役 `execute_core`／`execute_adoption` 的完整 Cargo 动作，并沿用工具选择、源码、工具链和环境输入；一组的过滤动作变化不会静默复用该组原来的兼容键。检查失败后仍保留真实失败，缓存只保存编译资料，后续命中仍执行原构建与测试。内容清单会读取登记 target 的文件；原生失败后的实际保存／健康命中及新增 IO、传输和存储成本仍须验收或测量，不能从登记采用推定完成。

每份计划记录准备它的 `chrono-cache` 精确摘要和版本；恢复与封存在执行前核对当前执行物。当前传输要求主 Git 检出；linked worktree 需要贯穿恢复／保存的租约，尚不支持。

本宿主标准检查 action 明确设 `CARGO_INCREMENTAL=1`。core、full、cache seed bootstrap 和 v5 发布配方设 `rust_incremental: true`，原构建调用收到对应环境值，包含 release 构建和 Rust 验证单元。发布配方通过明确 `rust_toolchain` 把设置限制在 Rust 消费者；非 Rust 单元保留自身环境。

bootstrap／发布回执记录传给子进程的值。发布导入／汇总与登记选择比较，并纳入生产者工具链指纹。省略设置保留原环境行为，格式错误在构建工具启动前失败。这些观察不证明缓存传输、实际编译器复用、构建输入完整或原生验收。Rust profile、标志、编译器和 SDK 身份仍须纳入持久缓存兼容合同。

## 宿主须明确登记什么

本宿主采用 `chrono-cache/v2`，登记归自身 `.chrono-harness/`。内容包括：

- namespace 和 `artifact_registry` 路径。宿主缓存产物精确引用该登记的 `artifacts` 路径，所有者须相同、`tracked: false`，且不得为执行证据。所有缓存产物使用相对宿主的字面路径；依赖下载缓存也须登记在宿主内。不发现目录或语言。
- 具名输入。各种输入的具体规则见下表。
- 具名产物，只声明 `owner` 和宿主内的 `path`。
- 具名缓存，声明 `owner`、登记 `producer` 操作 ID、`consumers`、产物 ID、互不相交的 `compatibility_inputs`／`source_inputs`、`restore`（`exact` 或 `compatible`）。每个列表均非空且成员唯一。
- `consumer_operations`，将每个消费者绑定到已有方法登记。每个引用用字面宿主 `registry` 路径和 JSON `pointer` 选择一个操作 ID，或非空且唯一的 ID 数组。引用缺失、值错误、订阅缓存的 producer 不在消费者所选操作内，都会在输入探测和原生恢复前失败。计划保留来源摘要和所选 ID，不从语言、路径或运行活动猜方法；声明有效也不证明实际执行。

本宿主的产物归属和 `cache.cargo-registration` 直接读取现役 `macos-27.json` 的 `artifacts` 与 `/tools/0`，发布暂存也复用其中的产物归属，不另维护内容相同的政策副本。这些字段是宿主声明，不是运行主机的观察；Linux 使用方读取它们不会执行 macOS 工具。缓存兼容键包含登记路径，切换来源后使用新的兼容键；原缓存不会因字段相同而继续命中，完整迁移成本未测。

| 输入种类 | 读取与绑定规则 |
| --- | --- |
| `file` | 字面路径和预期 `presence`（`present`／`absent`）；存在文件从真实字节计算摘要 |
| `environment` | 变量名及预期存在状态；只保留摘要和字节数，不保留值 |
| `literal` | JSON 声明；不能当作实际工具链或平台观察 |
| `json-value` | 字面 `path` 和明确 RFC 6901 `pointer`，选择存在 JSON 文档中的值，含明确 null。键不受对象顺序、格式和其他字段影响；空 pointer 选整份文档。无效 JSON／pointer 或值缺失会失败，不猜选择。另在键外 `source_file` 保留整文件摘要／长度作为来源，路径及输入／输出排斥规则与 file 相同 |
| `command` | 明确登记有限 runner 命令、继承变量名和 `result: stdout` 或 `file-path`。成功时绑定执行物字节、实际 argv、环境摘要及 stdout／stderr；后者另对输出的唯一绝对 UTF-8 文件路径计算文件摘要。探测失败先保留原进程报告再非零返回。原报告归宿主私有执行证据，可能含继承环境值；键使用观察结果，不用证据路径 |
| `registered-files` | 点名 FILEMAP v2 `registry`、`node` 和明确 `edges` 种类，对登记的传递源码范围及所选边算摘要；不扫描 import、目录、语言或未声明文件。FILEMAP 有效性仍由已有判官负责 |

消费者只选择明确列出自己的缓存，消费者缺失报错，不全选。只读取所选输入范围；仅属其他消费者的不可用输入不影响本计划。检查登记形状和输入／产物引用时不探测无关输入，也不读未选消费者的操作来源。来源登记自身不能缓存。

检查 job 引用 bootstrap 配方和 FILEMAP 计划，发布 job 引用发布配方的构建单元 ID 或验证操作。真实执行和生产者完成证据仍须分别取得。

兼容摘要绑定 namespace、缓存身份、owner、producer、恢复政策、产物声明及其所选登记记录，以及兼容输入观察。源码摘要绑定源码输入观察。键为 `chrono-v2-<compatibility digest>-<source digest>`。消费者路由另核；增加消费者不使未变的生产输出失效，但消费者特有构建输入仍须归入所选缓存的兼容或源码输入。

输入 ID 排序变化不改摘要；无关缓存声明或文件变化不使所选缓存失效。

**产品与宿主政策同改：**产物均为宿主内路径，外部产物分支及重复的 `kind`／`external` 字段已移除；当前登记、恢复和发布测试一并迁移。产物路径复用 runner 已有的 Verus 相对路径判断，每项产物仍须精确匹配未跟踪、非执行证据的宿主登记。字段删除会改变兼容摘要，旧缓存键失效并重新构建；完整构建、验证、CI、存储和维护成本收益未知。

只接受 `chrono-cache/v2` 登记和 `chrono-cache-plan/v2` 计划；旧格式在准备计划前拒绝，不生成或恢复 `chrono-v1-` 键。CI 投影也在写入前拒绝旧登记。格式准入复用现有同源 Verus 证明入口；JSON 解码、完整登记与恢复行为仍有各自的验证边界。

`compatible` 只提供一个到兼容摘要为止的恢复前缀，使源码变化可找到同范围旧编译数据。适用的编译器、target、profile、标志、SDK、构建脚本、配置、环境身份都须由宿主明确登记到兼容范围；planner 不推断这些依赖，也不证明列表完整。lockfile、manifest 也须按真实兼容作用归类。

每项计划仍要求登记构建生产者，绝不授权直接执行恢复的历史判官。当前执行物核验也检查缓存 planner 执行物身份，但不能证明恢复的项目执行物已重建。所选判断／测试仍由消费流程执行。

字面路径拒绝穿越、glob、符号链接组件。所选输出不能互相重叠、包含 Git metadata／harness 执行状态或任何声明文件输入。这只是有限路径／登记检查，不证明任意缓存内容安全或重建输入齐全。恢复和保存还须持有已采用生命周期保护，直到生产者全部等待结束且保存完成。

本宿主将当前登记的 32 个编译缓存目录选为 `finish-dispose`，由原生命周期所有者保留跨操作复用。
普通 bootstrap 返回后仍可供 check 使用，check 返回后仍可供 seal／save 使用；
明确 finish 或实际检出处置时才回收；只要检出继续保留复用，普通 use／check／bootstrap 及显式维护都不因创建年龄清理编译缓存。下载缓存继续按自己的登记保留。

已安装工具的关联检出，其普通 bootstrap 使用本树的生命周期程序保护构建，和本树普通检查采用相同清理实现；不再为每次构建调用主检出的旧程序。首次本地安装仍由现有协调者保护，具体入口见 [CI 启动](ci.md#this-repository)。本地程序采用与共享协调程序升级分别验收；仍在使用旧程序的其他入口不因此取得新行为。
这同时修改产品处置判法和宿主政策，增加的保留空间及总生命周期成本仍未测量；
支持平台、真实缓存封存／保存和宿主采用须分别验证。

FILEMAP 明确连接生命周期工具／cleanup 政策与 cache-tests 使用方，
让清理时机变化继续选择真实 cache 单元。该单元沿现有入口运行原构建、测试、
producer manifest 封存及 dev push 保存，不另设验证工作流。

返回 `chrono-cache-plan/v2`，包含所选项及输入观察。状态为 `prepared-unrestored`，执行为 `not-started`，`input_completeness_proven` 为 false。这些只描述计划，不是判断通过、命中、构建成功或同判证据。后端结果、失败链和原报告须由真实生产者提供。

## 恢复失败后如何清理

### 兼容恢复后的源码时间

编译缓存可显式选择 `refresh_sources: true`，要求已采用主检出的恢复入口。
在兼容命中且内容核验未要求丢弃时，恢复程序从原计划的 `source_inputs` 取得明确文件，
重核当前字节，再比较所选缓存产物的修改时间。只把时间较旧的登记输入调到产物之后；
源码字节、依赖和增量目录继续保留。文件输入、JSON 文件及 FILEMAP 的明确源码集合可参与，
环境和命令观察不产生可修改路径。不从目录或语言发现源码。

Cargo 1.95 稳定版仍按修改时间判断构建新鲜度。源码已变、时间却早于恢复产物时，
原构建可能返回成功并留下旧程序；校验和新鲜度选项尚需 nightly。
该恢复方法让现有构建重新判断当前源码，不新增工具链或构建配方。
精确命中和未恢复不调整时间；恢复原件保留方法、路径、实际更新／失败及调用成本。
重复消费原结果不再调整后续生产者的时间。

本宿主为现有 32 个编译目录对应的缓存条目采用此方法。**产品与宿主政策同改：**
方法进入兼容键，旧域不再命中；首次需要重新构建。兼容恢复增加一次所选产物的元数据读取，
必要时更新登记输入的时间；完整 CI、存储及维护成本仍未测量。
真实 Cargo 回归覆盖较旧的已变源码、原构建输出和增量目录保留；完整原生采用另验。

v2 可为隔离主检出明确启用 `recover_failed_restores: true`。该选择保留在计划中，不进入构建缓存键。planner 将所选方法交给原生 provider；旧登记和旧 planner 不执行恢复。

provider 在原生恢复后、原 bootstrap／构建前，用已有计划、登记及原步骤观察调用 `chrono-cache recover`。所有者使用原声明观察重核所选产物／消费者合同，不重跑输入探测。登记、宿主、所选路径或计划绑定变化，都在删除前失败。

只有原生恢复 **outcome** 为 `failure` 的缓存，才删除其登记宿主产物。success／skipped 保留输出；缺失、cancelled、未知 outcome 报错。嵌套符号链接只删链接、不跟随目标；产物根或祖先是符号链接则拒绝。

删除前先写 intent。结果保留原恢复观察、每个尝试路径及失败；清理不完整或发布失败均非零，阻止原构建。恢复结果和步骤原字节直接保留在计划所在 state 目录；恢复证据缺失或损坏时，重新消费原结果会失败。

重复已完成尝试返回原结果，不删除后来生产者重建的输出。未完成 intent 保留并拒绝，旧失败不自动重试。本宿主在已有 job 采用，步骤耗时归 `cache-recovery`，不增加 job、构建配方或业务测试重试。此失败政策覆盖报告的原生恢复失败和部分解压；下文可选内容核验还覆盖所选缓存声称有效的命中。后端归档替换、linked worktree 租约及完整原生失败场景仍待验收。

### 未确认恢复的处理

本宿主另采用 `recover_unconfirmed_restores: true`。此 v2 选项要求 `recover_failed_restores` 及其主检出边界；省略则保留只处理失败的旧行为。

同一恢复入口还处理以下情况：恢复步骤成功，但 matched key 缺失、为空或不在声明的 exact／compatible 范围内，此时也丢弃登记输出。原生缓存实现可能压下下载／解压错误后返回成功。缺 key 仍记 `miss-or-unavailable`，清理不猜是哪种原因，也不编造恢复失败。恢复按原 matched key 分类。

exact／compatible 命中和 skipped 保留输出；缺失／cancelled outcome 或格式错误的输出，都在删除前失败。政策进入保留恢复计划，不进入编译缓存键。原 outcome、原因及逐产物结果直接保留在恢复原件；完成后重入保全后续构建。

普通冷未命中且所选产物原本都不存在时，不增加恢复警告；删除残留和明确失败仍可见。原构建和所选测试继续运行，失败仍是失败。此扩展本身不核验有效 matched key 的实际内容，不保护外部所有权，也不证明原生损坏实验已完成。

## 可选的生产者内容核验

本宿主为已有 `check.unit_cache` job 消费的 `check.cache-tests` 选择 `verify_contents: true`。此 v2 选项要求已有主检出失败／未确认恢复政策，不支持外部产物。其他消费者合同不变。方法名 `producer-manifest/v1` 纳入兼容身份，使旧未封存归档无法通过宽前缀进入这个范围。

这项兼容范围还从 `projects.json` 选择真实生产 action（`test.cache-tests`，含 tool／argv），并选择采用的直接 Git 政策中 Cargo 工具记录。已有输入绑定检查环境、Cargo／编译器观察、manifest、lock、bootstrap 和平台声明。无关项目 metadata 不在所选值内。这些声明仍不证明编译器／SDK 输入完整。

provider 在原操作结束后、保存前，沿用保存阶段调用 `chrono-cache seal`。它重核计划／登记和当前 planner 身份，默认要求原生产步骤 **outcome** 成功，并枚举明确选择的产物目录。

缓存可明确选择 `save_on_failure: true`，前提是采用内容核验。本仓仅为 `check.cache-tests` 编译目录采用：同一步中的测试失败后，仍可封存实际留下的编译文件。只接受原 outcome 的 success／failure；skipped、cancelled、缺失或未知结果都拒绝，不按可能被 `continue-on-error` 改写的 conclusion 判断。封存报告保留生产步骤、原 outcome 和原生步骤原字节；封存成功只说明文件已列清，不改变原构建／测试失败。保存政策进入计划及 provider 输出，不改变内容兼容键。恢复后仍须运行原登记构建和测试。

**产品与宿主采用同时变更：**新增明确保存选择、同源封存判断及回归，宿主只为上述一项现役编译缓存采用。原业务操作、断言、退出及恢复条件继续执行。新增条件及原件字段的成本有限，内容枚举、原生保存和完整失败恢复的总成本仍待实测；本地行为不证明远端后端已保存。

每个产物得到 `.chrono-cache-content-v1.json`，绑定缓存 ID、生产操作、实际源码键、产物声明和内容。清单记录普通文件 SHA-256／长度／执行位、目录（含空目录）、字面符号链接目标，不跟随链接；特殊文件和非 UTF-8 名称失败。只排除该方法自己的标记。标记包含输出身份，不含测试结果、judge 响应或缓存的通过状态。原生步骤、manifest 和封存观察保存在宿主执行证据中。

保存已采用项时，除原生产／保存条件外，还须封存成功。不增加构建配方、job 或测试重试。

原构建消费声称 exact／compatible 命中的内容前，`recover` 比较生产者 manifest 与新内容清单。兼容恢复的 manifest 绑定**实际匹配**源码键，候选保留新源码身份。内容变化／缺失、manifest 缺失／错误、生产者绑定不兼容都记为不可用。

先保留 manifest 原字节、观察内容身份／错误和原生恢复观察，再按已有恢复合同丢弃所选登记输出。原构建／检查继续执行，重复已完成恢复保全之后重建的输出。报告保留原键分类，另以 `unusable-content` 区分未接纳的命中，并把引用的验证证据复制到已有上传目录。

摘要计算是新增成本。封存和恢复核验按所选缓存记录实际墙钟秒数，每份观察说明测量窗口，原生步骤归 `cache-verification`。总传输／存储／runner 成本未知，有限原生证据见前文。

本地 Rust 回归在小 fixture 中，从未改登记 action 读取真实 Cargo 调用：兼容源码变化产生候选二进制并保留增量数据，生产者设置变化则划分新兼容范围。它是编译器证据，不是原生归档验收。内容 manifest 是可信但可能出错的生产者提供的一致性证据，不是归档认证或构建输入完整证明。上文两项缓存已有冷／保存、精确、兼容源码变化及不兼容输入证据；不可用内容恢复的完整验收仍未完成，部分观察见末节。

## provider 的真实消费者

原生缓存登记必须明确 `save_on_push_only`。为 true 时，编译产物仍在 PR 中恢复，
但只在 push 事件按登记的生产结果条件保存；为 false 时，保存仍使用原事件范围。
只为保存服务的内容封存使用相同事件条件，恢复时的内容核验继续执行。
下载缓存不受这个字段限制，PR 可以保存自己的下载资料。
本仓普通检查选择 true，避免每个 PR 都上传一套大编译目录，挤占下载缓存；
发布工作流选择 false，因为其现役生产事件另有登记。
恢复、内容核验、原构建和检查继续执行。此选择不保证后端永久保留归档，
也不证明总编译、传输或存储成本已降低。

### 首次编译前的下载缓存

`persistent_cache.downloads` 使用已固定的原生 cache action，在 checkout 后、任何启动命令前恢复下载资料。
这里不能先运行 `chrono-cache`：编译它本身就需要这些依赖和工具链。
登记直接列出所有者、精确产物路径、兼容文件、源码文件、使用 job、唯一保存 job 和可选基础缓存。
生成器重核现役产物登记和文件存在性；路径只接受字面文件名，不接受 glob 或表达式。
原生 `hashFiles` 对明确文件列表求键，平台、架构及兼容文件限定恢复范围。
基础缓存必须使用相同路径和兼容文件；缺登记或不相容时，在写投影前失败。

本仓把初始 launcher、受保护 bootstrap 和业务 Cargo 调用统一到工作树自己的
`.chrono-harness/cache/bootstrap-owner-tooling/cargo`。缓存只运输包归档、索引、源码及 Git 依赖资料，
不运输 Cargo 配置、凭据或本轮结果。各 job 保存自己的依赖集合；首次未命中时可恢复 detector 的基础依赖。
固定 Rust 工具链由 detector 保存，各 job 在首次调用 Rustup 前恢复它。
CI 使用工作树内的 `RUSTUP_HOME`；本地继续使用调用方选择的 Rustup 安装。

命中后仍执行原构建和检查入口，由 Cargo 按锁文件及 checksum 校验依赖，bootstrap 继续核工具绑定。
保存发生在原 job 的工作返回后。下载恢复步骤成功且未精确命中时，即使后续构建或测试失败，也保存已取得的下载资料；取消、恢复失败或未执行恢复时不保存。
保存只运输可重建下载资料，不表示构建完成或检查通过；后续步骤的失败仍保留其真实结果。恢复后继续由原包管理器校验和补齐内容。
原生步骤保留命中键、下载与保存观察。
缓存动作的成功不能替代原构建结果，也不能证明后端归档仍存在。
下载缓存的跨运行命中、冷／热下载次数及总时间收益须以当前原生运行验证，生成或本地测试通过不表示已验收。

**产品与宿主政策同改：**下载恢复的登记、投影及 Verus 边界判断同时加入；
生命周期清理由笼统的 cache 父目录改为当前生产者的精确目录，避免父子选择重叠。
Cargo、本地专属 Rustup 下载目录及固定 Verus 包保留，临时 home、tmp、编译目录和发布中间目录继续回收。
Cargo 下载内容由其已有自动清理管理；CI 的工具链目录随隔离 runner 结束，后端按 GitHub 缓存保留政策清理。
固定工具链变更使兼容键失效；独立维护旧本地工具链时仍须核实际使用方，不能按目录年龄直接删除。
恢复和保存的传输、存储及完整生命周期成本尚未测量。

检查／发布 provider 可通过 `persistent_cache` 采用 `chrono-github-cache/v1`，包含登记 planner `program`、登记 `config`、固定 `restore_action`／`save_action` 和明确 `jobs` 映射。每个已有 job 点名 `need`、`output_use`、字面 `prepare` argv、`consumer` ID 及精确 `caches` 集合。前两个用途字段解释该 job 为何需要输出，不是实测使用证据。

未知 job、缺失／空／多余／不匹配的缓存引用都在写输出前失败。本宿主缓存登记明确 `require_primary_checkout: true`；planner 在恢复前拒绝 linked worktree。这个文件系统保护不证明一般 Git 所有权或生命周期排除。

### 本仓的缓存归属

本仓 detector 构建共享启动 profile，选择登记的编译缓存。检查单元和汇总安装绑定源码的 startup 产物，当前单元与缓存集合分别归 units.json／cache.json。单元缓存登记只选择各 FILEMAP 计划仍实际存在的 Cargo 生产者；汇总和没有编译操作的单元不订阅编译缓存。

发布缓存按登记选择项目输出，验证与包收集操作由现役 Rust 发布配方执行。分组测试中不同生产者使用不同条目。

缓存种子运行 `host.bootstrap-cache`，通过已有 bootstrap 所有者和 `build.cache` 构建候选 planner，再安装二进制。其配方明确 `report_path: .chrono-harness/state/cache-bootstrap.json`；之后 core bootstrap 保留默认 `.chrono-harness/state/bootstrap.json`。

宿主 bootstrap 接受可选的 `.chrono-harness/state/` 下字面报告文件，拒绝越界或符号链接位置，并传递报告发布错误。各配方拥有自己的报告位置；重复用同一位置会替换最新结果。已有产物上传保留两份独立记录。

计划保留 planner 的执行路径、实测文件摘要及版本。首次恢复按保留输入重核登记、宿主和所选观察，沿用原 planner 身份；封存另核当前执行物与原 planner 一致。独立 bootstrap 记录安装二进制摘要及源码身份。文件摘要不证明编译器输入完整或内存加载身份；bootstrap 证据缺失或源码不干净，不能证明绑定到干净源码。

detector 缓存绑定自身 bootstrap 配方，没有业务检查政策输入。检查编译缓存从检查配置选择 `/policy/environment`，并复用 `cache.cargo-registration` 读取现役完整登记的 Cargo 声明；单元成员、报告位置和已移除的 profile 工具副本不改这些键，环境或真实 Cargo 声明变化会改键。兼容键来源随之变化，旧缓存不会继续命中；完整迁移成本未测。

本仓编译缓存的生产者使用专属 `CARGO_HOME`：`.chrono-harness/cache/bootstrap-owner-tooling/cargo`。bootstrap 配方、检查政策和发布源中的缓存生产命令明确选择这个仓库相对目录；发布命令用已有 `/usr/bin/env` 转交变量，不新增调度入口。Cargo 子进程的工作目录是实际检出根，独立检出各自拥有该缓存。

缓存的 Cargo／rustc 探测同样明确选择该目录，不再继承另一份 `CARGO_HOME`。所有相关编译缓存复用 `bootstrap.cargo.home-config`／`bootstrap.cargo.home-config-toml` 两项缺失声明；存在任一配置时规划失败，避免恢复与实际构建配置不符的产物。已删除 `/Users/runner`、`/home/runner` 的四条主机目录声明，不展开 HOME，也不另存主机观察。这些选择不证明祖先 Cargo 配置及其余构建输入完整；实际环境、失败和原件继续归本轮 state／线上 CI。

专属 Rust 宿主采用回归覆盖这些区别，包括 detector 没有业务检查政策，以及 detector 构建参数变化。它们只验证这些声明依赖，不证明构建输入完整或所有登记都有必要。

<a id="verus-archive"></a>

### Verus 固定下载包

`verus.archive` 缓存存放 checksum 文件点名 ZIP 的专属目录，不缓存证明结果。
缓存与宿主产物清单均精确登记 `.chrono-harness/cache/bootstrap-owner-tooling/verus/`，
不把文件登记到只接受目录的宿主清单，也不以父目录声明替代精确匹配。
键绑定 projects.json 中的真实下载动作及 `verus-macos-arm64.sha256` 原字节，
采用精确恢复；编译器和无关源码变化不使这个固定包失效。
detector 在启动前恢复、启动成功后保存。检查单元通过已有共享产物取得原包，
只解包，不另恢复或保存这项缓存。
冷启动仍下载；本地有效包和恢复包均由[同一下载动作](verus.md)重新计算 SHA-256，
匹配后才复用。损坏包重新下载，失败不冒充命中，也不跳过证明、测试或后续解包。
该 Rust 动作也在新包发布前核对完整 SHA-256；启动配方随后直接解包，移除重复重读同一包的 shasum 操作及其工具登记。
既有下载测试继续覆盖缓存命中、坏包恢复、坏下载拒绝和失败保全；这项删减不提供并发写入隔离，完整耗时收益未测。

**启动实现与宿主政策同改：**新增归档准备判断、Rust 行为测试、独立 Verus 边界预期，
并登记实际产物、使用方和原生 restore／save 投影。每次命中增加一次包摘要计算，
省去一次下载；完整传输、存储及总时间收益尚未测量。
本地包只占用该目录的 `package.zip` 一个槽位。版本和 SHA-256 仍由当前下载动作及 checksum 文件决定，
升级后校验通过的新包替换同一路径；下载失败保留原包，不按版本名累积归档。
当前包供实际证明和共享启动使用，宿主保留这个槽位。下载临时文件使用现有 `TMPDIR`
选择，放在已登记 dispose 的 `bootstrap-owner-tooling/tmp`；正常返回由 tempfile 回收，
强制中断后的残留由原工作树生命周期入口回收，不进入归档缓存。
目录不存在时，下次仍按冷启动处理。
真实启动消费测试把父启动已准备的固定包复制到自己受保护的 fixture，
再执行原启动及内容校验；独立源码构建、证明和测试照常运行。

### 恢复、保存与写入者

provider 从 `chrono-cache plan --github-output` 取得输出键，保留计划和探测原输出，在原操作前恢复。默认业务成功后保存全部所选缓存。可选 `save_caches` 限定本 job 可保存项，空列表表示只恢复；未知或重复项拒绝。

同一 provider 配置内，每个缓存 ID 至多一个保存 job。生成／验证用 `E_CACHE_SAVE_OWNERSHIP` 拒绝冲突，指出缓存和两个 job。其他消费者明确只恢复；流程也可刻意没有写入者。独立登记生产范围使用不同缓存身份，包含不同发布平台。

本宿主把四项共享启动缓存交给已有 detector 保存，其他缓存交给登记生产单元。消费者仍恢复、重建、检查，不增加 job 或前置等待。写入者跳过或未完成生产时，该运行不提供新归档。

cache 专属测试也在自己的 target 编译生产 CLI，自己的计划不再执行 `build.cache`，`check.cache` 及其独用输入 `cache.producer` 随无使用方的恢复／保存路径退役。测试 target 仍走原内容核验、源码刷新和原未过滤检查；native 发布测试、bootstrap 和发行打包继续使用生产 cache 构建，完整成本收益未测。

runner 专属测试直接在自己的 target 编译生产 CLI，不再恢复或保存 `check.runner`。仍执行 `build.runner` 的现有 CI 测试单元接手这项缓存的唯一保存权；其他实际使用方继续恢复它，测试 target 的缓存和原检查操作保留。完整缓存与构建成本收益未测。

明确的 `save_after_bootstrap` 子集可在原操作结束后，改用独立 bootstrap 步骤成功作为保存条件，使后续测试失败时仍保留可复用启动编译。未知、重复或不在可保存集合的子集项会让生成失败；取消时不保存。该子集是宿主声明，不证明后续每次构建成功。恢复的编译候选仍须经过原构建入口。

planner／provider 按字典序排列消费者精确缓存集，用 `cache_0_key`、`cache_0_paths`、`cache_0_restore`，再用 `cache_1_*` 等作为 job 内输出名。这些短引用不改缓存身份或键。两者须实现同一输出合同，登记验证在生成前拒绝集合不符。

### 原件直接上传

缓存不跳过标准检查，不用缓存状态替代构建／测试结果。计划、探测、恢复、内容核验和封存由各自生产者在 `.chrono-harness/state/` 保留原件及摘要引用；所选原生 action 日志保留恢复／保存的实际输出。保存 action 成功不能证明归档已保存、编译器复用或当前执行物验证。

封存要求计划中的 planner SHA-256 和版本完整，且与当前执行物一致；缺失或不匹配在封存前失败。恢复核对原计划及当前登记，不比较当前执行物与原 planner 身份。普通检查复用已有失败诊断上传，detector 的 `job_gating.detector.evidence_directory` 继续通过已有固定上传 action 保留声明原件。本宿主选择 `.chrono-harness/state/`，包括 bootstrap、计划和探测原件；目录、隐藏文件和缺失原件规则保持。

采用缓存的发布单元在失败时用配方已有的固定上传 action 单独上传 `.chrono-harness/state/`，包含隐藏文件。诊断 artifact 与正式发布 artifact 分开命名；正式上传目录、归档根和 collector 下载合同保持。上传失败仍使对应步骤失败，原业务结果独立保留。原件随线上 artifact 的保留期清理，本地按既有生命周期处理，不入 Git。

`chrono-cache report`、`transport_report` 和缓存消费者的 `evidence_directory` 已退役。它们不参与构建、恢复、内容核验、封存或发布判断；失败原件改由上述入口直接上传，无需复制步骤观察、拼装传输报告或把原件搬进发布目录。仍有使用方的原件保留方法归缓存所有者维护。

明确坏缓存恢复，以及后续测试失败后保留任意项目编译，仍未完成。保存内容不包括 `.chrono-harness/state`、发布交接、所选测试结果、已安装 `bin`、Git metadata 或生命周期锁。

### 后端状态按需查询

缓存计划、恢复、内容核验、封存和发布汇总都不消费后端清单。保存后的持续查询只把 GitHub 已提供的状态另存一份，不能证明归档内容、编译器复用或创建来源，因此已删除 `backend` 配置、查询实现、凭据注入和专属测试。缓存本身不再要求 `actions: read`；发布工作流只保留 `contents: read`，单元汇总及共享启动按其实际 API 消费保留已有权限。

`chrono-cache` 不调用 GitHub 缓存 API；原生保存成功不能确认后端可用性。排查特定运行时直接读取其 action 日志及线上 API。失败诊断原件由现役上传入口直接保留，不再另造缓存传输报告。

### 输入和共享启动边界

宿主登记绑定真实编译器／Cargo 文件及版本、OS／架构、登记链接器／SDK 观察、声明标志缺失、manifest／lock、构建配置。源码变化使用 FILEMAP 明确 compile 依赖范围。检查、发布构建、发布验证和平台声明各用独立兼容范围。这份有限登记不证明编译器后端、SDK 库、构建脚本或环境配置输入完整，当前原生采用及缺失证据仍在覆盖表列明。

明确采用共享启动后，detector 一次构建核心工具，发布给登记消费者，见[共享启动传递](ci-units.md)。只有实际剩余的构建／测试生产者订阅 target 缓存；汇总和纯脚本消费者不恢复启动编译 target。

detector 独自保存 `detect.*` 启动缓存，真实单元生产者保存 `check.*`。detector 兼容输入不含自身 bootstrap 不消费的检查政策。导入 startup 验证不是编译生产者，不能取得 `save_after_bootstrap` 资格。各消费者证据保留原生产报告和明确导入来源；startup 二进制不进入证据上传。传输成本单独测量，绑定源码的产物不证明构建输入来源完整。

发布验证在 Cargo test destination 消费 `chrono-cache` 资产；bootstrap 安装的传输执行物仍归 bootstrap。发布配方不再把资产放到同一 bin 路径。Rust 发布集成回归覆盖真实 CLI 的 plan／recover／staging，以及 inode、摘要保留；本地证据不证明原生缓存后端或外层工作流成功。详见[发布说明](release-ci.md)。

<a id="retired-remote-acceptance-controls"></a>

## 已退役的远端验收控制

**产品与宿主政策曾同步删除：**可选原生 manifest 删除 fixture、对应 CLI／配置和 provider 登记、仓库 Variables 激活及实时 PR 正文读取器。provider 不再为激活增加 `pull-requests: read` 或读 PR metadata。不需要替代激活传输、变量管理、远端缓存删除或额外权限。退役字段按普通严格解码失败，不会静默当作激活成功。

已有 `check.unit_cache` 计划、恢复 `check.cache-tests` 编译归档，核 `producer-manifest/v1` 内容，通过普通恢复丢弃不可用宿主输出，执行原候选构建／测试，再按上述明确保存选择封存实际编译输出并经原生 action 保存。保存仍受必需生产者／封存 outcome 限制。原生 action 日志及恢复原件保留实际结果和原失败，其他构建／发布缓存行为不变。

删除控制层不代表缓存验收完成。修复后在原兼容范围健康精确命中仍未完整验收；当前缺口见[覆盖表](spec-coverage.md)。历史过程从 Git、原 PR／CI 查询，不在当前说明维护运行状态或旧测试入口；原记录、输出和必要证据继续按其保管者的保留政策处理。

普通执行不再需要激活进程／API 请求。删除该层移除读取器、fixture、响应／intent 证据 IO 及专用测试；原生耗时、传输、存储和总生命周期收益未测。现有同语言内容／恢复、无效输入、provider 回归，以及按登记变更范围选择的标准检查仍为必需。原生验收、并发、其他消费者采用、完整输入、同判和实际交付仍分别未完成。
