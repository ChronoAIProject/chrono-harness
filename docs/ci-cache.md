# 登记式 CI 缓存

`chrono-cache` 是独立 Rust 产品，拥有专属 `cache-tests` 项目、CI 单元和中央发行资产，不与 CI 生成器合建。

`chrono-cache plan --host-root H --config .chrono-harness/cache.json --consumer ID` 为一个明确登记的消费者准备键。它是 provider 内部入口；日常检查仍使用 `chrono-harness check`。

<a id="bounded-native-acceptance"></a>

## 真实 CI 已验证的范围

本宿主的 `cache` 单元消费 `check.cache`、`check.cache-tests`。两项 `producer-manifest/v1` 缓存已有有限原生观察，覆盖冷启动保存、健康精确命中、兼容源码变化后的重建，以及不兼容环境分域／重建。原候选、输入、恢复键、原件和结果直接查[冷启动运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39688684172)、[源码变化及精确命中运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39690377913)和[环境分域运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39695762671)，不在仓库维护运行表。

这些观察只覆盖原候选和登记消费者，不能证明当前候选、其他消费者或完整缓存行为正确。不可用内容恢复、持久修复及修复后在原兼容范围健康精确命中的验收仍未完成；访问失败及部分观察见[退役说明](#retired-remote-acceptance-controls)。实验中的临时优化设置已移除，当前参数以现役登记为准；历史摘要、后端条目、artifact ID 和耗时归原运行。并发、linked 所有权、完整编译器／SDK／输入范围，以及传输、存储和总生命周期成本仍待验收或测量。

planner 准备身份；GitHub provider 在原 bootstrap 和检查／发布命令两侧生成固定版本的原生 restore／save action。本宿主在检查、发布登记中明确采用。计划或工作流生成成功，不代表真实恢复／保存、编译器复用或验收通过。SPEC §4.2 仍要求后端证据、冷／热／源码变化／失败／并发验收和当前执行物验证。

每份计划记录准备它的 `chrono-cache` 精确摘要和版本。报告前与当前执行物比较，不符即拒绝发布传输证据。当前传输要求主 Git 检出；linked worktree 需要贯穿恢复／保存的租约，尚不支持。

本宿主标准检查 action 明确设 `CARGO_INCREMENTAL=1`。core、full、cache seed bootstrap 和 v5 发布配方设 `rust_incremental: true`，原构建调用收到对应环境值，包含 release 构建和 Rust 验证单元。发布配方通过明确 `rust_toolchain` 把设置限制在 Rust 消费者；非 Rust 单元保留自身环境。

bootstrap／发布回执记录传给子进程的值。发布导入／汇总与登记选择比较，并纳入生产者工具链指纹。省略设置保留原环境行为，格式错误在构建工具启动前失败。这些观察不证明缓存传输、实际编译器复用、构建输入完整或原生验收。Rust profile、标志、编译器和 SDK 身份仍须纳入持久缓存兼容合同。

## 宿主须明确登记什么

本宿主采用 `chrono-cache/v2`，登记归自身 `.chrono-harness/`。内容包括：

- namespace 和 `artifact_registry` 路径。宿主缓存产物精确引用该登记的 `artifacts` 路径，所有者须相同、`tracked: false`，且不得为执行证据。明确的外部依赖缓存可用绝对路径，其他产物相对宿主。不发现目录或语言。
- 具名输入。各种输入的具体规则见下表。
- 具名产物，声明 `owner`、`path`、`kind`（`dependencies`、`compilation`）和明确 `external` 布尔值。
- 具名缓存，声明 `owner`、登记 `producer` 操作 ID、`consumers`、产物 ID、互不相交的 `compatibility_inputs`／`source_inputs`、`restore`（`exact` 或 `compatible`）。每个列表均非空且成员唯一。
- `consumer_operations`，将每个消费者绑定到已有方法登记。每个引用用字面宿主 `registry` 路径和 JSON `pointer` 选择一个操作 ID，或非空且唯一的 ID 数组。引用缺失、值错误、订阅缓存的 producer 不在消费者所选操作内，都会在输入探测和原生恢复前失败。计划保留来源摘要和所选 ID，不从语言、路径或运行活动猜方法；声明有效也不证明实际执行。

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

只接受 `chrono-cache/v2` 登记和 `chrono-cache-plan/v2` 计划；旧格式在发布计划或报告前拒绝，不生成或恢复 `chrono-v1-` 键。CI 投影也在写入前拒绝旧登记。格式准入复用现有同源 Verus 证明入口；JSON 解码、完整登记与恢复行为仍有各自的验证边界。

`compatible` 只提供一个到兼容摘要为止的恢复前缀，使源码变化可找到同范围旧编译数据。适用的编译器、target、profile、标志、SDK、构建脚本、配置、环境身份都须由宿主明确登记到兼容范围；planner 不推断这些依赖，也不证明列表完整。lockfile、manifest 也须按真实兼容作用归类。

每项计划仍要求登记构建生产者，绝不授权直接执行恢复的历史判官。当前执行物核验也检查缓存 planner／report 执行物身份，但不能证明恢复的项目执行物已重建。所选判断／测试仍由消费流程执行。

字面路径拒绝穿越、glob、符号链接组件。所选输出不能互相重叠、包含 Git metadata／harness 执行状态或任何声明文件输入。这只是有限路径／登记检查，不证明任意缓存内容安全或重建输入齐全。恢复和保存还须持有已采用生命周期保护，直到生产者全部等待结束且保存完成。

返回 `chrono-cache-plan/v2`，包含所选项及输入观察。状态为 `prepared-unrestored`，执行为 `not-started`，`input_completeness_proven` 为 false。这些只描述计划，不是判断通过、命中、构建成功或同判证据。后端结果、失败链和原报告须由真实生产者提供。

## 恢复失败后如何清理

v2 可为隔离主检出明确启用 `recover_failed_restores: true`。外部产物所有权尚未建立，不支持。该选择保留在计划中，不进入构建缓存键。planner 将所选方法交给原生 provider；旧登记和旧 planner 不执行恢复。

最终传输报告把每个计划恢复步骤与恢复程序消费的原观察核对。可增加后续 bootstrap、构建、测试、保存步骤，但已有恢复观察不能消失或改变。不符时在上传目录保留两份快照，返回带缓存 ID 和两个原路径的 `E_CACHE_REPORT`，不发布正常传输报告，也不查后端。这只核保留观察一致，不证明原生 action 声明真实。

provider 在原生恢复后、原 bootstrap／构建前，用已有计划、登记及原步骤观察调用 `chrono-cache recover`。所有者使用原声明观察重核所选产物／消费者合同，不重跑输入探测。登记、宿主、所选路径或计划绑定变化，都在删除前失败。

只有原生恢复 **outcome** 为 `failure` 的缓存，才删除其登记宿主产物。success／skipped 保留输出；缺失、cancelled、未知 outcome 报错。嵌套符号链接只删链接、不跟随目标；产物根或祖先是符号链接则拒绝。

删除前先写 intent。结果保留原恢复观察、每个尝试路径及失败；清理不完整或发布失败均非零，阻止原构建。最终传输报告与原恢复／业务 outcome 一起保留恢复结果及步骤原字节。恢复证据缺失或损坏，报告失败。

重复已完成尝试返回原结果，不删除后来生产者重建的输出。未完成 intent 保留并拒绝，旧失败不自动重试。本宿主在已有 job 采用，步骤耗时归 `cache-recovery`，不增加 job、构建配方或业务测试重试。此失败政策覆盖报告的原生恢复失败和部分解压；下文可选内容核验还覆盖所选缓存声称有效的命中。后端归档替换、linked worktree 租约及完整原生失败场景仍待验收。

### 未确认恢复的处理

本宿主另采用 `recover_unconfirmed_restores: true`。此 v2 选项要求 `recover_failed_restores` 及其主检出边界；省略则保留只处理失败的旧行为。

同一恢复入口还处理以下情况：恢复步骤成功，但 matched key 缺失、为空或不在声明的 exact／compatible 范围内，此时也丢弃登记输出。原生缓存实现可能压下下载／解压错误后返回成功。缺 key 仍记 `miss-or-unavailable`，清理不猜是哪种原因，也不编造恢复失败。恢复与传输共用 key 分类。

exact／compatible 命中和 skipped 保留输出；缺失／cancelled outcome 或格式错误的输出，都在删除前失败。政策进入保留恢复计划，不进入编译缓存键。原 outcome、原因及逐产物结果继续提供给传输报告；完成后重入保全后续构建。

普通冷未命中且所选产物原本都不存在时，不增加恢复警告；删除残留和明确失败仍可见。原构建和所选测试继续运行，失败仍是失败。此扩展本身不核验有效 matched key 的实际内容，不保护外部所有权，也不证明原生损坏实验已完成。

## 可选的生产者内容核验

本宿主仅为已有 `check.unit_cache` job 消费的 `check.cache`、`check.cache-tests` 选择 `verify_contents: true`。此 v2 选项要求已有主检出失败／未确认恢复政策，不支持外部产物。其他消费者合同不变。方法名 `producer-manifest/v1` 纳入兼容身份，使旧未封存归档无法通过宽前缀进入这个范围。

两项兼容范围还从 `projects.json` 选择真实生产 action（`build.cache` 或 `test.cache-tests`，含 tool／argv），并选择采用的直接 Git 政策中 Cargo 工具记录。已有输入绑定检查环境、Cargo／编译器观察、manifest、lock、bootstrap 和平台声明。无关项目 metadata 不在所选值内。这些声明仍不证明编译器／SDK 输入完整。

provider 在原操作结束后、保存前，沿用保存阶段调用 `chrono-cache seal`。它重核计划／登记和当前 planner 身份，要求原生产步骤 **outcome** 成功，并枚举明确选择的产物目录。

每个产物得到 `.chrono-cache-content-v1.json`，绑定缓存 ID、生产操作、实际源码键、产物声明和内容。清单记录普通文件 SHA-256／长度／执行位、目录（含空目录）、字面符号链接目标，不跟随链接；特殊文件和非 UTF-8 名称失败。只排除该方法自己的标记。标记包含输出身份，不含测试结果、judge 响应或缓存的通过状态。原生步骤、manifest 和封存观察保存在宿主执行证据中。

保存已采用项时，除原生产／保存条件外，还须封存成功。不增加构建配方、job 或测试重试。

原构建消费声称 exact／compatible 命中的内容前，`recover` 比较生产者 manifest 与新内容清单。兼容恢复的 manifest 绑定**实际匹配**源码键，候选保留新源码身份。内容变化／缺失、manifest 缺失／错误、生产者绑定不兼容都记为不可用。

先保留 manifest 原字节、观察内容身份／错误和原生恢复观察，再按已有恢复合同丢弃所选登记输出。原构建／检查继续执行，重复已完成恢复保全之后重建的输出。报告保留原键分类，另以 `unusable-content` 区分未接纳的命中，并把引用的验证证据复制到已有上传目录。

摘要计算是新增成本。封存和恢复核验按所选缓存记录实际墙钟秒数，每份观察说明测量窗口，原生步骤归 `cache-verification`。总传输／存储／runner 成本未知，有限原生证据见前文。

本地 Rust 回归在小 fixture 中，从未改登记 action 读取真实 Cargo 调用：兼容源码变化产生候选二进制并保留增量数据，生产者设置变化则划分新兼容范围。它是编译器证据，不是原生归档验收。内容 manifest 是可信但可能出错的生产者提供的一致性证据，不是归档认证或构建输入完整证明。上文两项缓存已有冷／保存、精确、兼容源码变化及不兼容输入证据；不可用内容恢复的完整验收仍未完成，部分观察见末节。

## provider 的真实消费者

检查／发布 provider 可通过 `persistent_cache` 采用 `chrono-github-cache/v1`，包含登记 planner `program`、登记 `config`、固定 `restore_action`／`save_action` 和明确 `jobs` 映射。每个已有 job 点名 `need`、`output_use`、字面 `prepare` argv、`consumer` ID 及精确 `caches` 集合。前两个用途字段解释该 job 为何需要输出，不是实测使用证据。

未知 job、缺失／空／多余／不匹配的缓存引用都在写输出前失败。本宿主缓存登记明确 `require_primary_checkout: true`；planner 在恢复前拒绝 linked worktree。这个文件系统保护不证明一般 Git 所有权或生命周期排除。

### 本仓的缓存归属

本仓 detector 构建共享启动 profile，选择登记的编译缓存。检查单元和汇总安装绑定源码的 startup 产物，当前单元与缓存集合分别归 units.json／cache.json。单元缓存登记只选择各 FILEMAP 计划仍实际存在的 Cargo 生产者；汇总和没有编译操作的单元不订阅编译缓存。

已有原生 Rust 发布构建／验证单元选择自己的项目输出，Python 发布验证和包收集器无登记编译缓存。分组测试中不同生产者使用不同条目。

缓存种子运行 `host.bootstrap-cache`，通过已有 bootstrap 所有者和 `build.cache` 构建候选 planner，再安装二进制。其配方明确 `report_path: .chrono-harness/state/cache-bootstrap.json`；之后 core bootstrap 保留默认 `.chrono-harness/state/bootstrap.json`。

宿主 bootstrap 接受可选的 `.chrono-harness/state/` 下字面报告文件，拒绝越界或符号链接位置，并传递报告发布错误。各配方拥有自己的报告位置；重复用同一位置会替换最新结果。已有产物上传保留两份独立记录。

传输报告保留报告进程的执行路径、实测文件摘要及版本。消费者可与独立 bootstrap 记录中的安装二进制摘要及干净源码身份匹配。文件摘要不证明编译器输入完整或内存加载身份；bootstrap 证据缺失或源码不干净，不能证明绑定到干净源码。

detector 缓存绑定自身 bootstrap 配方，没有业务检查政策输入。检查编译缓存明确从检查配置选择 `/policy/environment`、`/policy/tools/cargo`；单元成员、报告位置不改这些键，环境或 Cargo 工具变化会改键。

四项 `detect.*` 缓存的生产者使用 `bootstrap-core.json` 登记的专属 `CARGO_HOME`：`.chrono-harness/cache/bootstrap-owner-tooling/cargo`。兼容输入用仓库相对路径声明其中 `config` 和 `config.toml` 缺失，存在任一配置时规划失败，避免恢复与实际构建配置不符的产物。默认 runner home 的配置声明仍供使用该环境的检查／发布生产者消费，其绝对路径迁移尚未完成；这两组输入不互相替代，也不证明其余构建输入完整。

专属 Rust 宿主采用回归覆盖这些区别，包括 detector 没有业务检查政策，以及 detector 构建参数变化。它们只验证这些声明依赖，不证明构建输入完整或所有登记都有必要。

### 恢复、保存与写入者

provider 从 `chrono-cache plan --github-output` 取得输出键，保留计划和探测原输出，在原操作前恢复。默认业务成功后保存全部所选缓存。可选 `save_caches` 限定本 job 可保存项，空列表表示只恢复；未知或重复项拒绝。

同一 provider 配置内，每个缓存 ID 至多一个保存 job。生成／验证用 `E_CACHE_SAVE_OWNERSHIP` 拒绝冲突，指出缓存和两个 job。其他消费者明确只恢复；流程也可刻意没有写入者。独立登记生产范围使用不同缓存身份，包含不同发布平台。

本宿主把四项共享启动缓存交给已有 detector 保存，其他缓存交给登记生产单元。消费者仍恢复、重建、检查，不增加 job 或前置等待。写入者跳过或未完成生产时，该运行不提供新归档。

明确的 `save_after_bootstrap` 子集可在原操作结束后，改用独立 bootstrap 步骤成功作为保存条件，使后续测试失败时仍保留可复用启动编译。未知、重复或不在可保存集合的子集项会让生成失败；取消时不保存。该子集是宿主声明，不证明后续每次构建成功。恢复的编译候选仍须经过原构建入口。

planner／provider 按字典序排列消费者精确缓存集，用 `cache_0_key`、`cache_0_paths`、`cache_0_restore`，再用 `cache_1_*` 等作为 job 内输出名。这些短引用不改缓存身份或键。两者须实现同一输出合同，登记验证在生成前拒绝集合不符。

### 传输报告与原件

缓存不跳过标准检查，不用缓存状态替代构建／测试结果。最终 `chrono-cache report` 记录原生 action 输出、请求／匹配键、原业务／bootstrap outcome 及不可变来源报告。

`report` 与 `seal` 都要求计划中的 planner SHA-256 和版本完整，且与当前执行物全文相等。缺失、null、非文本或不匹配的身份在发布报告原件前拒绝；不再为缺少身份的旧计划发布 `unavailable` 成功。当前 `plan` 始终提供身份。只观察传输步骤的库函数 `transport_report` 仍报告未观察执行物身份，不替代 CLI 的核验。

`evidence_directory` 可把原件放入已有发布上传目录，普通检查则放入上传的宿主状态。detector 可选择同一字段，通过已有 job 的固定上传 action 保留原件，包括失败后。目录须是字面宿主状态路径，并明确上传隐藏文件；文件缺失使上传失败，不能声称已保留。本宿主选择 `.chrono-harness/state/`，包括 bootstrap、计划、探测原件。未采用时，detector 文件只是 runner 临时状态。

每份上传原件直接写入选定目录，并返回其路径和 SHA-256。探测程序保留实际探测原输出；计划中的探测引用、恢复、内容核验、封存和后端观察继续保留各自返回引用所需的原字节。

报告在已有 job 运行。未登记 backend 就不发 API 请求。只有 action 成功且 key 匹配，才报告 exact／compatible；空输出记 `miss-or-unavailable`。原生保存成功仍为 `unconfirmed`，带 `W_CACHE_SAVE_UNCONFIRMED`，因为固定 action 可能捕获后端异常后成功退出。不能由 action outcome 推断归档已保存、编译器复用或执行物身份验证。

本 job 未选择保存且未观察到保存的缓存记 `not-requested`；实际发生未登记保存仍可见，带 `W_CACHE_UNREGISTERED_SAVE`。具体保存归因和归档内容诊断仍需后端／action 日志。

报告生产或证据保留失败使对应步骤和 job 失败，原业务结果独立保留；报告步骤没有 `continue-on-error` 豁免。后端查询失败若已成功记录，则只是非致命可用性警告。缓存后端失败不阻止后续构建。

后端观察原生验收、明确坏缓存恢复，以及后续测试失败后保留任意项目编译，仍未完成。保存内容不包括 `.chrono-harness/state`、发布交接、所选测试结果、已安装 `bin`、Git metadata 或生命周期锁。

### 可选 GitHub 后端查询

`chrono-cache/v2` 可登记 `backend`：`kind: github`、具体 `need`／`output_use`、有限 `command`、明确 `inherit`／`credential_environment` 列表，以及 `repository_environment`／`ref_environment` 名称。恰好一个完整命令参数为 `{endpoint}`。

本宿主使用 `gh api --paginate --slurp {endpoint}`，限 30 秒／4 MiB。命令收到 `repos/{repository}/actions/caches?ref={encoded-ref}&per_page=100`，返回 GitHub 响应页数组。政策保留在计划中，不参与构建缓存身份；计划阶段从不查询后端。

已有报告步骤只在原生保存成功但仍未确认时，每批查询一次；否则记 `not-required`，不启动查询。生成步骤提供 `GH_TOKEN`，缓存工作流允许 `actions: read`。子进程只继承声明变量。保留证据包含执行物摘要、命令、原 stdout／stderr、原退出／失败及环境摘要，但从保留环境中移除声明凭据值。命令不得向原始流打印凭据。

精确 key／ref 匹配唯一后端条目时，保存状态改为 `backend-entry-observed`，可用性 `confirmed: true`、`creator: unestablished`。这只确认查询时观察到条目，不确认哪个 action 创建、归档完整、编译器复用或当前执行物身份。

多归档身份记 `ambiguous`；无匹配记 `not-observed`，分页不是原子清单，因此不证明不存在。查询失败、超时、截断／错误响应或身份冲突保留原失败，发 `W_CACHE_BACKEND_UNAVAILABLE`，保存仍未确认。后端存在不能替换失败／跳过的原生保存或原业务结论。查询成本仍归登记 `cache-report` 步骤。

### 输入和共享启动边界

宿主登记绑定真实编译器／Cargo 文件及版本、OS／架构、登记链接器／SDK 观察、声明标志缺失、manifest／lock、构建配置。源码变化使用 FILEMAP 明确 compile 依赖范围。检查、发布构建、发布验证和平台声明各用独立兼容范围。这份有限登记不证明编译器后端、SDK 库、构建脚本或环境配置输入完整，当前原生采用及缺失证据仍在覆盖表列明。

明确采用共享启动后，detector 一次构建核心工具，发布给登记消费者，见[共享启动传递](ci-units.md)。只有实际剩余的构建／测试生产者订阅 target 缓存；汇总和纯脚本消费者不恢复启动编译 target。

detector 独自保存 `detect.*` 启动缓存，真实单元生产者保存 `check.*`。detector 兼容输入不含自身 bootstrap 不消费的检查政策。导入 startup 验证不是编译生产者，不能取得 `save_after_bootstrap` 资格。各消费者证据保留原生产报告和明确导入来源；startup 二进制不进入证据上传。传输成本单独测量，绑定源码的产物不证明构建输入来源完整。

发布验证在 Cargo test destination 消费 `chrono-cache` 资产；bootstrap 安装的传输执行物仍归 bootstrap。发布配方不再把资产放到同一 bin 路径。Python 发布集成回归覆盖真实 CLI 的 plan／recover／staging／report，以及 inode、摘要保留；本地证据不证明原生缓存后端或外层工作流成功。详见[发布说明](release-ci.md)。

<a id="retired-remote-acceptance-controls"></a>

## 已退役的远端验收控制

**产品与宿主政策曾同步删除：**可选原生 manifest 删除 fixture、对应 CLI／配置和 provider 登记、仓库 Variables 激活及实时 PR 正文读取器。provider 不再为激活增加 `pull-requests: read` 或读 PR metadata。不需要替代激活传输、变量管理、远端缓存删除或额外权限。退役字段按普通严格解码失败，不会静默当作激活成功。

已有 `check.unit_cache` 仍计划、恢复两项登记编译归档，核 `producer-manifest/v1` 内容，通过普通恢复丢弃不可用宿主输出，执行原候选构建／测试，封存成功生产输出并经原生 action 保存。保存仍受必需生产者／封存 outcome 限制。普通后端观察保留原有必要范围及原失败，其他构建／发布缓存行为不变。

历史证据仍绑定精确 source／run／attempt。Variables GET／list HTTP 403 是真实历史失败。后续有限 C1 观察记录两项坏内容保存／恢复／拒绝、重建和替换缓存，但原兼容范围健康精确命中未达成：所选 rustup 身份改变，最新 collection 返回 HTTP 502。

这些只是部分观察；删除控制层不会重跑或完成验收。原记录、输出、提交和证据继续由原保管者保留。旧 fixture 及有限调用步骤可从保留源码 `f52442addfa3079f644cc7be59809a4ca6e756d5` 复现，但不是现役指令。

普通执行不再需要激活进程／API 请求。删除该层移除读取器、fixture、响应／intent 证据 IO 及专用测试；原生耗时、传输、存储和总生命周期收益未测。现有同语言内容／恢复、无效输入、provider 回归，以及按登记变更范围选择的标准检查仍为必需。原生验收、并发、其他消费者采用、完整输入、同判和实际交付仍分别未完成。
