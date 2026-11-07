# SPEC 当前覆盖情况

本文维护产品要求的覆盖表，不是运行日志，也不是第二份政策。目标仍是完整 SPEC。`implemented` 表示所列合同有真实生产者和行为测试，不表示本宿主已启用或交付；`partial` 表示仍有缺口。五份完整宿主登记仍为 proposed，输入未完整、执行物绑定缺失。有限宿主的 registration／filemap 成功不等于七判官通过，本地检查不证明原生 CI 或落地。

普通完整检查通过，只表示 DELTA 满足现役登记合同。`declared-complete` 是受信任 AI 对宿主治理／输入范围负责的声明，判官核引用、身份、快照和实际操作。registration 当前输出 `completeness_proven:false`，不阻止必需登记及所选检查通过后的普通验收。已知缺输入、必需依赖未解决、缺绑定／快照、漂移仍失败，真实工具链／SDK／配置义务保留；普通验收不要求证明一切隐藏输入都不存在，也不要求 VM。

本地／CI 须用同一登记命令。普遍同判还要求完整有效输入相同、求值确定。当前输入生产者仍报 completeness_proven:false，报告保留 parity 未建立，不另阻断普通完整检查。独立比较器和固定 spec-status 已退役，因为没有当前成功生产者或实际宿主状态观察；报告匹配不证明输入完整或普遍确定性。

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

## 按 SPEC 章节查看

| SPEC 章节 | 状态 | 生产者与直接证据 | 仍需完成 |
| --- | --- | --- | --- |
| 1 权威与边界 | 部分 | runner `full.rs` 的外部 DAG；R `dag_*` | 其余判官政策和生命周期 |
| 1 真实消费者与用途 | 部分：带版本的内部共享启动消费合同 | registration 的 `chrono-internal-causal-consumption/v1`；本仓采用已有 runner-startup，复用 need／output_use、生产／调用 action、profile 安装对、claim、真实消费证据。标准 scoped 准入核两端受影响关系，要求原真实消费者测试；Rust 成功／失败／边界及固定 Git 端点回归。这些关系检查不判断价值；全对象必要性继续由 AI 核查当前使用方和验收终点 | 是否有用仍由 AI 负责判断。已有有限 Darwin 内部启动消费；Linux、外部／全消费者采用、完整路径执行尚缺。仅有测试／分发资产不构成真实消费者 |
| 2 独立项目与采用 | 部分 | 独立 manifest／lock／bootstrap 构建登记；SPEC §2.1 将共享数学归 trureturing、运行／模型对应归 chrono-harness | 通用完整 bootstrap 来源及启用；源码归属不证明实现满足模型 |
| 2.1.1 Verus 规范性权威 | 规范已明确；实现部分完成 | SPEC 统一定义真实使用方、证明与生产调用绑定及 `E_VERUS_COVERAGE`；现役局部证明和结果合同见 Verus 实施说明 | 全仓覆盖门、源码／编译结果／实际调用完整绑定、其余逻辑与 SPEC 对应仍未完成；不以当前 scoped 成功宣称本节已验收 |
| 2.2 实现／测试语言 | 部分：指令、full／scoped projects v2 配对、原生 helper | 中英文 test.same-language 原子用于新宿主默认及本仓指南；项目／脚本明确语言，受影响配对须同 ID 或 shell→python。实际配对准入已由同源 Verus 定义，独立预期核完整字面相等、shell→python 的方向及非法边界。Rust schema／配对／语言 metadata DELTA 用例及登记 CI transport、context judge、protocol judge、CLI child、生命周期、Cargo、Git fault helper；本仓 v2、Rust 显式 workflow 清单及配对测试、worktree 核心／采用分组及完整覆盖。固定历史支持退役后 projects 保留原未过滤 action；owner 字节发布与 stage 原件核验迁入现有 Rust worktree 所有者，Python 保留构建／资格读取；Rust 用例覆盖原回调义务，候选验收另记。Python 构建配方／测试对不需 manifest，单元／真实二进制集成组独立 | 完整候选验收、示例采用；迁移剩余内嵌跨语言测试／helper，真实外部接口 fixture 单独登记。声明不能证明实际源码语言 |
| 2.2 routes 回归 helper 迁移 | 已实现，范围有限 | 八项 judge-routes-tests 回归改用 Rust 测试自有子进程 helper。`tests/support/tool.rs` 是明确 Cargo binary 和 FILEMAP compile 输入，原 test:judge-routes-tests 计划沿用 Cargo test。保留字面／空／换行 argv、cwd／环境、回执篡改、错版本／执行物替换、PATH、版本观察复用、循环／预检拒绝、标准调用、汇总身份。真实标准 DELTA 消费沿原报告验证，运行事实不在表内复制 | 仅限此测试所有者。其余跨语言逻辑／helper、外部接口 fixture 登记、完整输入、启用、同判仍缺 |
| 3 五份登记 | 部分 | registration `schema.rs`，G `strict_five_schemas_*`、`real_host_pseudo_script_*` | Cargo／SDK 完整输入；有限配对、routes、迁移、快照、v5 工具链清单已实现 |
| 4 固定快照／上下文／CI | 部分 | runner facts.rs、registration lib.rs；G `context_and_config_identity_mismatch`、`ignored_untracked_staged_and_wrong_checkout_fail` | config v3 已为完整／初始消费者、明确 scoped v2、当前 provider v4 事件读取／fetch 绑定 Git，见 git-facts.md。完整原生 CI／同判及 Git／委托／Cargo／SDK 输入仍缺 |
| 4.2 CI 缓存及增量构建 | 部分：两项采用编译缓存已有键、生产／内容绑定及有限原生冷／保存、兼容源码重建、健康精确命中、不兼容环境分域／重建 | 独立 Rust chrono-cache plan 消费明确消费者／输入／产物，分开源码／兼容键；JSON pointer 只绑定所选值，整文件来源在键外，回归区分单元 metadata／构建参数。v2 探测前解析消费操作引用，只改消费者不改生产键。引用原输出登记，拒绝重叠或缓存证据／源码。Rust 失效、选择、缺失、路径、真实 CLI 环境、planner 不符回归；check.unit_cache 两项绑定真实 action／Cargo，消费命中前核 producer manifest。真实 Cargo 重建／恢复仍属本地证据，见[缓存合同](ci-cache.md)。bootstrap／check／release 明确增量，发布汇总核值。provider 为 main 消费者生成固定 restore／save，保存先保持未确认；可选 GitHub 查询保留原有限进程，区分 key／ref 可用性与创建来源，失败／歧义不确认，无待确认保存不查。声明启动缓存可在后续失败后保存，每键至多一保存 job，冲突在生成前拒绝。[有限原生验收](ci-cache.md#bounded-native-acceptance)保留两项原标准操作／汇总 | 不可用内容恢复及修复后健康精确命中的完整原生验收仍缺。Variables／PR 正文控制和专用 fixture 已退役。并发、完整后端生命周期、依赖／当前判官／bootstrap／逐项目输入仍缺，须扩大真实消费者验收 |
| 5 DELTA 图 | 部分 | filemap 类型化合并／记录／传递及 CI 适配；F 全部，P actual_runner_registration_filemap_*。SPEC §5.1 固定两态局部性及定理 6 有条件节点集结论（trureturing PR #10868 已合），复用冻结 ConsequenceClosure，保留先前精确应用编译和旧 execution-edge 反例 | 同源 Verus 图内核覆盖类型并集、名称／槽位索引、FIFO 最短前驱、有限原因链、行投影、原因集合、逐 seed 迭代状态／根对应及真实遍历记录选测；两处诊断按完整 seed ID 查找首个原记录也由同源根证明。返回集合沿用明确列出的外部标准库转换。外层原因关联与合并、seed-ID 生产及关联完整性、JSON 适配、额外选测和完整 SPEC 对应仍待证明；完整宿主输入及原生启用未完成 |
| 5.2 从消费者需要出发重构依赖 | 部分 | 已有 DELTA 合并／传递／所有者检查。无独立结构报告或自检，未用 SCC／深度／排名／claim 摘要已退役，循环、类型边、有限见证保留 | 真实消费者重构、适用本地／原生验证及核实落地 |
| 6 演进与移除 | 部分 | W 实际替换、共同退役、别名修复、候选迁移认证；已退役未接入正式 runner 检查的额外 guard、独占主机输入和观察入口；registration workflow v3 明确选端点版本。自含 fixture 用当前 decoder 确认真实登记数据转换 | 当前 schema 演进消费者验收，只在真实需要处转换数据，完整启用 |
| 7 协议 | 已实现，有限普通双提交 v1 及明确初始 profile；部分同源 Verus 证明 | runner wire.rs／full.rs／initial.rs、共享进程引擎；verified_identifiers.rs 证明实际摘要文本的完整 64 位小写十六进制格式，包含全零及非法字节边界；协议四态身份／顺序／克隆／退出码、实际 finding 克隆及 finding／状态一致性、响应头、finding 字段形状／引用允许条件、DELTA 克隆／完整字段比较与 evidence 身份由 verified_status.rs 证明，独立根核实际边界。独立完整 DAG 执行自检，含启动前错误、进程环境、输出回执、伪传输失败、身份／证据变异。R 传输／故障注入，G 真实根消费者 | 登记记录指针语法／目标归属、JSON／证据 IO 与完整响应组合仍待证明；bootstrap 来源及启用。初始清单不声称 DELTA 绿；平台仅已测 Unix。自检核一致性，不证明判官语义完整或全部运行错误可发现 |
| 7.1 标准日志 | 部分：Rust JSONL、运行入口和有界滚动日志 | diagnostics 的 Logger／runtime／RollingFile；本仓 logging.json 采用 10 MiB／份、共 5 份。runner、worktree、CI、cache、inputs、distribution、Rust bootstrap 和判官入口记录开始、返回及现有诊断文本；CI 阶段记录同时进入滚动文件和原 stderr。诊断库测试、runner CLI 回归及 bootstrap 位置参数回归覆盖大小／份数／重启／四进程并发、原始通道／错误链、非法配置、失败输出和明确宿主选择；细节归[诊断说明](diagnostics.md) | 独立 instructions、Python／非 Rust 和其他全部生产者接入，完整领域异常链、子进程上下文及持久证据发布仍缺；本地标准检查和原生 CI／落地分别验收，目的失败不宣称已持久保存 |
| 7.2 异常链与通用处理 | 部分：原生 IO、真实 judge 启动前／报告有类型传播 | diagnostics::error 包装／导入 chrono-error/v1；NativeIoError 保留原种类／消息／OS 码，LogFailure 共用 IO 模块。Unix 原生 writer 保留只读描述符 EBADF；发布错误保留通道／阶段／原业务、独立次要输出／诊断失败。专属原生／无原生／部分／flush 测试。完整 judge 以 chrono-runner-transport-failure/v1 保留准备原因／码、封存请求／judge／operation，区分未启动，保留子原件，晚期报告发布失败关联原原因。真实 IO、协议／schema／上下文变异、次要发布回归，见[诊断边界](diagnostics.md) | 已有有限 Darwin 原生观察；Linux、其他产品原生适配／类型传播、非 Unix 输出、任意自定义原因序列化、协议采用、持久引用、其他次要失败发布仍缺；完整输入／启用／SPEC 未完成 |
| 7.3 自我检测 | 部分 | 原接收器描述符耗尽回归保留 spawn 原错／共享观察；后端缺失／歧义／格式／失败回归不造可用性。资源比较消费登记实测，恢复保留失败／未确认 outcome、逐项清理和原报告。真实文件系统／CLI 回归；最终传输拒绝恢复观察缺失／变化，保留两快照 | 全生产者、原生后端及历史超时根因；有限故障测试不证明普遍检测 |
| 7.4 完成归属与等待 | 部分：原生引擎及生命周期等待 | runner 用一个已有内核退出观察器／私有通知。静默期间阻塞到真实事件或原期限，取消／释放／关闭提示沿原 socket；mapping v5 拒绝不兼容嵌套观察器。独立原生计数及原退出／期限／输出／launcher 中断／描述符预算回归 | 外部宿主／调用方通道选择、句柄、丢通道后不重复启动的恢复尚未实现；不控制其他任务，原生 Linux 待验 |
| 8 七判官／混合修改 | 部分 | 七个有限生产者及 W 完整链消费者 | 完整宿主输入及启用 |
| 9 报告／成本 | 部分 | runner full.rs；G full_report_* 核必需字段、配置／实测执行物区别、发现／具名输出来源、null／未解原因、pass／warn／fail／error 退出、旧明确存储／stdout 一致。声明成本报告已实现，有限 routes 工具／projects 证据已有 | 完整工具链／输入和可选测量；未知／冲突不能证明完整治理 |
| 9.1 资源诊断 | 部分 | 发布 v5 核明确消费／前置；cache 生成验证拒绝多保存者。父 gather 在当前 API attempt 视图暴露时间／未知、保留旧 attempt 排除，不跨 attempt 重计沿用结果；可选分类比较沿同原响应／Actions 摘要告警，无额外 job／请求 | 其他消费审计、编译／测试拆分、排队、内存／磁盘／计费、通用冗余检测、公开工具采用 |
| 10 生命周期 | 部分 | W 新鲜度、完整报告集成生产／消费。独立 worktree 和真实 Git 测试覆盖目标获取／创建、政策采用、明确重建、冲突／失败保全、recover／cleanup／cleanup-fetch、lease 远端退役、intent 中断检出／fetch、无旧回执 metadata rebind 及继续 | AI 协调冲突／歧义恢复状态，PR／合并／落地编排 |
| 10.2 兼容工作树复用、协作、交接 | 待实现，只有目标合同 | SPEC §15 固定设计参考；原生命周期保护／恢复不等于新合同 | 兼容／工作保全检查，语义读写及受 index／ref／operation 限制的整树范围，活消费者保护，区分已保存／已等待／已检查／已落地／可丢交接；新分支仍须最新 dev |
| 11 脚本／插件 | 部分 | 直接 argv 外部传输，R 原生协议／进程、L 外部脚本子进程；真实脚本配对／routes 有有限完整链证据 | 插件／完整外部输入 |
| 12 bootstrap／初始 | 部分 | 登记 bootstrap、明确 root profile、无伪 base 的候选清单；公开 macOS arm64 宿主核生成 v2 首 push／普通 DELTA | 完整源码／工具链来源、跨平台初始 profile、启用迁移 |
| 13 验收 | 部分，见下表 | 逐项映射 | 全部未完成项仍必需 |
| 14 边界 | 文档已实现 | README／SPEC／本表 | 后续落地时更新 |
| 15 通用参考经验 | 部分 | 已有固定来源引用，不改参考仓 | 应用其余通用工作经验 |
| 16 指令 | 已实现，限当前 manifest、可移植默认、宿主采用 | 中英文原子／工作流布局提供产品默认、本仓根、完整中文 Markdown、中文聚焦技能；instructions.md 定义保留定制的采用。生成测试覆盖不支持 manifest 拒绝且不发布、当前原方法字节／mode、别名身份、无变化／回滚。历史自动格式转换及专属测试已退役；先前中英／定制消费、上下文测量、有限固定文本审读见 methodology-extraction.md | 运行／公开／原生消费、平台／崩溃／并发限制、干净候选／原生／PR／落地；不新增完整启用／输入／发布保证 |

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
| 10 manifest 依赖未声明 | 有限实现：可选 Cargo 明确政策、TOML 路径依赖、缺边拒绝；v4 绑定 Cargo／编译器，v5 增 sysroot／后端／链接器／SDK／脚本清单，有直接／完整／scoped 真实消费者；diagnostics 的额外保护采用及专属运行时来源输入模式已退役，保留普通文件的显式摘要、快照与漂移检查 | 编译器库／后端／链接器／SDK 语义输入完整；其余输入声明的必要性、受保护 registry／Git 解析、完整宿主启用 |
| 11 已知缺输入／必需依赖未解决 | 有限实现：明确外部节点、必需项目边、declared-complete v3 回归、v5 清单、可选版本化输入范围，以及 proposed_and_incomplete_never_receive_governance_success | 本仓 Cargo／SDK／配置仍未完整，不发现依赖或证明普遍完整 |
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
| 43 源码变化增量缓存 | 完整目标待验，SPEC §4.2；两项有限原生证据见上文 | 同一构建入口复用兼容逐项目指纹／依赖／增量数据，绑定候选判官输出 |
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
| 72 固定版本读取，或继续使用兼容的现有工作树 | 待实现：目前只有 SPEC §10.2 目标 | 读取不切分支；复用前核身份、工作和绑定；新分支仍须满足最新 dev 和现有验收 |
| 73 协作者的编辑、index、ref 或整树操作重叠 | 待实现：目前只有 SPEC §10.2 目标 | 按真实语义依赖和冲突范围协调，保留其他参与者及其子孙任务的保护 |
| 74 工作已保存／检查，但仍有消费者或未核落地 | 待实现：目前只有 SPEC §10.2 目标 | 分清源码保存、任务加入、检查结果、实际落地与清理；单个成功或退出不释放其他消费者 |

## 当前实现的负责位置

覆盖表只说明当前能力与缺口。命令、格式、失败边界和证明合同归下列负责文档；运行结果直接查对应线上 CI／PR 或有期限的原报告。历史实现过程、候选 SHA、逐次测试数量、安装清单和实验叙事归 Git 历史，不作为当前采用依据。

| 范围 | 当前生产者及负责文档 | 验收边界 |
| --- | --- | --- |
| 登记、操作及测试配对 | registration、routes、projects；[执行合同](execution.md) | 显式登记和实际回执检查已实现；不发现隐藏依赖、不判断产品价值，完整宿主输入与其余跨语言 helper 迁移尚未完成 |
| 固定端点与输入 | runner、inputs；[Git 事实](git-facts.md)、[输入](inputs.md)、[Cargo](cargo-projects.md) | 当前候选读取固定端点数据；快照、摘要和配置选择只绑定已声明输入，不证明完整输入或本地／CI 同判 |
| DELTA 和实际选测 | filemap、routes、projects；[影响闭包](filemap-impact.md) | 使用两端显式登记和实际遍历结果；完整图／SPEC 对应及外围转换仍有未证明部分，不按目录或语言推断选测 |
| CI 与汇总 | ci、judge-ci；[CI](ci.md)、[单元](ci-units.md)、[完整上下文](full-ci.md) | scoped 短命令和原件汇总有现役使用方；full 宿主采用、完整来源与普遍同判尚未完成 |
| 缓存 | cache、CI／release provider；[缓存](ci-cache.md) | 现役检查／发布使用所选键、恢复、实际构建、封存和保存；有限原生观察不证明全消费者、并发或完整输入正确性 |
| 日志、错误和协议 | runner、diagnostics；[诊断](diagnostics.md)、[执行](execution.md) | 真实进程、原退出／输出和有限故障边界有测试；跨产品诊断、完整响应组合、JSON／IO 和通用错误覆盖尚未完成 |
| 工作树和任务等待 | worktree、runner；[工作树](worktree.md) | 已采用路径保护实际源码、消费者及可重建缓存，保留原失败／unknown；外部任务通道恢复、完整协作复用、PR／合并编排尚未实现 |
| 原件及输出保留 | runner、worktree；[原件生命周期](artifact-retention.md) | 已采用原件引用、消费者保护和有限恢复／回收；所有历史输出、全部生产者接入、通用中断及原生交接尚未验收 |
| 源码构建、安装与发布 | bootstrap、distribution、release；[CI](ci.md)、[发布](release-ci.md)、[分发](distribution.md) | core、共享启动和当前发布各有独立消费者；旧全工具安装已退役，原生获取、完整绑定和外部公开采用分别验收，不能由源码存在推导通过 |
| 指令默认与宿主采用 | instructions；[指令](instructions.md)、[方法来源](methodology-extraction.md) | 默认与宿主选择独立维护，生成根指南和完整输出；生成不证明 AI 阅读、遵守或翻译等价 |
| 形式证明 | 实际生产根及专属独立预期；[Verus](verus.md) | 同源证明、固定结果合同和部分生产调用已接入；未覆盖部分、外部合同及外围组合明确保留，不把局部成功当作完整 SPEC 证明 |

## 当前必要性与数据边界

示例和发布文档保留当前接入方式、协议与验证限制；历史 CI 运行、安装摘要、测试数量及报告大小归原 PR、发布和线上原件，不另维护运行表。原件缺失仍是未验证，删除文档副本不改变历史失败或完成尚缺的验收。

全对象“为什么”链是 AI 对真实使用方、替代方案、失败影响和验收终点负责的方法。产品默认和本宿主指南已呈现该规则；现有判官继续检查明确登记、引用、输入及实际回执。没有独立价值台账或机器价值门，规则文字也不证明全仓审计已经完成。

已退役的 runner／diagnostics 额外 guard、专属主机观察及运行时来源模式，不再列为待激活的宿主义务。普通锁定 Cargo 构建和行为测试继续执行；通用输入／Cargo 产品与原生获取必须分别核对自己的当前消费者。现役完整宿主登记仍为 proposed／not-implemented／incomplete。

旧安装身份、无人调用的本机观察声明和旧仓库审计快照已移除。四份宿主政策已去除无直接操作的编译器／rustup 重复项；本机 Cargo 通过调用环境解析，bootstrap 继续核官方组件字节，选择合同归[输入所有者](inputs.md#工具链入口与本机路径)。其余 host policy、完整安装及全部对象必要性仍须逐项验收。线上 CI 运行事实不在本表复制；当前仍需的来源／未完成工作由[输入保管者](input-source-custody.md)负责，历史失败和独有工作不能因本表收敛而被删除。

## 尚未完成的组合验收

各范围的局部测试、bootstrap、scoped CI 和实际交付分别消费自己的固定原件。完整输入、所有生产者生命周期接入、跨平台／外部采用和完整 SPEC 尚未完成。本表没有成功状态生产权；新增结论须由真实生产者和相应消费结果支持，不能从历史绿色、PR 关闭、缓存释放或某一阶段成功推导。

当前资源取舍只对实际输入、操作、测量窗口和单位成立。构建、证明、缓存传输、存储与维护的全生命周期成本仍有 unknown；历史实验耗时不能作为当前性能承诺。
