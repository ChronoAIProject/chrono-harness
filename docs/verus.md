# Verus 实施范围

规范性权威、真实消费理由、证明与生产调用的绑定，以及 `E_VERUS_COVERAGE` 的目标失败语义，由 [SPEC 第 2.1.1 节](../SPEC.md#211-verus-规范性权威与实际调用)统一维护。当前结果合同只覆盖已登记操作，全仓覆盖门仍未实现，不能把局部检查通过当作全仓证明。

本仓按批次把实际决策和状态逻辑迁到 Verus。完整目标是所有 SPEC 与仓内程序逻辑都有证明，
外部接口只采用明确登记的假设。目前只完成下列局部范围；全仓强制覆盖门和其他程序的迁移仍在实施。
Python 实现暂缓，不把未迁移程序列为已证明。
新增规格、决策、状态维护与独立边界预期直接写在 Verus 中，在源码所有者计划登记整 crate 证明；
同次 DELTA 检查必须汇总受影响证明和实际消费者的行为结果，消费 job 不重复同一证明。
新增未证明逻辑不能以“以后迁移”豁免；全仓存量覆盖门仍未完成。

已声明文件的证明选择由 `verified_sources.rs::validate_selected_roots` 核对：
沿两端显式影响闭包，每个受影响文件的全部声明根必须出现在单元拆分前的全局计划中，
且操作唯一、带有效证明合同、使用严格整 crate 参数并调用准确的声明根。
声明文件未受影响时，已声明根的 `file:` 节点、该操作所属的完整所有者节点或
`tool:` 节点受影响，也必须选中对应证明。所有者和工具只读取现役完整方法登记，
不能从已选中操作反推；操作参数变化沿既有所有者 seed 触发，不扫描源码猜依赖。
仅一个根受影响时不强制同文件的其他无关根；声明文件本身受影响仍要求全部根。
本地 routes、scoped CI 库存、full 调度及 collection 准备共用此 Verus 判断；
缺失、普通操作冒充证明、重复方法、参数或根不符均返回带文件路径的 `E_VERUS_COVERAGE`。
实际原进程、编译依赖和固定候选绑定继续由现有结果入口核对，计划准入不产生证明成功。
独立预期在现有 runner Verus 根证明并作为真实 Rust 测试运行。
未受影响文件不因此重验；无声明的存量文件仍是未知，分类完整性、证明实际执行和
实际生产调用的完整对应仍须继续实施。没有新增证明根、工具、进程预算或 Python 迁移。
额外工作随文件、声明根、显式影响节点、完整登记方法和所选方法数量增长，
总构建／验证／CI 成本未测。

当前覆盖登记批次包括：FILEMAP 的 `crates/runner/src/verified_verifier.rs`、
`crates/runner/src/verified_sources.rs` 及独立预期
`crates/runner-tests/tests/verified_verifier.rs`，以及 runner 的
`verified_paths.rs`、`verified_scope.rs`、`verified_status.rs`、`verified_limits.rs`
和对应的 `runner-tests` 独立预期。它们均已声明 `program` 角色、唯一
Verus 根及现役 `verify.verus-result`、`verify.runner-paths`、
`verify.runner-status`、`verify.runner-limits` 与对应测试操作；`verified_scope.rs`
沿 status 的真实整 crate 根验证，不另造重复操作。
同源判断还要求根路径规范化、根路径唯一、program／specification 至少一个根、
data 不得有根，以及每个登记操作存在于 execution plan。声明只建立待核的绑定关系；
本批正式 CI 已核整 crate 证明；`crates/judge-ci/src/verified_collection.rs`
及 `crates/judge-ci-tests/tests/verified_collection.rs` 现在也以
`verify.ci-collection`／`verify.ci-collection-tests` 注册其 program 根和独立预期，
并由本地与离线 collection 的实际汇总消费。CI 字面量、projects 语言配对和
filemap 图内核及其独立预期也分别注册 `verify.ci-literals`、
`verify.project-languages`、`verify.filemap-graph` 及对应测试操作，均保留原真实生产调用。
仍不代表全仓覆盖完成。

CI 判官的同一 collection 证明根还定义 action 工具引用的来源准入：采用完整登记时，工具必须存在于有效配置，profile 副本不能补齐缺失；独立 scoped 宿主继续使用其明确工具表。实际 snapshot 校验调用该 Verus 函数，独立预期覆盖两种来源的全部存在／缺失组合。工具声明解码、两端变化传播及实际进程由相应行为测试与正式检查验证，不由这个局部谓词证明；沿用现有两项整 crate 证明操作。

本批把已有整 crate Verus 根补入 FILEMAP 的同源绑定：`chrono-identifiers` 的
`verify.identifiers`／`verify.identifiers-tests`，runner 的
`verify.retention-release`／`verify.retention-release-tests` 与
`verify.runner-git-binding`／`verify.runner-git-binding-tests`，worktree 的
`verify.owner-publication`、`verify.worktree-lifecycle`、`verify.worktree-attention`
及其独立预期，以及 cache 的 `verify.cache-planner`／`verify.cache-planner-tests`。
这些根均已有 execution-plan 操作和真实 Rust 消费者；本批只补齐逐文件的
`verification` 声明，不新增证明源或重复实现。整 crate 证明、行为测试和实际
生产调用仍按同次 DELTA 验收，局部结果不能代替全仓覆盖门。

本批把 `judge-workflow` 的执行完成准入接入同源 Verus：
`crates/judge-workflow/src/verified_execution.rs` 定义并证明收集完成与普通执行
的全部身份、范围、状态和操作数条件，`evidence.rs` 的两个实际完成路径直接消费
`admit_collected`／`admit_execution`。独立边界预期位于
`crates/judge-workflow-tests/tests/verified_execution.rs`，分别登记为
`verify.judge-workflow-execution`／`verify.judge-workflow-execution-tests`，固定
Verus 版本与整 crate 参数；Rust workflow 核心测试同时核真实 consumer。JSON、进程
回执和每项 operation 的外部解码仍由适配器及其既有所有者负责，本批不扩大证明边界。

本批继续把 workflow 集成证书的字段绑定准入接入同源 Verus：
`crates/judge-workflow/src/verified_certificate.rs` 定义并证明字段数量与精确字段集合
必须同时满足，`certificate.rs` 的真实 `validate_bind` 路径直接消费
`admit_binding`。独立边界预期位于
`crates/judge-workflow-tests/tests/verified_certificate.rs`，分别登记为
`verify.judge-workflow-certificate`／`verify.judge-workflow-certificate-tests`，固定
Verus 版本与整 crate 参数；workflow 核心测试和 inventory 同次验收真实测试消费者。
JSON 解码和集合构造仍是明确登记的适配器输入，本批不把外部接口假设当作仓内证明。

本批继续补齐宿主原生边界的既有证明根：`.chrono-harness/ci/native-tests/tests/verified_cargo.rs`
登记 `verify.bootstrap-cargo-tests`，release 原生实现及独立预期登记
`verify.release-recipe`／`verify.release-recipe-tests`。三项操作均已存在于 execution plan，
FILEMAP 现在把源码、证明根和实际 Rust 测试消费者绑定到同一登记；本批不新增证明源、工具或
Python 迁移。整 crate 证明和对应原生行为测试仍须由同次 DELTA 验收，不能把宿主局部绑定当作
全仓覆盖完成。

本批继续补齐 bootstrap bindings 的实现与独立预期，登记现有
`verify.bootstrap-bindings`／`verify.bootstrap-bindings-tests`；同时把 runner 的
`verified_cargo.rs` 模块绑定到包含它的 `verify.verus-result` 整 crate 证明和
`verify.verus-result-tests` 独立预期。三项源码均已有真实 Verus invocation、整 crate
结果和 Rust 消费者；本批只补 FILEMAP 绑定，不新增证明源或 Python 迁移。

## projects 执行原件状态

`judge-projects/src/verified_state.rs` 拥有完整状态文本与原进程终态的组合、
`completed`／`unknown` 选择、全部操作的终态汇总、原件摘要和长度的精确核验，
以及输入交接后解除依赖／恢复保护的条件。`state.rs` 的真实原件路径直接消费这些函数；
进程终态继续复用现役已证明的 `process_exit_known`，不以回执存在或状态声称成功代替。
`verify.project-state`／`verify.project-state-tests` 使用固定 Verus 和整 crate 参数，
独立预期同时作为普通 Rust 测试运行，并与既有执行／中断／原件行为测试同次验收。

本批源码与宿主验证登记同时修改，沿用工具、CI 单元和预算。JSON、文件、时钟与保留
接口仍由现有适配器调用；外围原件发布事务和完整执行器尚未证明，`state.rs` 仍保持
未知整文件覆盖，不把局部内核声明成整个外围文件已证明。汇总额外保留每项操作的借用观察，
时间与临时空间随操作数量线性增长，构建／证明／CI 的完整总成本未测。Python 迁移暂缓。

## 发布交付字段判断

发布交付的字段判断由 `release/native/src/verified_delivery.rs` 负责，接入现有
`verify.release-recipe`／`verify.release-recipe-tests` 整 crate 根。
身份适配器把完整环境文本交给规范十进制、版本分量、channel 和显式版本判断；
发布适配器把原 GitHub JSON 的可选文本、整数和布尔值直接交给逐字段判断，
缺失或错型不与空文本、零或 false 混同。API 的原退出和精确 404 字段共同决定缺失，
公开版本只有在全部声明资产已经存在时才允许沿用；草稿继续上传所缺资产。
独立根列出缺字段、完整文本、数字、channel、名称和草稿状态边界，普通 Rust
行为测试继续核实际环境与 mock 命令。这里的局部绑定不代表整个发布编排已证明。

**实现与宿主验证登记同改：**复用固定 Verus、宏和两项现役证明操作，
不增加安装器、证明过滤、测试时限或预算。采用固定 Rust 的 `str::split`、
`strip_prefix` 提供原版本分量，`str::parse::<u64>` 对溢出返回错误，
以及固定 Serde JSON 的字段读取、`as_str`／`as_u64`／`as_bool` 类型观察。
这些接口只解码实际输入，不供应预先算好的合格标志；仓内字段判断由 Verus 执行。
比较成本随实际字段字节和数量增长；新增构建、证明、CI 与存储总成本未测。
本批把 registration 的 input-coverage 行状态准入迁入
`verified_input_coverage.rs`，由 `verify.registration-input-coverage` 和
`verify.registration-input-coverage-tests` 分别证明生产谓词及独立边界预期。
JSON 解析仍由适配器负责，但 `bound`／`absent` 的绑定要求、无绑定状态、
`declared-complete` 对 `unresolved` 的拒绝以及 `absent` 识别均直接消费同一份
Verus 执行函数；本批仍只覆盖该登记状态闭包，不宣称全仓证明完成。

本批把 `judge-cost` 的声明成本未知坐标判定迁入
`crates/judge-cost/src/verified_cost.rs`，由 `verify.judge-cost` 和
`verify.judge-cost-tests` 分别验证生产谓词与独立边界预期。`produce` 在生成
`unknown` 成本记录前直接调用该 Verus 函数；JSON 字段读取和坐标列表编码仍是
明确的适配器。现役 cost judge 的消费者回归同时核对已知、缺失和混合坐标，
本批只覆盖四个资源坐标的未知判定，不宣称成本模型、测量或全仓覆盖已完成。

## runner 构建中的证明库与缓存

本宿主的 `build.runner` 由 Rust 启动程序读取
`.chrono-harness/ci/runner-build.json`，调用固定版本的官方 Cargo-Verus。
core bootstrap 先准备同一 Verus，再调用此登记操作；调用前端本身仍由普通 Cargo 构建。
执行计划含 `build.runner` 的检查单元也采用现有 `bootstrap-unit.json`，
在原检查前解包共享 Verus ZIP，不另下载，也不因该单元自身没有独立证明而省略编译器。
没有自研编译器包装器，也不解析终端上的成功文字。

该官方 Cargo 调用显式使用 `--jobs 1`，使未带 crate 身份的证明记录与目标库产物保持相邻，
避免其他并行编译任务的产物插入其间。Verus 仍使用登记的两条证明线程。
实际命令由 Rust mock 核对；冷构建耗时的变化尚未测量。

`verified_cargo.rs` 负责完整 Cargo JSON 记录的顺序判断。冷构建必须依次出现
当前版本的整 crate 成功证明、点名库、点名二进制和唯一成功终态；
暖构建可以省去新证明，但必须引用保留的原成功证明。
Cargo 并行构建的无关依赖产物可以穿插；它们不推进点名目标的状态，也不要求证明与点名库在输出中相邻。
编译错误、未知记录、重复证明、缺少产物和终态之后的记录均拒绝。
目标选择比较完整 manifest、源码、名称和类型；库的 rlib／rmeta 必须来自同一登记目录，
并具有同一个非空的小写十六进制指纹。原始 dep-info 的完整多目标头、源码列表和
checksum 由 `verified_sources.rs` 核对，最终二进制的原 dep-info 还须包含这两份库的字节身份。
这些原件均由实际编译器产生，测试用 Rust mock 提供独立的外部命令样本。

证明缓存只占 runner target 中的一个 `.chrono-verus-proof.json`，保留原编译进程、
原 stdout／stderr、原 dep-info 和库身份。stdout 和 dep-info 使用无损 UTF-8 文本存储，
避免 JSON 字节数组的膨胀；stderr 保留任意原字节。
暖调用重新核对当前输入、工具、原记录和库字节，不制造一次新的证明成功。
缓存键只绑定实际选中的恢复方法，不因登记表里无关操作的变化而失效。
只有本次完整原输出有效且点名库标为 `fresh` 时，原证明或依赖原件损坏才可执行
登记的 `recover.identifiers-proof`，随后沿同一入口构建一次。失败或无效恢复非零返回，
原构建错误不会因缓存清理而重试。恢复只调用登记的 Cargo 清理，实际编译使用 `--locked`，
沿用调用方选定的 Cargo 依赖缓存；元数据所需的缺包由 Cargo 补齐，不额外强制离线。
清理前保留已有证明、依赖原件和两份库的原字节；原件无法读取或超过读取上限时，
本次恢复失败并保全原路径，不先删除再尝试重建。

成功调用直接输出当前库、二进制和证明缓存的身份，临时两流与操作目录在消费后删除；
缓存身份是当前产物观察，不是永久历史引用。失败和被拒缓存的原件保留在
`.chrono-harness/state/proved-builds/`，并通过已有 `check-original` 保留所有者登记
实际诊断使用方 `proved-build-repair:<目录>`。调用方完成调试和验收后释放该使用方，
所需的完整 `chrono-retention-consumer/v1` 声明由原件输出中的 `report_consumer` 给出；
调用方将声明存到待清理原件之外，按现有保留入口释放并回收。
可重建材料在使用方结束后七日内清理。未确认完成的使用方继续受保护。
无保留政策的外部宿主由调用方保管原目录；不扫描或清理其他调用的输出。

**源码与宿主政策同时修改：**runner 构建入口、启动顺序、当前构建配置、FILEMAP 和
Rust mock／独立 Verus 根同步登记。生产判断复用 `verify.verus-result`，独立边界根为
`verify.bootstrap-cargo-tests`；不增加版本、安装器、全局缓存索引或共享准入锁。
macOS 的正式工具表也登记 `bootstrap-native`，由同一原生前端的 `--version` 提供版本观察；
仅在 bootstrap 配方中给出路径不足以让普通检查消费该构建入口。
runner 已离开普通 Cargo 配方，其锁定参数和 Cargo／Rustup 目录保留改由
同语言的原生 Rust mock 验证；Python 的普通 Cargo 环境测试只覆盖仍采用该路径的使用方。
每个原件的读取上限为 8 MiB；最终二进制只枚举登记 target 的 `debug/deps`，至多 4096 项，
按明确前缀、合法指纹和实际二进制字节选择原件。这不是测试发现或选测规则。
构建、存储、原生 CI 和完整生命周期成本以实际验收为准，未测部分保持未知。
本节描述 runner 对点名 identifiers 库的局部绑定；其他生产者、独立库测试的编译产物采用、
完整工具链输入、外部接口假设及全仓生产调用覆盖，仍须分别完成，不能据此宣称完整 SPEC 已验收。

## 缓存 planner 身份

`crates/cache/src/verified_planner.rs` 负责报告与封存的身份准入。四个可选文本字段分别来自计划和当前执行物的 SHA-256／版本；必须全部存在，且两对全文相等。JSON 适配器把缺失、null、非文本交为 `None`，直接消费判断结果。复用 runner 已证明的 `equal_text`，没有第二份字符串比较实现。JSON 解码、执行物读取、摘要计算和外层发布仍是未证明的接口边界；此处不证明 SHA-256 真实性或编译器复用。

同一根的 `producer_can_seal` 维护封存结果条件：成功可封存，失败须有明确采用，其他完整文本或缺失均拒绝。独立预期在普通 Rust 和 Verus 中核成功、失败、取消、跳过、文本边界及缺失；实际封存入口直接消费此结果。证明不替代原生步骤观察、编译目录内容核验或恢复后的构建和测试。

独立根核匹配、四个缺失位置、大小写、前缀、NUL、换行及完整中文文本；真实 CLI 回归另核拒绝发生在发布前并保留输入计划。两项 planner 整 crate 证明归 cache 测试计划；共用结果判断的证明归 runner 所有者。

普通 Cargo 测试调用 cache 的实际公开模块，对格式与 planner 身份执行同一预期的失败分支；
原四项测试及全部证明断言同时保留。证明通过不代替普通运行库的检查，也不证明两者编译产物身份相同。

**产品与宿主政策同改：**删除报告对缺少 planner 身份的旧计划放行，报告与封存共用实际谓词；专属测试、FILEMAP、证明操作与 cache 单元的原有 Verus 启动同步登记。测试复用已锁定宏包版本，不增加包版本或安装器。原缓存操作的预算不变；共用结果证明沿用 runner 的既有限制，不在 cache 另设覆盖。比较最多读取两对完整文本，额外状态为常量，不增加运行时进程。构建、证明、原生 CI 与存储成本未测；声明和生成成功不代表已完成验收。

## 原件交接与定向维护

`crates/runner/src/retained_artifacts/verified_release.rs` 负责 `release-preserved`
请求的数量、字节与唯一性边界、完整角色集合比较，以及仅移除所选原件的成员变换。
`maintain` 的显式清单准入和是否进入全库枚举的选择也直接消费此实现。
独立 Verus 边界预期覆盖空／重复／超额清单、缺少或多出角色、仍在使用、理由和原件大小边界，
并核对释放后其他成员的原顺序和内容。

本仓在 `runner-tests` 的现有执行计划中先运行 `verify.retention-release` 和
`verify.retention-release-tests`，两项都执行整 crate 证明并编译，使用现役固定 Verus、
`--no-cheating` 和 `--no-external-by-default`。Rust 生产调用和专属测试导入同一源码。
标准字符串接口提供 UTF-8 字节和 Unicode `trim`；文件系统接口提供物理路径、身份、时间和长度，
既有摘要／租约／JSON 适配器继续负责真实 IO。上述接口和保留模块的其余组合仍是完整证明边界，
不能以本节的局部证明代替全仓覆盖或真实宿主验收。

**源码与政策同改：**本批新增两项证明前置及相应 FILEMAP 登记；原维护节点、字节和 25 ms
上限不变，不新增共享清单或锁。新增证明和总生命周期成本在正式检查前未测。

同一证明入口还覆盖逐 PR 的查询预约、结果绑定和当前观察替换。生产保留所有者直接调用
`settle_observation_members`：只移除已返回查询及被取代的当前观察，保留其他未结成员的内容和顺序；
只有更大的实际预约序号才能推进已接受序号，重放或较早返回不覆盖当前对象。
PR 适配器也直接消费这一源码中的原件完整性、未核实创建、接续绑定和对象状态判断。
独立预期覆盖较早／较新／相同序号、缺观察、终态、原件和代际错配、零尝试及序号耗尽。
FILEMAP 明确将这两处 PR 消费者连接到现有 runner 证明与测试计划，不新增证明进程、安装器或预算。
runner 测试 manifest 也明确登记 `verified_release`，普通 Rust 执行同一预期的真实失败分支，
Verus 则证明这些预期成立；单独的预期失败测试核对断言没有在普通编译中被擦除。
JSON、Unicode trim、原件读取、实际进程终结、租约与多文件发布仍由原适配器和行为测试承担，
局部证明不证明 GitHub 观察真实、跨中断自动恢复或完整 SPEC 已完成；新增 IO、构建及存储成本未测。

## CI 配置的字面文本

CI 的原生 job 终态选择和发布者准入也直接消费本节的同源根。
`native_job_state` 保留 completed 的原结论；只有已知非终态、明确存在的 null 结论和正 job ID
才返回一次精确详情读取。适配器按返回 ID 调用 provider 一次，再由同源判断核成功终态，
并逐项比较 id、run_id、run_attempt、head_sha、name 五个完整原字段。
缺字段与存在的 null 分别保留；JSON 字段的原值编码由固定 Serde JSON 接口供应，
比较由已有 identifiers 字节判断执行，不把字段先折成一个“已合格”标志。

发布登记的 v2／触发方式、非空字段、首个完整 publisher 匹配、必需依赖和权限选择，
以及 checkout 的固定源码表达式也由同一生产根拥有。独立根核终态失败不重读、
缺失／非 null／零 ID 不读详情、原字段完整性、首个匹配及发布者依赖边界；
原 Rust mock 仍核准确调用次数、参数、原错误和每个字段的替换拒绝。
实际 provider、YAML 执行与整个生成器的原生验收仍须分别完成。

**实现与宿主验证登记同改：**gating／release 的实际使用绑定到现有
`verify.ci-literals`／`verify.ci-literals-tests`，复用已锁定工具和整 crate 参数。
没有新 job、工具、预算、重试或审批。固定 Serde JSON 对实际 job 字段提供完整编码，
标准 Option／Result 的 map、transpose 和错误传播只转发编码结果；
普通适配器对发布 job 的 ID 数组保留输入顺序，实际搜索由 Verus 完成。
每个待核 native job 编码五对字段；新增编译、证明、CI、分配与存储成本未测。

`crates/ci/src/verified_literals.rs` 统一负责 full 与 release 的单行文本判断。
输入为空或含普通 UTF-8 文本时允许；出现 NUL、LF、CR，或任意位置的精确 `${{` 字节序列时拒绝。
孤立的 `$`、`${`、花括号及 shell 字面文本仍允许。两个现役配置校验保留各自错误信息，
直接消费同一字节循环；规格要求返回值与全部输入的允许条件充要相等，并证明终止。

独立 Verus 根直接调用生产函数，列出空串、普通字节、UTF-8、多处禁用字节、
完整模板开头及相邻边界的预期。原 full／release 渲染测试另核真实命令参数的接受与拒绝。
CI 测试计划先执行两项整 crate 证明及编译，再执行原有构建、测试与生成物核验；
原生 CI 单元沿用现有 Verus 安装前置，不新增安装器或旧版登记。

首次编译前的下载缓存复用这两个证明根，新增登记形状、输入字符、环境变量、
单一绑定及使用方／保存者／基础缓存关系的判定。
`downloads.rs` 将登记观察交给这些生产函数，再投影原生 restore／save action。
独立预期逐项拒绝缺输入、重复声明、错误路由和不相容基础缓存；
Rust 行为测试另核真实 JSON 登记、缺失文件不写投影，以及恢复早于 planner 编译的顺序。
JSON 解码、文件读取、集合观察和 GitHub 的 `hashFiles`／cache action 仍属于接口边界；
局部证明不建立原生传输、缓存内容完整性或全部生成器实现正确性。

**程序与宿主政策同时修改：**CI 所有者、专属测试、FILEMAP、证明操作和原生启动前置同步登记。
固定宏包已由现役依赖使用，本批增加直接声明；不改变其他包版本。
循环最多检查 N 个输入字节，使用常量额外状态，不新增运行时进程；
两项证明的构建、验证、CI、存储及完整运行成本未知。
本节描述此局部合同；已提交候选的正式检查、原生结果与实际落地仍须各自核验，
不能据此主张外层配置、YAML、JSON 或完整 SPEC 已证明。

## CI 工件下载的状态与重试

`crates/ci/src/verified_download.rs` 负责现役 gather 使用的完整 `HTTP nnn:` 字节扫描和重试准入。
它只接受三个 ASCII 数字；完整状态码重复出现可以接纳，任意两处状态码矛盾则返回缺失。
规格要求返回值覆盖全部输入，并证明字节下标、三位数计算及循环终止。
重试同时要求命令失败、没有进程传输故障、仍有次数、状态在登记白名单内、两条输出流仍有容量，
且等待时间严格小于剩余期限。期限与状态白名单由现有适配器观察，不由证明内核推测。

独立根覆盖空／截断输入、完整与畸形状态、重复与矛盾状态、非 UTF-8 字节、数值边界，
以及每个停止条件。普通 Rust 测试执行同一预期，原 gather 消费测试继续核对恢复、原失败保全、
不可重试错误、次数耗尽、共享时间／流预算及恢复后仍拒绝无效业务证据。

**程序与宿主登记同时修改：**FILEMAP 登记生产根与独立根，并加入原 ci 测试计划；
实际传输、报告、保留与原生单元入口继续由现有所有者负责。两项证明沿用固定 Verus 版本、
无函数过滤、原预算和现有安装，不增加运行时进程。新增验证与完整生命周期成本尚未测量；
局部证明不证明 GitHub 响应真实、整个采集事务或全部 SPEC 已完成。

## runner 标识符、路径与结果状态

实际 CI 选择类型 `Scope` 由 `runner/src/verified_scope.rs` 唯一维护，
`units::Scope` 只重导出它。现役 runner、路由、项目判官、CI 判官和工作流生成器
直接使用同一类型；状态证明根导入该模块，不另建证明操作。
比较要求变体相同且完整 unit／manifest 文本相同；克隆保留原变体和全文。
`Unit` 的两种参数均为 `--unit` 加完整名称，`Collect` 的短参数仅有 `--collect`，
完整参数另含原 manifest。类型本身不替调用方增加名称或路径限制，原准入继续执行。

独立预期列出参数数量、顺序、完整内容和不同变体的拒绝，覆盖空串、空格、UTF-8、
前缀、大小写、NUL 与换行；同一测试中的失败分支也在普通 Rust 中实际执行。
原请求编解码和 CI 行为测试继续验证实际使用；Serde 编码、配置版本分派及外层组合
仍是此局部证明的边界，不据它主张完整执行链或全仓覆盖已完成。
**源码与宿主登记同改：**FILEMAP 增加该真实源文件及明确消费者，复用现有状态的
两项整 crate 证明和原未过滤测试；无新增工具、项目、证明进程、共享状态或预算。
全文比较和文本复制成本随实际字符串长度增长；完整构建、证明、CI 和存储成本未测。

`crates/identifiers/src/lib.rs` 是实际 OID 和 CI 单元 ID 判断的唯一实现。
它是独立生产库 `chrono-identifiers`，有自己的 manifest、lock、target 和专属测试项目
`identifiers-tests`；runner 通过直接路径依赖使用它，`verified_identifiers` 重导出唯一真源。
其余证明根在证明模式下明确导入同一源文件。没有根 workspace 或第二份实现。
OID 必须恰为 40 或 64 个小写十六进制字节，且不全为零；ID 必须非空，
每个字节都属于 ASCII 字母、数字或 `._-`。循环证明返回值与各自规格充要相等，包含边界和终止。
`facts::full_oid` 直接重导出同一根的 `full_oid`，成功／失败分支也由 Verus 拥有：
返回 `Ok(())` 当且仅当完整输入字节满足 OID 规格；拒绝时返回完整原错误文本
`expected full nonzero lowercase commit OID`。不截取、不归一化输入，现役调用方无需迁移接口。
独立 Verus 预期核两种长度、仅一个非零位、全零、空／短／长、大小写、NUL、换行，
以及任意位置的非法字节；同一成功／错误文本预期也在普通 Rust 中实际执行。
文本借用和诊断字符串构造沿用固定 vstd 的 `str::as_bytes`／`ToString` 外部合同，
没有新增仓内受信函数、操作、依赖或证明预算。`units::id` 继续消费同一字节判断。
最多扫描 64 字节，拒绝时构造固定长度的原诊断；完整构建、证明及 CI 成本未测。

CI 生成器及其事件输入、汇总、detector 和 full 上下文，以及 CI 判官，
通过 runner 的重导出使用同一 `valid_oid`，不再维护各自的格式算法。
CI 的布尔适配和 action 固定 40 位限制保留；判官保留原 `Result` 接口及包含完整输入的错误文本。
这只收敛格式判断，未覆盖调用方全部控制流，也不扩大上文 runner 编译产物的局部证明绑定。

CI 判官的 ID、共享启动产物名和汇总仓库的 owner／name 各部分，
也通过同一重导出使用 `valid_id`，不再重复字符判断。
产物名原有的 128 字节上限、仓库恰为两部分、各自完整错误文本及其他准入条件保持；
不把这一字节判断的证明扩大为调用方整体已证明。复用原登记和测试，无新增依赖或证明操作。

同根的 `git_inventory_records`、`git_inventory_fields`、`git_inventory_row`、
`git_inventory_termination`、`git_inventory_text`、
`git_tree_metadata` 和 `git_index_metadata` 拥有 Git 库存的局部准入和错误分类。
`checkout::tree`／`changes` 直接调用它们：空输入或末字节 NUL 才通过终止检查；
UTF-8 解码失败、缺 TAB、字段数错误和非法 stage 保持原完整错误。
终止检查仍先于解码，字段判断仍先于路径／OID；tree 恰三字段，index 还要求
stage 恰为单字节 `0`–`3`，不加强其余字段内容或丢失空字段。
NUL 记录忽略空项，保留非空记录的顺序、重复及完整字节；metadata 只按 ASCII 空格拆分，
保留首尾和重复空格产生的空字段。行只在第一个 TAB 分割，其后全部字节归 path。
`ascii_split_records` 从完整字节递归定义序列；实际循环证明返回数量、每项完整字节及终止。
ASCII 分隔字节由固定 vstd 的 UTF-8 引理证明是字符边界，跳过该单字节后继续安全借用；
解码、记录和行拆分成功时保留完整原文，不作 Unicode 归一化或字符截取。
独立 Verus 预期核空／内嵌 NUL／末尾非 NUL、任意完整文本和字段、缺行、
0／2／3／4 字段以及精确 stage／相邻非法字节／尾随 NUL 和 LF；同一预期进入普通 Rust。
拆分的独立预期明确列空记录、重复记录、CR／LF／TAB、缺分隔符、首尾及重复空格，
以及多字节 UTF-8 的完整返回数组；普通运行逐项比较长度和全部字节。
复用现有两项标识符整 crate 证明、FILEMAP 源码边及标识符／runner 的未过滤测试，
无新增工具、操作、信任函数、依赖或证明预算。
外围仅使用标准安全 `str::from_utf8`／`Result::ok` 提供实际解码观察，拆分直接调用同源实现；
Verus 内使用固定 vstd 的切片、`str::as_bytes`／`str::len`／`str::split_at`、
Vec 存取、Option／Result 和 `ToString` 合同，没有新增仓内受信函数或 unsafe。
index 调用组合、重复 stage 与文件系统比较仍待证明。
拆分扫描 N 个字节、保存 K 个借用字段，时间 O(N + K)、额外引用 O(K)；
NUL 记录由惰性迭代改为借用数组，增加 O(K) 引用存储，meta 字段仍使用借用数组。
其余新增判断读取末字节、字段数和至多一个 stage 字节；完整构建、证明、CI 与存储成本未测。

tree 的条目构造、插入及重复路径拒绝改由实际类型所有者
`verified_status::insert_git_tree_entry` 维护，完整逐行解析由同根 `parse_git_tree_rows` 调用它。
证明新 map 恰为原 map 在该完整路径插入原 mode／OID／kind，所有其他键值均保留；
成功当且仅当原 map 没有该路径，否则完整返回 `duplicate Git tree path`。
沿用原标准 insert 的顺序：重复时条目也已替换，外围立即返回错误并丢弃整棵候选树。
条目字段与路径本身不在此函数重复准入，不归一化或截取文本。
独立 Verus 预期在普通运行中也核完整字段、map 数量、其他路径保留、不同字段及相同字段的重复拒绝、
完整错误和失败后的替换状态；覆盖独立克隆、空路径、前缀／内部 NUL 及 Unicode 规范化差异。
明确采用固定 vstd 的 `BTreeMap<TreePath, TreeEntry>::insert` 精确更新与原值返回合同，
其 `obeys_cmp::<TreePath>()` 前提由现有真实比较和顺序法则证明；不信任仓内状态函数。
复用状态所有者及独立预期的两项整 crate 证明、现有 FILEMAP 消费者边与未过滤测试，
无新工具、依赖、操作、信任函数或预算。每条记录仍构造一个键和三个字段，map 操作为 O(log K)
次完整路径比较，字符串复制及比较另计字节成本；完整证明、构建、运行和 CI 成本未测。
`parse_git_tree_row` 从完整输入字节定义首 TAB、三字段、路径及 OID 的原检查顺序。
`parse_git_tree_rows` 证明完整逐行循环：成功前缀的每条路径有效且两两不同，
真实 map 与这些行双向对应，全部字段、数量、无额外条目及终止均由合同维护；
错误必须位于首个失败行，前面所有行有效且无重复，不能以后续错误覆盖它。
标准 map 的完整严格有序迭代合同证明同文本的库存键唯一；插入后只创建迭代器，
不遍历或复制库存，不新增受信函数或标准接口假设。
动态路径诊断使用带原借用路径的错误类型；Verus 证明固定错误原样返回，
非法路径全文交给唯一标准 Debug 格式化 closure。`checkout::tree` 只转发到该实现；
明确外部接口为 `format!("invalid relative path: {path:?}")`，格式化语义由 Rust 标准库提供，
不声称仓内证明覆盖标准 Debug 算法。字节终止检查、UTF-8 解码与 NUL 拆分沿用原外围顺序。
独立 Verus fixture 明确给出分隔位置、完整字段及预期；普通运行核空库存、完整 map、
未归一化路径、首错、后续坏行和重新分配的同名路径拒绝，以及非法路径中的内部 NUL／后缀保留。
空 metadata 字段仍先通过三字段检查，路径先于空 OID 拒绝；同一预期也实际调用错误渲染分支。
**产品与宿主登记同步修改：**路径根被状态证明直接使用，FILEMAP 补齐其真实编译与测试使用方；
沿用原两项状态整 crate 证明及未过滤测试，无新工具、操作、预算或 Python 迁移。
新增迭代器只建立借用状态，不收集行；整体仍按 K 次 O(log K) map 操作及原字符串扫描计，
完整证明、构建、CI 与存储成本未测。index 重复 stage、文件系统比较与全仓覆盖门仍未完成。

原始 commit 物理头的 parent 行选择也由同一根的 `commit_parent_records` 实现。
以 LF 字节分行，在首个空行停止；仅选择精确 `parent ` 前缀，完整借用原后缀，
保持全部记录、顺序和重复，不删 CR、NUL、非 UTF-8 字节或空后缀，也不读取正文中的 parent。
规格 `commit_parent_lines` 定义完整序列，循环证明返回长度和每项原字节恰与它相等并终止。
`facts::commit_parents` 直接消费这些切片，继续执行原 UTF-8 解码、完整 OID 检查和错误分类；
该外围解析组合和 Git 获取仍未证明。独立预期核双空入口、正文／续行排除、完整顺序，
以及空、CR、NUL、非 UTF-8 后缀的完整长度和每个字节；同一失败分支也在普通 Rust 中执行。
复用原两项标识符整 crate 证明和原未过滤运行测试，无新增操作、信任、依赖或预算。
切片 `split_at` 和 Vec 存取沿用固定 vstd 的明确外部容器合同，不引入仓内受信实现。
扫描量随第一段物理头长度增长，额外保存所选记录的切片引用；完整成本未测。

标识符边界测试在普通 Cargo 模式下直接调用独立的 `chrono_identifiers` 库，
核对实际库的 OID、单元 ID、摘要、请求头和合并父提交判断。布尔预期经带前置条件的
运行时失败分支检查；Verus 同时证明这些预期成立，普通 Rust 也实际执行失败分支，
不把证明用的 `assert(...)` 当成运行时断言。独立 Verus 根仅在证明模式下
导入生产源码，保留原预期和整 crate 证明。两项原证明和未过滤运行测试归独立库的专属计划，
沿用 runner CI 单元；这建立测试与 runner 所依赖真源的对应，
独立测试的普通 Cargo 编译库仍未与 Verus 产物绑定。runner 的点名库采用见
[构建中的证明库与缓存](#runner-构建中的证明库与缓存)，其余使用方的完整绑定仍未完成。

**产品与宿主政策同改：**共享身份判断及原边界测试迁到各自的自然所有者，
同步当前路径引用、独立锁、FILEMAP、项目配对和单元；两个独立 target 明确位于
已采用清理的 `.chrono-harness/cache/identifiers/` 和 `identifiers-tests/` 下。
原判断、全部预期、两项证明及原预算保留；没有新增证明进程或共享状态。
专属测试自行编译真实库，只使用测试项目的 target；不把未被测试消费的日常生产构建列为前置。
两项整 crate 证明继续独立执行；完整构建、验证、CI 和存储成本未测。
发布计划也显式保留迁移的八项原测试：现有 Linux 和 macOS 平台各增加一个独立库测试单元，
使用既有发布执行器、原未过滤操作及汇总，不增加发布二进制依赖或缓存。

摘要格式也由同一根的 `digest_format`／`valid_digest`／`valid_digest_text` 规定和实现：
必须恰为 64 个 ASCII 小写十六进制字节，允许全零。`wire::is_digest` 直接重导出文本判断，
现役响应、原件、输入和工作树使用方消费同一实现。字节循环证明完整输入的充要条件和终止，
文本入口证明借用完整 UTF-8 字节；不把格式证明当作 SHA-256 计算、真实性或原件存在证明。
独立根核 0／40／63／64／65 长度、全零、数字和 a–f 端点、任意位置的非法字节，
以及完整混合文本、NUL、换行、大小写和 UTF-8 序列。复用身份判断的原两项整 crate 操作；
`str::as_bytes` 与切片长度／索引使用固定 vstd 的外部合同，没有新增受信仓内实现。
最多读取 64 个字节，使用常量额外状态；这是算法估计，完整构建、验证、CI 与存储成本未测。

普通请求和初始库存请求的头部判断也直接使用同一根的 `valid_request_header`。
返回真当且仅当协议和模式分别与调用方给出的精确值相等，且 judge ID 非空。
普通请求绑定 `chrono-judge/v1`／`evaluate`，初始请求绑定
`chrono-initial-judge/v1`／`inventory`；不额外限制 judge ID 的字符。
独立根核两个有效组合、交叉协议／模式、空名称、大小写、版本后缀、NUL、换行，
并证明任意非空名称均接受，包括 Unicode。复用身份判断的原两项整 crate 证明，
没有新增验证进程。请求封存、JSON／摘要和外层组合仍待证明。
比较最多读取协议和模式的完整字节，名称只查空值；构建、验证和完整运行成本未测。

`facts::artifact_exclusions` 的完整过滤迁到同源路径根的
`literal_artifact_directories`：只保留以 `/` 结尾、每个目录分量非空且不是 `.` 或 `..`
的原声明，保持输入顺序、重复和原文借用。
不归一化路径；此层仍保留 Unicode、glob 字符、NUL、换行和反斜杠，
字段准入仍归调用方。独立 Verus 预期直接列出拒绝、原文保留及完整过滤输出，
复用原两项路径整 crate 证明与 FILEMAP 已登记使用方。
长度、完整文本及原路径边界的同一预期同时进入普通 Rust 的失败分支，
文本检查复用已证明的 `equal_text`，不把证明专用的 `assert` 当成运行时断言。
固定 vstd 已有借用、UTF-8、Vec 追加和切片读取合同继续沿用，没有新增外部假设。
仓内过滤、完整借用序列与终止由 Verus 验证。原调用方继续通过标准库
`Vec::into_iter`／`Iterator::map`／`format!`／`collect::<Vec<String>>` 逐项加精确
`:(top,exclude,literal)` 前缀；这是明确保留的外部格式边界，本批不宣称其机械精化。
声明 P 项、共 N 个字节、合格声明 K 项时扫描为 O(P + N)，额外借用数组为 O(K)，
构建、证明与 CI 总成本未测。

`crates/runner/src/verified_paths.rs` 是 `relative_path` 的实际判断实现。
它精确描述当前 POSIX 词法路径合同：拒绝空、绝对路径、开头的当前目录、任意完整父目录分量，
以及 NUL、换行、回车和反斜杠；保留原本允许的内部 `.`、重复分隔符和末尾分隔符。
调用方的 `str` 保证 UTF-8 有效性。规格与执行循环证明全部字节的拒绝条件、分量边界及终止；
不据此主张文件系统解析、符号链接或目录穿越的完整安全合同。
缓存产物准入也直接调用同一 `relative_path` 实现，拒绝宿主外的绝对路径；
缓存测试核对真实 Config／plan 的相对路径接受、绝对／空／父目录路径拒绝及精确宿主登记。
复用现有证明根，不增加判断层；缓存自身继续核 glob、符号链接、受保护目录和输入／输出重叠。

同一根的 `valid_host_state_path` 还要求精确 `.chrono-harness/state/` 前缀和非空字节后缀。
scoped 报告、输入快照／保留内容及登记状态路径直接使用这一判断；登记 schema 继续单独拒绝
内部 `.`、空分量和末尾斜杠。独立 Verus 根核前缀、空后缀、相似目录名、中文及相对路径边界，
Rust 回归另核真实配置读取和保留引用。词法接受不表示文件存在或允许写入。

路径边界测试在普通 Cargo 中调用 runner 的实际导出；预期不符时执行失败分支，
独立 Verus 根证明同一分支不可达。证明断言不代替这些运行时检查。

**产品与宿主政策同改：**两项整 crate 路径证明归 runner，registration 和 inputs 保留实际消费测试，
FILEMAP 明确连接源码、所有者与消费者。判断最多先读 22 个前缀字节，再扫描 N 个路径字节，
使用常量额外状态；完整构建、验证、CI 与存储成本未知。

`crates/runner/src/verified_status.rs` 是实际 CI 结果 `Status` 的唯一类型定义。
四种状态 `Passed`、`Failed`、`NotRequired`、`Blocked` 的比较精确保持变体身份，
克隆保持原状态；独立根逐项列出全部 16 对比较预期。实际报告与汇总通过 runner 的公开类型使用同一实现。
状态边界测试在普通 Cargo 中调用实际 runner 导出，执行原有布尔、状态、顺序、完整文本及可选字段预期；
响应拒绝还核对实际错误变体和完整原消息。原测试和 ghost 中间断言全部保留，
独立 Verus 根证明同一运行检查；证明断言不代替实际运行中的失败分支。
原 Serde 派生和 JSON 名称保留，Rust 回归核四种名称、往返及非规范值拒绝；
序列化适配、外层状态转换和完整响应合同仍待证明，不把派生实现计为已证明。
同一模块也拥有真实 `CheckResult`，其克隆证明精确保留 ID、状态、原因和可选退出码。

七判官记录问题时直接调用同源 `append_finding`，输入为真实的可变协议状态、
原 finding 向量、已分类的严重程度与完整 finding。返回状态为原状态与传入状态的较高者，
结果向量恰为原向量追加该 finding；旧记录、顺序、重复和所有字段保持原值。
此函数不重新判断错误码或级别，也不替代字段与状态的一致性检查。
独立边界覆盖全部 16 对状态，以及空／非空记录、重复、Unicode 与 NUL 字段保留。

两项状态证明在 runner 所有者计划执行，实际判官保留自身构建和行为测试；
源码变化按 FILEMAP 的显式边同时选中所有者与受影响消费者。
四个所有者的写入式 Cargo fmt 入口和原格式检查继续保留。
不增加证明根或证明操作；完整构建与验证成本未知。
JSON、错误分类、其他响应组合和全仓覆盖门仍未完成，实际验收以原证明和消费者结果为准。

普通协议响应、full 保留判官结果及历史报告检查直接共用同源 `wire_header_matches`。
接受当且仅当完整协议、请求 ID、判官 ID 分别等于该入口的实际预期，且原进程退出精确符合
pass／warn 为 0、fail 为 1、error 为 2。此处不另加非空或名称格式限制。
独立 Verus 预期覆盖任意文本、任意 `i32` 退出码、逐字段不匹配，以及空值、Unicode、NUL、
前缀与 Unicode 规范化差异；现役使用方保留原错误及外围报告／进程检查。
复用状态的原两项整 crate 证明与登记消费者，不新增操作、依赖或外部接口。
比较最多读取三个字段的完整 UTF-8 字节，使用常量额外状态；完整运行、构建、CI 和存储成本未测。
`valid_finding_fields` 直接读取真实 `Finding`，核代码和消息非空、级别精确为
`info`／`warning`／`error`、引用数组非空。它不限制代码／消息的字符，也不按引用内容或 causes
增设条件；普通请求与初始库存共用原协议入口，保留原错误和后续引用归属检查。
`valid_evidence_identity` 借用实际路径、种类和摘要，核精确 `.chrono-harness/state/` 字节前缀、
种类非空及 64 位小写十六进制摘要。原前缀合同允许空后缀，也不额外拒绝后缀中的字符或分量；
文件存在、路径解析、链接及内容摘要仍由原 IO 路径检查，不把字段形状当作原件已验证。
两项判断证明返回值与规格充要相等；独立预期覆盖字段分别缺失、全部合法级别、相似／带后缀／
大小写级别、任意非空代码／消息／种类、Unicode／NUL、重复及空字符串引用、精确／相似／绝对前缀、
空后缀与摘要长度／大小写边界。

复用原两项状态整 crate 操作、实际使用方及 FILEMAP 登记，不新增证明进程或依赖。
固定 vstd 的 `<[u8]>::split_at` 是这里明确采用的外部标准库接口：索引在切片长度内，
返回两个借用切片，分别精确保留原字节的前段和后段；完整前段比较仍调用本仓已证明的 `equal_bytes`。
`str::as_bytes`／`str::is_empty`、String 字段借用、切片长度、Vec 长度和实际字段克隆沿用已列的固定 vstd 合同。
两项判断只借用原字段；路径前缀最多比较 22 字节，摘要最多比较 64 字节，级别最多三次完整文本比较，
不增加分配或运行时进程。构建、证明、CI 和存储总成本未测；候选正式检查与原生验收另行核验。
实际 DELTA 的唯一定义也在此根，`wire::Delta` 直接重导出；Serde 外部编码及六个字段保持原样。
克隆保留种类、路径、两个 blob 和两个 mode 的完整值；比较按六个字段的完整文本判断，
四个可选字段分别区分 `None`、`Some("")` 和其它内容，不把字符串的内容比较写成内部值身份。

实际 Git 树条目 `TreeEntry` 也由此根唯一定义，`facts::Entry` 直接重导出；mode、oid、kind 字段和
Serde 外部格式保持原样。克隆证明三个字段完整保留，比较证明三个完整文本全部相等的充要条件。
`delta_for_entry` 直接消费原路径和两端真实可选条目：两端均缺失或
三个字段全同返回 None；仅新端存在为 A，仅旧端存在为 D，其余差异为 M，包括 oid／mode 相同而
kind 单独变化。每条记录证明原路径、旧新 blob、旧新 mode 的完整投影，不把 Some("") 合并成 None。
这里不新增字段准入、OID／mode 解析或路径规范化。

独立 Verus 预期核任意相同字段的不同分配、克隆、两端缺失、增加、删除、每个字段分别变化及反向变化、
三个字段同时变化；保留空路径／字段、Unicode、NUL 和不同 Unicode 表示。预期直接检查 A／D／M 与
六个输出字段，不借用生产比较谓词决定预期。沿用原两项整 crate 状态操作及实际消费计划。
String 借用／克隆与 Option 枚举沿用已列固定 vstd 合同，没有新增外部假设或仓内受信实现。
逐条比较最多读取三个完整文本，只为变化记录复制路径和两端字段；完整构建、证明、运行、CI 与存储成本未测。
实际 `facts::Tree` 的键现在是同根 `TreePath`；原文 String 不归一化，内容比较、
UTF-8 字节词典序、Eq／Ord 法则、克隆和 `str`／String 借用都由 Verus 实现并证明。
真实 Git 树解析使用 `from_text`／`from_owned` 构造；checkout、export、registration 和
worktree 在明确文本边界借用或复制原文，judge-ci 向现役字符串库存移动原文，
CI 初始库存从键复制原文到现役字符串数组；
这些交接不维护第二份键或比较实现。
独立 Verus 预期在普通 Rust 中也执行失败分支，覆盖不同分配的同文、空文本、Unicode、
NUL／换行、不同 Unicode 表示，以及大小写、完整前缀和内部 NUL 的顺序。
固定 vstd 的 String／str 借用、to_owned、clone、as_bytes 合同沿用现役外部边界；
标准库 `BTreeMap::{new,insert,get,contains_key,keys,iter,values}` 及其它既有 BTreeSet 容器调用
仍是原实际使用的接口，未增加标准库假设或受信仓内包装。
worktree／CI 文本交接的 `Iterator::{map,chain,collect}` 仍是明确的外部容器适配，
未声称逐项机械精化。
键直接持有原 String，不另存比较用副本；比较最多读取公共字节前缀，完整运行／证明／CI 成本未测。
`facts::delta` 现在直接转发 `delta_for_trees`，移除原 BTreeSet 路径并集、逐键 map 查找及
filter_map 组合。该函数借用两棵实际 BTreeMap 的全部原键和值，证明枚举与各自 map 的完整
键值集合相等且 UTF-8 字节严格有序；两个索引线性合并，逐条调用 `delta_for_entry`。
完整结果合同按任意路径投影：输出恰为该路径实际两端三个字段决定的零或一条 A／D／M，
并且全部输出严格有序。因此完整 map 的遗漏、额外路径、重复和乱序均被合同排除。
各端按自己的原键取值，内容相同但不同分配的两端路径也对应同一变化；不以跨树内部值身份查找。
枚举、合并循环、递归规格、逐路径完备性、map 绑定及严格顺序都在同源根证明，包括终止。

明确采用的外部接口是固定 vstd 的 `BTreeMap::iter` 与 `btree_map::Iter::next`：
在本仓已证明的 TreePath Eq／Ord 法则下，枚举序列恰为原 map 的全部键值、键严格递增，
每次 next 借用当前位置的原键值或仅在末尾返回 None。仓内完整借用序列和合并逻辑不作为受信接口。
独立 Verus 预期保留双空树、增删及 kind 单独变化、不同分配但全同字段不输出、反向变化、
空／Unicode／内部 NUL 路径和空字段，直接列出完整长度、顺序与六字段；普通 Rust 执行相同失败分支。
独立 fixture 的字段表与预期序列不用生产合并递归生成；全部逐路径投影唯一确定严格有序输出的
数学推论也由 Verus 证明。沿用原状态生产／独立根两项整 crate 操作，不新增依赖或信任项。
新增借用数组占 O(|A|+|B|) 引用，合并步数至多 |A|+|B|；路径比较与变化字段复制另计字节成本。
完整构建、证明、运行、CI 与存储成本未测。Git 命令、原始输出解析、JSON 适配及整个 Git 事实
获取仍待证明，不把完整已解码 Tree DELTA 的证明扩充为完整 Git 事实证明。

`valid_finding_reference` 直接借用普通请求的实际 DELTA 切片：引用首个 UTF-8 字节为 `/`，
或完整文本等于任一 `Delta.path` 时接受；初始库存传 `None`，只接受前一类。
匹配不做大小写转换、Unicode 规范化或路径规范化，也不使用 DELTA 其余五个字段。
`valid_finding_references` 对真实 finding 的整个引用数组证明全称条件与终止，保留顺序和重复；
空数组在此全称条件中为真，原 `valid_finding_fields` 仍单独拒绝空数组。
这只证明现有允许条件，不证明 `/` 后面的登记记录 JSON pointer 语法、目标存在或真实归属。

独立 Verus 预期覆盖任意路径的精确匹配、未列路径拒绝、初始库存和空 DELTA、
空文本、Unicode／NUL、重复引用、首／中／末位置的非法引用，以及 DELTA 六字段分别变化、
可选值缺失和完整克隆。同步删除规格与实现的 DELTA 授权、同步跳过首个引用，
均须由这些独立预期拒绝；候选验收另核原结果。
继续复用原两项整 crate 状态操作、实际消费计划及已有文件登记。
固定 vstd 的 `Vec<T>::as_slice`／切片借用转换用于保持原数组的完整元素、长度和顺序；
String 借用／克隆及切片／Vec 索引沿用上列外部合同，可选文本由本仓的显式分支处理。
引用匹配不建立路径副本，最坏对 R 个引用各比较 D 个实际路径；完整构建、证明、运行、CI 和存储成本未测。
JSON 解码、字段转发、登记记录指针归属、原件与完整响应组合仍有各自未完成义务。

普通与初始库存的 `wire::validate_response_at` 直接把原响应字段、完整 finding 切片及实际 DELTA
交给同源 `validate_response_fields`，并传播原 Result。
`valid_response_findings` 对全部 finding 证明字段与引用条件的全称判断及终止；
`response_fields_error` 明确错误顺序，实际实现证明成功充要条件和每个错误的完整文本：
先返回 `E_PROTOCOL: response identity/status/exit mismatch`，再返回
`E_PROTOCOL: invalid finding`，最后返回 `E_PROTOCOL: finding/status disagreement`。
这包括字段与引用都有效后才检查状态的原行为，不接受调用方计算的有效标志。

独立 Verus 预期核任意相同／不同头部、退出码边界、空／Unicode／NUL 文本、
首／中／末非法 finding、空引用、普通／初始引用范围、四态成功与状态不一致，
同时存在多种错误时仍保留上述优先级和完整错误文字。
沿用原两个状态整 crate 证明、实际使用方和 FILEMAP，不增加项目、操作、依赖或证明预算。
错误文本生成明确采用固定 vstd 的 `str::to_string` 完整字符合同；
`wire` 的 `?` 直接返回同一个错误。成功路径只借用原列表，扫描使用常量额外状态；
错误路径分配一条固定消息，不增加进程。完整构建、证明、运行、CI 和存储成本未知。
后续 evidence 遍历与真实文件 IO 仍在原负责方；不据此前置组合主张完整响应已经证明。

留存执行报告的 `full::validate_execution` 直接调用同源 `validate_record_response`，
传入真实响应头、finding 切片、进程退出及重建请求的预期身份；原 Rust 私有组合函数已移除。
`record_response_error` 先选择 `response identity/status/exit mismatch`，
再选择 `response findings/status disagree`，实际实现证明成功充要条件、完整错误文本和上述优先级。
外层 `E_SELF_DIAGNOSTIC` 记录上下文、JSON 解码、请求重建、进程与原件一致性仍由原审计器负责，
不据此声称完整报告审计已经证明。

独立 Verus 预期覆盖任意完整头部、逐字段不匹配、退出码边界、空／Unicode／NUL、
四态成功、状态不一致，以及头部与状态同时错误时的优先级；继续使用原两项状态整 crate 操作与消费者。
同步调换规格／实现的错误优先级、同步删除状态一致性检查，均由独立预期拒绝；正确源码按摘要恢复。
文本转换沿用明确列出的固定 vstd `str::to_string` 合同，没有新增外部假设或依赖。
头部最多比较三个完整文本，状态扫描为 O(F) 条 finding，使用常量额外状态，错误时生成一条固定消息。
完整构建、证明、运行、CI 与存储成本未知；候选正式检查和原生验收另核。

独立 Verus 根直接读取生产源码，声明成功、失败和边界预期。Cargo 消费同一源码并擦除 ghost；
Rust 测试不能代替证明。其他 runner 逻辑、调用适配器与完整 SPEC 对应仍未证明。
把错误长度的 OID 改为接受，会直接违反返回值后置条件（6 verified / 1 error）；
正确源码按 SHA-256 恢复，再运行正式检查。

实际启动准备判定也使用同源 `verified_consumption::matches_preparation`，证明命令、宿主配置、
唯一先行 verifier、执行工具和项目登记必须精确对应。消费关系判官读取显式 preparation，
再核原共享 core 合同；解码和外层组合仍待证明。独立根与 Rust 回归核缺前置、重复、
把校验放入普通操作、工具／schema／命令／项目错绑及缺文档。registration 已登记三项整 crate 证明。

## 协议状态与 finding

实际 `wire::Status` 也归 `verified_status.rs`，保持 pass／warn／fail／error 的外部编码。
身份比较、`pass < warn < fail < error` 的完整顺序、克隆和退出码 0／0／1／2 均由同源 Verus 证明；
两两汇总也直接调用同源 `WireStatus::maximum`，证明结果精确等于两个实际输入中严重程度较高的变体。
full 执行、留存审计、初始库存、projects 留存汇总、workflow 证书汇总及 finding 记录均直接使用这一实现，
替换原标准库 `Ord::max` 调用；不接收调用方计算的等级或有效标志。
独立 Verus 预期逐项列出全部 16 对结果，并核任意三种状态下的幂等、交换与结合性质。
复用原两个状态整 crate 证明、FILEMAP 和实际使用方，不新增项目、操作、依赖、外部假设或预算。
每次合并只比较两个四态值，不分配或增加进程；完整构建、证明、运行、CI 与存储成本未知。
此处仅证明二元合并，外层列表遍历、缺失／失败分类、JSON 与完整执行状态仍待证明。
真实 `Finding` 的克隆保留 code、level、message 及两个引用数组的完整字符串、顺序和重复。
`wire::validate_response_at` 经上述前置校验、`full::validate_execution` 经同源留存审计组合消费同一 `findings_match_status`，
只读取原 finding 的完整 level 字段：
fail／error 当且仅当存在 `error`，pass 不允许 `warning`，warn 必须存在 `warning`。
该循环证明完整列表的存在量词及终止，不由调用方提供预先计算的标志。

独立根列出全部 16 对状态比较、四个退出码及全部 error／warning 存在组合，
另核空列表、info、排列和重复、大小写／Unicode／NUL 级别、其他字段含误导文字及完整克隆。
现有协议进程测试继续核四态真实 JSON、退出码与错误状态拒绝。
finding 字段形状、引用允许条件和响应头的局部证明见上文；
登记记录指针的语法／目标归属、证据 IO、JSON 解码和外层组合仍待证明。
状态判断为常量工作；finding 扫描为 O(F) 条记录，使用常量额外状态，不增加进程。
这是当前算法估计；完整运行、构建、验证、CI 和存储成本未测。

## 实际图内核

图内核的字节词典序、完整比较和严格性／传递性／非对称性／三歧性证明，
统一归 `chrono-identifiers`；`verified_graph.rs` 直接重导出同源实现，
原图使用方和独立边界预期继续读取它。比较逐字节保留空串、前缀、NUL 和任意字节的顺序，
最多读取两个输入的公共长度，使用常量额外状态，不新增运行时进程。
这是现有 Verus 实现与证明的归属调整，没有新增算法或受信接口。
共享定理显式给出尾序列长度；前驱链证明保持原查找合同，分清末边与前缀递归，
使同一完整证明在原资源限制内通过，不增加证明预算或函数过滤。
FILEMAP 显式连接共用源码与图生产、专属测试及执行节点；复用原整 crate 操作和固定限制。
完整构建、证明、CI 与存储成本未测；此调整不证明真实 Git 树的完整 DELTA。

`crates/judge-filemap/src/verified_graph.rs` 是实际有类型边比较、两端边并集、名称索引、
FIFO 遍历、稀疏前驱树、消费者行投影和原因集合的唯一实现。
`graph::union`、`graph::closure` 和原因链查询直接消费这些实现；Cargo 擦除同一源码的 ghost。

证明保留完整 UTF-8 字节身份、边类型、两端来源、每个槽位的目标及 seed 根索引。
FIFO 首次发现生成确定的最短跳数和前驱；传播行逐槽对应原名称和完整出边，包含平行边、环和自环。
返回名称恰好存在有限根路径，稀疏前驱集合精确等于首次发现槽。
前驱树按名称字节严格有序；原因链查询终止，缺失、错目标、环及自环有限失败。

节点及遍历边的原因状态使用 `CauseSet`。其私有字符串向量按完整 UTF-8 字节严格有序，
二分定位、成员关系、精确新增／重复信号、集合和长度更新、首项、克隆及归一化均有证明。
重复插入保持原序列；归一化保全输入的精确字节键集合。JSON 仍使用有序字符串数组，
反序列化后调用已证明的归一化。没有新增仓内受信字符串排序接口。

逐 seed 调用使用同源 `SeedPropagations` 流式状态。每次成功恰推进一个位置，
保持原 seed 的 ID、node、reference、reason，传播行和最短前驱对应同一位置的拓扑根；
只有全部位置消费后返回 `None`，重复终态不再推进。实际闭包直接消费该状态，移除外层 `zip`。
独立根核空输入、重复 seed／ID、不同根、完整 Unicode／NUL 字段；根数量不匹配不满足构造前置。
状态只保留拓扑／seed 借用和一个游标，不同时留存所有 seed 的传播行；完整成本未测。

实际 `TraversedEdge` 也归同一 Verus 根，克隆保留完整边键和原因集合。
`selected_test_targets` 只从这些原记录的 `TestExecution` 边选择目标，
输出按完整 UTF-8 字节严格排序，集合恰等于全部执行边目标，重复记录和重复目标不增加成员。
空输入以及仅含 compile／build-input／runtime-input／judge-trigger 的输入均不选测试；
名称内容、源节点名称和原因集合不改变边类型判断。循环有终止证明。
`Closure::selected_tests` 直接转发此结果，再用原标准库转换接口返回 `BTreeSet<String>`。
独立根核全部边类型、空输入、重复目标、完整 Unicode／NUL 名称与记录克隆；
现有 Rust 消费者回归保留原返回类型、JSON 与选测预期。

实际边原因状态通过同源 `merge_traversed_causes` 更新原 `BTreeMap<Edge, CauseSet>`。
其键集合精确等于原键和本次实际出边的并集；每个完整有类型边的原因恰为原原因加上本次起点 ID，
不在出边中的记录保留原原因序列。重复边、重复起点 ID 不增加成员，空出边不改变状态，循环终止。
`graph::closure` 的真实内层更新直接调用该函数，没有另造边表或预计算是否命中的标志。
独立 Verus 预期覆盖已有／新增／缺失／无关边、重复槽位和 ID、全部五种边类型、
同起终点自环、Unicode 规范化／大小写／NUL／前缀差异及空起点 ID。
复用原图生产／独立整 crate 根和 FILEMAP 的真实消费边；本地及原生验收仍待完成。
固定 vstd 尚无此处 `entry` 的可用规格，因此每个槽位使用原 map 的 `remove`／`insert`，
每次两项有序 map 操作并保留原完整边克隆，不复制已有原因集合；完整性能、构建、CI 和存储成本未知。

实际诊断的两处 seed-ID 查找直接消费同源 `seed_by_id`：按完整 UTF-8 字节读取原 ID，
有匹配时借用输入顺序中的首个完整记录，无匹配时返回 `None`，不提供默认起点。
证明保全同一记录的 ID、node、reference、reason，且扫描有限终止；空 ID 仍可匹配。
独立根核空输入、首中末项、缺失、重复 ID、Unicode／NUL、前缀／大小写／规范化差异，
另以任意字符串证明重复 ID 的首项语义。生产者是否为每个原因提供有效 ID、
诊断调用方的非空断言与原因链组合继续保留未完成义务。
查找最多比较 S 条原记录的完整 ID、使用常量额外状态，不分配或复制起点。
复用图原有两项整 crate 证明，不增加操作、依赖或外部接口；完整成本未测。

外层节点原因关联、seed-ID 生产及关联完整性、完整遍历／输出组合、JSON／诊断适配、
外层额外选测和完整 SPEC 对应仍未证明。局部图证明不证明实际登记完整，也不代表全仓覆盖完成。

## 执行资源上限

`crates/runner/src/verified_limits.rs` 负责执行资源准入的唯一判断：超时为正 `u64`，
输出为 1 至 67,108,864 字节（64 MiB，含两端）的 `usize`。
`valid_execution_bounds` 和单独的 `valid_output_limit` 分别证明返回值与规格充要相等。
实际执行器的命令检查、不可变输入传输、registration 的默认／逐操作上限和 routes 的计划检查直接调用它们；
使用真实字段类型，无中间数组、整数转换或新增外部接口假设。
生产根覆盖任意类型有效的参数；独立 Verus 根直接调用实现，核零值、最小值、最大值、越界一字节、
`u64::MAX` 超时与 `usize::MAX` 输出。原 Rust 消费者测试仍负责外围计划和进程行为。
同一边界预期在普通 Cargo 中调用 runner 的实际导出并执行失败分支；Verus 同时证明预期成立。
runner、registration 和 routes 的登记计划把两项整 crate 证明放在原构建和测试之前。
判断使用常量时间与额外空间，不增加运行时进程；构建、验证、CI 和存储的实际成本未测。
外层解析、计划组合、启动状态和完整 SPEC 对应仍待证明。

## CI 离线汇总的结果集合

现役 `judge-ci` 的执行结果转换和终态也由同一证明根拥有。
`operation_result` 精确保留原操作 ID 与错误文本，只把完整 `passed` 映射为 Passed、
完整 `blocked` 映射为 Blocked，其余状态全部映射为 Failed。
原错误缺失时采用既有默认原因；空错误和 Unicode／NUL 错误均原样保留。
有原退出时，Passed 保留任何退出码，其余状态只保留非零退出；
零退出后的回执／基础设施失败不伪造成非零子进程退出，原回执继续保留真实零值。
来源是当前 projects 原结果的直接字段转发；projects 的状态生产、回执准入与 JSON 适配仍待证明。

`response_status` 直接读取完整真实 `CheckResult` 列表：返回 Failed 当且仅当存在 Failed 或 Blocked，
否则返回 Passed，包含空列表；ID、原因、退出码及重复不参与该终态规则。
本地／单元执行和离线 collection 共用同一实现，循环证明完整遍历和终止。
库存异常由 `inventory_failure` 同时生成原原因的唯一失败记录和失败状态；
诊断发布失败由 `diagnostic_failure` 追加真实失败记录并返回失败，保全已有全部记录及顺序。
独立 Verus 根核四态、未知／大小写／NUL 状态、缺失／零／正负及整数端点退出、
默认／空／Unicode 错误、首尾失败、重复与空列表，以及两类失败状态与实际记录的对应。
复用现有 CI 两项整 crate 操作，不增加证明进程、依赖或外部假设。
终态最多遍历 N 条记录、使用常量额外状态；完整成本未知，不据此主张全仓覆盖门完成。

`crates/judge-ci/src/verified_collection.rs` 是离线汇总实际消费的结果 ID 集合比较。
期望列表由同一证明根构造，精确包含 `ci.inventory`、登记操作和未要求测试。
返回真当且仅当实际 ID 与此期望列表互相包含；构造和两个方向的比较循环均证明终止，
空集合、排列、缺项、额外项、缺库存标记及完整 Unicode／NUL 身份由同源独立根核对。
集合含义不依赖顺序或重复次数；响应协议继续单独拒绝重复结果 ID。
实际汇总直接消费同一证明根对真实 `CheckResult` 的状态判断：操作成功须有同一条精确 ID、
`Passed` 和 `Some(0)` 的记录；库存标记须是 `Passed`／`None`，所有未要求结果须为 `NotRequired`。
未要求状态不额外限制退出码，完整 ID 覆盖仍由集合合同承担；各遍历均证明终止。
独立 Verus 根核字段分散在不同结果时拒绝、四种状态、缺失／错误退出、空结果及逐字段克隆。
实际结果 ID 的借用提取也由同一根证明：输出逐项等于真实记录的 ID，保留顺序、重复、完整 Unicode 与 NUL，
集合合同直接关联原报告字段；调用方不再提供结果 ID 投影。
登记操作和未要求列表的字符串借用同样由 `borrow_texts` 证明等长且逐项相等，
实际调用方直接消费；独立根核顺序、重复、Unicode、NUL 和空列表，并组合验证真实报告覆盖。
未要求列表直接保留原有序唯一 `BTreeMap` 键遍历的向量，不再重复构造同序集合；JSON 数组合同保持。
原报告、流摘要、JSON 解码、登记操作／未要求列表的外层提取和组合仍有各自未完成义务。

比较使用借用字符串向量，不再为集合比较克隆和排序结果 ID。
实际与期望长度为 A、E，最长键为 L 字节时，最坏比较成本为 O(AEL)，
借用向量占 O(A+E)；完整运行、构建、验证、CI 和存储成本未测，不宣称性能收益。

## 工作树清理选择与结果阶段

`crates/worktree/src/verified_lifecycle.rs` 直接供自动清理所有者调用：终态进入清理，活跃记录仅在有缓存责任时进入，等待整树回收的保留项仅由显式维护重试；指定 artifacts-only 的已完成处置不重试。证据阻挡要求实际存在且未经释放。清理原结果失败时记失败，缓存成功仅记缓存释放，整树成功还须有 `verified-absent` 或 `already-absent` 才记移除。

同一源码定义产物处置时机：普通释放可选择 dispose，finish-dispose 在独立工作树明确 finish、
主检出明确释放当前代际或实际检出处置时才可回收；复用年龄本身不释放缓存。
retain 不选，evidence-retain 只由证据权限选择。主检出释放请求须与当前缓存代际相等，
过去或未来代际不能授权当前删除。独立预期覆盖普通／完成／证据权限组合、无请求、
旧／未来代际与 u64 最大值。真实回归另核跨操作保留、主检出失败后恢复、新消费、
独立工作树 finish、连续年龄维护保留缓存及过龄 finish 释放缓存；JSON、原件引用和物理删除仍属适配器验收范围。

分支采用的待办判断也由同一源码负责：取得原独占租约后，仅活跃且没有终态、未交接 use 和待交接结果的记录可以采用分支。上次清理的进度不参与这个判断，也不被迁移改写。独立预期核每项阻断条件及状态全文边界；真实生命周期回归核等待租约和缓存回收后复用同一检出，并保留清理原件及缓存历史。

整体临时分配的回收另要求已声明所有权、保留窗口届满、实际消费者释放和独占内核凭据四项同时成立。
成功、失败／诊断和中断的窗口选择也由此同源根负责；中断过期不授予任务成功状态。
独立边界分别核每项缺失及三个窗口，真实目录回归核活读取者、失败原件、复制输入、外部链接和元数据交接恢复。

`verify.worktree-lifecycle`／`verify.worktree-lifecycle-tests` 使用现役固定版本和严格参数验证整 crate。独立边界包含活跃、未知、已处置记录不获工作终态，证据存在／缺失／释放，以及失败或缺少移除结果不能声明整树移除。现有分支的显式 import 与新建 birth 的分支规则由同一源码区分：import 不要求创建前缀，birth 仍要求已登记前缀，未知活动拒绝。已登记 import 的自动清理消费原 enrollment kind 和身份 guard；直接 cleanup 的前缀边界另有同源独立预期。普通 Cargo 同时执行这些独立预期和真实 Git 行为回归。文件存在性、JSON、内核租约、Git 和物理删除仍由原适配器负责，这些判断不证明完整事务及全部历史状态已形式化。

## 本地工作树提醒

`crates/worktree/src/verified_attention.rs` 是本地提醒分类的唯一判断实现。`automatic.rs` 把现役账本状态、终态交接有无、可读性、Git 登记、物理路径及已观察附件匹配交给该函数，并直接消费分类与提醒判断。正常协调者无需终结；任务 active、终态清理待办、retained、未登记和 unknown 均需关注。只有 disposed、已有终态交接且两处都已缺失才取消清理提醒；单凭 disposed 字符串不够。

生产根及独立边界根通过登记的 `verify.worktree-attention`／`verify.worktree-attention-tests` 整 crate 证明并编译，版本及参数归 projects.json。独立边界核活跃／保留／待清理、只缺一处、缺终态、未知附件和协调者；Cargo 行为测试继续消费同一源码，核真实跨命令状态、原报告一致、损坏项隔离与原件保全。OS／Git 观察、JSON 编解码、namespace 枚举及已有账本读取是外部接口；此局部分类证明不证明它们的事务性、完整迁移或全仓状态逻辑。

创建 agent 的提醒选择也由 `verified_attention.rs` 同源执行：只有相同、非空的创建者身份且分类结果仍需关注的对象才进入该 agent 的清单。独立预期覆盖同一 agent、其他 agent、空身份及已处置对象；配置读取、身份来源、原回执绑定和提醒传递仍由 Rust 适配器与真实 Git 回归负责。

终态历史到期选择同源判断已处置、使用方释放、实际确认时刻与当前时刻、保留时长及容量条件。未知时刻、未来时刻、活跃使用方和未到期记录受保护；独立预期覆盖到期边界、容量触发和 u64 最大值。原回执、物理／Git 移除状态及租约仍由真实适配器重验，不能用时间选择代替这些核验。

## 所有者可执行文件发布

`crates/worktree/src/verified_owner_publication.rs` 是实际发布身份比较、原件保全、意图读取／准备、
所选文件替换／重入和最终绑定检查的唯一判断实现。绑定包含 inode 身份、SHA-256 和长度；
完整阶段还绑定目标路径、保留阶段路径和旧文件绑定。发布意图要求精确 schema 和完整阶段绑定。
阶段原报告摘要由外层 Rust 所有者核验，不属于此局部证明。不可变原件按完整字节比较，保留 NUL 和非 UTF-8 字节身份。

原件已有时保留，随后仍核其绑定；缺失时只允许从精确旧绑定创建。无意图却有准备文件时拒绝，
有意图则读取原件；所选文件必须是精确旧绑定或精确已发布绑定。两绑定相等时先执行旧绑定分支。
文件观察核 inode、长度、修改秒及纳秒全部不变；保留路径仅证明登记的字节前缀。
全部执行结果与规格充要相等，独立根逐字段核身份／内容／长度漂移、原件及意图有无、重入、
错误 schema、内容摘要漂移、完整阶段错绑、任意字节和整数边界。

Rust 所有者在原准入和全部受影响登记的独占租约内直接消费这些判断，再执行硬链接、同步和原子替换。
阶段报告的逐值匹配、操作名非空／唯一、原流地址唯一、退出码／PID、64 位小写摘要及执行物路径／搬迁许可也由同一根判断。
JSON 适配器分别提供原报告值和由当前声明／实际观察产生的预期值；有限检查序列返回首个失败，证明其前缀通过及无失败当且仅当全部条件成立。
操作登记歧义只核实际阶段消费者，名称二次比较限于所选操作及原流，不把无关项目的全局名称检查引入阶段发布。

同一证明根也负责共享生命周期命名空间的比较：所有权协议、物理协调者地址和状态目录须全部相同。
独立边界根核相同值，以及三项分别改变时的拒绝；普通 Rust 也执行相同的失败分支。
每个工作树的完整 cleanup／worktree 文件、清理选择和保留条件不属于共享命名空间，分别绑定其已提交输入。
实际 Git／文件读取、迁移回执、租约和处置仍由 Rust 行为回归验证，这项局部比较证明不代表完整迁移事务已证明。
原 Rust 行为测试继续验证真实文件系统、失败恢复及租约排除。Serde 派生及显式字段投影只承担明确的编码接口，
JSON 解码、哈希、metadata／inode 观察、路径解析、Git、文件系统操作和外层部署／采用组合不在此局部证明内；
不据判断证明声称整个文件系统事务已证明。

## 真实 Verus 结果准入

`crates/runner/src/verified_verifier.rs` 拥有真实 `OutputContract`、类型化 `CompilerReport`、
合同字段校验、完整参数形状与结果准入。action／Method 显式登记可选 `output_contract`，
其 version／commit 由计划摘要、回执 Method 摘要和两端操作比较一并绑定；未按名称推断证明操作。
仓内选择、字段比较、失败返回及遍历都直接使用同源 Verus，独立边界预期也直接使用 Verus。

结果准入与源码绑定的专属测试在普通 Cargo 中调用 runner 的实际公开接口。
布尔预期不符时执行失败分支；字节结果和原 argv 文本分别复用已证明的
`equal_bytes` 与 `equal_text`。独立 Verus 根保留序列证明步骤，并证明同一运行预期成立。
被 Cargo 擦除的证明断言不代替运行检查；这些检查也不建立普通运行库与 Verus 输出库的完整编译身份。

routes 原回执身份及 stdout 字节摘要通过后，只对已声明合同的原 stdout 作一次类型化解码。
准入要求同一进程退出 0 且无失败，精确保留完整 crate、两项强制参数、JSON、编译、固定线程和 checksum 参数，
link／dep-info／`-o` 输出同源，源码和输出参数非空。额外参数和函数过滤均拒绝。
同一份报告必须同时满足固定版本／完整 commit、success、两类错误标记为假、零 errors、
至少一项 verified 和整个 crate；缺失或错型字段不能从另一份报告补齐。
普通操作向解码接口传空字节，不解析其业务输出；其既有退出规则由原所有者保留。
本地 projects 和离线 collection 都消费 routes 的同一准入结果。

本地 judge-ci 和离线 collection 还共用 `bind_operation_output` 的组合准入。
对声明为 passed 的普通操作，`Ok(None)` 与 `ordinary_output_accepted` 充要相等：
必须有唯一登记方法、匹配的完整原操作和参数、退出 0 且无失败的原进程，并且没有编译库或依赖观察。
这个正向成功合同不依赖报告解码成功；独立预期保留无效 UTF-8／NUL 业务 stdout，核合法普通操作仍成功且不产生证明条目。
唯一方法的独立正例显式给出两种完整名称的不同位置和存在见证，原大小写、后缀与重复拒绝预期保持。

本宿主为各项证明操作逐项登记固定 Verus 版本及 commit；报告准入的两项整 crate 根归
runner 所有者计划。同次 DELTA 汇总核受影响证明及 registration、routes、judge-ci 和 CI 生成器的实际消费测试。
未执行的证明、未声明合同的普通操作及现有原件身份核验，不因此取得全仓覆盖保证。
真实编译依赖的候选绑定见下一节；完整 SPEC 对应及全仓新增逻辑覆盖门仍须继续实施。
新增检查为固定形状比较和原报告解码，完整构建、验证、CI、存储及运行成本未测。

## 原编译依赖与固定候选

`verified_sources.rs` 拥有原 dep-info 观察准入、路径解析／词法规范化、完整内容比较和实际源码绑定。
routes 在原进程完成后取得规范输出参数对应的 `.d` 原字节或原获取错误，先放进同一回执再计算回执摘要。
证明操作必须同时满足原报告合同和有界、成功的原依赖观察；普通操作必须没有该观察。
长度限为原操作登记的输出限额，必须为 1 至 64 MiB；获取只读上限加一字节以区分恰好和超限。
原进程失败、过滤参数、缺失／失败／超限观察不能成为通过，原进程退出码和原报告仍保留。

同一回执还保存实际编译库的 `compiler_artifact`：规范 `-o` 路径的完整 UTF-8 字节、
SHA-256 和实际长度。只有成功原进程与规范参数才读取该文件；路径、摘要格式、非空长度和
观察有无的准入由 `verified_verifier.rs` 负责。缺失、读取失败、空库、错误路径或无效摘要均拒绝；
普通操作必须没有库观察。身份复制也由同源实现证明保留全部原字节及长度，包括尚未准入的无效观察。
routes 复用现役 `no_symlink_parents`／`file_identity`，
拒绝链接并以原 64 KiB 缓冲区流式计算身份，不把库内容复制进报告，也不把 stdout 限额当作库大小限额。
文件系统和 SHA-256 接口提供实际普通文件身份；此处不证明密码学算法，也不保证未声明的并发写入。
库观察和原依赖观察在同一次回执摘要中封存，本地与离线汇总消费同一准入。
离线消费不要求库仍在原路径，不以之后的文件替换原成功或失败观察。

独立 Verus 预期覆盖成功、缺失／失败观察、空库、错误路径、摘要长度与非法字符、普通操作、
非规范参数及失败原进程，另证明任何准入回执都含同一输出路径的非空库身份。
真实外部接口夹具核实际 IO、超出 stdout 限额的库、回执往返、链接拒绝及原失败保全；
夹具报告本身不代表 Verus 证明。复用原结果的两项整 crate 证明、FILEMAP 和实际消费计划，
不新增证明操作、运行时进程、共享状态或政策门。每次证明增加一次实际库读取及常量大小身份字段，
完整构建、证明、CI、存储和运行成本未测。
**产品与宿主政策同时变更：**routes 专属测试补登记并实际使用写入式 Cargo fmt；
原格式检查和未过滤测试入口保留。实际证明继续使用原有登记，不增加证明根或工具依赖。

本地 judge-ci 与离线 collection 共用 `bind_operation_output`，由它调用 `bind_compiler_sources`。
入口读取现役计划的借用投影和同一原回执，按完整操作文本查找唯一方法；缺失或重复均拒绝。
实际 argv 去掉执行物后，必须逐项、按序等于该方法的参数，原回执操作名也必须相同，
原进程必须退出 0 且没有失败。报告、库和依赖先共同准入，再绑定固定候选源码。
对本次操作的每条已有文件声明，入口还要求声明根等于实际 invocation 的根，
且声明文件出现在已绑定的候选编译源码中。任一错绑返回完整 `E_VERUS_COVERAGE` 原因；
按文件及根的登记顺序保留首错，先判根错绑，再判该文件未参与编译。
同源错误类型保留已选原因和完整借用路径；渲染只转交标准库 `format!("{message}{path}")`，
沿用已有动态路径错误的边界，不增加信任假设。Verus 核错误字段和原参数转交，
普通 Rust 回归核完整渲染文本；标准格式化算法属于明确的外部接口。
只有明确声明证明合同、且全部准入成功，才返回含原类型化报告和源码绑定的 `ProofAdmission`。
普通成功、原失败或阻断只返回空条目，不能充当证明。构造器不接收自由的“已通过”布尔值或 token。
CI 的结果生产者直接接收此类型化结果；准入错误仍使原 passed 变为 Failed，
保留原进程的实际退出码与失败文本。离线汇总在原 routes 比较后消费同一入口。

`CompilerReportRead::decode` 是标准 serde_json 解码的明确外部接口；其观察模型 `decoded`
只关联同一输入字节及实际解码结果，不保证报告成功，成功与否仍由 Verus 判断。
适配器仅把计划及原回执字段交给入口，不负责方法选择、字段准入或错误分类。
routes 原比较继续保留原结果合同；统一入口另从相同原 stdout 解码，不沿用自由的成功标志。
两次解码的完整成本未测。每个单元只建立一份 O(M) 方法借用表；每个操作最多比较 M 个方法名，
另按实际参数与依赖字节计成本。没有新增证明操作、工具、进程、过滤或预算。

实际 argv 去掉执行物后须满足同源规范参数；依赖头须精确绑定实际输出路径。
每个编译器路径按 POSIX 分量规范化，拒绝绝对、控制字节、反斜杠和越过根的父分量；
保留空格及 UTF-8 字节，解码后的文本必须重新编码为原规范化字节。
源码必须逐项属于所选固定 FILEMAP 的 files，且固定候选树模式为 `100644` 或 `100755`。
固定 Git 候选的实际 blob 长度和 SHA-256 与原编译器 checksum 逐项核对；
保留每项原路径、顺序和 checksum；同一规范路径的别名仅在摘要及长度一致时接受，冲突拒绝。
实际根文件必须出现，缺失／额外／重排 checksum、错长度／摘要及未终止尾行均拒绝。
只允许 checksum 之前的空行及 `# env-dep:` 原行；此处不证明环境完整性。
成功后置条件关联全部候选条目、模式、明确成员和完整 dep-info；任何源码失败使原 passed 转为 Failed，
保留原错误文本，不把原零退出伪造为非零进程退出。

新增外部接口假设逐项限定如下；适配器只转发原输入／输出，仓内准入及状态转换由上述 Verus 根拥有。

| 接口 | 实际消费者与假设 |
| --- | --- |
| `DependencyRead::read` | routes 的 UTF-8 解码、`Path::join`、`File::open`、`Read::take`／`read_to_end`；返回指定原输出的字节或原 IO 错误，读取不超过请求上限 |
| `CandidateSourceRead::text` | judge-ci 的标准 UTF-8 解码；Verus 另核完整字节往返，不仅依赖解码成功 |
| `CandidateSourceRead::mode` | 同一固定请求树的完整路径查询和原模式字段投影；文件模式准入仍由 Verus 比较 |
| `CandidateSourceRead::blob` | 现役 request-bound `Reader::blob(host_root, candidate, path)` 的固定 Git 字节观察；其现有运输和外层实现仍有未证明义务 |
| `CandidateSourceRead::digest` | 现役 SHA-256 接口返回实际 blob 的十六进制摘要；Verus 核格式和完整 checksum，不证明密码学算法 |
| `CompilerReportRead::decode`／`decoded` | judge-ci 的固定 serde_json 类型化解码；只关联点名原字节与完整字段，不作成功判断或从其他报告补齐字段 |
| FILEMAP／回执编解码 | 固定 serde／serde_json 的字段解码及原字节编码；不授予外层仓内判断 `external_body` 例外 |

独立 Verus 预期覆盖解析、规范化、根缺失、一致／冲突别名、摘要／长度、完整 checksum、原报告与观察的组合、
上限两端、原失败、真实 argv、缺成功回执，以及源码失败后的终态／原因／退出码保全。
独立 Verus 预期另核声明根与实际根的完整身份、文件参与编译、空声明、无关操作、
UTF-8 路径、同文件的后续根和首错顺序。Rust 组合回归用真实 SHA-256、完整合成 dep-info
和类型化报告解码，验证正常证明成功，以及根错绑和未编译文件的两种覆盖拒绝。
统一入口的独立 Verus 预期另核方法的缺失、重复、大小写与 NUL 完整身份，
普通成功不产生证明、失败进程不能取得通过，以及任意实际证明条目的原报告、库、根与候选关联。
**产品与宿主政策同时变更：**FILEMAP 连接实际新适配器，并把原结果两项整 crate 根加到 projects 测试的必需前置；
projects 原生单元同步选择现有 Verus bootstrap，先核共享 core 再安装固定证明工具；
其余实际消费者复用原登记证明。候选正式检查与原生 CI 尚须各自验收。
每项证明增加一份有界原观察；每次源码绑定按实际编译依赖读取固定候选字节，复用现役 Reader，
完整运行、构建、验证、CI 和存储成本未知。
**产品与宿主政策同改：**FILEMAP 移除 `ci/src/release.rs`、`ci/src/gating.rs`，以及
`release/native/src/publication.rs`、`release/native/src/identity.rs` 的不实文件级证明声明。
它们调用了已证明辅助函数，自身未进入所声明根的编译依赖；调用辅助函数不能证明整个调用文件。
原编译／测试依赖、辅助函数证明和行为测试保留，四个文件恢复未知覆盖，外围逻辑仍须迁移和验收。
没有把它们改成 data，也没有据此宣称完整覆盖。新绑定沿现有证明、组合入口和 DELTA 检查验收；
按实际登记及编译依赖增加比较，总构建、验证、CI 和运行成本未测。

此处只绑定已经声明的证明调用、所声明文件的编译参与及编译库身份；文件到证明根的完整登记、编译产物与实际生产调用的完整对应、
其余 SPEC 与逻辑及 `E_VERUS_COVERAGE` 全仓覆盖门仍未完成，Python 迁移继续暂缓。

## 实际节点与类型关系

`crates/judge-registration/src/verified_nodes.rs` 拥有实际八种 `NodeKind`，
复制及相等的返回值与变体身份充要相等；自然所有者只重导出这个类型。
filemap 的原 `valid_types` 已移到现役 `verified_graph.rs`，直接消费该同源类型，
按五种边与八种起点／终点的完整关系准入。规格按终点定义关系，实现按边派发；
独立固定表用40行目标位图逐项核全部320种组合，含43项允许与277项拒绝。
节点独立根另核八种变体克隆及全部64对相等／不等，不通过另一套整数类型代替真实枚举。

**产品与宿主政策同时变更：**新节点生产／独立根进入 registration 实际计划的必需前置；
图生产／独立根直接编译同一节点源，FILEMAP 逐项登记这四个消费者。
新增逻辑、规格与独立预期均直接使用 Verus；未改 Python、固定工具、验证过滤或预算。
`NodeKind` 的 `Copy` 是无状态值复制，`Debug` 派生仅保留原诊断名；后者明确采用
固定 Rust 1.95.0 标准派生接口，不产生仓内判断。Clone／PartialEq 由仓内同源证明拥有，未豁免。
JSON 解码、节点提取、诊断和完整图外层组合继续保留未完成义务。
额外证明、编译、运行、CI与存储总成本未测；不据此声明全仓覆盖门已完成。

## 工作流来源分支准入

`crates/judge-workflow/src/verified_branch.rs` 拥有来源分支类型的唯一准入判断。
它要求 feature／integration 前缀恰好命中一个，且来源分支不能是目标分支；结果为
`Feature`、`Integration` 或拒绝。`branch.rs` 只负责读取登记前缀、判断字符串前缀是否命中，
再直接消费该 Verus 函数并把枚举映射为现有 JSON 的 `feature`／`integration` 文本。
因此前缀字符串和 Git ref 校验仍是明确适配器，仓内来源类型选择由已证明谓词决定。

同一证明根还拥有提交落后数的边界判断：`commit_count_within_limit` 只在实际落后数不超过
登记上限时返回通过。`branch.rs` 负责把已验证的 JSON 浮点上限正规化为无符号计数，随后直接
消费该 Verus 结果；等值通过、超一拒绝和 `usize` 最大值边界由独立预期覆盖。年龄判断同样由
`age_within_limit` 拥有：`branch.rs` 将非负 `Duration` 纳秒和 JSON 浮点阈值正规化为有界无符号
纳秒值后直接消费该 Verus 结果；零、等值、超一纳秒和 `u64` 最大值边界由独立预期覆盖。时间
解析、浮点正规化、Git 历史遍历和 JSON 解码仍是明确适配器。

生产根和独立边界根分别登记为 `verify.judge-workflow` 与
`verify.judge-workflow-tests`，由 workflow 的真实 branch／consumer 测试计划共同验收。
独立预期覆盖单一前缀、零／双前缀、目标分支、提交落后数和时间新鲜度边界；本批仍不证明
Git 历史、JSON 解码和完整 workflow 事务。

### 工作流执行证据准入

`crates/judge-workflow/src/verified_evidence.rs` 拥有 `record` 的唯一组合准入判断。
JSON 解码器和进程原件适配器先提供执行状态、judge／request 身份、请求摘要、零退出码、
协议状态和 stdout 解码相等性七项事实，再由 `admit_record` 统一判断；`record` 只消费
这个 Verus 结果并保留原错误。协议状态的严重度排序继续复用 runner 的已证明
`verified_status.rs`，解码、原件读取和摘要计算仍是明确外部接口。

独立 `verified_evidence.rs` 预期覆盖全通过、每一项单独缺失、全拒绝及单项边界；生产根
与测试根加入现有 `verify.judge-workflow`／`verify.judge-workflow-tests` 整 crate 计划。
本批不把 JSON 语义、进程字节真实性或完整 workflow 事务宣称为已证明。

### 工作流 schema migration 准入

`crates/judge-workflow/src/verified_transition.rs` 拥有 schema migration 的唯一准入谓词
`migration_admissible`。它要求恰有一条声明；无 `script` 时必须没有实际 conversion；有
`script` 时同时要求 replacement test 通过、conversion 存在、声明的 script／test 与实际
输入一致、进程成功、输入／输出摘要一致、解码后的输出等于实际 stdout，且 mappings 与声明
一致。`admit_declaration` 直接读取实际筛选结果的数量，先拒绝零条、多条及数量上界；
`admit_migration` 读取同一数量及 `MigrationFacts`，拥有准入、五类拒绝原因及其诊断文本。
script／test 和两份输入摘要的条件组合，以及缺少回执、摘要计算失败、stdout 解码失败的
拒绝判断都在该证明根内。适配器只转发已证明的结果，不再另行选择 migration 错误分类。

生产根和独立边界根分别登记为 `verify.judge-workflow-transition` 与
`verify.judge-workflow-transition-tests`，并加入 `test:judge-workflow-tests` 的正式执行计划。
独立预期覆盖无 script／有 conversion、零／一／多条／`usize::MAX` 声明、各项事实缺失及
错误优先级；同一预期在普通 Rust 中通过真实分支和 panic 检查返回分类，不依赖被擦除的
证明断言。Rust `consumer` 测试继续核实际 migration 消费。

`transition.rs` 的匹配记录筛选、JSON 字段读取及结构比较、`results.tests` 状态观察、
ProcessResult 解码、既有进程回执校验、摘要计算和 stdout JSON 解码仍属于外围未证明范围。
这些实际观察分别进入事实字段，失败的摘要／解码保留为 `None`；标准 `Option`／`Result`
转发和 JSON／进程接口不授予仓内准入豁免。FILEMAP 只把新生产根及独立预期列为
`verification`，不把整份 `transition.rs` 宣称为已证明。本批不宣称完整 workflow 或全仓
`E_VERUS_COVERAGE` 已完成；构建、证明、CI 与存储总成本未测。

## 专属测试的语言配对

projects 的实际 `dedicated_pair` 直接把登记中的 implementation／testing 字符串借给
`verified_languages.rs`。同源定义与实现要求完整文本相等，或恰为 implementation `shell`、
testing `python`；不归一化、不从扩展名推断，也不扩大 Python 例外。
独立根证明任意相等文本接受、其余不同文本拒绝，并列明反向配对、大小写、前后缀、
空格、NUL、Unicode 和空字符串边界。语言非空仍由既有 schema 负责。
原 Rust 语言／metadata DELTA 回归继续验证实际 projects 使用方。

**产品与宿主政策同时变更：**两项固定工具的整 crate 证明加入 projects 原测试计划的必需前置，
源码和独立预期逐文件登记，并复用 runner 的已证明文本相等判断及现役 Verus 启动路径。
外部接口仅沿用固定 vstd 的字符串／字节接口；没有新增依赖、仓内受信实现或跳过证明的分支。
配对身份、错误分类、JSON、完整调用组合和全仓覆盖门仍待证明；本批不建立源码语言完整性。
运行时只比较实际字符串；新增两项证明的完整构建、CI、存储成本未知。

## full 准备状态

`judge-registration::readiness` 的五份登记状态与输入闭合判断由
`verified_readiness.rs` 负责。实际 Serde `as_str`／`as_array` 观察直接借用给同源函数：
只有精确的 `active` 接纳登记；只有 `enabled`、`declared-complete` 和存在的空数组
同时满足时，输入准备判断才通过。缺值、错型、文本不符或非空未解决项均拒绝。
数组元素类型保持泛型，不把 JSON 元素再映射成另一份状态。

端点快照绑定也由同一生产根判断：存在快照 schema、任一文件输入，或严格环境下
存在继承变量时，必须提供与所选端点全文精确相等的 commit。没有这些义务的旧格式
保留原有可选绑定语义；不额外增加 OID 形状或非空限制。实际快照消费者直接调用该判断。
独立根核三个单独前提、缺值、数量上界、空文本、前缀、NUL、Unicode 规范化差异，
并对任意文本和数量证明绑定条件。复用现有 registration 整 crate 前置，没有新增验证入口。

**产品与宿主政策同时变更：**生产调用替换原有普通 Rust 比较；三项整 crate 证明作为
registration 专属计划的必需前置，独立预期和 Rust 测试编译同一生产源。
沿用固定 Verus、完整验证参数和原错误码 `E_ACTIVATION`／`E_EVIDENCE_UNRESOLVED`。
新增依赖为零，验证与存储总成本未测；JSON 解析、错误记录、其余准备判断及 full 启用仍待完成。
本节描述实现范围，成功验收须消费实际证明和真实 registration 进程原结果。

## 固定执行入口

宿主 bootstrap 的 core、Verus 准备及所有者阶段已采用薄 shell 入口和 Rust 生产者；
共享安装的编排也使用 Rust；实际生产、安装和后续使用仍须分别验收。
`.chrono-harness/ci/native/src/verified_bindings.rs` 拥有工具绑定来源、摘要形状、
字节与版本比较、可重定位路径、重复旧路径退役、继承编译器和两个 wrapper 的判断，
以及环境名／值、宿主路径、共享目录排斥、工具探测分支和进程退出状态。
版本前缀另要求完整空格分隔；Cargo 可接受精确版本字符串，rustc 必须保留版本后的空格。
Cargo 不可用时选择 minimal 安装，缺少 rustfmt 时只补该组件；
退出状态保留原退出码，缺退出码时仅把正信号转换为负数，无有效终态时返回缺失。
同一根还拥有实际源码观察的四种状态与报告投影。任一观察失败优先为 unavailable，
字段改变为 changed，相同脏树为 dirty；只有相同 clean 观察向报告提供 commit／tree。
原 before／after 字段在报告中保持不变；比较保留可选字符串的完整字节身份。
独立根按成功、缺输入、冲突和边界声明预期；生产根和独立根都登记整 crate 证明。
普通 Rust 适配已直接调用这些判断来解析声明、查询 Rustup 和观察真实工具；
环境适配先解析完整选择，再允许安装，不修改全局环境或默认工具链；
操作执行复用 registration 的方法读取器和 runner 的摘要／描述符转交接口，
实时保存并转交两条原字节流，等待原始子进程和两个转交任务结束后返回。
Git 适配只观察当前检出；失败保留已有字段和错误，不执行历史程序。
JSON 解码、字段提取、环境和文件系统操作、Git／进程 IO，以及完整启动组合仍未证明。
Rust 回归另核真实 Cargo／rustc 的身份、环境选择、安装器退出、
非 UTF-8 两流在子进程结束前的转交和原件摘要，以及实际 Git clean／dirty／changed／unavailable 观察。
这些回归不替代证明，也不证明共享安装、冷启动和发布调用方的实际使用已验收。

原生配置预检只接受当前注册表绑定路径；所选操作必须存在且唯一，前置工具须显式配置。
同源根证明操作名非空／唯一、复用名属于原操作集合、当前 schema、所有者模式与工具角色判断。
构建完成判断要求原操作及安装顺序精确相同、每项都有零退出码、SDK 已选择，且 owner stage 有无符合配置；
只有完整且没有失败的构建可产生 passed。独立根核重复、空名、缺项、重排、失败退出、缺安装和错误 stage。
`profile.rs` 复用 registration 读取器，`construction.rs` 直接消费上述判断，
按前置→SDK→原操作→安装的顺序组合现有环境与执行适配，再保存原始报告。
原生进程夹具验证安装产物的实际消费、失败前置不启动 SDK、不覆盖旧成功报告、
缺执行／安装不发布成功，以及报告 IO 失败时仍保留原安装器退出码。
临时目录复用本仓已有版本的 tempfile，原目录回收仍归宿主生命周期所有者；
没有增加独立清理账本或调度协议。JSON、文件与进程 IO、报告字段投影和完整外层组合仍未证明，
这些模块已接通 core、Verus 准备、所有者阶段及共享安装的登记入口；实际使用验收仍须核对各自原件。
新增构建、验证、CI 和存储成本未测。

原生 `original.rs` 校验固定 producer 的原始源码、登记、SDK 选择、操作和两条原字节流。
候选 Git 只按完整且非零的 commit／tree 读取固定对象，先核实际 tree 和普通 blob 类型；
不执行旧 bootstrap、判官或 SDK，不以当前工作树文件替代固定输入。
固定旧 Python producer 原件按其声明校验额外的 owner／shared 模块摘要；
当前 Rust producer 不因此增加旧入口或解释器的执行依赖。读取固定配置不选择或创建其可写环境。
同源根证明原件头、精确源码观察、操作参数／cwd／PID／退出码和流摘要／长度的判断，
完整 OID 复用 runner 的既有规则，接受 SHA-1／SHA-256，拒绝全零、短值和非小写十六进制。
独立根核失败、导入、缺字段、观察漂移、参数变化和整数边界；
Rust 行为测试另核子进程报告、非 UTF-8 流、固定源码漂移、符号链接、环境目录无副作用，
以及固定原件的宿主和流路径绑定。当前 `owner::qualified_selected_original` 直接调用候选校验库，
为已安装工具的资格核验服务；原件校验结果只覆盖源码、登记、执行和两流，
不单独证明安装集、owner stage、发布或整个升级事务通过。
JSON、Git、文件系统、平台识别、字段投影和完整消费组合仍不在上述局部证明内。

`owner.rs` 的身份、阶段、意图及发布比较直接复用 worktree 的 `verified_owner_publication.rs`，
不另造一份宿主私有定义。新增同源谓词只核 publication／adoption 头和阶段声明绑定。
阶段发布由 worktree 直接核原件；已移除没有发布调用方的构建解释器校验回调。
发布者按 profile 的显式入口核登记，固定旧 Python 生产者原件仍核其实际使用的模块，
Rust 阶段生产者使用已登记的 shell 入口。文本比较复用既有同源证明；
真实发布回归另核自定义生产者入口，不把某个 Python 文件名当成所有生产者的输入。
原生回归覆盖真实构建的 stage、原件损坏／缺失、当前候选改变后的固定读取、
原操作沿用而不重执行，以及真实子进程失败退出和非 UTF-8 输出。
`lifecycle.rs` 委托原生命周期入口，不取得或解释租约，也不自行替换选中所有者。
解码、Git／文件观察、编排、报告消费和整个升级事务仍须各自行为验收，不属于这些局部证明。

本地和 CI 使用宿主上下文登记的 bootstrap 与 canonical check。
每项整 crate 证明只登记在源码所有者的测试计划：runner 拥有共用状态、限制、路径和结果判断，
worktree 的 core 计划拥有发布证明；其他证明分别归其源码所有者。
FILEMAP 显式连接证明源码、工具、启动配置和测试所有者，变更按两端 DELTA 选中证明与消费测试。
同次汇总核所有受影响所有者的原报告，各消费计划保留原 Rust 构建和行为测试；
证明编译输出没有下游构建消费者，不增加证明缓存、运输或独立 job。
独立根直接读取生产源码，声明成功、失败和边界预期；
Rust 测试另外验证真实消费者，不替代证明。

官方 Verus 固定为 `0.2026.04.24.f8e1704`／Rust 1.95.0，强制
`--no-cheating --no-external-by-default`，没有函数过滤或新增证明预算。
现有通用 bootstrap 通过下载动作校验并准备归档，成功后解包。
`host.verus-download` 委托当前 Rust 启动程序，先按 `verus-macos-arm64.sha256`
核对本地包；匹配时输出 `cache-hit`，不启动 curl。包缺失或内容不符时才从 projects.json
登记的固定 GitHub release 地址下载，请求采用 `Accept: application/octet-stream`。
下载使用 tempfile 的私有临时文件；core 和独立 Verus profile 都把 `TMPDIR`
选为已登记可回收的 `bootstrap-owner-tooling/tmp`，不把临时字节放进保留归档目录。
校验通过后原子替换归档并输出 `downloaded`；下载失败保留原退出和响应头，
校验失败不覆盖原包。正常返回或报错时由 tempfile 回收；强制中断后的残留归现有工作树清理所有者。
临时目录与归档须在同一文件系统；跨文件系统的原子发布失败，不另复制或覆盖旧包。
原响应头保留在 `.chrono-harness/state/verus-download.headers`，由现有失败工件上传供下载故障排查。
CI 由 detector 恢复归档并在启动成功后保存，检查单元复用已有共享原包；
包身份及成本边界见[缓存说明](ci-cache.md#verus-archive)。
含证明操作的单元先核共享 core，再安装工具；无证明操作的消费单元只核共享 core。
启动参数由 units.json 维护，工作流由登记的 ci.generate 产生。
共享 core 的校验不代表 Verus 已安装。
当前归档只适用于 macOS arm64。

## 此批使用的外部接口

调度声明覆盖由 `judge-registration/src/verified_scheduling.rs` 的
`claims_cover_operations` 直接判断：参与操作集合必须包含在声明键集合中，
允许额外的真实生产声明。登记解释器消费同一返回值；方法唯一性、资源和产物的原校验仍执行。
源码所有者登记 `verify.registration-scheduling` 与独立的
`verify.registration-scheduling-tests` 两项整 crate 证明，边界覆盖空集合、缺少首尾操作、
同数量但键不同、额外声明及声明值与覆盖无关。

这个局部证明复用固定 vstd 的 `BTreeSet::iter`、迭代器 `next` 和
`BTreeMap::contains_key` 合同，要求键遵守 `obeys_cmp<K>`。
生产键 `String` 的完整内容比较及标准容器／`FromIterator` 转换采用固定 Rust 1.95.0 的外部接口合同；
不新增仓内受信比较器或集合层。独立证明以标准整数键核通用判断，Rust 消费测试另核真实字符串及 JSON 边界。
实际判断不再复制全部声明键，最多遍历 P 个参与操作并查询 C 个声明，比较次数上界为 O(P log(C+1))；
字符串比较成本另计。新增两项证明的构建、CI 和完整生命周期成本未测。

本批新增生产逻辑采用下表的具体外部接口。证明内的标准接口合同来自固定 Verus／vstd；
边界转发采用固定 Rust 1.95.0，以及各使用方 lockfile 绑定的 Serde 接口。
类型或库名本身不授予其他仓内逻辑豁免。没有新增 `external_body`、`assume` 或证明过滤。
其余存量证明根和程序使用的外部接口仍须完整清点，不据本表主张全仓覆盖。

| 外部接口 | 本批用途及采用的合同 |
| --- | --- |
| `BTreeMap::<Edge, CauseSet>::remove`、`insert` | 采用固定 vstd 的精确 map 视图删除／插入合同；同类型借用键的对应由 vstd 明确公理提供。适用前提 `obeys_cmp::<Edge>()` 由本仓实际 Edge 比较及其顺序定律证明，仓内原因选择和状态合并仍在 Verus 内。 |
| `str::as_bytes` | 借用完整 UTF-8 字节，返回切片视图等于 `str::spec_bytes()`；后者由固定 vstd 定义为字符串字符序列的 UTF-8 编码。摘要判断读取原字节。 |
| `String::as_str` | 借用原字符串，字符视图相等；finding 判断读取真实 level。 |
| `str::is_empty` | 返回真当且仅当字符序列长度为零；请求头只要求 judge ID 非空，不附加 ASCII ID 限制。 |
| 切片 `len`、`<[T] as Index<usize>>::index` 及 `<usize as SliceIndex<[T]>>::index` | 长度和逐项读取对应切片视图；本仓循环另行证明索引界限、完整遍历、返回条件及终止。`T` 在本批为 `u8` 或 `Finding`。 |
| `<String as Clone>::clone` | 复制完整字符串；本仓 Finding 克隆证明各实际字符串字段均保留。 |
| `<Vec<String> as Clone>::clone` | 保留长度和逐项克隆关系；本仓 Finding 克隆证明两个引用数组的完整字段、顺序和重复均保留。 |
| `str::to_string` | 生成完整字符串；CI 结果 ID、原错误、默认原因、准入失败、响应前置及留存审计错误文字均保持完整字符／UTF-8 身份。 |
| `Vec::new`、`Vec::push`、`Vec::len`、`Vec::as_slice`、`Vec` 的 `Index<usize>` | 空序列、追加后保全原前缀、长度、切片及逐项读取；本仓证明 CI 终态循环界限、完整遍历、记录对应、参数完整形状、字节拼接与终止。独立边界的 `Vec<String>` 索引赋值采用固定 vstd 的单槽更新合同。 |
| `Option::as_ref`、`Option::map` | 无回执保持 None，有回执仅转发同一原记录的 `process.exit_code`；不在适配器中计算状态或选择退出。 |
| `serde_json::from_slice::<CompilerReport>` 与该类型的 Serde `Deserialize` 派生 | 从本次调用传入的完整字节解码；必需字段、JSON 类型、整数范围和重复具名字段按固定 Serde 合同校验。未知的 Verus 统计字段忽略；所需字段不设默认值。此处明确采用外部解码合同，不声称本仓证明了 JSON 解析器。 |
| `serde_json::from_value::<RegisteredAction>`、`from_value::<Option<OutputContract>>` 及对应 Serde 派生 | 一次转换对应输入对象；action／合同拒绝未知字段，合同缺失或 null 对应 None，非空要求由 Verus 判断。现有 owner、ID、工具与 argv 登记校验保留。 |
| `<serde_json::Value as Clone>::clone`、`<serde_json::Value as Index<&str>>::index` | 复制完整 JSON 值；`output_contract` 键存在时读取该原值，缺失时得到 Null，再交给上述同一次类型化解码，不在边界适配器中构造或筛选合同字段。 |
| `Result::map_err`、`<serde_json::Error as ToString>::to_string` 及此处 Result 的 `?` 传播 | Ok 保留原解码值，Err 转交外部解析器的完整诊断字符串；`?` 直接返回这个错误。边界不自行解释错误种类或生成准入状态。 |
| `Result::ok`、`Option::is_some` | 解码 Err 对应 None、Ok 对应同一完整报告；进程 failure 的 Some／None 原样转发，仓内不拼接成功字段。 |
| `<Option<OutputContract> as Clone>::clone`、`<Option<OutputContract> as PartialEq>::eq` | 克隆保留 None／Some 及其中完整合同；相等要求同为 None，或同为 Some 且内部合同相等。Some 的字段复制及比较直接调用本仓已证明的 `OutputContract` 实现。 |
| `<(String, String, Vec<String>, Option<OutputContract>) as PartialEq>::eq`、`<String as PartialEq>::eq`、`<Vec<String> as PartialEq>::eq` | 两端操作比较要求 owner、tool、按原顺序及重复保留的 argv、可选输出合同四项全部相等；字符串按完整内容比较，合同按上一行比较。此固定标准库接口未在本仓机械精化，不据它豁免仓内额外选择或状态逻辑。 |
| `OutputContract` 的 Serde `Serialize` 与各类型的 Debug 派生 | 原 version／commit 字段进入 Method JSON 身份；Debug 仅用于标准诊断。这些外部派生不产生本仓准入状态或成功标记。 |

实际字段借用使用固定 vstd 的 `String::as_str` 合同；协议和模式比较复用已证明的
`equal_text`／`equal_bytes`，其 UTF-8 转换采用 `str::as_bytes` 的完整字节合同、
切片长度／索引合同和固定 vstd 的 UTF-8 编解码引理。没有新增仓内受信实现。

`Status`／`WireStatus`／`CheckResult`／`Finding` 的 Debug 与 Serde 派生是明确保留的外部编码接口，沿用原 JSON 名称和字段合同，
不据这些派生主张解码、协议校验或仓内决策已经证明。工具下载、摘要、进程与 IO 的现有接口继续保留原验收义务。
`TraversedEdge` 的原 Debug／Serde 派生同样只承担外部编码。
选测返回转换明确保留固定 Rust 1.95.0 的
`<BTreeSet<String> as FromIterator<String>>::from_iter`：外部合同为保全输入字符串的精确字节集合，
按标准 `String::Ord` 排序并去重。此处没有自写受信实现；该标准库转换未由本仓机械精化。
Verus 证明覆盖转换前的完整选择、去重和顺序，现有 Rust 回归另外核实际返回值。

选测复用 `CauseSet`。遍历 E 条记录、选择 U 个不同目标、最长键 L 字节时，
按字节比较和字符串句柄移动估算，向量插入的最坏成本为
O(E(1 + (L+1)log(U+1)) + U²)，再有原标准库集合转换。
这是当前算法的上界估计，不是 Verus 机械成本证明。临时向量保留 U 个目标字符串及句柄，
不增加运行时进程；完整运行、构建、验证、CI 和存储成本未测。

finding 克隆使用固定 vstd 对 `<String as Clone>::clone` 和 `<Vec<String> as Clone>::clone` 的外部合同，
前者保留完整字符串，后者保留长度及逐项克隆关系；本仓实现证明各实际字段均保留。

**产品与宿主政策同时变更：**实际判断、状态类型与图改为同源 Verus，FILEMAP、工具、启动及执行计划逐项登记证明。
固定宏依赖通过 runner 传播到现役依赖方的独立 lockfile，没有升级其他包。
runner 判断仍为线性字节遍历，未增加运行时进程；`CauseSet` 定位需要 O(log C) 次字节比较，
新增插入最多移动 C 个字符串句柄，存储为 O(C)。原图索引与遍历成本继续保留；
构建、验证、CI、存储及完整运行成本未测，不宣称性能收益。
