# chrono-harness 宿主上下文

本文件只登记本仓当前采用、入口与限制；通用规则在生成的 CLAUDE.md／AGENTS.md。命令从仓库根运行，下面的文档链接按本文件位置解析；按当前任务读取对应所有者文档。

## 所有者与范围

本宿主以当前规范为合同，无需兼容历史版本、格式、接口和执行路径。
演进时同步迁移现役消费者与登记，移除仅为历史兼容保留的实现和测试。
下文的旧版本材料只保留其事实用途，不形成支持义务；固定基线 DELTA、必要原件与
活跃消费的保护继续按当前合同处理。

本仓维护 Rust harness 产品和独立宿主采用数据。[SPEC](../../SPEC.md) 是尚未全部实现的产品合同，当前结果与缺口归[覆盖表](../../docs/spec-coverage.md)。生产／专属测试项目各有独立 manifest、lock、target，无根 workspace；独立 Python 脚本及测试也显式配对。

产品默认归 `assets/instructions/catalog.json`、`assets/instructions/default-manifest.json`；本宿主选择归本目录的 [catalog](catalog.json)、[manifest](manifest.json) 和本上下文。产品资产是编译／测试输入，宿主数据与现有投影是运行输入；升级二进制或重复 init 不覆盖宿主选择，采用新默认须比较所选产品版本的 catalog／default-manifest，保留定制后编辑宿主源并 generate；具体步骤归[指令所有者](../../docs/instructions.md#新默认的宿主采用边界)，不是自动更新。CLAUDE.md 是生成的中文根，AGENTS.md 是字面相对链接 `CLAUDE.md`，不手改投影。

[FILEMAP](../FILEMAP.json) 是唯一执行计划源，逐文件登记所有者、输入、消费者、边与成本；不从路径、语言或调用推断选测。routes 规划、绑定工具，projects 核配对、隔离和真实回执；scoped CI 复用该链。只按 DELTA 与两端显式依赖选检查，漏登须修登记，不全测兜底。

本宿主采用“不阻塞、不使用额外权限、关注性能”：局部失败保留真实结果，只阻断显式依赖闭包，独立授权工作继续；只用现有权限，无必要特殊权限功能退役，不增加替代激活运输；普通必要流程保留已有权限范围。消费者、IO、进程及验证按当前需求和 DELTA 影响收窄，成本未知保持未知，必需验收不旁路。产品默认与宿主 catalog 已同步采用，具体规则及验证成本边界归[指令所有者](../../docs/instructions.md)。

## 当前固定入口

### 安装候选工具

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
```

检查前从候选源码 bootstrap。本 macOS 宿主绑定 `/usr/bin/python3`／Python 3.9.6；Rust 版本、操作、安装产物归 bootstrap-core.json／projects.json，不以环境默认替代或改全局默认工具链。工具观察／摘要不证明完整来源。linked checkout 缺兼容协调者／所有权则在构建前失败；Git main 可初始化自己的工具。详见 [CI 启动](../../docs/ci.md#this-repository)及[生命周期参与](../../docs/worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)。

### 本地／CI 检查

```sh
.chrono-harness/bin/chrono-harness check
```

候选必须已提交且检出干净。`--unit ID` 和无值 `--collect` 是同一入口的单元／汇总模式，汇总不重执行业务。schema4 从 config.json 的 `canonical_check` 选择 profile 与输入 action；`CHRONO_CHECK_SOURCE` 未设或 `local` 使用登记 worktree 生产者，`ci` 使用登记 CI 事件生产者，其余值或缺输入失败，不猜范围或回退。本地固定登记 remote 的 workflow target 和 clean HEAD；不手工 prepare。

本宿主采用 scoped `chrono-ci-check/v3`、24 个单元、一个 DELTA detector 和 always aggregate，只要求 `chrono / collection`。开发／integration 分支通过指向 dev 的 PR 检查；push 只验 dev 落地 DELTA。parent 按 PR 号替换旧运行，push 按 run ID 保留各次 before/after。单元／报告归 check.json，拓扑／共享启动／上传归 units.json；[CI 单元所有者](../../docs/ci-units.md#conditional-jobs-and-the-required-aggregate)拥有原始 producer/artifact attempt 核验、成功前置沿用及共享安装合同，不能按当前重跑 attempt 猜产物。业务构建仍归独立计划，本地／发布保留源码构建。

检查退出 0 须有有效 passing response；判官失败为 1，用法／配置／运输／协议／报告 IO 失败为 2。本宿主采用 `retained-reference/v2`：固定 check.json 引用原报告，报告引用 stdout／stderr 原字节；消费者核原件／流摘要后从原 stdout 解码，上传完整登记目录。`report_bytes` 约束元数据＋两流总量，固定引用另受同一上限约束。详见[报告消费](../../docs/ci-units.md#offline-collection)；生成／上传成功不等于检查通过。

### 指令生成

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

这是 projects.json 的 `instructions.generate` action。本宿主采用四阶段方法；全局核实际需求、SPEC、能力、所有权、显式影响与成本后收窄实现，验证真实消费者并再核 SPEC。改宿主 catalog、manifest、登记 file 源或上下文后运行；schema 2／`atomic-rules/relative-alias/v3`、需求→审计→重构→开发短根及同 manifest 的英文／skill 输出合同归[指令所有者](../../docs/instructions.md)。生成／短 prose 不证明 AI 阅读、遵守、翻译等价或执行检查。[实际消费范围](../../docs/methodology-extraction.md#实际消费者边界)还含上下文与块外内容，不截断规则或改全局配置。

### 受保护消费与完成

```sh
.chrono-harness/bin/chrono-worktree use --operation <已登记操作> --path <工作树>
.chrono-harness/bin/chrono-worktree finish --path <工作树>
.chrono-harness/bin/chrono-worktree maintain
```

消费登记操作须持有 use 保护。调用方加入全部自有／后台任务、核实际完成／落地后，**从存活协调宿主** finish；maintain 是独立重试。创建、重建、旧 checkout import 和显式政策 migrate 按[worktree 所有者](../../docs/worktree.md)执行，不能从本文推断 Git／GitHub 生命周期授权。

本宿主在 worktree.json／cleanup.json 采用 automatic_cleanup/v2，协调锚是 Git owner inventory 的 main worktree，状态在 `.chrono-harness/state/automatic-cleanup-v2/`。start/reconstruct 自动登记并只维护目标记录；采用的 bootstrap/check 委托同一 owner，不扫描无关工作树或输出。跨对象清理走显式 maintain。每个工作树有独立地址和 metadata 租约；最后内核租约释放后该对象的普通入口可回收未 finish 的可重建缓存，保留活子孙、源码、分支、未保留提交、bin 与证据，不设完成。只采用已验证的描述符转交链；不按年龄、目录名或空闲删除。

显式维护对无关登记项的清理失败保留原回执／`cleanup_failures`，合法目标仍执行；目标身份、政策、所有权及协调状态写入失败继续阻断，显式 maintain 仍报失败。未封口 birth 只在实际排除和原附件／政策验证后恢复，保留原缺失／部分结果；新消费在已发布缓存尝试后开新代际，旧 intent/result 不覆盖。命令原 stdout／stderr／退出码保留，非空 stderr 不等于生命周期失败。

兼容 binary 与两端已提交 policy/config/participation 必须配套；调用方负责加入／排除旧生产者、保留 v1 状态、有序部署与实际切换。只升级 binary 或 linked policy 应拒绝，旧版／未知所有权保留；migrate 不清理或宣称完成。[v2 切换及租约限制](../../docs/worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)仍需提交候选、支持平台和真实协调宿主验收。宿主 Git 身份阻断时，唯一 bootstrap 适配是该节登记的独立干净 Git main clone／最新 dev integration 候选路径；main-only seed 不替代真实协调者部署与随后 canonical start。

输出保留采用 retention/v2：每个输出和真实消费者各有独立声明，普通发布、读取、完成不取得全库准入锁或扫描总清单。512 上限归实际关系集，删除全库数量／总字节准入；单输出和每轮节点／字节／25 ms 上限继续生效。显式维护非阻塞处理实际关联对象，局部繁忙或损坏保留错误并继续兄弟；未遍历或无法计数时报告 unknown。源与宿主政策同改，其他协调宿主的现役总清单须先由保管者排除旧消费者、保留原字节并按精确声明切换；普通程序不兼容旧总清单。本仓原主检出已完成受保护切换、共享 owner 部署与普通 bootstrap 验收，并验证无关旧工作树持有 metadata 独占租约时可启动新工作树；原件与独有提交继续保留。覆盖表记录固定交付范围，不据局部成功声明完整 SPEC 完成。

## 按任务读取现有文档

| 当前任务 | 自然所有者文档与材料条件 |
| --- | --- |
| 改登记、执行计划或测试 | [影响闭包](../../docs/filemap-impact.md)、[执行／分组／语言](../../docs/execution.md)、[单元库存](../../docs/ci-units.md)：本宿主 cap=2，不越依赖、共享 claim 或登记时限；跨调用隔离归调用方。分组、真实未过滤库存与原断言／线程／发布操作的保留合同在单元文档；语言声明不证明其余跨语言逻辑已迁移。 |
| 改输入、Cargo 或 full 采用 | [输入与 parity](../../docs/inputs.md)、[Cargo guarded](../../docs/cargo-projects.md)、[Git facts](../../docs/git-facts.md)、[full CI](../../docs/full-ci.md)：快照不补依赖／改预期摘要／启用治理；candidate decoder 消费历史数据，不执行旧判官。full unscoped 需原 origin receipt／历史证据，缺失失败。旧 scoped 宿主的专用解码器、工具 fixture、历史对象获取和迁移测试已退役；当前 projects 测试使用原未过滤 action。通用转换接口由自含合成 fixture 验证，不获取旧仓库，也不形成历史兼容入口。 |
| 改 CI 缓存 | [缓存所有者](../../docs/ci-cache.md)：普通 plan/restore、producer-manifest 内容验证、失败恢复、原构建／测试、seal/save 和原始 outcome 报告保持；专属 Variables／PR 正文激活、manifest-removal fixture 及新增权限依赖已按用户要求退役，不增加替代入口。历史 C1 对两项缓存观察到坏内容保存／恢复／拒绝、重建与替换缓存，但原兼容域健康精确命中未达成（rustup 身份变化，最新 collection HTTP 502），保留部分结论及原件。外部所有权、linked transport 租约、其它消费者及完整验收仍未完成；只支持 primary checkout。缓存测试只在失败时保留原 fixture／探测配置／报告。 |
| 改发布或消费关系 | [发布所有者](../../docs/release-ci.md)：v5 显式 needs／rust_toolchain／verification_consumers、原未过滤操作与 producer/run/attempt、失败证据；declared 不证明必要性，本地／原生／公开采用分别验收。 |
| 读成本或排查失败 | [汇总观察](../../docs/ci-units.md#github-projection-and-transport)、[诊断边界](../../docs/diagnostics.md)：读原件；当前 attempt API 视图可含沿用结果，新 job ID 不证明重跑。canonical-check 含构建／测试／汇总，读数不是完整历史、重跑成本、计费或全流程总量，告警不证明冗余／节省。 |

## 当前未完成边界

五份 full 治理登记仍 proposed，`enforcement=not-implemented`、`input_closure=incomplete`；未采用 Cargo guarded policy。Cargo／编译器／SDK、配置及其它适用输入／执行物绑定仍有缺口。宿主 Git facts 采用 `chrono-git-configs/v3` 精确摘要选择，worktree 采用 v3 主身份与登记的补充身份；原生队列现役的 `61e77a2d…`（run `41927937485/1`）与 `b8763cf2…`（run `41941690768/1`）分别绑定完整政策，并显式共用固定基线数据路径，另保留实测 macOS 27.0.1 本机身份。版本预期与完整验收仍须实际 bound probe 验证，未知摘要仍在执行前拒绝。政策与产品同改：候选决定实际执行物；固定端点先按候选执行物身份读取其显式登记；端点未登记该身份时只读候选声明的 registry_path 和端点原字节，不启动历史 Git，不猜其它路径。额外读取和完整验证成本未测。日常本机检查的 CARGO_HOME 与输入声明采用当前检出的 .chrono-harness/cache/bootstrap-owner-tooling/cargo，与 bootstrap 的可重建工具缓存登记一致，HOME／TMPDIR 沿用既有继承环境，移除对旧检出目录的写入绑定；保持摘要及只读工具链／SDK 绑定，实际采用仍待验收。原生仍使用 macos-26，当前生产 `72b1d08` 与临时环境 `c515cd05` 的 scoped 检查已有有界观察；它们不证明完整启用，开发工具目录仍未显式绑定。

普通 full check 在登记要求和所选义务通过后只表示 DELTA 满足现役合同；`declared-complete` 是负责的工程声明，registration 仍报告 `completeness_proven:false`。已知缺输入、必需依赖未解决、缺绑定／快照及漂移必须失败，不另要求普遍隐藏输入证明或 VM。同命令已采用，同判仍需完整相同有效输入与确定性求值；现役生产者不建立更强输入完整性，报告保持 parity 未建立；独立比较／改写命令及固定 `spec status` 已退役，也不另加普通 full 门。

两项 producer-manifest/v1 缓存原生 cold/save、健康精确命中、兼容源码校验后实际重建与不兼容环境分域／重建已有有界观察，分别来自 `39688684172/1`、`39690377913/1,2` 与 `39695762671/1`。原 Variables GET/list HTTP 403 与后续 C1 部分证据保留，坏内容恢复／替换不等于修复后的原域健康精确命中；可选远端激活层已退役，不作为任何当前推进依赖。并发、其它消费者及完整成本／生命周期合同仍未完成。临时优化环境已由普通 revert `c72c141` 移除，恢复树等同生产 `72b1d08`，保留实验历史。现役 scoped 路径仅采用 runner-startup 的版本化内部消费关系检查；full judge 的类型化 prelaunch／报告诊断已有有界实现。当前 detector→startup 导入／安装→runner 检查→原件汇总链已有 Darwin 原生观察，runner 原始输出含 166 passed／1 ignored 及具名 prelaunch 回归；Linux、外部／全消费者采用仍未完成，具体边界归所有者文档及覆盖表。完整宿主启用、full 原生 CI、端到端交付和完整 SPEC 验收仍未完成；阶段成功不是交付。成本仍有 unmeasured 项，来源／许可资料是可选资料，不是执行政策。Git 生命周期按当前任务授权；不改全局配置、参考仓库或其它工作树，不把过程转录写入产品源。
