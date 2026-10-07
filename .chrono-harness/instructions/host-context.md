# chrono-harness 宿主上下文

本宿主的开发分支通过指向 dev 的 PR 运行检查，integration 分支同样使用其 PR 检查；push 仅检查 dev 的落地 DELTA，不重复触发整套开发分支检查。parent workflow 按 PR 号替换该 PR 的旧运行；push 按各自 run ID 隔离，保留每次完整 before/after 义务。

本仓维护 Rust harness 产品与独立宿主采用数据。SPEC.md 是尚未全部实现的产品合同；当前生产者、调用与限制见 README.md、docs/spec-coverage.md 及对应专用文档。34 个独立 Cargo manifest/lock/target 构成生产／专属测试配对，没有根 workspace。

产品默认归 assets/instructions/catalog.json、default-manifest.json；本宿主独立拥有 .chrono-harness/instructions/catalog.json、manifest.json 和本上下文。产品资产是编译／测试输入，宿主数据及已存在投影是运行输入；FILEMAP 的 owner、输入、消费者与边仍显式登记。升级二进制或重复 init 不覆盖既有宿主选择；采用新默认须明确编辑宿主数据。

当前指令 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。116 个 atom（100 个双语内容叶子、16 个聚合）。产品与本宿主根明确选择 22 个叶子及 workflow 五节短流程；完整 core.general/general 仍可用于自定义输出。本宿主 general-en 输出含 100 个叶子，repair-skill 仍为原 7 叶子闭包。CLAUDE.md 是生成的中文根，AGENTS.md 是字面相对链接 CLAUDE.md；修改宿主源后使用登记入口，不手改投影：

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

现役 FILEMAP v2 是唯一执行计划源。routes 合并显式所选操作、核规范调用并绑定工具；projects 核独立配对、输出隔离与实际回执。scoped CI 复用该执行链，仍保留专用选择／环境边界。ci.verify 属于真实 ci/ci-tests 配对；有限历史转换及方法替换由登记的 candidate decoder 消费原字节，不执行旧判官。

本宿主采用 chrono-ci-check/v3 的 26 个显式单元，并在 units.json 显式采用 chrono-job-gating/v1：一个完整 Git DELTA detector、26 个 job-level if 独立单元和一个 always() aggregate，共用一份 parent workflow；只要求 aggregate 的 chrono / collection。当前宿主采用 config schema4；short check 固定读取 .chrono-harness/config.json 的 canonical_check.profile 和 inputs.local/ci action。候选仍须已提交、检出干净；CHRONO_CHECK_SOURCE 未设或 local 选择登记的 worktree 生产者，ci 选择登记的 CI 事件生产者，不猜范围或回退。detector 的缓存准备构建 chrono-cache，随后 bootstrap-shared.json 调用既有 bootstrap-core.json 一次构建 runner、judge-ci、ci 与生命周期参与者 worktree，并供 27 个显式消费者安装。消费者先核原提交／树、平台、配置、文件摘要及原始构建报告，再安装所选 profile；原缓存准备与核心启动步骤只验证已安装字节，业务构建仍归独立单元计划。本地及发布入口保留原源码构建。startup 传递目录在 state 之外，产物按原 producer artifact ID 下载，不能用当前重跑 attempt 猜地址。共享原生采用及重跑仍须实际验收。当前入口：

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-harness check
.chrono-harness/bin/chrono-harness check --unit ID
.chrono-harness/bin/chrono-harness check --collect
```

本地固定取得登记 remote 的 workflow target 与 clean HEAD。单元、共享操作及报告位置归 .chrono-harness/ci/check.json，parent／job 与启动参数归 units.json；无值汇总沿同一 check 入口消费登记的 manifest，CI owner 从显式报告路径及独立执行物 pins 生产 manifest，不重执行业务。生成的 scoped CI check 步骤使用同一短命令，在命令内调用现役事件准备／原生报告收集，不增加手工 prepare。本宿主不触发 integration push 检查；PR／dev push 的范围由 detector 一次固定给所有依赖 job；汇总核 exact parent/current attempt 与最新 job 状态；实际原始 producer／artifact attempt 归 generated production output，API carried attempt 不充当生产证据，支持成功前置不重跑。已退休的 generated workflows 保留历史语义，旧 provider 未显式采用 job_gating 时行为不变。现役 topology 的原生 Actions 已有验证；workflow-inventory、release-build 与 worktree-adoption 的组合仍需完整候选的原生验证。现役 scoped 宿主采用未启用 proposed full 治理。full-v3/v4 独立单元短入口及七判官原始证据汇总已有源实现；chrono-github-units/v2 通过显式 full_contexts／上传映射连接现有自动事件及 gather。本宿主仍采用 scoped 检查，projects v2 的受影响配对与语言规则复用 projects 所有者；未启用 full；full unscoped 可以从 start/reconstruct 自动发布的目的地 origin receipt 生产 schema2 context，无历史证据时具名失败。macOS 宿主显式绑定 /usr/bin/python3、Python 3.9.6；工具链版本及操作来自登记，不能以本地默认值替代。具体合同与既有原生证据可选择读取 docs/ci.md、docs/ci-units.md。

五份 full 治理登记仍 proposed，enforcement=not-implemented，input_closure=incomplete；Cargo/编译器/SDK、配置及其它适用输入、执行物绑定仍有未解决项。本宿主尚未采用 Cargo guarded policy；schema4 的 scoped/provider 绑定当前 /usr/bin/git 版本与字节，26 个单元及汇总 workflow 登记 runs_on=macos-26。开发工具目录未显式绑定；macos-26 原生 scoped 匹配已在合并的 PR104 验证，完整宿主启用与 full 原生 CI 仍未完成。chrono-inputs 只捕获声明文件／变量的两端快照，不补依赖、不改预期摘要、不启用治理。普通 full check 在登记要求及所选义务通过后表示 DELTA 满足现役登记合同；declared-complete 是 AI 对宿主范围负责的工程声明，现役 registration 仍报告 completeness_proven:false。已知缺输入、必需依赖未解决、缺绑定／快照和漂移仍须失败，不要求普遍隐藏输入证明或 VM。详见 docs/inputs.md、docs/cargo-projects.md、docs/git-facts.md。

本地/CI 同命令已经采用；普遍同判仍以完整相同有效输入和确定性求值为条件。独立 parity 比较器要求更强 completeness_proven:true 和完整观察，当前未建立；它不另加普通 full check 门，也不从成对读数证明普遍确定性。完整宿主启用、full 原生 CI、端到端交付及完整 SPEC 验收仍未完成。worktree 生产者支持显式创建／重建／恢复／清理，冲突协调及 PR／合并／落地生命周期归调用方，不能把阶段成功报成完成交付。

本宿主 scoped check 在 policy.report_publication 显式采用 retained-reference/v2：固定 check.json 保存 chrono-check-reference/v1 的原件路径和摘要，原件采用 chrono-check-report/v2 保存报告及进程元数据。判官 stdout／stderr 各保留原字节，通过 stdout_original／stderr_original 引用；不再另存文本、数字字节数组和 response。控制台直接给原件路径；本地及原生汇总通过既有登记上传映射核全部原件与摘要，从原 stdout 解码 response，继续执行全部原判定。report_bytes 累计约束元数据＋stdout＋stderr，固定引用另受同一上限约束。外部消费者须先解析引用、核原件和流摘要；上传仍包含完整登记目录。生产者、汇总判官与宿主政策须配套更新，普通命令不变。其它 gather／业务回执内部重复、完整资源计量及 SPEC 义务仍未完成。

详细历史与限定仍在可选 docs/ 下，来源／许可资料不是必读政策。指令生成不证明 AI 遵守或翻译等价；成本仍 unmeasured。Git 生命周期按当前任务授权，不从本文推断提交、推送或启用门已获授权；不改全局配置、参考仓库或其它工作树，不把过程转录写入产品源。

本宿主现将同一个 judge-workflow-tests 项目显式绑定为 core、full_units、full_short 三个 test_groups／CI 单元；成员由当前 inventory 动态核定，原 test ID、生产／测试项目配对、未过滤 execute／release 操作、历史测试正文与限制保留。注册所有者统一解析组 ID、本项目 action 与终端操作；FILEMAP 完整登记各组边、计划、成本及共享前置。宿主 workflow-inventory guard 从真实未过滤／过滤 libtest 列表核非空、无忽略、互斥且全集相等；长期核当前集合，不冻结历史数量。当前变化同时含产品合同与宿主政策，验证成本包括受影响 owner 专属测试、三组完整执行及独立库存核验。原生时限充分性、发布平台／公开分发和 main/examples full 启用仍归调用方后续验证；本地通过不证明激活。

本宿主在 worktree.json 显式采用 cleanup.json 的 automatic_cleanup。协调锚明确选择现有 Git owner inventory 的 main worktree；state 固定在该存活宿主的 .chrono-harness/state/automatic-cleanup/，不扫描同级目录。当前政策逐项采用已登记输出目录，保留 bin 与 state，默认仅回收已 finish 且无 managed use 的缓存，未解决证据不删除 checkout；不允许 finish 自行处置证据。调用方负责加入后台任务与实际落地，并从协调宿主运行固定 `.chrono-harness/bin/chrono-worktree finish --path <工作树>`；独立重试为 `.chrono-harness/bin/chrono-worktree maintain`，消费登记操作为 `.chrono-harness/bin/chrono-worktree use --operation <操作> --path <工作树>`。start/reconstruct 已接入同一 drain 与新生登记，不是定时空闲清理。旧工作树只按明确 `import --path <工作树>` 迁移；旧配置缺 automatic_cleanup 仍 opt-out。源与宿主政策同改，验证成本为 worktree owner 全套真实 Git 用例、受影响登记及指令生成／专属验证；落地与真实宿主 finish 事件仍由调用方交付。

本宿主源码登记 automatic_cleanup/v2 与新的 .chrono-harness/state/automatic-cleanup-v2/，不重解释旧 v1 状态。短 check 和 bootstrap 显式委托 worktree 所有者，Python pass_fds 与嵌套 runner 复用原进程 engine。最后内核租约关闭后，普通入口可回收未 finish 的缓存，但不设 terminal 或宣称落地。源码与宿主政策同时修改；本地真实 Cargo test／原生子进程回归验证包装器终止后的租约保留，候选仍需已提交组合与支持平台验收后才能完整采用。兼容协调者二进制部署、旧生产者排除、已提交组合、支持平台与主宿主实际切换归调用方。

普通入口的清理拒绝按登记项隔离：无关项的原失败回执与 cleanup_failures 保留，合法当前目标仍执行；目标身份／政策／所有权及协调状态写入失败继续阻断。显式 maintain 对清理失败仍返回失败。check participation 转发原 stdout、stderr 与退出码；非空 stderr 不等于生命周期失败。

切换顺序归调用方：先在独立、干净且固定提交的候选 Git main clone 运行同一 bootstrap/check；当前 scoped 本地生产者 full_context=None，无需伪造 linked origin receipt。真实协调宿主先加入／排除不兼容生产者、保留 v1 状态、部署兼容 binary 与匹配且已提交的 v2 policy/config/participation，再通过普通 start/reconstruct 自动登记新 linked checkout，执行原 bootstrap/check。只升级 binary 或只升级 linked policy 均应保留并拒绝，不能放宽字节一致性。fixture 与本地结果不替代真实切换、原生及已提交 canonical 验收。产品源码与独立宿主政策同时受影响，成本含新行为回归、完整 worktree/CI、runner/instructions、格式与登记生成。
本宿主独立在 FILEMAP v2 采用 execution_scheduling，max_running=2；priority 显式先排 workflow 的共同前置与 inventory，再排 units、short、core 三组，未列项保留规范就绪顺序；优先级不越过依赖或共享 claim，不改变结果顺序和时限；82 个参与操作逐项明确 resources／outputs（含空列表），Cargo 输出来自已有 config.artifacts，workflow 三组与 inventory 读写共用独占资源与测试 target。16 个 900s、4 个 600s、1 个 30s 实际计划及 2400s 外层或 native bounds 保留登记时限；历史 v1 600s fallback 保留。生产 scheduler／runner 与宿主采用同时改变，须分别评估：owner 专属行为套件验证实现，已提交干净候选的完整固定 check、受影响原生 CI、Git／有序交付和生命周期由调用方验证完成；cap 2 不声明加速幅度或时限充分性。跨调用隔离仍由调用方的独立 checkout／现有 lifecycle 负责。

本次有界修复在 worktree 原所有者协调已持久登记但未封口的 v2 birth；实际 EX 排除及原附件／政策验证后，普通入口保留原缺失／部分结果并发布可重复中断恢复回执，不制造原成功 birth 或 finish。已发布缓存尝试后新消费开启新代际，旧 intent/result 保留。bootstrap 与 check 同用 managed_command_failed 区分原命令失败及后续生命周期拒绝。宿主 cap 2 的 canonical 验证仍归调用方；本次独立 owner suites 顺序执行且不改 libtest 默认线程。产品与宿主政策同改，成本包含实际进程／Git 回归、完整 worktree/CI/runner/instructions 与登记生成，原失败不被覆盖。

本宿主 projects v2 显式登记 34 个 Rust 项目与三对 Python 脚本。workflow-inventory 及其 Python 测试分别持有脚本身份；Python 单元测试归独立 CI 单元，只使用临时目录。三个 Rust workflow 测试组继续运行真实 inventory 校验，Python 单元测试按自身 DELTA 登记独立选择。语言判官核声明配对，不证明其余嵌入式跨语言测试已全部迁移。

发布配方采用 v5。独立 Python 项目 release-build 与其唯一测试项目 release-build-tests 没有 Cargo manifest/lock/target；两个显式测试组分别为普通 Python 单元和依赖 build.distribution 的真实打包集成。发布验证按显式 needs 及 rust_toolchain 布尔选择取得资产和编译工具链；五个 Python 单元不安装 Rust，15 个原始 Rust 验证仍执行未过滤 Cargo action。instructions 与 routes 验证不消费发布二进制，needs 显式为空；其生产二进制仍由独立 build 单元构建并参与打包。routes 的普通执行计划保留自身编译与全部测试，其 Cargo 库依赖由 Cargo 编译，不另行构建未调用的 runner、registration、filemap 可执行文件。普通 CI 共 26 个业务单元，原生发布每个平台 16 个 build、22 个 verification 和一个 collector。产品／测试、宿主采用与生成投影同时受影响，完整候选、本地与原生验证及公开采用仍分别验收。

解码器专属 Python 测试在 migrations/test-hosts.json 显式选择平台测试输入：本机与 macOS CI 核主宿主 config.json 的固定解释器，Linux 原生发布核独立测试 fixture 的固定解释器。未知平台不回退、不从实测版本生成预期；Linux fixture 不表示主宿主 full 治理已采用。全部解码及版本断言在两个登记平台执行。

worktree-tests 的 core 与 adoption 两个测试组分别对应 worktree、worktree-adoption CI 单元。adoption 显式列出六个已采用宿主入口的进程交接与租约集成用例，core 排除这些同名成员；原测试 ID、断言、时限与默认 libtest 线程设置不变。登记的 worktree.inventory 复用独立 Python inventory 所有者，以 worktree-inventory.json 核验真实未过滤列表和两组列表非空、无忽略、无重叠且全集相等。两组沿相同工作树测试 target 与 inventory 资源互斥；未过滤 execute 及发布验证仍保留。分组提供独立检查与重跑边界，不宣称消除所有时序失败。

标准日志与错误记录归独立 diagnostics / diagnostics-tests 配对，独立 CI 单元使用同一 check 入口。runner 的无效 UTF-8 参数路径采用此库；库保留 typed source、相关失败、JSON Lines、显式线程上下文与 sink 原错误。其它产品路径、原生错误适配、应急发布、持久证据身份、子进程上下文及其它语言适配仍未完成，不宣称 SPEC §7.1／§7.2 全面启用。diagnostics 专属发布验证显式登记空 needs 与 rust_toolchain=true，独立启动且核验 Rust 工具链。Rust 发布验证的 needs 按实际二进制消费逐项登记；全部原始未过滤操作保留。

宿主 bootstrap.py 与其 Python 测试独立登记为 host-bootstrap / host-bootstrap-tests。专属测试通过显式 Python SDK／操作 fixture 检查配置选择、安装字节与摘要、Git 源状态、工具链安装及失败传播。测试直接由 Python 执行，不经 Cargo 转发；独立 host-bootstrap CI 单元使用宿主共同的 harness bootstrap，两个平台的专属发布验证显式登记空 needs 与 rust_toolchain=false。Rust worktree 的宿主入口与租约集成测试仍验证真实跨程序边界。其余跨语言测试迁移、完整候选原生验收及公开采用仍需完成。

bootstrap-cache.json 显式将 report_path 登记为 .chrono-harness/state/cache-bootstrap.json，核心 bootstrap 保留默认 bootstrap.json，两者随既有证据上传。缓存 transport 报告记录实际程序路径、文件摘要与版本，供消费者核对独立构建记录及干净源码身份；路径摘要不证明完整输入或加载内存身份。bootstrap 拒绝越界及 symlink 报告位置，保存失败非零；同一登记槽位保留最近结果。缓存测试只在失败时保留原 fixture、探测配置及原始报告，正常退出清理临时宿主。

runner-tests 的协议 transport fixture 由同项目独立 Rust 测试二进制执行；用例参数是临时 JSON 数据，判定与子进程断言归 Rust。初始／DELTA 响应、四态退出、原始字节、直接前置及保留进程身份、超时和输出上限控制保留，FILEMAP 显式登记编译输入。此范围不包含其它文件中的脚本 fixture，也不宣称全仓语言迁移完成。


worktree 源码提供显式 migrate --path 入口，在相同 v2 协调与原附件租约下采用两端已提交的新政策；旧登记保留，后续操作核不可覆盖迁移链。迁移不清理、不设完成；未知 birth／租约、待执行终态及地址搬迁保持拒绝。候选本地／原生和实际宿主采用仍须验证，不从源码新增声称已迁移。

发布 v5 在 verification_consumers 为全部 22 个验证单元登记实际调用、输出用途与发布资产子集；instructions／routes／diagnostics 的发布资产列表为空。预检拒绝消费合同与 needs 冲突，未说明的非空发布前置在原始回执与 stderr 报 W_RELEASE_CONSUMPTION_UNVERIFIED。declared 只表示登记一致，不能证明必要性或说明真实。

本宿主在 units.json 的 gather.resource_observation 显式登记原生步骤分类，在输入环境继承 GITHUB_STEP_SUMMARY。现有汇总通过同一 check 入口复用 jobs 响应，向原始 gather 报告及 Actions summary 输出有效已完成步骤的时间分项、未分类与未知项；不新增请求／job／业务重跑。canonical-check 包含其内部构建、测试与汇总，不能称为纯测试时间；总数与比较只取当前 attempt 的 API 视图，旧 attempt 行列为排除项并保存在原始响应。当前视图可含沿用结果，不能把新 job ID 当作重新执行；读数不是本次重跑成本、完整执行历史、计费或全流程总量。其余消费核验、完整资源分项及公开二进制采用仍未完成。

同一登记的 comparisons 采用 bootstrap-vs-check：已完成 job 的 bootstrap 至少 60 秒且超过该 job 的 canonical-check 时，原始汇总与 Actions summary 输出 W_CI_RESOURCE_COMPARISON 及实际读数。两类别读数不完整则报告无法比较，跳过 job 不适用；不改变准入或推断依赖冗余。规则可由宿主调整，缓存准备／传输也必须纳入优化成本。产品实现与宿主政策同改，验证包含 CI 专属行为测试、原始同命令检查及候选原生汇总；公开版本采用另行验收。

缓存候选采用 chrono-cache/v2，在 cache.json 用 JSON 指针引用各消费者的 bootstrap、FILEMAP 计划或发布操作；缺引用及退出计划的生产者恢复前报错，消费者增减不改变未变生产输入的缓存身份。detector 订阅实际 core 启动生产者 target；单元只订阅仍在业务计划中的生产者，aggregate 及没有编译操作的单元不订阅 target。routes 不订阅未调用的 registration／filemap target。生成器在原工作结束后记录缓存动作原始读数；原生保存步骤成功仍先报 W_CACHE_SAVE_UNCONFIRMED；cache.json 显式登记 gh 后端查询，仅在有待确认保存时由原 report 步骤执行，保留精确 key/ref 条目与有界原始进程证据。条目存在只确认观察时可用，不能归因到本次 action；查询失败及歧义保持未确认，原业务失败保留。凭据只传给查询且不写入保留环境，查询成本归 cache-report；没有待确认保存不发请求。完整原生、损坏恢复及成本验收尚未完成。显式启动缓存子集可在后续测试失败后保存，取消不保存，不把缓存读数当测试结果。宿主登记 cache-prepare／restore／save 分类及准备、恢复相对检查的阈值告警，成本比较不证明浪费。

本宿主用显式 save_caches 将四个共享启动缓存的保存归既有 detector，其余缓存各归实际生产单元；aggregate 不订阅编译缓存。每个缓存 ID 在同一 provider 中至多登记一个保存 job；generate／verify 发现冲突报 E_CACHE_SAVE_OWNERSHIP 并点名双方，允许显式全只读。单元仍恢复、重建及检查，不增加 job 或前置等待；指定保存者未运行或生产者未成功时，本轮不产生新归档。未请求保存与实际异常保存分别报告，取消不制造保存结果。detector 的 evidence_directory 选择 .chrono-harness/state/，在原 job 内 always 上传原始探测、计划与启动证据；缺文件报错。成本比较另覆盖 bootstrap／缓存准备、保存相对 delta-detection，以及缓存保存相对 canonical-check；只提示实测成本，不证明冗余或节省。完整本地／原生验收及缓存剩余合同仍须分别验证。

projects 的专属测试项目显式分为 core 与 migration 两组，分别绑定 judge-projects 和 judge-projects-migration 单元。core 运行 consumer／execution／languages，migration 运行原 migration 二进制及其真实宿主嵌套 CI 测试。projects.inventory 复用 Python 库存所有者，以 projects-inventory.json 核当前未过滤列表与分组列表非空、无忽略、互斥且全集相等。原测试正文、断言、测试内时限、默认 libtest 并发与未过滤发布入口保留；每组操作限时 900 秒，明确区别于原单项合并总预算。两组共享 target 与库存资源，本地互斥，原生独立 checkout／报告／重跑。产品测试替身的就绪修复与宿主分组政策同时进入候选，完整本地／原生验收及公开采用仍须验证，不从分组声明节省或超时已修复。

宿主 cache.json 明确采用 recover_failed_restores，生成器在原生 restore 后、原 bootstrap 前调用缓存所有者的恢复入口，并把步骤时间归 cache-recovery。基础策略清理原始 outcome=failure 对应的已登记宿主产物，保留原始 outcome、intent、逐项结果及最终报告；重复完成尝试不再删除新构建输出，未完成尝试拒绝自动重放。有效命中及明确 skipped 保留产物；缺失／未知结果、登记或路径漂移、清理及证据发布失败阻断。旧注册未采用则不执行恢复；不新增 job、构建配方或业务测试重试。本宿主同时采用 recover_unconfirmed_restores：success 但没有有效 exact／compatible 命中键时，同一入口清理对应输出并记录 miss-or-unavailable／incompatible；原生实现抑制下载／解压错误不允许留下部分缓存供原构建使用。策略不改变编译缓存键；普通冷缓存没有残留时不新增恢复告警。原始 outcome、后续构建失败及已完成恢复的重入语义保留，畸形 outputs 在删除前阻断。该源实现不完成有效命中后的内容校验、外部缓存所有权、linked worktree 租约或完整原生故障验收。
