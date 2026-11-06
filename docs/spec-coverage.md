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
| 1 真实消费者与用途 | 部分：带版本的内部共享启动消费合同 | registration 的 `chrono-internal-causal-consumption/v1`；本仓采用已有 runner-startup，复用 need／output_use、生产／调用 action、profile 安装对、claim、真实消费证据。标准 scoped 准入核两端受影响关系，要求原真实消费者测试；Rust 成功／失败／边界及固定 Git 端点回归 | 是否有用仍由 AI 负责判断。`72b1d08`、`c515cd05` 已观察有限 Darwin detector／startup／install／当前 runner／collection 消费；Linux、外部／全消费者采用、完整路径执行尚缺。仅有测试／分发资产不构成真实消费者 |
| 2 独立项目与采用 | 部分 | 独立 manifest／lock／bootstrap 构建登记；SPEC §2.1 将共享数学归 trureturing、运行／模型对应归 chrono-harness | 通用完整 bootstrap 来源及启用；源码归属不证明实现满足模型 |
| 2.2 实现／测试语言 | 部分：指令、full／scoped projects v2 配对、原生 helper | 中英文 test.same-language 原子用于新宿主默认及本仓指南；项目／脚本明确语言，受影响配对须同 ID 或 shell→python。Rust schema／配对／语言 metadata DELTA 用例及登记 CI transport、context judge、protocol judge、CLI child、生命周期、Cargo、Git fault helper；本仓 v2、Rust 显式 workflow 清单及配对测试、worktree 核心／采用分组及完整覆盖。固定历史支持退役后 projects 保留原未过滤 action；Python 发布配方／测试对不需 manifest，单元／真实二进制集成组独立 | 完整候选验收、示例采用；迁移剩余内嵌跨语言测试／helper，真实外部接口 fixture 单独登记。声明不能证明实际源码语言 |
| 2.2 routes 回归 helper 迁移 | 已实现，范围有限 | 八项 judge-routes-tests 回归改用 Rust 测试自有子进程 helper。`tests/support/tool.rs` 是明确 Cargo binary 和 FILEMAP compile 输入，原 test:judge-routes-tests 计划沿用 Cargo test。保留字面／空／换行 argv、cwd／环境、回执篡改、错版本／执行物替换、PATH、版本观察复用、循环／预检拒绝、标准调用、汇总身份。候选 `af0e2432d849fddbcd233491d428d7322fd32925` 对基线 `c159392f0a9c16049be11933013ab946ddc22293` 的标准 DELTA judge passed，报告 `.chrono-harness/state/preparation/check-a6eeb18c816192599c44d33a6532c4215da67c8d35b700dc5c0671d07ce77472.json` | 仅限此测试所有者。其余跨语言逻辑／helper、外部接口 fixture 登记、完整输入、启用、同判仍缺 |
| 3 五份登记 | 部分 | registration `schema.rs`，G `strict_five_schemas_*`、`real_host_pseudo_script_*` | Cargo／SDK 完整输入；有限配对、routes、迁移、快照、v5 工具链清单已实现 |
| 4 固定快照／上下文／CI | 部分 | runner facts.rs、registration lib.rs；G `context_and_config_identity_mismatch`、`ignored_untracked_staged_and_wrong_checkout_fail` | config v3 已为完整／初始消费者、明确 scoped v2、当前 provider v4 事件读取／fetch 绑定 Git，见 git-facts.md。完整原生 CI／同判及 Git／委托／Cargo／SDK 输入仍缺 |
| 4.2 CI 缓存及增量构建 | 部分：两项采用编译缓存已有键、生产／内容绑定及有限原生冷／保存、兼容源码重建、健康精确命中、不兼容环境分域／重建 | 独立 Rust chrono-cache plan 消费明确消费者／输入／产物，分开源码／兼容键；JSON pointer 只绑定所选值，整文件来源在键外，回归区分单元 metadata／构建参数。v2 探测前解析消费操作引用，只改消费者不改生产键。引用原输出登记，拒绝重叠或缓存证据／源码。Rust 失效、选择、缺失、路径、真实 CLI 环境、planner 不符回归；check.unit_cache 两项绑定真实 action／Cargo，消费命中前核 producer manifest。真实 Cargo 重建／恢复仍属本地证据，见[缓存合同](ci-cache.md)。bootstrap／check／release 明确增量，发布汇总核值。provider 为 main 消费者生成固定 restore／save，保存先保持未确认；可选 GitHub 查询保留原有限进程，区分 key／ref 可用性与创建来源，失败／歧义不确认，无待确认保存不查。声明启动缓存可在后续失败后保存，每键至多一保存 job，冲突在生成前拒绝。[有限原生验收](ci-cache.md#bounded-native-acceptance)保留两项原标准操作／汇总 | C1 部分观察坏内容恢复／重建／替换，rustup 漂移及最新 collection HTTP 502 后，原范围修复后健康精确命中仍缺。Variables／PR 正文控制和专用 fixture 已退役。并发、完整后端生命周期、依赖／当前判官／bootstrap／逐项目输入仍缺，须扩大真实消费者验收 |
| 5 DELTA 图 | 部分 | filemap 类型化合并／记录／传递及 CI 适配；F 全部，P actual_runner_registration_filemap_*。SPEC §5.1 固定两态局部性及定理 6 有条件节点集结论（trureturing PR #10868 已合），复用冻结 ConsequenceClosure，保留先前精确应用编译和旧 execution-edge 反例 | 同源 Verus 图内核覆盖类型并集、名称／槽位索引、FIFO 最短前驱、有限原因链、行投影及原因集合；外层逐 seed／原因关联、JSON 适配、选测和完整 SPEC 对应仍待证明；完整宿主输入及原生启用未完成 |
| 5.2 从消费者需要出发重构依赖 | 部分 | 已有 DELTA 合并／传递／所有者检查。无独立结构报告或自检，未用 SCC／深度／排名／claim 摘要已退役，循环、类型边、有限见证保留 | 真实消费者重构、适用本地／原生验证及核实落地 |
| 6 演进与移除 | 部分 | W 实际替换、共同退役、别名修复、候选迁移认证；registration workflow v3 明确选端点版本。自含 fixture 用当前 decoder 确认真实登记数据转换 | 当前 schema 演进消费者验收，只在真实需要处转换数据，完整启用 |
| 7 协议 | 已实现，有限普通双提交 v1 及明确初始 profile | runner wire.rs／full.rs／initial.rs、共享进程引擎；独立完整 DAG 执行自检，含启动前错误、进程环境、输出回执、伪传输失败、身份／证据变异。R 传输／故障注入，G 真实根消费者 | bootstrap 来源及启用。初始清单不声称 DELTA 绿；平台仅已测 Unix。自检核一致性，不证明判官语义完整或全部运行错误可发现 |
| 7.1 标准日志 | 部分：独立 Rust 库、无效参数入口、runner 分发后发布 | diagnostics JSONL／tracing，专属目的／上下文测试。所有 CliOutput 消费者独立尝试写／flush 两通道，含用法错误；失败尝试一条正确分行 stderr 事件并保留 logging／finish 失败。直接用例和真实二进制回归含只读 stdout 的成功 help，健康字节／状态和原协议／进程通道保留 | 全生产者、非 Rust 适配、登记格式／目的、子进程上下文、统一应急／持久证据发布；目的失败不宣称持久保留。候选标准／原生验收另验 |
| 7.2 异常链与通用处理 | 部分：原生 IO、真实 judge 启动前／报告有类型传播 | diagnostics::error 包装／导入 chrono-error/v1；NativeIoError 保留原种类／消息／OS 码，LogFailure 共用 IO 模块。Unix 原生 writer 保留只读描述符 EBADF；发布错误保留通道／阶段／原业务、独立次要输出／诊断失败。专属原生／无原生／部分／flush 测试。完整 judge 以 chrono-runner-transport-failure/v1 保留准备原因／码、封存请求／judge／operation，区分未启动，保留子原件，晚期报告发布失败关联原原因。真实 IO、协议／schema／上下文变异、次要发布回归，见[诊断边界](diagnostics.md) | `72b1d08`／`c515cd05` Darwin 原流有 166 通过、1 已有忽略及具名 prelaunch。Linux、其他产品原生适配／类型传播、非 Unix 输出、任意自定义原因序列化、协议采用、持久引用、其他次要失败发布仍缺；完整输入／启用／SPEC 未完成 |
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
| 10 manifest 依赖未声明 | 有限实现：可选 Cargo 明确政策、TOML 路径依赖、缺边拒绝；v4 绑定 Cargo／编译器，v5 增 sysroot／后端／链接器／SDK／脚本清单，有直接／完整／scoped 真实消费者 | 编译器库／后端／链接器／SDK 语义输入完整；受保护 registry／Git 解析、完整宿主启用 |
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
| 22 本地／CI 同命令 | 有限实现：当前 scoped v4 生成短命令并明确 push 基线；CI 覆盖重复 push、远端前进、ref 缺失、真实 CLI 保留。C units::short::generated_native_check_step_executes_same_short_command_and_event_rules 执行当前生成的短命令，核真实事件和本地调用；并发单元汇总保留完整输入原件。旧长命令生成及专属测试已退役。[原生证据](native-ci-adoption.md)含固定旧发布的公开 Go／TS／mix push／PR 汇总及每单元，不证明当前候选采用 | 完整有效输入、当前候选原生宿主采用、确定同判、完整启用 |
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

## 已实现范围与证据

登记执行这批改动加入 routes／projects、FILEMAP v2 和固定端点输入，详见[执行合同与范围](execution.md)。routes 专属测试核有序计划、实际 argv／环境／工具绑定、回执篡改、PATH 遮蔽和字节替换。projects 专属测试在已提交的项目／脚本宿主上执行真实 runner→registration→filemap→routes→projects 链，覆盖唯一配对、无 manifest／自定义 action、显式输出隔离、保留输入失败、真实退出和副作用、依赖阻断、文档不执行和替代方法映射。删除测试的映射须执行当前替代计划并实际成功；缺声明在产生副作用前拒绝，生产者和测试一同退役时不执行已删方法。固定历史 scoped 宿主的解码器、fixture、对象获取和迁移测试已退役；当前通用转换使用登记的自含 workflow fixture。回归还覆盖整链继承环境变化（缺失／空值、覆盖和无关输入）、操作前拒绝中间 workspace、两种协议内嵌无效 UTF-8，以及任意操作字节。宿主解释器绑定是明确的 macOS 数据；原生失败原件及修复后的验证结果见 [CI 说明](ci.md)。

Config v3 在获取输入前绑定候选 Git，并将绑定带入七个 full 判官、initial 清单和输入捕获。可选 `chrono-git-inputs/v1` guard 在版本探测前及每次 Git 调用前后，检查点名文件的身份或缺失；调用后漂移仍保留原子进程结果。真实 full／initial 消费者和 runner 回归覆盖采用、未启动即失败、逐调用变异、请求绑定及旧合同对照。专属回归保留 v1／v2 语义和原始进程失败；七判官 integration／delivery 消费者拒绝环境中的未绑定 Git。这只绑定执行物、环境和声明文件，Git／操作系统／委托进程的完整输入仍未闭合。

full 入口的原生平台选择复用严格的 `chrono-git-configs/v1` 解析器和 OS-ARCH 映射，覆盖 full、initial、输入捕获、scoped 可选 registration、full-CI 配置和 worktree 登记获取。`runner-tests::selector_snapshot_keeps_entry_and_independent_endpoint_targets` 检查固定基线／候选目标、两端独立路径、缺目标拒绝和真实路径原字节保留。registration 测试执行选中的 initial 清单；workflow 消费者执行选中的七判官 integration／delivery。输入消费者把真实 capture／pair 的选中 v3 快照传过七判官，覆盖直接配置与映射切换、独立目标路径、历史目标删除、旧端独有依赖／测试边和文档不执行；还执行带 v3 解码器和兼容测试的 schema 迁移，拒绝缺失／替换绑定、改写历史字节或配置。projects 测试以直接 v3 配置作对照，核 scoped 可选 registration 的实际工具／环境、操作及 canonical 调用拒绝；workflow 测试准备并消费选中的 full-CI 命令。

full／initial／registration／mixed／workflow 的现有请求和视图都传递入口身份、实际生效身份及选择器信息。选中的历史视图使用版本化 view／decoder 绑定，输入捕获使用 snapshot v3。这解决了 full 配置入口障碍。worktree 共用的 callback 快照获取在 fetch 前解析源 workflow，并独立解析取回端点，保留完整选择器／实际登记摘要。真实 CLI 测试覆盖：worktree 政策不变而实际配置／workflow 路径不同；保全脏源码；缺失或不支持的端点；选中 workflow 目标变化时不创建检出；选中配置下的重建、恢复和清理，并保留原失败进程证据。直接配置对照仍在。worktree 生产者与绑定 Reader 共用有限的显式登记批量获取；测试将两端绑定到原 metadata／内容字节，在 fetch 或创建前保留真实失败／畸形批输出；Reader 还保留输出分段、guard、身份及旧合同对照。完整宿主启用、有效输入、原生 full 派发、生命周期交付、同判和 Rust 形式化细化仍未完成。

当前 CI provider v4 通过必需的 full-v3 `facts_config` 绑定事件获取，旧 v1／v2／v3 在生成物发布前拒绝，未绑定 Git 的事件回退已删除。`ci-tests/tests/git_facts.rs` 用真实 Git 对象／远端和 CLI，核缺历史获取、精确探测解释、失败 probe／fetch 证据、候选政策／检出绑定、声明环境及生成的 initial profile 选择；`retired_scoped_providers_are_rejected_before_publishing_outputs` 核旧格式拒绝且不写输出。当前候选的原生宿主采用、网络／凭据／配置输入闭合和同判仍待完成。

公开 beta.17 的 `chrono-git-configs/v1` 选择器让 scoped／provider 的 facts 入口按登记选择原生平台政策，保留一个正式入口。Reader 回归覆盖真实 Git 选择、两个固定候选 blob、无效映射／目标、请求替换、调用前后漂移和失败原件。真实 scoped／provider CLI 消费者核源码／文档局部性及 push／PR 获取；直接政策合同保留。这是原生宿主采用的前置能力；该扩展不选择 full 根／initial 配置，也未证明启用、完整输入或跨平台同判。详见 [Git 平台配置](git-facts.md#explicit-platform-configurations)。

Scoped profile v2 可通过 `policy.facts_config` 单独采用 Git 绑定。judge-ci 专属测试用真实 CLI，覆盖环境 Git 被遮蔽、文档不执行、两端、v1 迁移与旧端独有删除、initial 根／子／浅历史、固定政策／摘要／版本错误、dirty／index／checkout／执行后 guard，以及二进制输出、退出、超时和超限原件。真实 projects 消费者通过同一 Reader 核可选 full-registration 和有序执行，检查进程数量、字节哈希并拒绝不符的 canonical argv。v1 对照保留；这批历史证据时宿主尚未采用 scoped v2。

Scoped-v3 汇总允许显式配置 manifest／report 字节上限，默认仍为 64 MiB，原证据验证全部保留。边界消费者核精确上限、超限、无效政策和大于 64 MiB 的原报告。紧凑序列化移除缩进造成的膨胀，保留进程原字节，并让保存报告与 stdout 一致。公开 beta.18 发布这项改动；固定源码原生发布在十五个专属项目中通过 macOS 601 项、Linux 596 项测试，35 个公开资产的哈希／大小均与原件一致，并核匿名 manifest 和两平台安装器。Go 原生采用范围另述；这不证明有效输入完整或确定性同判。

独立 parity 比较器及其合成完整报告 fixture 已退役。现役有效输入生产者都输出 `completeness_proven:false`，没有生产路径满足比较器更强的前提。固定 `spec status` 字符串也已退役：它把 v3 宿主报成 CI v1。SPEC、CLI 文档和 FILEMAP 同步更新。普通报告、用法错误、不可变原件保留、独立输出通道发布和原生 broken-pipe 诊断，仍由现有所有者覆盖。两个命令现在都返回用法错误，不加兼容别名。验证按 runner／registration 显式依赖选取；构建、CI、存储总成本未测。

完整启用及其他未尽义务仍有效。通用项目 fixture 验登记方法执行和所有权；独立 Cargo fixture 验显式采用的 TOML 一致性及有限 v5 工具链清单。两者都不证明完整 Cargo／SDK 输入。单个执行者的检查不证明独立评审、原生 CI 或实际落地。

full／initial／scoped 及 registration／projects 的源码快照获取，现在从 Git 未跟踪输出中排除明确登记的产物目录，与 worktree 共用字面排除项；原始清单 API 保留。scoped 及真实 full／initial CLI 回归覆盖产物超过 Git 输出上限，仍拒绝忽略的相邻文件、字面近似路径、不规范声明和绑定输入漂移。真实迁移消费者允许登记 decoder 输出，拒绝未登记文件和已跟踪输入变更。这只验清单边界。公开 beta.19 的固定源码原生配方在十五个专属项目中通过 macOS 606 项、Linux 601 项测试；35 个公开资产摘要／大小匹配原包，匿名 manifest 和两平台安装器下载也已核。详见[产物清单](git-facts.md#registered-artifact-inventory)和[发布证据](distribution.md)；有效输入完整和原生 full 启用仍未完成。

公开 [beta.11](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.11) 在既有 Git 绑定、CI 生成、远端退役及 recipe v3／report v4 上，加入显式 metadata rebind 和字面检出身份。[固定源码原生构建](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36566708631) 在 macOS arm64、Linux x86_64 各十个专属项目中通过 391 项测试，包含 full／scoped Git 绑定消费者。两份原报告各保留 31 个真实进程、28 个 staged 消费目的地和 24 项一致身份观察。证据仅覆盖这些原生消费者；生成 provider-v3 的宿主采用、完整输入及确定性同判仍未完成。当前发布 workflow 从明确的 `chrono-github-release/v1` 源生成，生成扩展已在 beta.10 发布，所链原生发布实际使用该 workflow，见[发布 CI](release-ci.md)。

公开 [beta.16](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.16) 发布报告比较、显式输入绑定／覆盖、Cargo v5 清单和 Git 文件 guard。[固定源码原生发布](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36655069168) 在十五个专属项目中通过 macOS arm64 588 项、Linux x86_64 583 项测试；差额是既有五项 macOS 文件系统别名测试。两份报告各保留 36 个成功进程、30 个消费目的地和 34 项一致身份观察。在本地 Go 宿主 clone 中匿名安装 macOS 版本，核两个既有单元、配置／缺失状态漂移拒绝、恢复及历史原字节。Git 报告需要显式提高宿主传输上限，见[公开二进制消费](git-facts.md#public-binary-consumption)。这未将本地 facts 配置采用到远端原生 CI，也未证明完整输入、full 启用或完整本地／CI 同判。

公开 [beta.17](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.17) 发布原生平台 Git facts 显式选择。[固定源码发布](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36661720249) 在十五个专属项目中通过 macOS arm64 596 项、Linux x86_64 591 项测试；每平台均有 36 个成功进程、30 个消费目的地、34 项一致身份观察。35 个公开资产哈希／大小均匹配。Go 原 beta.17 候选核过原生 Linux 政策选择，但报告过大导致汇总失败；beta.18 修复汇总，Go PR #18 采用该版并有相符的本地、原生 push／PR 证据。详见[采用范围](native-ci-adoption.md#native-git-policy-adoption)。完整宿主启用、有效输入和同判仍待完成。

这批改动同时加入 mixed 分类和宿主登记。**政策与产品同改警告：**项目配对、语义比较、bootstrap 绑定和执行计划扩大了验证范围；成本未测。专属测试验证 mixed 报告及共用成本消费者，宿主启用仍须完成治理与输入义务。

cost 判官及专属测试实现[登记成本合同](costs.md)：分别保留两端数值和来源指针、旧端独有成本、已知／未知坐标、身份去重、受影响项目成员、纯声明变化和文档局部性。真实子进程消费者核 pass／warn 退出、缺 impact 失败、来源一致性，以及 Git DELTA 为空时的保留环境变化影响。当前明确没有测量值，不能相加得出资源总量或声称 full 治理完成。

mixed 判官实现[规则／产品显式分类](mixed.md)。行为测试覆盖普通成员／边新增、真实 action 变化、两端接口、数组身份和顺序、历史模式、具名 JSON 及畸形输入；真实子进程测试核警告、pass／error 退出、精确成本和来源、纯规则输出、缺前置、陈旧绑定拒绝和严格 UTF-8。稳定性与 integration 认证仍归 workflow，这批只提供规则变化输入。

FILEMAP impact v2 在 route 执行前落实显式 workflow 稳定性和条件 integration 测试要求。测试保留基线要求、旧成本和因果来源；真实消费者证明新增测试实际执行，共享操作只跑一次。具名语义输入错误会停止 FILEMAP 阶段。mixed 分类器保留唯一实现，编译依赖中已移除下游 FILEMAP／cost。独立 [workflow 判官](workflow.md)检查有限范围的新鲜度、已完成 integration、合法共同退役、别名修复和有限迁移，详见[选择方式](workflow-selection.md)。

Workflow 使用带显式 integration／delivery 角色的 context v2。W 执行真实七判官链，覆盖变化／过期证据、缺最终报告、真实回执、跳过判官、测试失败、文档局部性、自定义分支前缀、无效名称及浅历史。runner 传递真实前序进程身份，保留唯一完成报告；cost 同时消费历史定义和当前声明。这些检查范围有限，未证明独立评审或完整输入。

[保留输入生产者](inputs.md)提供真实 capture／pair 快照，区分环境缺失和空值，并在明确根之间传输按内容寻址的状态 blob。registration 流式处理保留／候选文件身份；报告不依赖具体表示。这样，大型工具链文件不再受内嵌 JSON 大小阻断。v5 已提供有限的显式 Cargo 工具链清单，完整 Cargo／SDK 依赖声明和原生 full 治理仍未完成。

宿主中立这批改动移除了强制 manifest／lock／root 字段和封闭 action 词表。`registered_intermediate_workspace_rejected_before_operations_and_standalone_passes` 现归 judge-cargo-tests；旧配对／manifest／route 混合回归拆成通用配对／所有权／routes 与真实 Cargo 缺边消费者。独立 Cargo 适配器支持 `chrono-judge/v1` 或登记的独立前置；仅结构检查仍拒绝外部依赖。另一个登记 run 入口核保留 registry／Git 包、真实 locked／offline metadata、精确 features／aliases／kinds、配置顺序和调用前后身份。`inputs`／`input_consumer` 通过 direct／full／scoped 消费者执行真实 Cargo，含拒绝和纯文档 DELTA。

受 guard 保护的配置清单核显式查找的存在／缺失、有序递归 include 和前后变异。v3 要求在祖先清单与“所有祖先均不存在”声明之间明确选择，并加入宿主相对 Cargo home；v2 解释仍受支持。v4 绑定 Cargo／编译器；v5 加入真实 sysroot、backend、linker、SDK 和 build-script 输入清单，拒绝选择冲突和身份漂移。专属 configuration／inputs 及 full／scoped 消费者核这些路径。委托进程／操作系统／网络的完整输入、启用和原生 full 同判仍待完成。

Config／snapshot v2 将外部文件缺失与空字节分开保留。capture／pair 和真实七判官消费者覆盖两种状态切换、旧边、不匹配和旧版本拒绝、操作后变异、符号链接边界，以及无关输入／文档不被选中。Cargo direct／full／scoped 消费者在执行前后核关联缺失；需要包字节时拒绝缺失 ID。Config v1 的 null 仍是未绑定。Workflow v3 接受宿主登记的 config v1→v2 转换／兼容检查，由两端明确登记的版本选择。真实消费者保留原 v1 快照，拒绝历史配置改写、歧义／不完整选择器、缺失或失败的兼容证据，采用后仍保持纯文档局部性。这是 decoder 合同，不会自动改写宿主配置。完整输入和 full 宿主启用仍未完成。

可选 Cargo 输入政策 v4 将委托 Cargo／编译器绑定到明确关联的输入 ID、路径、哈希和实测版本；v5 加入目录清单及选中 linker、sysroot、backend、SDK、build-script 输入，保留调用前后身份核验和报告。测试执行真实编译器／工具链选择、metadata 前拒绝、配置／include／wrapper 冲突、消费后持久变异及 full／scoped 文档局部性。v2／v3／v4 含义保留；编译器库、委托进程、SDK metadata、未发现的 build-script 读取和完整原生同判未获证明。

显式 initial profile 用 `chrono-initial-judge/v1` 检查单个候选，不虚构基线或 DELTA。默认 registration 消费者保留 proposed／incomplete 状态，核所有 initial 登记引用及检出事实；普通 DELTA 引用选择不变。根清单完成是独立报告状态。runner／registration／CI 专属测试覆盖根、浅历史、Git replacement overlay、dirty／index、畸形／悬空对象、真实进程和协议。initial 父提交检查在新 profile、scoped 判官和事件准备器间共用原始 header Reader。CI v2 源明确声明并生成 initial profile 和 workflow；测试核真实根登记／拒绝、精确事件 argv、v1 行为、漂移和采用冲突。公开[初始宿主示例](examples.md)在 macOS arm64 核 beta.5 的无父首次 push 及后续 integration／PR／dev DELTA。源码／工具链 bootstrap 来源、完整输入、跨平台 initial、启用和确定性同判仍未完成。

登记的 [worktree 生产者](worktree.md)实际获取最新目标、创建隔离分支，并报告源码／进程身份。专属测试用真实远端和 checkout hook 执行公开库／CLI，复用严格登记加载器和有限进程引擎。创建不证明 full 治理、陈旧恢复、PR 交付或落地。采用政策及原生资产构建均已登记；公开 beta.8 在 macOS arm64／Linux x86_64 发布该二进制，原生配方在两平台各通过 49 项 worktree 专属测试。

显式重建复用 worktree 生产者和原进程／身份辅助方法。真实 CLI 测试覆盖目标前进、完整 carry／retire 选择、保全源、二进制／权限模式／链接／增删、字面路径、apply 冲突、源变异、patch 输出上限及空／复用目的地。成功只得到已暂存的 index 树，不产生候选提交或治理／integration 结论。AI 协调和 provider 交付仍未完成；登记维护见下文。

worktree 清洁检查单独观察已跟踪变化，在有限捕获前，从 ignored／nonignored Git 清单排除明确登记的产物目录。回归复现旧产物输出超限，仍能发现未知相邻项、字面／大小写近似名、已跟踪变化，以及占用产物目录名的文件／链接。Git pathspec 环境规范化仅用于这些查询，并逐进程记录；完整 Git 配置及外部输入仍未闭合。

产品原生发布配方按显式顺序消费登记的项目 action，build v2 报告保留预检拒绝和真实进程字节。消费者测试核字面参数、action 歧义、失败退出及失败时不打包。beta.8 包含 worktree；已完成原生发布在每平台通过 15 项 distribution 和 49 项 worktree 测试，匿名下载核 manifest 及两平台 worktree／安装器资产。完整输入和确定性同判仍未获证明。

worktree 登记维护复用 Git／进程、严格登记加载器及产物分类器。真实消费者覆盖 hook 失败恢复、已协调重建、保留原报告、身份／锁／index 拒绝、显式产物处置、祖先／squash 保留、重试、真实 removal／ref 竞态，以及保留 ref 竞态后的 cleanup 锁恢复。这不证明治理或远端落地。基于保留 intent 的 rebind 继续执行见下文；自主 provider 编排仍待完成，缺旧回执／metadata 损坏使用显式 rebind 合同。

绑定回执的临时 ref 清理复用维护回执和已保存提交验证。真实 Git 测试覆盖保全、明确缺失重试、无效身份／保留声明、符号 ref、并发 ref 更新，以及删除后 Git 退出失败。原失败保留，未验证副作用与已核缺失分开报告。它需要完整原报告，不能恢复缺失或中断回执。

已发布的中断检出恢复在创建或获取 cleanup 锁前发布不可变身份，消费明确选择的 intent 及缺失／部分结果身份。真实 CLI 中断测试覆盖创建、重建、清理、原字节保留、拒绝已有终态、身份漂移及发布冲突。它复用已协调检出的释放检查，原结果保持 unknown。缺 intent／metadata 损坏不属此入口；“原进程已停止”是调用方须核的前提，intent 本身不能证明。此入口已收入 beta.8。

中断 fetch 清理在 fetch 前另发不可变身份，不虚构基线或结果。`cleanup-fetch-interrupted` 复用普通直接 ref／预期 OID 清理、显式提交保留和缺失核验；ref 已缺失时要明确重试计划。真实 CLI 测试在 start／reconstruct fetch 后、start 临时 ref 删除后终止进程，核缺失／部分原结果、身份／保留漂移、终态拒绝、发布冲突及删除后证据变化。原结果保持 unknown；此入口不重建缺失身份，已收入 beta.8。四个已落地公开示例在 macOS arm64 验证中断 checkout／fetch 消费者，见[示例索引](examples.md)。

远端分支退役已有 `cleanup-remote` 消费者，复用登记 Git／进程及已保存工作检查。真实 CLI 测试覆盖唯一预期 push 端点、本地／远端目标绑定、精确分支 lease、squash 保留、脏工作保全、删除后失败退出、并发分支更新、目标／计划变化和明确缺失重试。beta.10 已发布；公开 macOS 二进制在核落地后退役发布及四个示例采用分支。远端事务、Git／委托输入、PR／merge／落地编排、full 治理和确定性同判仍未获证明。

Full CI 上下文传输采用 `chrono-github-full-ci/v2` 短投影，旧 v1 在发布前拒绝；长命令生成、旧准备步骤及重试归一化退役。CI 测试复用真实绑定 Git fixture，核精确字节、candidate／config／provider／workflow 绑定、缺历史、失败进程、原件保留／当前视图替换、输出归属和生成 Bash 的字面内容。workflow 测试保留显式 runner 的真实本地与路径消费者，迁移派发到 schema4 短入口，执行七判官的 integration／delivery 并拒绝陈旧上下文。准备阶段仍报告治理 `not-evaluated`、同判 `unestablished`，见[采用和重试范围](full-ci.md)。消费者测试未启用产品 full 宿主，也未执行原生生成派发。

公开 beta.10 的 CI 消费使用匿名下载的 macOS 二进制，核 release／full 生成、漂移处理和精确上下文传输。四个[示例宿主](examples.md)通过真实本地／integration／PR／dev 检查采用固定公开版本，随后核 public-origin start、本地清理和带 lease 的远端退役。证据范围仅为产品和 scoped 宿主；full 生成派发采用、完整输入、启用、同判、AI 协调／PR／merge 编排及形式化细化仍未完成。

显式 metadata rebind 分 inspect／execute 计划，声明分支、HEAD、index 树、原 metadata 成员、backup 和 donor。真实 CLI 回归覆盖 gitfile／metadata／parent／旧回执缺失、index 损坏、带自有 backlink 的相对／损坏指针、字节／权限／链接保全、输入／引用漂移、错误所有权、冲突、原失败进程字节及 donor 部分副作用。已协调的 attached 失败复用 `recover` 并保留原失败报告；成功前重验 backup。此能力已在 beta.11 发布，不能恢复丢失的历史 index。保留 intent 后继续执行见下文；并发／崩溃原子性及 full 治理不在合同内，详见 [worktree](worktree.md#explicit-metadata-rebind)。

字面检出比较修复了已证实的、被配置隐藏的快照不匹配。runner 测试将原 blob 哈希与真实 SHA-1／SHA-256 Git 对象核对，覆盖可执行类别、CRLF 规范化、clean filter、链接模拟、物理父路径和 staged 抵消。full／initial／scoped 消费者拒绝隐藏的权限模式变化，含操作后变化。worktree 在创建／恢复／清理时保全未保存工作，通过登记 Git runner 共用 tree／index 比较器。此能力已在 beta.11 发布；完整输入和确定性同判仍未完成，见[字面检出身份](git-facts.md#literal-checkout-identity)。

源码清理先持有自有锁，只处置明确选中的未跟踪产物，再执行有限 Git removal。真实消费者核副作用顺序、处置前拒绝已跟踪内容、保全外部链接目标，以及原 Git 失败和明确重试。beta.12 起已发布；更早的部分源码删除恢复、并发／断电原子处置不属本批范围，见[产物处置](worktree.md#declared-artifact-disposal-before-checkout-removal)。

beta.11 发布及[四个示例升级](examples.md)都有精确落地和 dev 证据。公开 macOS mixed-host 消费者另核隐藏权限变化拒绝／保全，并在 index 损坏、缺旧回执时执行 metadata rebind，保全已存／未存工作和旧 metadata。该消费者未验中断 rebind。完整输入、full 启用、确定性同判及 Rust 形式化细化仍未完成。

公开 beta.12／13 scoped CI 提供明确的整计划单元归属、共享操作声明、独立 workflow 和离线汇总。先验全局 DELTA 义务再分单元；gather 绑定原 workflow 源、端点和 attempt。[公开示例](examples.md)核独立 SDK profile、真实失败／取消隔离、选择性重试和零 DELTA 汇总。三个业务宿主升级二进制时，CI／SDK 源和生成 workflow 逐字保留。产品宿主当时登记十六个独立整计划 workflow、一个 collector 和明确的三工具 bootstrap 配置。bootstrap 回归核所配操作／安装选择、默认行为不变及失败传递。历史修复和 initial 消费者使用采用的 provider，不重新解释历史定义。beta.15 历史方法替代绑定精确原／现方法及映射所有者，拒绝缺失／过期／重复／未绑定声明，保留字段省略时的严格合同，并实测漂移发现和修复。provider run 有歧义仍明确失败；完整输入和治理未获证明。

beta.13 加入从明确 scoped v1／units 源迁移到 units 的自有投影迁移。测试保全定制初始化、源字节和无关 workflow，在写入前拒绝旧编辑或新目的地冲突。真实公开 macOS 消费者迁移三个已交付宿主并核 workflow 字节。initial／full／release 迁移、多文件原子性、完整输入、full 启用、确定性同判和形式化细化不在保证内，见[定制与更新](ci-units.md#host-customization-and-updates)。

full-v3 单元核心在 scoped CI 和七判官 full 入口复用 runner 的纯分配／选择函数，先验全局义务和操作 DAG 再分单元。integration／delivery 切片核本地真实证据、产生贡献，暂缓完成结论。封存的原请求模板重建各判官 stdin；选中报告在登记上限内，以无损紧凑编码保留进程精确字节及按地址关联的上下文／输入／证据闭包。汇总不启动业务执行物（含版本命令），核保留的真实 success／warning 及声明合同，输出带来源的覆盖，不虚构聚合执行行。普通 full workflow 政策和 delivery certificate 消费者核最终汇总及全部原证据。普通不分单元 full check 保留本地流式 blob 验证，不受可搬运单元／报告上限限制。provider 生成及自动传输沿用 CI 所有者和明确的 full-context／upload 映射。原生 full 宿主启用、Go 采用、公开发布仍待完成；同判和运行提速未建立。

源码 `resume-rebind` 消费原 v1 rebind intent、精确计划和明确缺失／部分／失败的结果身份。首次与继续执行共用保全、attach 步骤。worktree 专属测试在每个受治理 Git 阶段附近终止真实生产者，保留可见工作、原 result／intent／plan 和 backup 字节，并重试继续执行自身的中断。冲突输入、外来所有权、index 漂移、已完成结果和有歧义部分分配都拒绝，工作不丢弃。未知临时文件／Git 锁协调、并发写入、断电恢复由调用方负责，超出此有限合同。此源码扩展已收入 beta.15，见[继续执行](worktree.md#continue-an-interrupted-or-failed-rebind)。

可选 `chrono-input-coverage/v1` 记录明确的消费者／领域范围，每个登记配对必须恰有一项处置。bound／absent 行引用同一消费者的非空绑定；absent 要求登记文件真实不存在。declared-complete 配置不得省略绑定或留下 unresolved 领域。真实 runner 回归覆盖不改基线原字节的历史 v3 采用，缺失／空／错绑／重复行，未知消费者，真实字节与缺失声明冲突，以及 draft／旧合同对照。省略时保留旧 v3 合同。这提供宿主自选输入领域的可检查账目；未披露读取、完整输入、原生启用和确定性同判仍未完成。

simple-fixed 这批通过现有 runner／worktree／CI 所有者，支持 schema4 全局／scoped 短 check 和无值 scoped collection。真实 CLI 测试覆盖远端／HEAD 前进、脏或缺输入、真正短入口、独立单元、零业务汇总、固定值／过期／失败拒绝和本地执行生成原生步骤。不分单元 full 上下文保留原 worktree birth／fork 和精确原生字节。本地不可变轮次复用现在把新观察交给 workflow age 判定，过期则更新上下文并保留原字节。单元文件捕获／验证及逐单元汇总比较共用 registration／input 的显式投影，共享及治理义务仍必须完成。有限源码 fixture 不证明原生 Go／TS 执行。托管原生执行、发布／示例采用和完整 SPEC 验收仍待完成；full 传输 fixture 成功不启用 proposed 宿主治理。

full core／provider／short 源码集成支持声明的 full-v3／v4 独立单元、全局 DELTA 无参数 check、full 原证据汇总，以及经 v2 full-context／upload 映射复用自动 units provider。源码 fixture 不启用本宿主或发布新公开二进制。干净候选准入、原生 full workflow 和完整 SPEC 验收仍未完成。

原生输入准备这批加入版本化流式保留 blob，复用历史单元投影离线组合真实端点对，并加入可选 units/v2 原生采用扩展。生成适配器委托既有事件端点，在 detector 发布唯一 schema2 原始 seed，不轮询地获取其固定生产者绑定，再由真实短入口生产者转发真实获取字段。detector 捕获登记治理输入；collector 将原件与选中单元输入组合，不做业务／SDK 操作。独立当前观察送入 workflow age 判官，不改共用原上下文或早前成功报告。delivery 准备传递可空证据，由 workflow 独自判断是否必须有最终 certificate，九项绑定均保留。见[输入组合](inputs.md)、[外部闭包](execution.md)和[宿主声明](ci-units.md)。

**政策与产品同改警告：**可复用代码／模板与宿主 FILEMAP、验证操作登记同步改变，成本未测。宿主仍选 scoped；发布、主宿主／示例启用、真实原生 push／PR／dev 证据和同判仍归调用方，尚未完成。

<a id="user-directed-conditional-job-generator-correction"></a>
## 按用户要求修正条件任务生成

源码及 scoped 宿主投影已实现：units v1／v2 采用可选 `chrono-job-gating/v1` 后，生成一个 detector、逐 job 的显式条件单元，以及依赖所有 job 的 `always()` 汇总。唯一必需检查是汇总；实际检查仍走固定短入口和最终原报告判官。产品宿主在这批改动中生成一个含 18 单元的 parent，并精确退役 16 个自有旧投影。Git 提供完整树和登记 blob，不设文件数门。旧 provider 和既有公开原生证据保留原含义。

`crates/ci-tests/tests/gating.rs` 用真实 Git／命令覆盖：单项／多项／空 DELTA；两端边／归属变化；删除／改名；超过 3000 路径的完整输入；多提交 push；PR base／head；固定 integration 基线；浅历史缺端点获取且不检出业务源码；创建／删除／结构失败；真实选中单元报告及零操作汇总；失败／取消／跳过／缺 job 或 detector；artifact／source／run／attempt 不匹配；独立 job 重试沿用原前置 attempt；定制／自有退役／init／迁移冲突；已安装工具示例的真实成功／失败短命令。full 声明调度使用 FILEMAP／routes，不取 context／SDK 快照；最终有效输入准入及七判官原证据仍必需。scoped／full 调度在获取解释器前均拒绝依赖 decoder 的历史。

这里没有原生 Actions／重跑／分支保护启用或公开发布结论。原生 full 上下文、原始 birth／输入准备、大型流式产物组合和需 decoder 的历史调度另有边界。产品行为与宿主政策／投影同改，适用 `W_MIXED_JUDGE_PRODUCT`，不声称获得审批或共识。

同项目显式测试组扩展了共用 registration 绑定、full／scoped routes、DELTA 记录目标、配对验证和现有成本节点。产品宿主将 workflow 测试明确分成三组；清单 guard 核组非空、互斥且完整，未过滤发布 action 保留。本地所有者／分组结果只对记录源码有效；原生分组检查、发布平台验证／发布及主宿主／示例 full 启用仍归调用方。这也同时改产品和宿主政策；新增验证成本为三项清单检查、直接受影响所有者测试及三组穷尽执行。

<a id="report-and-evidence-retention-target-101"></a>
## 报告与证据保留目标（§10.1）

PR121 的既有目标继续保留在 SPEC §10.1。运行时只部分实现：当前仅缓存清理会保护生命周期用户和恢复原件，尚未实现通用生产者 intent／输出登记、有限时间／数量／逻辑与实际字节预算、显式引用获取／释放、按当前根回收历史、独立操作间去重、有限清理 metadata，以及每个报告／fixture 生产者的自动恢复。历史引用环本身不能成为保留根。压缩能减少表示字节，不能释放或退役产物。

目标要求在可控写入前准入，如实报告实测或未知占用，超限时保护处理；它不保证任意活构建器下的全文件系统物理上限。必需源码、当前输入／恢复快照、活子孙、未保留提交、失败原件和旧部署语义继续保护。显式迁移、普通入口／无需 finish 的真实回收、实测可删收益和真实宿主采用仍未完成；本次承接规范不改清理政策或已安装行为。

可搬运原证据覆盖加入已发布 v2 codec／marker、Git 观察、完整前序／报告表示、projects／workflow 共用原始流计量、进程启动／流保留和失败参与原件。Rust 测试核二进制／空／损坏记录、旧 inline／v1 与当前 v2 stdin 重建、原预算执行、外层终止后的部分流，并保留 PR165 输入容量。独立 runner 依赖 lock 固定 `miniz_oxide 0.8.9`／`adler2 2.0.1`。projects 迁移和 CI 发布测试从既有登记前置及原生 staging 目的地消费当前源码构建输出。Cargo 政策失败保留原 code／message／政策路径，同时引用候选登记，避免未变政策产生无效 finding。原测试操作、断言、选择器、线程和上限全部保留。

这只是源码基础，单独不足以形成 beta21 兼容发布。Cargo／full-input／scoped／collection DELTA、剩余前置、已提交干净候选的 canonical／native 检查、评审、落地、安装／发布和完整 SPEC 验收仍须后续按序完成。产品与宿主登记同改，`W_MIXED_JUDGE_PRODUCT` 不豁免验证。组合的构建／测试／存储成本仅有源码验证回执测量；物理回收收益未测，仍为未知。

<a id="registered-automatic-lifecycle-cleanup"></a>
## 登记的自动生命周期清理

候选 worktree 所有者实现可选精确产物政策、存活协调状态、成功 birth 登记、显式 import、登记命令托管 use、终态 finish，以及 finish／start／reconstruct／maintain 共用 drain。旧 Cleanup 仍负责整检出的机械清理。本地真实 Git fixture 覆盖全删、受保护／仅缓存工作、use 准入、身份／保留 ref 漂移、链接边界、部分文件系统／Git 副作用、原报告保全及重试。主宿主采用保留 state／bin 的仅缓存政策和生成生命周期指令。真实二进制部署、主宿主 finish／落地及旧宿主回收仍须验收；本地 fixture 不证明这些事件或周期空闲清理。中断的未知 job／状态须显式协调。此处没有通用归档、调度器、进程扫描或并发／断电事务保证，见[合同](worktree.md#opt-in-automatic-lifecycle-cleanup)。

v2 候选加入稳定内核准入／登记租约、无需 finish 的缓存恢复、不可变重试 intent、短 check／bootstrap 参与及明确 Python／嵌套 runner 转交。本地真实 Cargo 回归验证两层 wrapper 死亡后，独立原生子进程仍保有租约。已提交组合、支持的原生平台及完整宿主采用仍待验收。旧 v1 记录继续保护，不补造历史所有权。

自动重试证据采用版本化保留输入引用，就地保全不可变回执，在存活协调者只存一次不透明部分字节。`automatic_retries_keep_original_evidence_without_recursive_growth` 和 `cache_retries_reference_failed_and_partial_originals_without_growth` 反复触发真实终态／缓存失败，核报告增长有限、原字节不变、传递引用漂移／缺失／链接拒绝和部分结果保留；不把失败报成成功。显式 maintain 形状不变。它不处置历史证据，也未部署协调者或完成原生验收。

runner 在匿名启动树中保留单调内核退出观察。恢复后的 monitor 可加入 deadline 前已观察终止的子进程；迟到／缺失证据不会增加执行时长。真实进程协议测试只暂停嵌套 monitor，独立观察子进程退出，区分准时、迟到和输出超限。取消和所有权清理仍走各自失败路径。共用布局拒绝不兼容所有者，参与二进制须同步升级。这不解释其他宿主超时，也不代表候选完整验收。

启动失败也保留启动区间内共享观察器的 receive 错误和不完整 handoff，含最后原 errno／message flags。这些有限观察附加在实际 spawn 错误后，不把共享故障归给某个子进程。macOS 真实回归在嵌套 launcher 就绪后，只耗尽接收进程的描述符表，核原 receive 错误、handoff 失败不变且子进程未执行。原登记错误、fork 分配、取消、并发启动断言保留。这里只补 receive 诊断，历史超时原因仍未解；兼容共享布局要求重建全部参与执行物。

登记调度器允许显式可选启动优先级，同时保留依赖就绪、并发 cap、失败传递、独占资源／输出和 canonical 报告顺序。真实进程 rendezvous 测试覆盖启动重排、未列工作、前置阻断和冲突；schema 测试拒绝无效 ID，FILEMAP 测试覆盖两端变化、重排及删除。空优先级保留旧计划身份，非空则绑定进计划和保留比较。主宿主明确优先 workflow 前置及长测试链，cap=2。时限是否足够、已提交 canonical／native 验收、部署和完整 SPEC 交付另行验收。

release recipe v5 使用显式构建子集，且每个 verifier 必须明确布尔 Rust 工具链选择。只接受当前 v5 schema；旧顺序／v4 路径及专属 fixture 已删，原失败、证据上限、字面路径、staging 断言迁到现任测试所有者。Python 验证无需 Rust 工具绑定；真实 Rust 资产消费者保留声明的构建依赖，消费时不要求编译器。调度器依登记依赖和有限并发执行；collector 核每个选中生产者、工具证据、进程原件及完整单元成员。Rust 行为测试仍是 Cargo action；Python 行为／驱动／断言和外部 fixture 各有登记所有者。这不证明原生候选验收、公开发布、其余 helper 迁移或 full 启用。

宿主 bootstrap 有明确 Python 生产／测试脚本配对及独立 Python SDK／操作 fixture。独立 CI／release 单元直接执行登记 Python 测试，核源码身份、所选配置、已装摘要、工具链安装和失败传递。Rust 生命周期集成测试保留真实 bootstrap 边界。可写环境现在只解析到既有 cache 产物根，三个采用 profile 一同迁移，原报告／流仍在 state。一份原 release-unit ZIP 测得可重建工具压缩字节为 179,203,963，其他成员为 636,057；候选原生载荷和耗时收益尚未验证。候选／原生完整验收、剩余语言所有权和公开采用另行完成。

bootstrap 的可选显式 `report_path` 将宿主 cache-tool 记录与后续 core-tool 记录分开。Python 回归核连续 selected／default bootstrap、无效或符号链接位置及写入失败。core／cache／default profile 均要求并声明 Cargo／rustc 执行物摘要和版本；bootstrap 解析所选 rustup 文件，拒绝 PATH／配置／编译器选择不一致及持久执行物漂移，记录实测路径和生效编译器。干净源码身份附带原操作命令、执行物摘要、退出及 stdout／stderr 字节；失败操作保留自身原件。

共享 startup 安装前按源码声明核这些原件，独立检查各 profile 本地选择，并在 install／verify 前传递和重验绑定；预检失败保全已装工具，身份相同的等价搬迁可接受。cache 传输报告记录实际报告执行物的摘要、路径和版本，Rust CLI 回归与所调二进制核对。测试宿主 panic 时保留失败探测输入／原报告，成功 fixture 清除。现有观察支持保留干净 bootstrap 记录的源码绑定检查；backend、linker、SDK、build-script、Cargo 配置／环境及后续独立 Cargo 执行仍未闭合，native／full 宿主验收也未完成。

mixed 判官的陈旧成本消费者采用登记 Rust fixture：调用真实 cost 判官、核退出，只破坏 candidate-tree 绑定。原 mixed 拒绝和空输出断言保留；其他外语 fixture 迁移仍未完成。

runner CLI／Git 输入测试采用原生 Rust 子二进制和明确 JSON case 数据，保留协议／状态不一致、原 UTF-8／流哈希、pipe／EOF、调用路径、真实退出、Git 输入漂移和畸形批输出观察。Git clean-filter 配置通过 Git 命令接口调用登记 Rust fixture，行为由 Rust 实现；CLI response case 直接调用编译 helper。必需执行别名采用完成构建字节，不留可写启动描述符：macOS 用独立 file clone，其他支持 Unix 用普通 hardlink。Git 探针位于待测快照外的相邻目录；工具漂移 case 先替换自有链接再写变更字节。其他所有者语言迁移、原生验收和公开采用不在此范围。

worktree 所有者已有显式 cleanup 政策迁移候选源码。它在独占内核准入及已提交端点验证后，保留原登记／birth 证据，另附单独绑定的政策切换。Rust case 覆盖新输出、活租约排除、政策漂移、保留输入损坏、中断采用及终态证据保留。此候选的 canonical 本地检查、native 验收、兼容二进制部署和真实宿主迁移尚未完成；它不处置产物、不交接终态，也不搬移协调者／state／policy 地址。

共享 startup 有明确 provider 合同及宿主 Python 实现。现有 detector 发布一次；27 个登记消费者传递原 artifact ID／绑定，核源码／配置／平台及全部文件，保留生产者原件和 import 来源。Rust 投影测试核顺序、重跑 ID 传输和 cache-step 身份；Python 故障 case 覆盖缺失／变化绑定、源码或平台，后段载荷损坏、链接和发布失败，不声称本地重建。原生传输／重跑验收、完整编译器／SDK 来源仍未完成。历史超时原因仍未知，有限回归不证明普遍发现能力。

scoped check 发布支持显式 retained-reference 政策。runner 保留完整原件，在登记位置只发布路径／摘要；既有 collection 判官按声明的本地／原生传输解析引用，先核原摘要、路径、大小，再验结论。Rust 回归覆盖本地重复、原件缺失／变更、无效引用和传输损坏，汇总不重跑业务。宿主采用需要兼容的生产者／消费者二进制；canonical／native 验收和归档成本实测另行完成，其他报告重复及完整 SPEC 未由此解决。

显式 v2 政策将判官 stdout／stderr 分别保为精确原件，从已核 stdout 解出 response；metadata 移除内嵌文字、数值字节数组和另行序列化的 response。collector 累计限制 metadata 与两流总量。Rust 测试覆盖非 UTF-8 往返、业务／传输失败、损坏、移除全部本地原件后的缺上传，以及汇总零业务重跑。宿主采用仍须同一候选及原生检查；嵌套业务回执／获取重复未被移除。

worktree 的既有 migrate 入口加入历史 main 当前保管切换，保留原生产／处置结果，绑定精确操作者声明，复用已采用的排除与 cache drain。中断 use 协调可追加 use 历史并移除陈旧 token，可变登记／use 字节并非无条件不变。Rust 行为 case 覆盖选中回收、活消费者／源码／未知保护、已发布切换恢复、部分处置及普通重入。此源码改动不证明 true-dev canonical／native 验收、报告保管采用、PR 交付或完整 SPEC；真实回收／部署观察归调用方保留证据。

精确证据别名退役保留失败原件，并要求其引用的保管声明和权限持续可用。`removed` 只数完整核验完成项；unlink 已发生后，后续核验仍可能失败。原生 gather 不改下载报告，当前引用发布避免重复正文。这些切换不能补回历史缺失的 861 个 stdout 相邻文件，也不改原 115 通过／5 失败结论。历史证据限制与当前候选检查、原生／公开采用和有序交付分开。

四个有限 §2.2 回归消费者现用 Rust 驱动、断言及请求绑定子行为：registration 的 initial DAG、具名 warning 输出、缺失／冲突 response，以及 projects 的内嵌 UTF-8 边界。registration-tests 自有二进制消费真实封存 initial／full 请求；既有 projects 测试二进制加入两种编码模式，保留字面 Step 副作用。registration helper 断言独立前序为空及成功 registration 前序精确相同，并保留真实外部 `/usr/bin/python3 --version` 的流与状态；该工具观察是外部接口，不是 Python 断言或驱动。无效／错误请求和不支持模式不得返回 pass；畸形 stdout／退出 9、冲突 impact、null tests、相同 cost 来源及精确 EF BF BD／FF 字节仍有区别。

两个 helper 的 manifest 和 FILEMAP 编译边绑定专属测试所有者、现有 Rust runner 依赖、登记操作和未知成本。这四类证据保留在既有 test target 产物中；行为通过或源码 token 测试不证明实现语言合规。直接源码验证通过登记 registration action（含 initial-DAG 两分支）、projects core／完整清单 action，以及新鲜历史迁移 action（含真实嵌套 CI 测试、workflow 漂移／恢复）。既有 FILEMAP 消费者、未过滤 release action、迁移断言、依赖、cap=2 和登记时限保留。历史迁移须新鲜生产结果，空历史结果不是终态。托管 use 保留 2400 秒及 64 MiB 流上限；迁移测试阶段耗时 1246.21 秒，不能证明符合另一个 900 秒 scoped 计划。其他 fixture 族、完整 §2.2、已提交 canonical／native、交付、完整来源／输入／同判及成本比较收益仍未验收。

<a id="native-bootstrap-selection-and-real-consumer-custody-boundary"></a>
### 原生 bootstrap 选择与真实消费者保管范围

core／cache／default／detector Python bootstrap profile 共用 `.chrono-harness/ci/bootstrap-toolchains.json` 中明确的 Darwin/arm64、Linux/x86_64 执行物／版本预期，绑定实际 rustup 选择和生效 rustc。官方 manifest／component／member 身份提供独立准备来源；验收预期不从待验安装自身推导。共享报告将登记表绑定为源码输入。相同字节允许搬迁，原个人路径可缺失或过期；精确路径、当前选择、wrapper 和执行物漂移仍拒绝。

真实 Python fixture 生产者在稳定 attempt 路径保留成功／非零／超时的终态流和源码／报告／描述符原件，最多一个 unresolved attempt。后续调用只有核过前次成功流及所有者原 release／disposal 回执，才可开新位置；仅成功状态标签不能释放占用。调用方绑定源码的现有 worktree 所有者在生产前登记新 primary fixture；一个登记托管子进程覆盖完整消费／断言期。既有 main-cache release／disposal 回执证明排除后精确输出回收。Python 核真实所有者原件，不制造生命周期成功。缺所有权或活／未知消费者会保持 pending 保管并阻止堆积；历史失败既不采用也不删。合成 Python 成功／失败／超时／活消费者 case 调现有二进制，在保全源码／证据时回收自有生成输出；它们不证明真实 Cargo 消费或通用断电恢复，已提交干净消费者保管仍待实际运行。

真实消费者 fixture 另留 setup-recovery 结果，供源获取命令均已加入、但 owner 管理尚未开始的失败 attempt 使用。当前候选 owner 明确 import 未变且干净的物理 main，取得原独占 release，登记输出必须仍缺失。失败 attempt 和流保持原字节／失败结果；准入核两份 owner 原报告及精确失败结果绑定。running／timeout／unknown-command／源码变化／未知输出／恢复不完整的 attempt 继续保护。已安装源码及 owner 字节预检先于位置预留。**产品与宿主准入同改：**可选 main import 获得此仅缓存含义，本宿主真实 bootstrap 消费者选择该恢复。它不代表托管生产成功或通用中断 setup 恢复，仍须本地／原生候选验收。

**登记与运行行为同改：**源码预期输入、FILEMAP 消费边、canonical 必需输入和 cache 兼容声明与 Python 行为一起改变。直接 Python 回归只证明源码这一段。已提交干净源码的真实 cache／core／shared／独立 runner 执行、canonical DELTA、Darwin／Linux 原生消费、生命周期闭合、评审和实际交付仍必需。完整 SPEC、CPU 加速、full 启用，以及 compiler／backend／SDK／输入未闭合范围不变。

Q2 在既有 registration／routes 所有者加入可选逐操作合同，scoped DELTA 和离线汇总共用合同。生命周期 drain 经现有输入验证器引用保留子原件，支持的内嵌历史及原托管命令流／退出仍可读。projects 迁移 fixture 明确绑定历史 commit／tree，用当前代码获取缺 metadata／blob 并保留原 Git 诊断。原生历史 fixture 修复用显式平台 fixture、有限且预先登记的本地／原生 Git 身份，以及无远端的临时 bare 对象视图作 probe／读取；只有明确有限 fetch 才访问原检出 origin。cold／warm／error 原件及全部历史解码、别名／歧义、workflow 修复检查保留。本地通过不证明 Linux／macOS 原生通过，合同及 Rust 回归见 execution.md／worktree.md。

产品源码与宿主测试输入政策同改：FILEMAP 声明历史 manifest／helper、平台 fixture 消费者及 routes 测试真实 registry 输入。有限原生修复同时改 fixture 实现和政策，不扩大输入治理合同；cap=2、操作／输出／CI 上限及 CPU 加速不变。已提交 canonical、两原生平台、评审和交付仍归调用方。Q3／Q4 及完整 SPEC 未完成。PR #169 报告找不到的九份旧本地证据原件和三个工具二进制仍未找到，当前通过不能补回原字节。

落地后 bootstrap fixture 修正保留成功的空 runner DELTA；单元未选工作时，另执行既有 `test.runner-tests` 登记 action。若已选执行，仍须有自己的 Cargo pass，复用结果不重跑。Python 回归区分两路，拒绝缺失／失败选中执行，保留 direct-action 失败流。修正前干净 Q2 源码曾在本地以非空 DELTA 消费，这不认证修后源码或已合 dev 行为。已提交修后源码消费、原生检查、独立评审和落地仍待完成。

**fixture 行为与宿主登记同改：**FILEMAP 现登记 fixture 的直接 runner-tests／Cargo 依赖，不增加操作、输出、CI 上限。发布计划预留 `v0.1.0-beta.27` 给获准 Q2 及此修正，不表示已发布、远端验收或完整 SPEC。空选择新增 Cargo 成本和原生总成本须实际运行后再测。

L1 fixture 保管候选在既有保留生产者所有者处，加入预先分配／输出及复制成员归属、精确原生 stdin／EOF／流捕获、真实子孙能力、嵌套诊断引用，以及 worktree／CI／projects／workflow 共用和历史迁移发布者的离线搬迁。Git 清单只加诊断观察，不登记或删除。产品与有限宿主 fixture 政策同改；Git v2 身份选择、worktree v3 不变。release／migration 合同归 worktree。源码适配不证明 canonical／native 验收、部署、物理收益或 SPEC 10.1 完成。最终原件有限保留及共享副作用组合、精确兼容输入／原生交接、历史 release／ci-persistent-cache 迁移和落地仍待完成，全部生命周期成本未知。

L2 最终原件候选将固定 PR172 源适配到当前 report、preparation／blob、scoped 原生捕获、full-check、生命周期和 gather 所有者。当前 slot／reader、原流依赖、失败／中断重验根及精确 upload／recheck release 共用演进后的 Lease 和物理副作用所有者。fixture 副作用核显式重叠的最终原件成员，fixture 保管仍独立。PR207 报告引用、launch／stream 接口和共享原生别名保留，见 artifact-retention.md。

**源码与宿主政策同改：**候选为未来输出选择零秒保留和每轮 25 ms 上限，数量／字节容量不变；这不采用历史记录。旧结果／到期、未知／中断消费者、512／493／19 及 OS39／OS54 原件、ci-persistent-cache 迁移不匹配仍未解决。所有者局部源码／行为验证、组合输入／原生验收、独立评审、部署、落地和完整 SPEC 各需真实证据，总成本未知。

生产者准入现在只读取一个经排除保护的登记快照；登记缺失才进入物理 birth，预期租约身份缺失／被替换时保留保护。解码直接构造类型化对象，动态映射仍严格拒绝重复／未知字段。紧凑登记替换沿用持久发布，历史带缩进 JSON 仍可读；保留诊断区分真实根保护原因。即时创建在既有持久 reservation／materialization 写入间复用同一准入账本和租约，免去重复全量加载；独立 plan、完成、恢复和获取上限不变。

Rust capability fixture 用独立测试进程，断言、预算和明确内部并发不变；受控 fork／exec 边界核临时继承保护和真实 last-close 退役。先前失败原件和缺失嵌套清单仍是历史限制。候选 canonical、独立评审、原生组合和落地各需实际证据；此修正不采用或迁移历史政策。

<a id="bounded-input-source-reconciliation-i1"></a>
### 有限输入源码协调（I1）

observer 获取、明确原生／准备源及 Cargo v6／directory v3 行为已与当前传输／进程所有者协调。登记的 host-bootstrap→隔离 guarded-runner 消费链、14 个安装参与者、34 对 fetch／metadata 仍明确列出。通用 Rust 行为 case 和 Python 准备／运行／转交 case 保留；源码登记本身不是执行结论。当前宿主绑定／采用、受保护已装参与者消费、候选 canonical／native、落地、完整输入及同判仍待完成。PR163 保留源码、可选当前方法处置和 scoped 剩余项见[保管说明](input-source-custody.md#remaining-consumer-prerequisites)。

精确源码 `51d85d833e4fdf9f7a607af901bfa52d3e64939a` 的原生 `41021444210/1` 为对应 95 个当前操作中的 37 个提供资格证据，余下 58 个在该组中只有登记、没有执行证据。当前方法保留 94 个直接 Cargo 配方和一个 diagnostics guarded build。diagnostics 的 27 节点原生 metadata／offline-build 结果只覆盖本领域。普通 full 输入、隔离 home 一致性、本地 diagnostics、七项 full 绑定、default／portable／distribution 采用及历史原件仍是 P1–P6 义务。合格 startup 产物 `12961832338` 含 18 个原成员和五个执行物，只支持有限 native core／detector 供给，不证明 default 的 14 工具安装或完整 SPEC §12.1。PR163 作为历史保管事项已关闭，当前义务在 PR214 继续；历史兼容不是当前支持义务。

I3 将所选 macOS 27 runner guard 的 2398 个字面输入 ID 连通，保留 2387 个包文件预期，并选择私下获取的精确 Rustup 1.29.1。当前 SDK 和外层 Xcode Contents 清单保留真实 Developer 别名拓扑；原 Developer-only 拒绝仍是失败。源码迁移未启用 full 治理，也不认证历史保留端点、原生获取、guarded 消费、评审或落地。候选实际检查及已装消费者后置条件须各有固定源码原件；完整 SPEC 和确定性同判未完成。

I3 实测失败修复加入明确的配置级 FILEMAP 边投影和可选 scoped-CI 图证据，不重复存放派生见证表。原生 unknown-input 边错误及本地 8 MiB 传输失败仍保留。当前获取直接声明外层 Xcode Contents，历史 native Developer guard 数据不采用。fixture 自有编译器选择与真实父进程 RUSTC 隔离。源码存在、行为测试本身不证明新鲜 native／guard 验收。

<a id="bounded-combined-runnercacheconsumer-increment"></a>
## runner、缓存与消费者的有限组合改动

组合候选保留 runner 诊断、缓存内容和内部消费实现的完整历史。真实消费者是当前 judge／report 边界、既有 `check.unit_cache` 生产／恢复路径，以及 detector shared startup→已安装 core／runner→canonical runner 单元→原件汇总。scoped 合同只为这条内部链增加一致性和证据义务，历史消费者保持原范围。严格 judge-v1 不变；可选版本化 runner 报告扩展保留历史数据解码，不给未出现的子进程补结果。

**历史产品与宿主政策同改：**runner 传输／报告诊断、选中 cache 生产者绑定／内容清单、内部因果准入与有限原生验收 fixture 曾组合在一起。两个既有 unit-cache 编译入口和一个 runner startup 链继续采用。可选 manifest-removal fixture 及两条远端激活传输已按下文用户修正退役。早前精确 run／attempt、checkout-source、merge／head 回归证据仍附在固定提交，只作原范围证据。普通内容验证／恢复和真实 build／seal／save 消费者继续有效。

实现交接提供候选 bootstrap 和已提交干净 DELTA check 原件，含真实失败与修复结果，覆盖登记两端选择、真实源码消费和候选二进制；精确结论以生产者报告为准，覆盖表不手填 passing。原 macos-26 `39688684172/1`、`39690377913/1,2`、`39695762671/1` 按 cache 所有者记录范围，证明两缓存的 cold／save、兼容源码核验后重建、健康精确命中、不兼容环境分域／重建。最后一运行通过 28 个 job，collection 核 26 份原单元报告，通过 27 行结果。生产与场景 runner 流各保留 166 项通过、1 项既有 ignored 测试，含具名 prelaunch case；startup import／install 绑定匹配干净源码和真实生产 attempt。

普通 revert `c72c141` 精确恢复生产 `72b1d08`，场景历史保留。最初 Variables GET／list HTTP 403 阻止当时激活。后续 C1 在两入口部分观察到坏内容 save／restore／拒绝／重建及替换缓存；rustup 身份变化使修后原域健康精确命中未达成，最新 collection 返回 HTTP 502。退役和这些局部观察都不证明完整原生验收。独立评审、Linux 验证及落地仍必需；公开／全消费者采用、完整输入、形式化细化、full 启用、完整 SPEC 和同判未完成。无新增第三方依赖。本地构建／测试窗口与编译、原生运行、传输／存储和总生命周期成本分别记录；完整总量及增量字节未知。

<a id="user-directed-remote-activation-retirement-and-principles"></a>
## 按用户要求退役远端激活并调整原则

**产品与宿主政策同改：**仓库 Variables／实时 PR 正文的可选 cache 激活、manifest-removal CLI／配置／fixture，以及专属 provider／test／FILEMAP 声明已退役，不增加替代传输或权限。真实消费者仍是普通 cache plan／restore／内容验证、恢复、候选构建／测试、producer seal／save 和原结果报告。产品默认及宿主采用的双语规则叶，现要求局部失败后继续独立授权工作、只用现有权限，并按实际需要收窄消费者／IO／进程／DELTA 验证。生成指南仍是投影，不证明强制执行或翻译同等。

历史来源与证据保持原范围：Variables 原 HTTP 403；后续 C1 两缓存的坏内容 save／restore／拒绝／重建及替换；rustup 选择身份变化，修后原域健康精确命中未达成；最新汇总 HTTP 502。退役不证明原生验收、落地、完整输入、同判或完整 SPEC。原生运行／存储／生命周期收益未知，候选检查和命令耗时以保留交接报告为准。

<a id="lifecycle-composition-acceptance-boundary"></a>
## 生命周期组合的验收范围

固定生命周期、已完成输入 `cf7ee4d8` 及已评审原则 `24949168` 的源码历史已组合，保留独立 fixture 保管／最终原件释放、配置级图选择和远端 cache 激活退役。类型化进程／full-report 诊断保留原失败，采用的发布保留精确修复根和 slot 排除。这是源码组合，尚非交付或共用验收。旧生命周期 CI／projects 登记准入、runner 退役失败、8 MiB 不完整全局 response 和历史迁移义务，须由真实组合源码检查给出精确结果后才可解决。不增加纠正分配、不抬上限、不减弱断言。

<a id="composed-input-and-lifecycle-consumer-correction"></a>
## 组合后的输入与生命周期消费者修正

**产品与宿主登记同改：**组合候选保留中断输入原件恢复、新前缀／双线程目录验证、runner 报告消费完成和原件保留生命周期合同。可选隔离观察沿用登记所有者及当前宿主输出路径。输入预期身份、产品／准入／维护／累计输出上限均不变。worktree stdout 改为紧凑 JSON，所有解码值和原捕获身份保留。仍须显式两端 DELTA 检查及真实 bootstrap 消费；先前生产验收只对原固定源码／范围有效。独立评审、原生验收、落地、协调者部署、完整输入／同判及其余 SPEC 各自验收；新增验证／存储成本仅以保留命令回执为测量依据。

<a id="native-diagnostics-cargo-domain-repair-source"></a>
### 原生 diagnostics Cargo 领域修复源码

首批 PR211 原生原件显示：私有 diagnostics-only offline home 中 21 个普通 Cargo check 失败，guard bootstrap 在 diagnostics 生产前失败。普通原生配置保留 dev 环境及当前 action 必需的 guard／observer 工具声明；既有 guarded diagnostics action 选择自己的原生配置，保留 v6 offline／package／toolchain／native-target／configuration 合同。guard／observer bootstrap 前置先用普通 Cargo 获取自身 locked manifest，再做私有 diagnostics fetch／metadata 和 guarded 消费。共享 startup 只传工具及原报告，不传 Cargo registry。

**宿主政策与消费者同改：**私有环境范围收窄到真实 diagnostics 消费者。Python 所有者测试通过 bootstrap 进程所有者和原始流 fixture，定义冷前置、选中 home 一致，以及 cold／wrong-home／wrong-lock 的失败预期。新增源码、manifest／lock 输入有明确 FILEMAP 边。这些预期不等于执行结果；兼容候选 owner 部署、登记本地检查、原生 guard／build／test／普通消费者验收、独立评审和落地仍必需。

后续取得资格的 managed-use 原件显示同一个普通 fixture cache-bootstrap offline 领域失败；没有生命周期功能改动，修复后的真实 fixture 仍须验收。额外 guard selector 读取及有限 fixture 操作的总成本未测，不提高预算或扩大获取组。完整输入／同判、Linux／公开采用及完整 SPEC 仍未完成。

受保护普通 owner bootstrap 修正加入不可变 produced first-issuer 核验、保留候选构建和原 owner EX 发布／采用。登记排除修复用真实原 admission，加上选中物理协调者所有未解决普通登记的 EX（含 linked attachment），替代不受支持的宿主 scope 断言；回执记录受保护身份。具体受影响所有权未知、旧版或不可用时仍拒绝，其他物理协调者不在范围内。stage／源码 fixture 证据不证明正式 bootstrap 恢复、当前两端 canonical、原生采用、独立评审或落地。精确保留及产品／政策同改范围归 worktree 文档。

固定修正 `d04bf3004d17f438bfcbadde3bd4d85023eb08c3` 通过 3 项 Rust／68 项 Python 定向测试及真实 51 项保护下构建。正式 bootstrap 仍因不合格的 unmanaged 消费交接退出 2，已装 owner `985e45d9…` 未变。这只是固定源码证据，本说明不新增候选 canonical、采用、评审或落地通过结论。

<a id="native-diagnostics-retained-build-inventory-source"></a>
### 原生 diagnostics 保留构建清单源码

**可见宿主政策变化：**native diagnostics 选择固定 directory-v3 清单，含 158 个合格共用 Rust payload 文件及原 22 个 directory 行／根／别名。旧 159 文件 raw 身份还绑定 `lib/rustlib/components` 内容；新 build certificate 不覆盖该内容、成员语法、类型或原内容漂移，但仍要求该子项名称／存在。完整 raw observer 输出不变并独立保留。[原生采用合同](inputs.md#native-diagnostics-originals-before-guard-adoption)记录精确保留摘要、源码范围和排除项。通用 observer／guard／startup、普通前置、私有 offline、macOS 27／default profile、configuration／package／SDK 及子进程后保留漂移合同均保留。

这不证明 registry 成员／类型、仅顺序差异等价、完整输入或同判。本次源码／数据修正仍须自己的候选 canonical、native guard／build／tests／aggregate 原件、最终独立三方核验、落地／后验及干净 dev 同步；历史 J／N 结果仅对原范围有效。验证／CI／IO／存储总成本未测，raw 观察成本及资源上限不变。

当前 scoped CI profile 在清单、准备、操作及后快照采用已有结构化日志。request／plan／operation、真实 launch 和既有 wait／join 阶段，会在最终汇总前送入保留 judge stderr。Rust v3-unit／retained-engine 定向测试覆盖这个有限消费者，见[采用与限制](diagnostics.md#scoped-ci-execution-progress-adoption)。固定原生超时根因、已提交 canonical／native 验收、更广 SPEC 7.1／7.2 采用和完整 SPEC 交付仍未解决。

当前生命周期和最终输出实现使用按地址区分的 enrollment、allocation、output、consumer 记录。独立对象不共用可变成员总表或独占准入。全局输出数量／字节配额已移除，关系集、单输出、单轮限制仍在。普通入口维护自己的 worktree 及声明依赖；显式 maintain 保留繁忙／损坏记录，继续独立对象。共用执行物替换仍排除真实用户。保管、迁移、中断和 owner 部署消费者已用此存储；旧活状态须按文档受保护切换，不设运行时回退。

产品和宿主政策同改。干净候选 `21a5e6acebabe285fbec0bbe821e297327d51f98` 通过源码 bootstrap（12 操作、6 工具）、本地 canonical worktree 单元（215 测试，其中 6 项归 adoption 组）及原生 `41950114155`（24 个业务单元及 collection 全过）。下载汇总归档匹配 provider 长度／摘要，原报告及流摘要核验通过，25 行聚合结果全部通过。登记 runner action 另通过 241 项测试，1 项既有 ignored。

PR #221 落地为 `3b605cf80aa9e96a6442b2e54991602713acca31`，树与受验候选相同。随后 PR #227 落地为 `2ec6e5ce6c16cd6e56433729a06db7a4cab3ba49`，落地原生 `41965898150` 核过全部 13 项选中汇总结果。原主检出的九项登记已完成受保护状态拆分及共用 owner 部署；普通 bootstrap 通过。无关旧 worktree 的 metadata EX 租约持续持有至完成时，另一个 linked start 仍成功。该 linked 检出的同端 canonical check 未选业务测试，不能重复证明早前 worktree 测试结果。旧原件及两份独立独有提交保留；其他协调者仍须各自受保护切换。完整 SPEC 交付和运行／CI／存储总收益未建立。

原生 CI 输入准备迁入 Rust `chrono-ci native-adoption`，扩展改为 `chrono-native-adoption/v2`，不生成 Python 适配器、不要求解释器。原时间、传输与有界失败预期迁至 `ci-tests` 的 Rust 配对用例，完整 seed／转发／交付消费者同步采用。Python 模板、专属脚本登记和重复发布验证 job 退役；真实 Python 业务／外部接口与其余 bootstrap、shared、release 迁移尚未完成。

该批源码与政策同步交付于 PR #273，dev 落地 `1cc909d9999baed15bd2f43135debe85e1abe605` 的文件树与受验候选 `69c1d8541614cdd6cd58a99f9acd06099d917560` 相同。PR 原生运行 `42265991709` 和落地运行 `42269755637` 均成功，原汇总各 21 行全部通过，归档长度／摘要、原报告和输出流摘要均已核对。这些结果覆盖对应 macOS scoped DELTA，不能证明 Linux、两平台公开发布或完整宿主采用。

测试清单验证由 Rust `chrono-ci test-inventory` 负责，两个宿主配置继续显式绑定原 Cargo action、分组和二进制清单。Rust 配对测试保留完整／互斥覆盖、ignored、漂移、损坏清单、非零退出及有界失败原件；worktree 的真实后代租约消费者同步调用 Rust CLI。Python 实现、导入／字节码测试、独立 CI 单元和重复发布验证任务退役。程序与宿主政策同时改变；当前候选的正式检查、原生结果和 dev 落地分别需要实际证据。

## runner 的 Verus 分批迁移

实际完整 OID、CI 单元 ID 与相对词法路径判断已迁到同源 Verus，生产调用方直接使用；
独立证明根覆盖零值／长度／大小写、ASCII 与路径分量边界。实际 CI `Status` 也由同源 Verus 定义，比较和克隆保持四种变体身份，独立根列出全部 16 对比较预期；Serde 与外层转换仍未证明。
登记计划强制 runner 六项及启动准备两项整 crate 验证，
再运行原 Rust 库存。范围和固定工具见 [Verus 实施范围](verus.md)。
其他程序、调用适配器和全仓强制覆盖仍未完成，Python 实现暂缓。
