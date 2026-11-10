# SPEC 当前覆盖情况

Verus 已声明根的计划覆盖由现有同源内核核对：受影响文件的全部根必须由全局计划中的唯一严格 Verus 调用承担，不能以普通操作或仅登记未选择替代。只改变声明根、登记操作的所有者或工具节点，也要求选中对应证明；完整方法登记提供关系，不依赖操作已经入选。实际原结果仍由既有绑定入口消费；无声明存量文件保持未知，完整覆盖门尚未完成，具体范围见 [Verus 实施范围](verus.md)。

本文维护产品要求的覆盖表，不是运行日志，也不是第二份政策。目标仍是完整 SPEC。`implemented` 表示所列合同有真实生产者和行为测试，不表示本宿主已启用或交付；`partial` 表示仍有缺口。五份完整宿主登记仍为 proposed，输入未完整、执行物绑定缺失。有限宿主的 registration／filemap 成功不等于七判官通过，本地检查不证明原生 CI 或落地。

宿主 Git facts 的当前必要性是运行内固定同一实际 Git，保留原结果并拒绝后续字节漂移；没有消费者要求跟踪每台主机的预期 SHA／版本。本仓已退役 scoped 政策的白名单和重复输入，根入口复用 v1 平台映射，实际身份归 state／线上原件。声明判断和可选预期比较使用同源 Verus 及独立边界，mock Git 回归覆盖观察、显式预期拒绝和执行前漂移；full v3 的显式合同保留。[退役及采用交付](https://github.com/ChronoAIProject/chrono-harness/pull/419)已核对标准检查、macOS 原生范围和真实共享协调者的公开入口；其他平台、完整输入及全对象必要性仍须各自的验收证据。

普通完整检查通过，只表示 DELTA 满足现役登记合同。`declared-complete` 是受信任 AI 对宿主治理／输入范围负责的声明，判官核引用、身份、快照和实际操作。registration 当前输出 `completeness_proven:false`，不阻止必需登记及所选检查通过后的普通验收。已知缺输入、必需依赖未解决、缺绑定／快照、漂移仍失败，真实工具链／SDK／配置义务保留；普通验收不要求证明一切隐藏输入都不存在，也不要求 VM。

本地／CI 须用同一登记命令。普遍同判还要求完整有效输入相同、求值确定。当前输入生产者仍报 completeness_proven:false，报告保留 parity 未建立，不另阻断普通完整检查。独立比较器和固定 spec-status 已退役，因为没有当前成功生产者或实际宿主状态观察；报告匹配不证明输入完整或普遍确定性。

CI 的额外资源分类、比较告警和摘要投影已退役：没有当前准入、构建、恢复或发布结果依赖它，成本排查直接查原生 CI。删除专属测试及宿主环境／映射登记；真实运行归属、job、artifact、原报告和失败核验继续沿原汇总流程。此范围不证明完整成本已测或其余对象均有必要。

旧固定宿主迁移的证据生产者、临时宿主和发布上传目录登记已退役。当前 projects 合同仍由原未过滤测试验证；移除登记不删除旧输出。持有旧政策绑定的协调者仍须沿已有迁移流程保全原件、排除活使用方，再切换政策。

宿主 bootstrap 的私有完整工具链安装只剩合成测试调用，现役 profile 没有采用，已与其配置字段、报告字段、默认 profile 判法和专属测试共同退役。保留当前构建的 minimal／rustfmt 补齐、环境选择、工具身份及原件验证；共享目录排斥仍有实际消费者。这项退役不证明完整输入闭包、工具链来源或其余对象的必要性已完成。

## 宿主默认合并方案

产品默认 `candidate.landing` 与本仓采用数据提供[并行验收、按依赖合并](merging.md)。CI 生成器和输入生产者改用固定 PR 合并树 M 及第一父 B，单独绑定事件 head H；`chrono-ci-detection/v2` 和汇总按 H 核对 GitHub 运行／job／artifact，按 M 核对测试原件。原生部署使用本仓登记的 admission 文件，生成器不修改远端保护。Rust 回归覆盖并发目标变化、head 检出拒绝、固定父身份和合并树汇总；普通 full 单元／汇总按各自现役测试验证。完整检查、原生事件、远端 strict 实际切换与落地核验须分别取得真实结果；不据源码或本地结果宣称部署、完整治理或严格同树保证。

admission 的目标声明覆盖 dev 和所有层级的 integration 分支，复用已有 PR 工作流与必需汇总，不新增检查或程序。线上是否生效须查原生有效规则；部署权限不足时该缺口仍未解决，不以仓内声明代替保护。

可选 `native_adoption` 全量输入准备扩展及专属种子、转发和治理采集接口已退役；当前宿主及已审计的固定外部宿主未发现采用，未知外部使用仍未知。普通 scoped 原生 CI、full 上下文／汇总、输入 capture／pair／compose 与共享启动链按各自当前使用方保留。完整输入、真实 full 采用及同判仍未建立，成本收益未测。

## 测试来源

下文保留代码里的名称，方便查找实现。`full` 是完整治理检查，`scoped` 是限定范围的检查，`initial` 用于没有基线的首次检查；三者的证据不能互相替代。fixture 是测试用的临时宿主或输入材料，guard 是执行前后核验输入的保护程序。所谓“输入闭合”，指所有适用输入及其依赖都已明确登记、绑定并验证。

| 缩写 | 源文件 |
| --- | --- |
| R | `crates/runner-tests/tests/protocol_v1.rs` |
| G | `crates/judge-registration-tests/tests/registration_contract.rs` |
| L | `crates/runner-tests/tests/cli_contract.rs` |
| C | `crates/judge-ci-tests/tests/behavior.rs` |
| F | `crates/judge-filemap-tests/tests/impact.rs` |
| P | `crates/judge-filemap-tests/tests/consumer.rs` |
| W | `crates/judge-workflow-tests/tests/{branch,consumer}.rs` |

指令和 CI 生成器测试各归专属项目。G 用分别构建的 runner／registration，在真实合成 Git 提交及真实宿主登记材料上执行；生产构建前置在 CI 明确绑定。P 在已提交有限宿主上运行真实 runner→registration→filemap，历史二进制不存在，报告来源指针精确。F 按独立指定的端点边核见证。本表不宣称已有同伴／独立评审证据。

CI／worktree 测试准备复用已有 PATH 解析器选择实际 Git，两份只保存主机路径的 JSON 及其 FILEMAP 输入边已退役。现役 fixture 仍在本轮绑定实际路径、版本和字节，原测试断言及未过滤测试义务保留；这只移除重复的主机选择数据，不证明完整输入、跨平台验收或 §2.2 的外部命令 mock 合同完成。剩余真实 Git fixture 的执行边界迁移仍未完成。

固定 Verus 下载包的候选采用改用 `bootstrap-owner-tooling/verus/package.zip` 单槽，保留当前 checksum、原子发布、失败保全和证明消费者。旧版本目录仅由现有生命周期入口回收，不增加 TTL 或清理器；候选 bootstrap、标准检查、原生消费和真实协调者迁移尚待验收。路径和政策变更本身不证明旧目录已回收。

下载临时文件改用现有 `TMPDIR` 选择，core 和独立 Verus profile 均指向已登记 dispose 的目录；不随保留归档进入下载缓存。既有 mock 回归核实际输出位置、失败保全和正常回收；强制中断、生命周期恢复及真实入口采用仍须分别验收。

## 按 SPEC 章节查看

| SPEC 章节 | 状态 | 生产者与直接证据 | 仍需完成 |
| --- | --- | --- | --- |
| 1 权威与边界 | 部分 | runner `full.rs` 的外部 DAG；R `dag_*` | 其余判官政策和生命周期 |
| 1 真实消费者与用途 | 部分：带版本的内部共享启动消费合同 | registration 的 `chrono-internal-causal-consumption/v1`；本仓采用已有 runner-startup，复用 need／output_use、生产／调用 action、profile 安装对、claim、真实消费证据。标准 scoped 准入核两端受影响关系，要求原真实消费者测试；Rust 成功／失败／边界及固定 Git 端点回归。这些关系检查不判断价值；全对象必要性继续由 AI 核查当前使用方和验收终点 | 是否有用仍由 AI 负责判断。已有有限 Darwin 内部启动消费；Linux、外部／全消费者采用、完整路径执行尚缺。仅有测试／分发资产不构成真实消费者 |
| 2 独立项目与采用 | 部分 | 独立 manifest／lock／bootstrap 构建登记；共享身份判断有独立生产库、专属测试项目和 runner 真调用；runner 登记构建核对点名库的原证明、库字节及最终二进制 checksum，暖调用复用原证明，独立测试及其余使用方的编译产物绑定仍缺；SPEC §2.1 将共享数学归 trureturing、运行／模型对应归 chrono-harness | 通用完整 bootstrap 来源及启用；源码归属不证明实现满足模型 |
| 2.1.1 Verus 规范性权威 | 规范已明确；实现部分完成 | SPEC 统一定义真实使用方、证明与生产调用绑定及 `E_VERUS_COVERAGE`；本地与离线汇总共用 Verus 方法选择及原报告／库／候选源码组合准入，普通成功不产生类型化证明条目；runner 的 paths／scope／status／limits、registration 的 nodes／readiness／consumption／input-coverage 行状态、CI collection 的状态源、CI 字面量、projects 语言配对和 filemap 图内核及其独立预期，以及 identifiers、retention release、runner Git binding、worktree lifecycle／attention／owner publication、cache planner、宿主 bootstrap cargo、bootstrap bindings、runner cargo module、release recipe 和 judge-workflow 的来源／提交数／年龄／执行完成／证书字段／schema migration 绑定准入根和独立预期均完成 FILEMAP `verification` 角色、证明根和 execution-plan 操作绑定，并由现有同源整 crate Verus 调用验收；`evidence.rs` 的收集完成与普通执行路径、`certificate.rs` 的字段绑定路径、`transition.rs` 的 migration 路径直接消费相应证明结果，Rust workflow 核心 consumer 同次验收；范围和外部接口见 Verus 实施说明 | 组合入口已核本次操作的声明根等于实际调用根，且声明文件参与候选编译；CI release／gating、原生发布 publication／identity 与 workflow certificate 五个外围文件的整文件证明声明不成立，已移除，仍为未知覆盖；相应已证明内核的调用、编译与行为测试继续保留。全仓覆盖门、存量文件分类、源码／编译结果／实际调用完整绑定、其余逻辑与 SPEC 对应仍未完成；不以当前 scoped 成功宣称本节已验收 |
| 2.2 实现／测试语言 | 部分：指令、full／scoped projects v2 配对、原生 helper | 中英文 test.same-language 原子用于新宿主默认及本仓指南；项目／脚本明确语言，受影响配对须同 ID 或 shell→python。实际配对准入由同源 `verified_languages.rs` 定义并已登记整 crate 根，独立预期核完整字面相等、shell→python 的方向及非法边界。Rust schema／配对／语言 metadata DELTA 用例及登记 CI transport、context judge、protocol judge、CLI child、生命周期、Cargo、Git fault helper；本仓 v2、Rust 显式 workflow 清单及配对测试、worktree 核心／采用分组及完整覆盖。固定历史支持退役后 projects 保留原未过滤 action；owner 字节发布与 stage 原件核验迁入现有 Rust worktree 所有者，构建／资格读取及原件核验也由 Rust bootstrap 承担，旧 Python 实现及专属测试退役；Rust 用例验证现役边界，候选验收另记。Python 构建配方／测试对不需 manifest，单元／真实二进制集成组独立 | 完整候选验收、示例采用；迁移剩余内嵌跨语言测试／helper，真实外部接口 fixture 单独登记。声明不能证明实际源码语言 |
| 2.2 routes 回归 helper 迁移 | 已实现，范围有限 | 八项 judge-routes-tests 回归改用 Rust 测试自有子进程 helper。`tests/support/tool.rs` 是明确 Cargo binary 和 FILEMAP compile 输入，原 test:judge-routes-tests 计划沿用 Cargo test。保留字面／空／换行 argv、cwd／环境、回执篡改、错版本／执行物替换、PATH、版本观察复用、循环／预检拒绝、标准调用、汇总身份。真实标准 DELTA 消费沿原报告验证，运行事实不在表内复制 | 仅限此测试所有者。其余跨语言逻辑／helper、外部接口 fixture 登记、完整输入、启用、同判仍缺 |
| 3 五份登记 | 部分 | registration `schema.rs`，G `strict_five_schemas_*`、`real_host_pseudo_script_*` | Cargo／SDK 完整输入仍缺；有限配对、routes、迁移和快照已实现 |
| 4 固定快照／上下文／CI | 部分 | runner facts.rs、registration lib.rs；G `context_and_config_identity_mismatch`、`ignored_untracked_staged_and_wrong_checkout_fail`。原 commit 的 parent 物理头选择已接入同源 Verus，证明首个空行前精确前缀选择的完整原字节序列、顺序及终止，独立预期核完整长度与字节。`facts::full_oid` 重导出同源 Verus 文本准入，成功条件及原错误全文由同一实现拥有；独立预期核边界及任意位置非法字节。Git 库存终止、UTF-8 错误分类、NUL 记录选择、首 TAB 分割、空格字段拆分、三字段及单字节 stage 0–3 判断也直接使用同根 Verus，证明完整结果序列、顺序、重复、空字段规则及终止，保留原错误和检查顺序；独立预期同时普通运行并逐项核完整字节。tree 实际条目构造、map 插入及重复拒绝也由同源 Verus 证明，精确保留其他全部键值、原错误和失败后的替换状态，独立预期核完整字段与数量。tree 完整逐行解析也由同源 Verus 维护，规格核全部有效前缀、首错、完整 map 双向对应及数量，非法路径全文经已证明分支转交标准 Debug 格式化；独立预期在普通 Rust 核完整字段、原路径及错误顺序。标准安全 UTF-8 解码和 Debug 格式化为明确外部接口，外围解码组合、index 重复 stage 及 Git 获取仍待证明 | config v3 已为完整／初始消费者、明确 scoped v2、当前 provider v4 事件读取／fetch 绑定 Git，见 git-facts.md。完整原生 CI／同判及 Git／委托／Cargo／SDK 输入仍缺 |
| 4.2 CI 缓存及增量构建 | 部分：两项采用编译缓存已有键、生产／内容绑定及有限原生冷／保存、兼容源码重建、健康精确命中、不兼容环境分域／重建 | 独立 Rust chrono-cache plan 消费明确消费者／输入／产物，分开源码／兼容键；JSON pointer 只绑定所选值，整文件来源在键外，回归区分单元 metadata／构建参数。v2 探测前解析消费操作引用，只改消费者不改生产键。引用原输出登记，拒绝重叠或缓存证据／源码。Rust 失效、选择、缺失、路径、真实 CLI 环境、planner 不符回归；check.unit_cache 两项绑定真实 action／Cargo，消费命中前核 producer manifest。真实 Cargo 重建／恢复仍属本地证据，见[缓存合同](ci-cache.md)。bootstrap／check／release 明确增量，发布汇总核值。provider 为 main 消费者生成固定 restore／save，保存成功不确认后端；持续后端查询及额外传输报告、CLI、上传目录政策已退役，排障直接查原生日志与线上 API。普通 CI 复用失败原件上传，发布以独立失败 artifact 保留 state，正式发布根和 collector 合同保持。声明启动缓存可在后续失败后保存，每键至多一保存 job，冲突在生成前拒绝。[有限原生验收](ci-cache.md#bounded-native-acceptance)保留两项原标准操作／汇总 | 不可用内容恢复及修复后健康精确命中的完整原生验收仍缺。Variables／PR 正文控制和专用 fixture 已退役。并发、完整后端生命周期、依赖／当前判官／bootstrap／逐项目输入仍缺，须扩大真实消费者验收 |
| 5 DELTA 图 | 部分 | 实际 Tree DELTA 已由同源 Verus 完整枚举并线性合并两棵原树，逐路径结果与完整 UTF-8 严格顺序均有证明；独立预期核完整长度、六字段、顺序、空／Unicode／NUL 及反向变化。Git 原始事实获取／解析仍待证明。filemap 类型化合并／记录／传递及 CI 适配；F 全部，P actual_runner_registration_filemap_*。SPEC §5.1 固定两态局部性及定理 6 有条件节点集结论（trureturing PR #10868 已合），复用冻结 ConsequenceClosure，保留先前精确应用编译和旧 execution-edge 反例 | 同源 Verus 图内核覆盖类型并集、名称／槽位索引、FIFO 最短前驱、有限原因链、行投影、原因集合、逐 seed 迭代状态／根对应及真实遍历记录选测；两处诊断按完整 seed ID 查找首个原记录也由同源根证明。返回集合沿用明确列出的外部标准库转换。外层原因关联与合并、seed-ID 生产及关联完整性、JSON 适配、额外选测和完整 SPEC 对应仍待证明；完整宿主输入及原生启用未完成 |
| 5.2 从消费者需要出发重构依赖 | 部分 | 已有 DELTA 合并／传递／所有者检查。无独立结构报告或自检，未用 SCC／深度／排名／claim 摘要已退役，循环、类型边、有限见证保留 | 真实消费者重构、适用本地／原生验证及核实落地 |
| 6 演进与移除 | 部分 | W 实际替换、共同退役、别名修复、候选迁移认证；已退役未接入正式 runner 检查的额外 guard、独占主机输入和观察入口；registration workflow v3 明确选端点版本。自含 fixture 用当前 decoder 确认真实登记数据转换 | 当前 schema 演进消费者验收，只在真实需要处转换数据，完整启用 |
| 7 协议 | 已实现，有限普通双提交 v1 及明确初始 profile；部分同源 Verus 证明 | runner wire.rs／full.rs／initial.rs、共享进程引擎；独立 chrono-identifiers 库证明实际摘要文本的完整 64 位小写十六进制格式，包含全零及非法字节边界；协议四态身份／顺序／克隆／退出码、实际 finding 克隆及 finding／状态一致性、响应头、finding 字段形状／引用允许条件、DELTA 克隆／完整字段比较与 evidence 身份由 verified_status.rs 证明；普通／初始响应前置及留存头部／状态组合保留完整错误文字与优先级，实际二元状态汇总共用 maximum，独立根核实际边界。负责合同见 [Verus](verus.md#协议状态与-finding)。独立完整 DAG 执行自检，含启动前错误、进程环境、输出回执、伪传输失败、身份／证据变异。R 传输／故障注入，G 真实根消费者 | 登记记录指针语法／目标归属、外层遍历／状态分类、JSON／证据 IO 与完整响应组合仍待证明；bootstrap 来源及启用。初始清单不声称 DELTA 绿；平台仅已测 Unix。自检核一致性，不证明判官语义完整或全部运行错误可发现 |
| 7.1 标准日志 | 部分：Rust JSONL、运行入口和有界滚动日志 | diagnostics 的 Logger／runtime／RollingFile；本仓 logging.json 采用 10 MiB／份、共 5 份。instructions、runner、worktree、CI、cache、inputs、distribution、Rust bootstrap 和判官入口记录开始、返回及现有诊断文本；CI 阶段记录同时进入滚动文件和原 stderr。诊断库测试、instructions／runner CLI 回归及 bootstrap 位置参数回归覆盖大小／份数／重启／四进程并发、原始通道／错误链、非法配置、日志失败时的业务结果保持和明确宿主选择；细节归[诊断说明](diagnostics.md) | Python／非 Rust 和其他全部生产者接入，完整领域异常链、子进程上下文及持久证据发布仍缺；本地标准检查和原生 CI／落地分别验收，目的失败不宣称已持久保存 |
| 7.2 异常链与通用处理 | 部分：原生 IO、真实 judge 启动前／报告有类型传播 | diagnostics::error 包装／导入 chrono-error/v1；NativeIoError 保留原种类／消息／OS 码，LogFailure 共用 IO 模块。Unix 原生 writer 保留只读描述符 EBADF；发布错误保留通道／阶段／原业务、独立次要输出／诊断失败。专属原生／无原生／部分／flush 测试。完整 judge 以 chrono-runner-transport-failure/v1 保留准备原因／码、封存请求／judge／operation，区分未启动，保留子原件，晚期报告发布失败关联原原因。真实 IO、协议／schema／上下文变异、次要发布回归，见[诊断边界](diagnostics.md) | 已有有限 Darwin 原生观察；Linux、其他产品原生适配／类型传播、非 Unix 输出、任意自定义原因序列化、协议采用、持久引用、其他次要失败发布仍缺；完整输入／启用／SPEC 未完成 |
| 7.3 自我检测 | 部分 | 原接收器描述符耗尽回归保留 spawn 原错／共享观察；资源比较消费登记实测，恢复保留失败／未确认 outcome、逐项清理和原报告。真实文件系统／CLI 回归；最终传输拒绝恢复观察缺失／变化，保留两快照 | 全生产者及历史超时根因；有限故障测试不证明普遍检测 |
| 7.4 完成归属与等待 | 部分：原生引擎及生命周期等待 | runner 用一个已有内核退出观察器／私有通知。静默期间阻塞到真实事件或原期限，取消／释放／关闭提示沿原 socket；mapping v5 拒绝不兼容嵌套观察器。独立原生计数及原退出／期限／输出／launcher 中断／描述符预算回归 | 外部宿主／调用方通道选择、句柄、丢通道后不重复启动的恢复尚未实现；不控制其他任务，原生 Linux 待验 |
| 7.5 所有工具本地状态与终态追踪 | 部分：PR 实时原件、明确接续与候选逐对象保管 | PR v2 保存独立操作身份、配置／调用输入、实时启动与部分流原件、接续前核实际原件并恢复封存后中断的输入交接、返回后核对的进程结果、命令退出与 PR／mergeCommit 观察；缺留存不派发，发布失败不能成功；已封存本地结果可由明确只读查询接续，新结果封存后释放指定旧恢复引用，Rust 测试核操作及依赖原件实际回收、其他调用方保留，以及真实外层中断后 unknown 原件继续保护。候选按仓库＋PR 编号保存当前有效 OPEN 观察及未结查询，完整失败和被替换的独立查询沿有限窗口回收；较早查询晚返回不覆盖较新事实，未知任务与真实使用方继续保护。复用现有输出保留所有者，范围见[PR 工具](pr.md)；现役候选／原生交付另验 | 其他工具持久记录、跨会话恢复及命令与对象终态分离、发布失败恢复；终态确认且消费者释放后滚动清理，索引／摘要／回执也有限保留，活跃及 unknown 状态继续保护 |
| 7.5 工作树登记、恢复与清理 | 部分，候选验收待完成 | 现役创建／导入／use／finish／maintain、逐地址登记、内核保护及缓存恢复已有有限实现；候选增加独立 cleanup 阶段、显式维护重试 retained、证据实际不存在时整树回收及同源 Verus 判断／独立边界；候选在检出继续复用时保留 finish-dispose 缓存，显式维护仅按登记回收普通 dispose；明确 finish 或实际检出处置才选择复用缓存，主检出释放请求持久绑定缓存代际，原件和物理效果另验；其他客户端与全部本地工作树接入未验收 | 漏登／空记录核对、明确接管、完整交接中断恢复、终态历史有限滚动及本地堆积实际回收；未处置对象不先丢当前记录 |
| 8 七判官／混合修改 | 部分 | 七个有限生产者及 W 完整链消费者 | 完整宿主输入及启用 |
| 9 报告／成本 | 部分 | runner full.rs；G full_report_* 核必需字段、配置／实测执行物区别、发现／具名输出来源、null／未解原因、pass／warn／fail／error 退出、旧明确存储／stdout 一致。`judge-cost` 的 `verified_cost.rs` 由 `verify.judge-cost`／`verify.judge-cost-tests` 证明四个声明坐标的未知判定、固定顺序的缺失列表，以及两端缺失拒绝和删除列表追加；schema 比较复用 identifiers。真实 `produce` 和 cost judge 消费者回归继续消费，独立边界预期同时作为 Rust 断言执行；声明成本报告已实现，有限 routes 工具／projects 证据已有 | 声明查找、成员选择和完整成本报告尚未证明；完整工具链／输入和可选测量；未知／冲突不能证明完整治理 |
| 9.1 资源诊断 | 部分 | 发布 v5 核明确消费／前置；cache 生成验证拒绝多保存者。父 gather 保留原生运行、job、artifact 和原报告核验所需响应；当前 attempt 视图可能沿用旧结果，不跨 attempt 重计。无当前使用方的额外分类、比较告警和摘要已退役，成本调查直接查线上 CI | 其他消费审计、编译／测试拆分、排队、内存／磁盘／计费及公开工具采用；未测成本保持未知 |
| 10 生命周期 | 部分 | W 新鲜度、完整报告集成生产／消费；`judge-workflow` 的 `verified_branch.rs` 由 `verify.judge-workflow`／`verify.judge-workflow-tests` 证明来源分支类型和提交落后数准入，`verified_evidence.rs` 证明 workflow 执行证据七项事实的组合准入，`verified_execution.rs` 及独立预期证明收集完成和普通执行的身份／范围／状态／操作数准入，`verified_certificate.rs` 及独立预期证明集成证书字段数量与精确字段集合准入，`verified_transition.rs` 及独立预期证明 schema migration 的声明唯一性、conversion 绑定和脚本事实准入，并由真实 branch／consumer、certificate、transition 路径及核心 workflow 测试消费。独立 worktree 和真实 Git 测试覆盖目标获取／创建、政策采用、明确重建、冲突／失败保全、recover／cleanup／cleanup-fetch、lease 远端退役、intent 中断检出／fetch、无旧回执 metadata rebind 及继续 | AI 协调冲突／歧义恢复状态，时间新鲜度、Git 历史、JSON 解码、进程／摘要外部接口和 PR／合并／落地编排及完整事务 |
| 10.2 兼容工作树复用、协作、交接 | 部分：显式采用当前分支 | 原 migrate 在逐工作树独占租约下核当前引用、HEAD、物理附件、政策和 Git 锁；创建分支／birth 原件保留，当前绑定单独维护；普通操作仍拒绝未采用的分支变化。Rust 回归覆盖工作／旧提交保全、活租约／待交接拒绝、重复采用、政策同改及当前分支清理；候选和真实宿主采用分别验收 | 完整兼容／工作保全判断，语义读写及受 index／ref／operation 限制的整树范围，分项保存／等待／检查／落地／清理交接；新分支仍须最新 dev |
| 10.3 AI 的 PR 操作 | 源码实现，范围有限 | 独立 `pr`／`pr-tests`；`pr-open` 核远端 SHA、创建／复用及回读，`pr-status` 核完整必需检查，`pr-watch` 原生等待后重核身份；JSON 与原进程回执。合同见 [PR 工具](pr.md) | 本地 Rust CLI／子进程测试覆盖已登记边界；真实 GitHub 创建／等待、公开分发、自动合并、落地核验仍待验收，不证明完整生命周期或 SPEC |
| 11 脚本／插件 | 部分 | 直接 argv 外部传输，R 原生协议／进程、L 外部脚本子进程；真实脚本配对／routes 有有限完整链证据 | 插件／完整外部输入 |
| 12 bootstrap／初始 | 部分 | 登记 bootstrap、明确 root profile、无伪 base 的候选清单；公开 macOS arm64 宿主核生成 v2 首 push／普通 DELTA | 完整源码／工具链来源、跨平台初始 profile、启用迁移 |
| 12.2 CI 驱动的公开交付 | 有限实现，按固定源码验收 | 发布身份绑定源码和 CI run／attempt；v2 delivery 投影成功落地事件、固定检出和独立发布权限；Rust 核平台原件，复用 assemble，draft 续传、公开资产摘要与标签核验。本仓 integration 两平台 build、原生自动合并、dev 落地及公开安装的分阶段实际结果见[交付验收记录 #638](https://github.com/ChronoAIProject/chrono-harness/pull/638)；合同见[发布说明](release-ci.md#持续交付与恢复) | 只按记录中已核对的固定源码、版本和原生回执确认采用；未完成或失败阶段不能算通过。稳定版、其他宿主、完整恢复场景及总成本仍须分别验收；单元测试不代替远端行为 |
| 13 验收 | 部分，见下表 | 逐项映射 | 全部未完成项仍必需 |
| 14 边界 | 文档已实现 | README／SPEC／本表 | 后续落地时更新 |
| 15 通用参考经验 | 部分 | 已有固定来源引用，不改参考仓 | 应用其余通用工作经验 |
| 16 指令 | 已实现，限当前 manifest、可移植默认、宿主采用 | 中英文原子／工作流布局提供产品默认、本仓根、完整中文 Markdown、中文聚焦技能；instructions.md 定义保留定制的采用。生成测试覆盖不支持 manifest 拒绝且不发布、当前原方法字节／mode、别名身份、无变化／回滚。历史自动格式转换及专属测试已退役；当前内容所有者、宿主采用和实际载荷边界见 methodology-extraction.md | 运行／公开／原生消费、平台／崩溃／并发限制、干净候选／原生／PR／落地；不新增完整启用／输入／发布保证 |

## 按 §13 验收场景查看

以下顺序与 SPEC §13 相同；按范围证据只证明对应 profile。

| §13 场景 | 状态与直接证据 | 仍需完成 |
| --- | --- | --- |
| 1 已登记源码修改选测 | 有限实现：F typed_content_mode_binary_*、P 影响、projects 真实执行 | 完整原生启用 |
| 2 未登记新增／ignored 脏文件 | 已实现：G changed_unregistered_fails_and_explicit_registration_repairs、ignored_untracked_staged_and_wrong_checkout_fail | 下游测试执行另验 |
| 3 源码与成员关系同改 | 有限实现：登记修复、FILEMAP 选择、mixed 普通成员／边对照 | 完整原生启用 |
| 4 删除文件用基线边／成本 | 有限实现：F／P 事实、W 共同退役／旧成本 | 完整原生启用 |
| 5 改名按 D+A | 有限实现：G／F 两端事实、W 替换／旧成本 | 完整原生启用 |
| 6 改／删边作为起点 | 有限实现：F file_record_field_and_edge_*、file_edge_only_*、共享 scoped 执行 | 完整原生启用 |
| 7 合法退役 | 有限实现：W 共同移除生产／测试、明确别名修复 | 完整原生启用 |
| 8 移除必需测试 | 有限实现：W 明确成功替换或已验共同退役 | 完整原生启用 |
| 9 配对不唯一／缺失 | 有限实现：projects 和真实消费者负例 | 完整原生启用 |
| 10 manifest 依赖未声明 | 部分：通用登记检查显式引用、必需依赖边及已声明输入；普通 Cargo 操作仍实际执行。无当前终端使用方的 Cargo 适配器及其专属断言已退役，不能以其历史测试或剩余 Cargo 执行声称语言语义已覆盖 | 当前没有产品内的 Cargo manifest／TOML 路径依赖语义检查；编译器库／后端／链接器／SDK 输入完整及完整宿主启用仍未完成。已知缺依赖／输入仍须按现役合同拒绝 |
| 11 已知缺输入／必需依赖未解决 | 有限实现：明确外部节点、必需项目边、declared-complete v3 回归、可选版本化输入范围，以及 proposed_and_incomplete_never_receive_governance_success | 本仓 Cargo／SDK／配置仍未完整，不发现依赖或证明普遍完整 |
| 12 README 不全测 | 有限实现：F、C、projects 真实不执行无关历史缺陷 | 完整原生启用 |
| 13 独立脚本测试 | 有限实现：真实完整链脚本对 | 完整原生启用 |
| 14 重复操作 | 已实现：routes 预检／完整消费者 | 已测范围无剩余 |
| 15 绕过／缺操作证据 | 有限实现：真实调用／回执比较 | 不监控任意 shell 行为 |
| 16 mixed 警告及集成 | 有限实现：W 已认证混合交付保留警告、退出 0 | 完整原生启用 |
| 17 引擎稳定性 | 有限实现：明确 FILEMAP 选择、W 完成集成 | 完整原生启用 |
| 18 政策要求集成 | 有限实现：W 缺凭据失败、完整证据接纳 | 完整原生启用 |
| 19 集成绑定变化 | 有限实现：W 拒绝 base／tree／回执／完整报告变化，接受同树提交 | 完整原生启用及有效输入完整 |
| 20 分支过时 | 部分：W 按 dev／fork／时间拒绝任一超限；reconstruct 将完整 carry／retire 选择重放到获取目标，保留冲突 | AI 语义协调、自主交付编排 |
| 21 阈值相等 | 已实现：W 数量／年龄相等、纳秒超限、配置上限 | — |
| 22 本地／CI 同命令 | 有限实现：当前 scoped v4 生成短命令并明确 push 基线；CI 覆盖重复 push、远端前进、ref 缺失、真实 CLI 保留。C units::short::generated_native_check_step_executes_same_short_command_and_event_rules 执行当前生成的短命令，核真实事件和本地调用；并发单元汇总保留完整输入原件。旧长命令生成及专属测试已退役。公开 Go／TS／mix 的固定旧发布有有限采用，当前接入及限制见[示例索引](examples.md)，原运行直接查宿主线上 CI，不证明当前候选采用 | 完整有效输入、当前候选原生宿主采用、确定同判、完整启用 |
| 23 完整确定输入相同 | 保留合同；独立比较器退役，无生产者建立更强前提 | 编译器／SDK／外部输入完整证据、确定求值、原生本地／CI 采用未建立 |
| 24 同判未建立 | 字段已实现：普通 full runner 报告、G 外部宿主 | 更强前提未建立，不另卡普通 full；已知缺必需输入仍失败 |
| 25 暂存／未暂存／未跟踪 | 已实现：G 脏文件、共享 raw tree／index／物理身份，clean filter／CRLF／mode／链接反例，full／initial／scoped／worktree 消费 | 配置／OS 输入完整及并发隔离另验 |
| 26 历史缺失 | 有限实现：G 对象、W 共同祖先／浅历史；worktree 获取明确目标 | 通用历史恢复／重建归外部 |
| 27 stdout 空／无效 | 已实现：R malformed_stdout_crash_and_bounds，R／L 内嵌无效 UTF-8 子进程／CLI 与合法 U+FFFD 对照 | — |
| 28 崩溃／超时／摘要 | 已实现：R 限制、启动前摘要测试 | 资源保护是基础设施，不是功能结论 |
| 29 成本未知 | 已实现：成本消费者告警、退出零，保留 null／已知坐标及无 DELTA 保留输入用例 | 完整宿主启用 |
| 30 候选迁移验证 | 有限实现：W 原 schema 版本、真实转换／兼容、旧方法不执行、judge 退役；自含 fixture 明确选端点并真实转换，不用已退役 scoped decoder | 其他宿主转换器、原生启用 |
| 31 无 DELTA | 有限实现：G 无变化／脏文件；W no_delta_still_runs_the_full_workflow_gate_without_business_tests 以空 DELTA 跑完整判官链、零业务 | 完整原生启用、宿主输入完整 |
| 32 初始根 | 有限实现：G 根清单、真实／浅历史父拒绝、检出／引用失败、不启用治理 | 完整 bootstrap 来源／启用 |
| 33 check CLI | 有限传输／登记及现有实现 | 全配置宿主义务未完成前仍非零 |
| 34 未知／错误 CLI | 已实现：L unknown_and_malformed_commands_are_errors | — |
| 35 已退役 spec status／parity | L 同一错误用例，不固定状态、不改报告 | — |
| 36 help／version | 已实现：L help_and_version_are_information_only | — |
| 37 标准诊断 | 部分：Rust 库、runner 边界、登记 scoped CI 进度 | 全生产者、通用登记目的、跨进程上下文 |
| 38 多层／跨进程错误链 | 部分：有类型 source／related API、版本记录导入 | 原生适配、judge 传输采用、不可变引用 |
| 39 处理异常后成功 | 部分：恢复事件保留原错误 | 全产品／适配器处理、重试、恢复边界采用 |
| 40 次要处理／发布失败 | 部分：source／related、目的失败保留尝试记录 | 经声明应急／证据边界发布合并失败，不造引用 |
| 41 开放／无限输入类别 | 待实现：只有 SPEC §7.2 | 通用语义规则、未知成员行为；代表／边界／反例不证明穷尽 |
| 42 冷／精确命中缓存 | 有限实现：planner／report 身份、真实 cache 冷未命中／保存／精确恢复，两状态均跑原 bootstrap／标准检查，传输保留原观察 | 扩至其他消费者，完整后端／生命周期 |
| 43 源码变化增量缓存 | 完整目标待验，SPEC §4.2；两项有限原生证据见上文；候选本地真实 Cargo 回归已验证较旧时间的已变源码重新编译且增量目录保留 | 当前候选已为明确编译缓存采用兼容恢复后的登记源码时间调整，方法进入兼容键；精确命中／未恢复／重放边界及同源判断证明通过。正式本地入口、原生采用和 dev push 封存／保存仍待验收，有限验证不替代；完整输入绑定及生命周期成本仍缺 |
| 44 不兼容编译输入 | 完整目标待验，SPEC §4.2；两项有限分域证据见上文 | 按不兼容工具链／target／profile／feature／输入分域，使用前重建 |
| 45 缓存缺失／损坏／不可用 | 部分：明确恢复、采用 producer manifest；可选远端 fixture 退役 | 真实文件系统／CLI 保留有效／无效／缺内容及失败生产者边界，丢不可用登记输出、保原失败，普通构建／封存／保存继续。C1 部分观察不证明原范围修复后健康精确命中，见[退役边界](ci-cache.md#retired-remote-acceptance-controls) |
| 46 并发／重跑／后续测试失败 | 待实现：SPEC §4.2 | 活输出隔离、等待缓存生产者、真实构建／测试／证据状态 |
| 47 实现测试同语言 | 部分：v2 声明、受影响双向配对、登记 Rust CI／生命周期 helper | 完整迁移及真实执行，声明不证明源码语言 |
| 48 声明或实际跨语言／内嵌测试 | 部分：projects 拒绝声明不符，Rust 回归含项目／脚本 | 其余外语逻辑迁移及实际入口对应；外层 wrapper 不证明符合，不宣称发现任意隐藏逻辑 |
| 49 shell 实现／Python 测试 | 部分：有方向 shell→python 例外、专属配对测试 | 真实宿主独立所有权／依赖／方法／执行证据 |
| 50 混合语言隔离 | 部分：任意字面语言 ID／路径、只改语言 DELTA 测试 | 混合宿主采用、逐单元完整执行，不推断语言、不共享巨型测试、不要求 Rust 宿主 |
| 51 循环／共享前置／菱形／类型边 | 部分：F cycles_diamonds_and_multiple_causes_*，合并传递保留真实边／全部起点，routes 保留执行 DAG 及资源／输出准入 | 真实受影响消费者验证，不要求 SCC／深度报告 |
| 52 可比较拆分／改名／合并 | 待实现：SPEC §5.2 | 为真实重构保留必要端点映射、接口、测试义务 |
| 53 实际自主拆分 | 待实现：SPEC §5.2 | 绑定真实消费者，按需迁移代码／输入／配对／操作，跑标准检查 |
| 54 只改图或丢义务 | 待实现：SPEC §5.2 | 删必要边／测试或造空 wrapper 不足，缺证据检查仍必需 |
| 55 取舍和未知优化成本 | 待实现：§5.2 消费者实测 | 无必需结构向量，报告方法／输入／单位／窗口／未知，不虚报加速 |
| 56 优化交付／中断／无关旧热点 | 待实现：§5.2 | 真实落地／检查／清理、原失败、只判 DELTA |
| 57 仅测试或假想未来消费者 | 待实现：§1 | 真实现有需要和消费路径，必要中间实现随集成完整交付 |
| 58 明确外部消费者 | 待实现：§1 | 真实身份／仓库、固定版本、入口、输入输出用途、采用证据；不可得保持未验 |
| 59 受影响消费者缺失／退役／不符 | 待实现：§1 | 针对 DELTA 报精确引用／证据失败，不猜消费者、不因无关问题拒绝全仓 |
| 60 成功报告／fixture 释放过期 | 部分：原 worktree 维护做有限当前保管文件退役，精确副本／保留原件、内核排除、独立同步 journal、真实部分重试 | 生产结束后普通入口自动退役、原生／交付验收 |
| 61 无当前根的历史引用环 | 有限实现：runner-tests/tests/retention.rs::current_references_protect_closure_and_released_historical_cycles_are_not_roots 在有根时保护 A→B→A，有限轮次回收无根环，保留原退出 0／9；当前 Darwin 原件确认执行 | 完整宿主／原生采用、输入完整 |
| 62 报告／流／下载重复字节 | 部分：寻址传输去重声明原件，v2 进程无损压缩，retained-reference/v2 避免报告路径复制正文，原生 gather 保留原依赖。生命周期 Git／版本／托管流复用同一编码；显式 recover／cleanup-fetch 只引用原失败报告，父报告按现有逐输出依赖保护原件。worktree 行为测试覆盖原件保留／释放及缺失、改写、错绑和符号链接拒绝 | 其余报告生产者的重复输入表示、生产者间唯一内容实体、独立操作结果／异常关系；压缩／引用不等于路径退役，完整生命周期成本未知 |
| 63 state 内嵌套 SDK／构建／增量 | 有限实现：runner-tests/tests/retention.rs::ordinary_entry_reclaims_completed_nested_outputs_with_real_savings 登记真实 SDK target/incremental，普通维护回收恰好 2097152 逻辑字节，保留退出 7，物理收益未知；当前 Darwin 原件确认 | 所有真实 SDK／fixture 生产者明确采用，目录位置不授予权限 |
| 64 发布／回收中断 | 部分：worktree intent／内核租约；runner-tests/tests/retention.rs::no_finish_preserves_live_producer_then_recovers_unknown_partial_publication 保护活发布，能力关闭后无需 finish 回收真实部分树，保留 completion not-established、拒绝已退役原件消费；Darwin 原件确认 | 通用中断及完整宿主恢复采用仍有限 |
| 65 活消费／恢复下数量字节超限 | 部分：候选 per_relation_capacity_preserves_independent_outputs_and_recovers_pending_record 要求关系集上限 1 时独立发布、恢复点名 pending；执行待完成 | 字节／未知占用组合、完整生产／恢复采用 |
| 66 删除前新增引用／身份变化／后代未等待 | 部分：准入／排除／身份复核 | 在现有所有者同步全部声明产物发布／引用／释放／处置 |
| 67 原件退役／重复清理 | 部分：缓存重试证据避免递归膨胀 | 真实退役摘要、证据不足消费、通用有限退役 metadata |
| 68 大型旧／未登记／重复宿主产物 | 部分：runner-tests/tests/retention.rs::historical_cli_requires_registered_producer_receipt_exclusion_and_release_evidence 用当前 CLI，拒绝活旧所有者和内部可丢回执，要求精确 source／receipt／lease／root／release，回收 2048 字节、保原退出 101 和外部回执；Darwin 原件确认 | 任意历史产物仍须真实迁移、消费者／释放证据；未知所有权保护，不产生历史 API 兼容 |
| 69 启动时通知／阻塞或两者皆无 | 部分：引擎 exec 前绑定原生退出身份，经原通道阻塞，有静默等待计数回归 | Linux、宿主／调用方能力选择，无通用外部任务通道 |
| 70 无终态的超时／静默／丢通道 | 待实现：§7.4 | 保原句柄／结果／unknown，不重复启动地恢复收取，继续独立授权工作 |
| 71 launcher 退出而自有子进程活着，或终态错任务 | 待实现：§7.4 | 真正等待自有工作，交接前核任务身份、原退出及结果 |
| 72 固定版本读取，或继续使用兼容的现有工作树 | 部分：原 migrate 显式采用现有 checkout 的当前分支，保留 birth 和未提交工作；完整复用判断仍缺 | 读取不切分支；复用前核身份、工作和绑定；新分支仍须满足最新 dev 和现有验收 |
| 73 协作者的编辑、index、ref 或整树操作重叠 | 待实现：目前只有 SPEC §10.2 目标 | 按真实语义依赖和冲突范围协调，保留其他参与者及其子孙任务的保护 |
| 74 工作已保存／检查，但仍有消费者或未核落地 | 待实现：目前只有 SPEC §10.2 目标 | 分清源码保存、任务加入、检查结果、实际落地与清理；单个成功或退出不释放其他消费者 |
| 75 工具返回后会话结束／机器重启 | 部分：PR 通过持久操作路径接续已封存的返回结果；无完整结果的任务恢复及全部工具仍待实现 | 从本地记录恢复原操作及对象、结果和句柄，不重复派发未知工作 |
| 76 worktree 创建／pr-open 成功，对象仍活跃 | 部分：PR 返回结果分别保存命令终态与 OPEN／CLOSED／MERGED 观察；worktree 及完整接续仍待验收 | 命令终态与对象状态分别追踪，打开不等于合并，创建不等于可删除 |
| 77 pr-watch 超时／中断／失败／身份不符 | 部分：实时启动／部分流、返回后的等待原件、超时与身份失败已保存；外层中断保护已验，候选加入逐查询预约、最终回读排序及已终结历史滚动；跨中断自动收取原结果仍待实现 | 保留原观察与失败，未确认对象保持 unknown，沿原通道恢复 |
| 78 状态发布中断或副作用后发布失败 | 待实现：§7.5 | 原 intent／部分结果及错误可恢复，不报完整成功、不重复创建 |
| 79 终态状态达到滚动清理条件 | 部分：PR 候选按对象替换已完成独立查询，完整只读失败也沿既有 diagnostic 窗口回收；真实使用方、未知任务和发布失败保留，Rust 行为测试核物理效果；完整宿主及全工具仍待验收 | 只清理已确认终态、消费者已释放且满足政策的状态；活跃／unknown／恢复状态保护，超限据实报告 |
| 80 清理终态状态后再次查询 | 部分：PR 接续已回收原件明确拒绝且不派发远端任务；其他工具及完整有限保留待实现 | 索引／摘要／回执也有限保留，已回收结果明确不可取得，不反推成功或取得对象删除权 |
| 81 Git 工作树漏登／空记录／其他客户端创建 | 待实现：§7.5 | 报明确差异，受管使用前接管当前保管，历史结果保持 unknown，不猜删除资格 |
| 82 工作树创建／交接／清理中断或漏调 finish | 待实现完整合同：§7.5 | 原所有者在存活协调宿主持久保存待办及恢复条件，后续入口接续，仍须真实终态与消费者排除 |
| 83 缓存清理或 PR 合并后工作树仍存在 | 待实现：§7.5 | 工作／交付与清理阶段分别记录，清缓存不报整树移除，保留须有具体对象和解除条件 |
| 84 必要原件已释放或工作树仅部分移除 | 待实现：§7.5、§10.1 | 核原件／引用及提交保全后继续处置；路径和 Git 登记核实移除前不丢恢复状态，仍保留／复用对象的当前记录持续可用 |

## 当前实现的负责位置

覆盖表只说明当前能力与缺口。命令、格式、失败边界和证明合同归下列负责文档；运行结果直接查对应线上 CI／PR 或有期限的原报告。历史实现过程、候选 SHA、逐次测试数量、安装清单和实验叙事归 Git 历史，不作为当前采用依据。

| 范围 | 当前生产者及负责文档 | 验收边界 |
| --- | --- | --- |
| 登记、操作及测试配对 | registration、routes、projects；[执行合同](execution.md) | 显式登记和实际回执检查已实现；不发现隐藏依赖、不判断产品价值，完整宿主输入与其余跨语言 helper 迁移尚未完成 |
| 固定端点与输入 | runner、inputs；[Git 事实](git-facts.md)、[输入](inputs.md) | 当前候选读取固定端点数据；快照、摘要和配置选择只绑定已声明输入，不证明完整输入或本地／CI 同判 |
| DELTA 和实际选测 | filemap、routes、projects；[影响闭包](filemap-impact.md) | 使用两端显式登记和实际遍历结果；完整图／SPEC 对应及外围转换仍有未证明部分，不按目录或语言推断选测 |
| CI 与汇总 | ci、judge-ci；[CI](ci.md)、[单元](ci-units.md)、[完整上下文](full-ci.md) | scoped 短命令和原件汇总有现役使用方；单元 provider 只支持明确的父工作流，不保留独立单元地址和等待／轮询参数；下载重试、原件拒绝及 full 汇总回归使用父拓扑，full 夹具的原生 job 值由登记适配器留存并转交真实采集程序；旧发布示例不证明当前源码采用，full 宿主采用、完整来源与普遍同判尚未完成 |
| 缓存 | cache、CI／release provider；[缓存](ci-cache.md) | 现役检查／发布使用所选键、恢复、实际构建、封存和保存；有限原生观察不证明全消费者、并发或完整输入正确性 |
| 日志、错误和协议 | runner、diagnostics；[诊断](diagnostics.md)、[执行](execution.md) | 真实进程、原退出／输出和有限故障边界有测试；跨产品诊断、完整响应组合、JSON／IO 和通用错误覆盖尚未完成 |
| 工作树和任务等待 | worktree、runner；[工作树](worktree.md) | 已采用路径保护实际源码、消费者及可重建缓存，保留原失败／unknown；外部任务通道恢复、完整协作复用、PR／合并编排尚未实现 |
| 原件及输出保留 | runner、worktree；[原件生命周期](artifact-retention.md) | 已采用原件引用、消费者保护和有限恢复／回收；所有历史输出、全部生产者接入、通用中断及原生交接尚未验收 |
| 源码构建、安装与发布 | bootstrap、distribution、release；[CI](ci.md)、[发布](release-ci.md)、[分发](distribution.md) | core、共享启动和当前发布各有独立消费者；旧全工具安装和原生初始盘点已退役，完整绑定和外部公开采用分别验收，不能由源码存在推导通过 |
| 指令默认与宿主采用 | instructions；[指令](instructions.md)、[方法来源](methodology-extraction.md) | 默认与宿主选择独立维护，生成根指南和完整输出；生成不证明 AI 阅读、遵守或翻译等价 |
| 形式证明 | 实际生产根及专属独立预期；[Verus](verus.md) | 同源证明、固定结果合同和部分生产调用已接入；未覆盖部分、外部合同及外围组合明确保留，不把局部成功当作完整 SPEC 证明 |

## 当前必要性与数据边界

§2.3／§8 的来源边界复用现有 FILEMAP 投影与 config／CI policy 产物声明，不新增逐文件 provenance 标签或通用源预算准入。全对象必要性仍由 AI 核查真实使用方和验收终点。现役 CI 快照检查已拒绝产物范围内的受跟踪路径；原有 Rust 回归补充“文件已登记仍拒绝”的分支，与未登记拒绝及未跟踪产物允许共用同一用例。不改生产判法，不新增证明根；适用 Verus 与行为检查沿原登记入口执行。此覆盖不证明所有运行事实都已识别或全仓必要性审计已经完成。

大文件获取回归已移除没有现役调用的外部历史文件选择分支及环境登记；自含输入继续核对原传输、容量和漂移合同。

runner 与 CI 诊断、保留回归中没有现役调用的临时过滤操作已退役。inputs／projects 迁移契约，以及工作树年龄、目录回收、部署排除和登记原件保护回归的开发期定向入口也已移除；测试本体及断言继续由原未过滤操作或正式 core／adoption 分组执行。这只移除重复入口，不新增证明或检查路径，也不证明全部对象的必要性已完成。

full 收集回归只验证当前合同：开发期接受修复前错误、跳过当前断言的开关已退役。同名成功／拒绝用例由原有 `test.workflow.short` 分组执行，核对收集复用原件、忽略无关报告，以及拒绝必需原件缺失、畸形或过期。

core 只准备当前标准入口实际使用的四项启动工具。没有当前调用方的 `chrono-inputs` 预先获取、构建、安装、共享传递及宿主工具声明已移除，专属绑定和 FILEMAP 边同步删除；inputs／workflow 的真实项目测试和公开发布资产保留。当前公开 bootstrap 和 macOS 原生启动链已实际消费这组工具，交付范围与原件见[对应 PR](https://github.com/ChronoAIProject/chrono-harness/pull/419)。总成本收益未测，全对象必要性审计继续保留未验证项。

inputs、worktree-tests、pr 和 pr-tests 没有现役消费的独立预取动作已移除。原输入安装与 owner 验证消费者已退役，PR 工具没有独立预取要求；当前锁定构建和测试由 Cargo 获取依赖。core／所有者启动仍需要的四项预取继续由原入口负责。

示例、CI、缓存、发布和指令文档保留当前接入方式、协议、使用方及验证限制；历史 CI 运行、环境摘要、测试数量、报告大小和指令载荷读数归原 PR、发布及线上原件，不另维护运行表。原件缺失仍是未验证，删除文档副本不改变历史失败或尚缺的验收。

工作流退役登记只保留当前固定两端变化与现役迁移需要的声明。已完成、两端都没有原身份和引用的历史条目交还 Git／PR；当前删除、替代测试实际通过和固定旧输入的保护仍由[工作流检查](workflow.md#移除测试或检查程序)负责。本次只清理宿主历史数据，不改生产判法或增加测试／证明入口，适用验收仍须由候选实际执行。

宿主首次接入 CI 的旧基线例外也已移除。现役 PR／dev 两端均有检查 profile；日常输入由当前事件或登记远端产生，不再保留已完成首次接入的精确提交。产品供实际首次接入使用的合同及拒绝无声明旧基线的检查，仍由[CI 说明](ci.md)维护。

全对象“为什么”链是 AI 对真实使用方、替代方案、失败影响和验收终点负责的方法。产品默认和本宿主指南已呈现该规则；现有判官继续检查明确登记、引用、输入及实际回执。没有独立价值台账或机器价值门，规则文字也不证明全仓审计已经完成。

工作树的 v3 多 Git 身份选择已退役，宿主采用现有 v2／null 预期。一次运行仍观察实际 Git 并对后续命令核同一摘要；可选单一预期不符的执行前拒绝和真实创建测试继续保留。产品与宿主政策已一起交付，本仓受验 macOS 原生范围、存活共享协调者的显式迁移及公开入口采用已完成。其他协调者、平台、全部主机政策及跨环境同判仍须分别验收，完整验证与迁移成本未知。

已退役的 runner／diagnostics 额外 guard、专属主机观察、运行时来源模式和旧分支原生初始盘点，不再列为待激活的宿主义务。旧盘点的工作流、安装／观察声明、探测 helper、专属动作和测试同步退役；core／所有者 bootstrap 实际使用的 fetch 保留。普通锁定 Cargo 构建和行为测试继续执行，通用输入产品仍须核对当前使用方。现役完整宿主登记仍为 proposed／not-implemented／incomplete。

旧安装身份、无人调用的本机观察声明和旧仓库审计快照已移除。宿主政策已去除无直接操作的编译器／rustup 重复项；本机 Cargo 通过调用环境解析，bootstrap 继续核官方组件字节，选择合同归[输入所有者](inputs.md#工具链入口与本机路径)。CI／worktree 测试准备也已[移除两份主机 Git 路径表](https://github.com/ChronoAIProject/chrono-harness/pull/462)，复用已有解析器及运行内实际绑定；这不证明外部命令 mock 的完整迁移。保留的政策和功能仍按当前使用方逐项核必要性。线上 CI 运行事实不在本表复制；当前仍需的来源／未完成工作由[输入保管者](input-source-custody.md)负责，历史失败和独有工作不能因本表收敛而被删除。

Cargo 配置输入由宿主内的 `CARGO_HOME` 负责。缓存探测、所选平台检查和发布缓存生产命令使用同一相对目录，复用已有 `config`／`config.toml` 输入，退役 runner 主目录绑定。可重建 Cargo home 归已有缓存生命周期，实际路径和原进程结果归 state／线上原件。具体消费归[缓存](ci-cache.md)和[发布](release-ci.md)所有者；真实原生缓存、平台发布及完整配置输入闭包仍须分别验收。

Verus 下载的 curl 路径由现役 bootstrap 操作参数负责；两份 macOS 工具表及 scoped 检查工具表中的重复 curl 槽位已移除。scoped 也移除无直接 action 的 rustup，bootstrap、发布和缓存各自需要的 rustup 选择继续由原所有者负责，具体边界见[输入说明](inputs.md)。

Verus 包的缓存命中与新下载均由现役 Rust 下载动作核对完整 SHA-256；外层重复的 shasum 动作及其专属工具登记已移除。原下载拒绝、恢复、失败保全测试和证明继续执行，完整成本仍未知。

## 尚未完成的组合验收

各范围的局部测试、bootstrap、scoped CI 和实际交付分别消费自己的固定原件。完整输入、所有生产者生命周期接入、跨平台／外部采用和完整 SPEC 尚未完成。本表没有成功状态生产权；新增结论须由真实生产者和相应消费结果支持，不能从历史绿色、PR 关闭、缓存释放或某一阶段成功推导。

当前资源取舍只对实际输入、操作、测量窗口和单位成立。构建、证明、缓存传输、存储与维护的全生命周期成本仍有 unknown；历史实验耗时不能作为当前性能承诺。
