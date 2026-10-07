# chrono-harness 宿主上下文

本文件只登记本仓当前采用、入口与限制；通用规则在生成的 CLAUDE.md／AGENTS.md。命令从仓库根运行，下面的文档链接按本文件位置解析；按当前任务读取对应所有者文档。

## 所有者与范围

本仓维护 Rust harness 产品和独立宿主采用数据。[SPEC](../../SPEC.md) 是尚未全部实现的产品合同，当前结果与缺口归[覆盖表](../../docs/spec-coverage.md)。生产／专属测试项目各有独立 manifest、lock、target，无根 workspace；独立 Python 脚本及测试也显式配对。

产品默认归 `assets/instructions/catalog.json`、`assets/instructions/default-manifest.json`；本宿主选择归本目录的 [catalog](catalog.json)、[manifest](manifest.json) 和本上下文。产品资产是编译／测试输入，宿主数据与现有投影是运行输入；升级二进制或重复 init 不覆盖宿主选择，采用新默认须编辑宿主源。CLAUDE.md 是生成的中文根，AGENTS.md 是字面相对链接 `CLAUDE.md`，不手改投影。

[FILEMAP](../FILEMAP.json) 是唯一执行计划源，逐文件登记所有者、输入、消费者、边与成本；不从路径、语言或调用推断选测。routes 规划、绑定工具，projects 核配对、隔离和真实回执；scoped CI 复用该链。只按 DELTA 与两端显式依赖选检查，漏登须修登记，不全测兜底。

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

本宿主采用 scoped `chrono-ci-check/v3`、26 个单元、一个 DELTA detector 和 always aggregate，只要求 `chrono / collection`。开发／integration 分支通过指向 dev 的 PR 检查；push 只验 dev 落地 DELTA。parent 按 PR 号替换旧运行，push 按 run ID 保留各次 before/after。单元／报告归 check.json，拓扑／共享启动／上传归 units.json；[CI 单元所有者](../../docs/ci-units.md#conditional-jobs-and-the-required-aggregate)拥有原始 producer/artifact attempt 核验、成功前置沿用及共享安装合同，不能按当前重跑 attempt 猜产物。业务构建仍归独立计划，本地／发布保留源码构建。

检查退出 0 须有有效 passing response；判官失败为 1，用法／配置／运输／协议／报告 IO 失败为 2。本宿主采用 `retained-reference/v2`：固定 check.json 引用原报告，报告引用 stdout／stderr 原字节；消费者核原件／流摘要后从原 stdout 解码，上传完整登记目录。`report_bytes` 约束元数据＋两流总量，固定引用另受同一上限约束。详见[报告消费](../../docs/ci-units.md#offline-collection)；生成／上传成功不等于检查通过。

### 指令生成

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

这是 projects.json 的 `instructions.generate` action。改宿主 catalog、manifest、登记 file 源或上下文后运行；schema 2／`atomic-rules/relative-alias/v3`、22 叶子短根及同 manifest 的英文／skill 输出合同归[指令所有者](../../docs/instructions.md)。生成／短 prose 不证明 AI 阅读、遵守、翻译等价或执行检查。[实际消费范围](../../docs/methodology-extraction.md#实际消费者边界)还含上下文与块外内容，不截断规则或改全局配置。

### 受保护消费与完成

```sh
.chrono-harness/bin/chrono-worktree use --operation <已登记操作> --path <工作树>
.chrono-harness/bin/chrono-worktree finish --path <工作树>
.chrono-harness/bin/chrono-worktree maintain
```

消费登记操作须持有 use 保护。调用方加入全部自有／后台任务、核实际完成／落地后，**从存活协调宿主** finish；maintain 是独立重试。创建、重建、旧 checkout import 和显式政策 migrate 按[worktree 所有者](../../docs/worktree.md)执行，不能从本文推断 Git／GitHub 生命周期授权。

本宿主在 worktree.json／cleanup.json 采用 automatic_cleanup/v2，协调锚是 Git owner inventory 的 main worktree，状态在 `.chrono-harness/state/automatic-cleanup-v2/`。start/reconstruct 自动登记并 drain；采用的 bootstrap/check 委托同一 owner。最后内核租约释放后普通入口可回收未 finish 的可重建缓存，保留活子孙、源码、分支、未保留提交、bin 与证据，不设完成。只采用已验证的描述符转交链；不按年龄、目录名或空闲删除。

无关登记项清理失败保留原回执／`cleanup_failures`，合法目标仍执行；目标身份、政策、所有权及协调状态写入失败继续阻断，显式 maintain 仍报失败。未封口 birth 只在实际排除和原附件／政策验证后恢复，保留原缺失／部分结果；新消费在已发布缓存尝试后开新代际，旧 intent/result 不覆盖。命令原 stdout／stderr／退出码保留，非空 stderr 不等于生命周期失败。

兼容 binary 与两端已提交 policy/config/participation 必须配套；调用方负责加入／排除旧生产者、保留 v1 状态、有序部署与实际切换。只升级 binary 或 linked policy 应拒绝，旧版／未知所有权保留；migrate 不清理或宣称完成。[v2 切换及租约限制](../../docs/worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)仍需提交候选、支持平台和真实协调宿主验收。

## 按任务读取现有文档

| 当前任务 | 自然所有者文档与材料条件 |
| --- | --- |
| 改登记、执行计划或测试 | [影响闭包](../../docs/filemap-impact.md)、[执行／分组／语言](../../docs/execution.md)、[单元库存](../../docs/ci-units.md)：本宿主 cap=2，不越依赖、共享 claim 或登记时限；跨调用隔离归调用方。分组、真实未过滤库存与原断言／线程／发布操作的保留合同在单元文档；语言声明不证明其余跨语言逻辑已迁移。 |
| 改输入、Cargo 或 full 采用 | [输入与 parity](../../docs/inputs.md)、[Cargo guarded](../../docs/cargo-projects.md)、[Git facts](../../docs/git-facts.md)、[full CI](../../docs/full-ci.md)：快照不补依赖／改预期摘要／启用治理；candidate decoder 消费历史数据，不执行旧判官。full unscoped 需原 origin receipt／历史证据，缺失失败。解码测试由 `.chrono-harness/migrations/test-hosts.json` 显式选平台，未知平台不回退、不以实测版本生成预期；Linux fixture 不表示主宿主 full 采用。 |
| 改 CI 缓存 | [缓存所有者](../../docs/ci-cache.md)：原生消费者、唯一保存者、恢复／重入、bootstrap 报告槽位、原始 outcome 与后端查询合同。缓存不替代构建／检查；有效命中内容校验、外部所有权、linked transport 租约未完成，只支持 primary checkout。缓存测试只在失败时保留原 fixture／探测配置／报告。 |
| 改发布或消费关系 | [发布所有者](../../docs/release-ci.md)：v5 显式 needs／rust_toolchain／verification_consumers、原未过滤操作与 producer/run/attempt、失败证据；declared 不证明必要性，本地／原生／公开采用分别验收。 |
| 读成本或排查失败 | [汇总观察](../../docs/ci-units.md#github-projection-and-transport)、[诊断边界](../../docs/diagnostics.md)：读原件；当前 attempt API 视图可含沿用结果，新 job ID 不证明重跑。canonical-check 含构建／测试／汇总，读数不是完整历史、重跑成本、计费或全流程总量，告警不证明冗余／节省。 |

## 当前未完成边界

五份 full 治理登记仍 proposed，`enforcement=not-implemented`、`input_closure=incomplete`；未采用 Cargo guarded policy。Cargo／编译器／SDK、配置及其它适用输入／执行物绑定仍有缺口。宿主绑定 `/usr/bin/git` 字节／版本，原生使用 macos-26；已有 scoped 原生匹配结果不等于完整启用，开发工具目录仍未显式绑定。

普通 full check 在登记要求和所选义务通过后只表示 DELTA 满足现役合同；`declared-complete` 是负责的工程声明，registration 仍报告 `completeness_proven:false`。已知缺输入、必需依赖未解决、缺绑定／快照及漂移必须失败，不另要求普遍隐藏输入证明或 VM。同命令已采用，同判仍需完整相同有效输入与确定性求值；独立 parity 要求 `completeness_proven:true` 和完整观察，当前未建立，也不另加普通 full 门。

缓存已有一个登记消费者的原生 cold miss/save 与 exact-hit 路径验收；changed-source、损坏／不可用、并发、其它消费者及完整成本／生命周期合同仍未完成。完整宿主启用、full 原生 CI、端到端交付和完整 SPEC 验收仍未完成；阶段成功不是交付。成本仍有 unmeasured 项，来源／许可资料是可选资料，不是执行政策。Git 生命周期按当前任务授权；不改全局配置、参考仓库或其它工作树，不把过程转录写入产品源。
