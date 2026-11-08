# chrono-harness 宿主上下文

本文件说明本仓当前采用的配置、入口和限制。通用规则见生成的 CLAUDE.md／AGENTS.md。命令均从仓库根目录运行；下文链接以本文件位置为基准。按当前任务读取负责该事项的文档。

## 所有者与范围

本宿主以当前规范为合同，无需兼容历史版本、格式、接口或执行路径。变更时，同步迁移当前使用方和登记，移除只为历史兼容保留的实现与测试。旧版本材料仅保留其事实用途，不产生继续支持的义务。固定基线 DELTA、必要原件和正在使用的对象，仍按当前合同保护。

本仓维护 Rust harness 产品，以及独立的宿主采用数据。[SPEC](../../SPEC.md) 是产品合同，尚未全部实现；当前结果和缺口见[覆盖表](../../docs/spec-coverage.md)。每个生产项目及其专属测试项目都有独立的 manifest、lock 和 target，没有根 workspace。独立 Python 脚本与测试也明确登记配对。

产品默认数据归 `assets/instructions/catalog.json` 和 `assets/instructions/default-manifest.json` 维护。本宿主的选择归本目录的 [catalog](catalog.json)、[manifest](manifest.json) 和本文件维护。产品资产用于编译和测试；宿主数据及现有生成副本用于运行。升级二进制或重复 init 不会覆盖宿主选择。采用新默认时，要比较所选产品版本的 catalog／default-manifest，保留定制，再编辑宿主源并 generate；步骤见[新默认的宿主采用边界](../../docs/instructions.md#新默认的宿主采用边界)，没有自动更新。CLAUDE.md 是生成的中文根指南；AGENTS.md 是目标字面值为 `CLAUDE.md` 的相对符号链接，不手改生成内容。

[FILEMAP](../FILEMAP.json) 是唯一执行计划来源，逐文件登记负责方、输入、使用方、依赖边和成本。不得根据路径、语言或调用关系猜测测试选择。routes 负责规划和绑定工具，projects 核对配对、隔离和真实回执，限定范围的 CI 复用这条链。只按 DELTA 及前后两端的明确依赖选择检查；漏登就修登记，不改跑全测。

本宿主采用“不阻塞独立工作、不使用额外权限、关注性能”。局部失败保留真实结果，只暂停沿明确依赖关系受阻的工作，其他已授权工作继续。只用现有权限；没有必要使用方、又依赖特殊权限的功能应退役，不增加替代激活或传输层。普通必要流程保留已有权限。使用方、IO、进程和验证范围按当前需求及 DELTA 影响收窄；成本未知就保持未知，不能跳过必需验收。产品默认与宿主 catalog 已同步采用这些要求，规则和验证成本边界见[指令说明](../../docs/instructions.md)。

## 自动交付授权

本仓所有已授权工作都自动完成提交、创建 PR、等待适用检查、合并和落地核验。无论先经 integration 分支集成，还是直接创建面向 dev 的 PR，都无需用户额外输入、确认或合并前提示；先合入 integration 的工作继续自动交付到 dev。

按[宿主默认合并方案](../../docs/merging.md)执行必需检查和现有保护，处理实际依赖与冲突。失败时保留原始结果并在授权内修复，继续独立工作；不能把创建 PR、启用自动合并或检查通过当作已经合并，须核对实际合并状态、落地 SHA 和必要的落地检查。

## 文档规范

本仓编写和维护的文档统一使用简体中文，包括 README、SPEC、操作说明、宿主上下文和技能说明。“优雅”以准确、自然、清楚、简洁为准；白话表达仍须保留必要条件。面向其他语言使用方的产品内容、第三方引文、许可原文和真实接口样本，按其实际用途保留原语言。

1. **写法直白。** 用常用词和短句，写清谁在什么条件下做什么。删去套话、重复话和空泛修饰，避免晦涩术语、官话和直译腔。
2. **方便阅读。** 围绕读者的任务组织，先交代用途或结论，再给必要条件、步骤和结果。标题直接说明内容；短段落写一件事，步骤用有序列表，对照用表格。正文用中文标点，命令和代码遵守原语法。
3. **解释术语。** 读者需要理解的专业词，首次出现时用白话解释，再给必要的原名或缩写。同一概念保持同一叫法，例如“本次变更（DELTA）”；普通说明中的 consumer 写作“使用方”，代码标识仍保留原名。
4. **保留技术原文。** 命令、代码、字段、接口名、路径、URL、版本、摘要、运行号及真实输入／输出样本，保留准确内容、大小写和必要原字节；解释写在原文外。数值、单位、日期和许可原文也须准确，不能随文案润色而改变。
5. **事实和含义准确。** 分清规范要求、当前实现、测试观察和后续计划，如实写明失败、未测和未完成。改写或翻译须保留“必须／可以”、触发条件、作用范围、依赖关系、“同时／任选”和证据限制，不能改变技术含义或历史结论。
6. **在负责处维护。** 同一规则或操作由一处负责，其他入口用链接引用。实现或入口变化时，同步更新受影响的规范、说明和当前链接。生成文档只改登记源，再运行正式生成入口；AGENTS 的链接关系按现有登记维护。
7. **按改动检查并交付。** 核对语义、示例、技术标识和链接，按登记的 DELTA 影响执行适用检查。生成成功只说明生成已完成，检查结果只覆盖实际验证范围。交付沿用当前任务授权及下文登记的入口，报告实际结果并核对落地。

本仓采用[宿主默认合并方案](../../docs/merging.md)：独立 PR 并行检查，GitHub 关闭 strict，保留唯一必需汇总检查。PR 验固定合并树，dev push 检查实际落地变化；只有实际依赖或冲突决定先后。远端规则的部署状态以实际 API 结果为准，提交配置不代表已经生效。

## 当前固定入口

### 安装候选工具

```sh
/bin/bash .chrono-harness/ci/bootstrap-native.sh . .chrono-harness/ci/bootstrap-core.json
```

检查前，从候选源码执行 Rust core bootstrap。薄 shell 入口先委托已有所有者保护前端编译，随后运行保留在 bin 中的前端；首次 Git main 检出可以初始化。Rust 版本、操作和安装产物由 bootstrap-core.json／projects.json 决定，不用环境默认值替代，也不改全局默认工具链。core 和 Verus 准备已使用 Rust；其他仍采用 Python 的 profile 和测试继续绑定本 macOS 宿主的 `/usr/bin/python3`／Python 3.9.6。工具观察或摘要不能证明完整来源。关联检出缺少兼容协调者或所有权信息时，在构建前失败。详见 [CI 启动](../../docs/ci.md#this-repository)和[生命周期参与](../../docs/worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)。

### 本地／CI 检查

```sh
.chrono-harness/bin/chrono-harness check
```

候选必须已提交，且检出干净。`--unit ID` 和无值的 `--collect` 分别选择同一入口的单元模式与汇总模式；汇总不重执行业务。schema4 从 config.json 的 `canonical_check` 选择 profile 和输入 action。`CHRONO_CHECK_SOURCE` 未设置或为 `local` 时，使用登记的工作树输入程序；为 `ci` 时，使用登记的 CI 事件输入程序。其他值或缺输入都报错，不猜范围、不回退。本地固定登记 remote 的 workflow target 和干净 HEAD，不手工 prepare。

本宿主采用 scoped `chrono-ci-check/v3`、一个 DELTA detector 和始终运行的汇总任务，唯一必需检查是 `chrono / collection`。目标为 dev 或 integration/** 的 PR 以固定 workflow 合并提交的第一父提交为基线、固定合并提交为候选，另保留事件 head 核对 GitHub 运行和工件，dev push 检查落地 before／after。integration 分支需要候选检查时创建 PR，草稿 PR 也支持；推送本身不另行触发主 CI。父运行按 PR 号替换旧运行，push 则按 run ID 保留每次运行。单元和报告由 [check.json](../ci/check.json) 维护，拓扑、共享启动和上传由 [units.json](../ci/units.json) 维护，不另维护单元计数。

[CI 单元说明](../../docs/ci-units.md#conditional-jobs-and-the-required-aggregate)规定如何核对原始 producer／artifact attempt、沿用成功前置结果，以及共享安装。不能按当前重跑 attempt 猜产物归属。业务构建仍使用独立计划；本地和发布继续保留源码构建。

退出 0 必须对应有效且通过的 response。判官失败退出 1；用法、配置、传输、协议或报告 IO 失败退出 2。本宿主采用 `retained-reference/v2`：固定 check.json 引用原报告，原报告引用 stdout／stderr 原字节。使用方核对原件和输出流摘要后，从原 stdout 解码；上传时包含完整登记目录。`report_bytes` 同时限制元数据与两条流的总量，固定引用也受同一上限限制。详见[报告消费](../../docs/ci-units.md#offline-collection)。生成或上传成功不等于检查通过。

### 指令生成

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

这是 projects.json 的 `instructions.generate` action。本宿主按“需求→审计→重构→开发”推进：先核对实际需求、SPEC、已有能力、职责归属、明确影响和成本，再收窄实现；验证真实使用结果后，再核 SPEC。修改宿主 catalog、manifest、登记的 file 源或上下文后，都运行此入口。

schema 2／`atomic-rules/relative-alias/v3`、四阶段简短根指南，以及同一 manifest 的完整中文指南和中文 skill 输出，见[指令说明](../../docs/instructions.md)。生成成功或文字简短，都不能证明 AI 已阅读、遵守、完成等价翻译或执行检查。[实际消费范围](../../docs/methodology-extraction.md#实际消费者边界)还包括上下文和根块外内容，不截断规则，也不改全局配置。

### 受保护消费与完成

```sh
.chrono-harness/bin/chrono-worktree use --operation <已登记操作> --path <工作树>
.chrono-harness/bin/chrono-worktree finish --path <工作树>
.chrono-harness/bin/chrono-worktree maintain
```

执行登记操作时，必须持有 use 保护。调用方等待全部自有任务和后台任务，核对实际完成及落地后，从仍存活的协调宿主执行 finish；maintain 负责独立重试。创建、重建、导入旧 checkout 和显式政策 migrate，按[工作树说明](../../docs/worktree.md)执行；Git／GitHub 交付沿用上文的自动交付授权。

本宿主在 worktree.json／cleanup.json 中采用 automatic_cleanup/v2。协调锚是 Git owner inventory 的 main worktree，状态位于 `.chrono-harness/state/automatic-cleanup-v2/`。start／reconstruct 自动登记，只维护目标记录；本宿主采用的 bootstrap／check 委托同一负责方，不扫描无关工作树或输出。跨对象清理走显式 maintain。

每个工作树有独立地址和 metadata 租约。最后一个内核租约释放后，该对象的普通入口可以回收未 finish 的可重建缓存；活子孙进程、源码、分支、尚未保全的提交、bin 和证据都保留，也不因此设置完成状态。只采用已验证的描述符转交链，不按年龄、目录名或空闲删除。

显式维护遇到无关登记项清理失败时，保留原回执和 `cleanup_failures`，继续执行合法目标；目标身份、政策、所有权或协调状态写入失败，仍会阻断。显式 maintain 仍报告失败。未封口 birth 只有在实际排除旧使用方，并核对原附件和政策后才能恢复；原来缺失或部分完成的结果继续保留。已发布缓存尝试后出现新使用时，开启新代际，不覆盖旧 intent／result。命令原 stdout、stderr 和退出码都保留；stderr 非空不等于生命周期失败。

二进制必须理解当前已采用的 policy／config／participation。各工作树分别绑定自己的已提交政策，共享状态只核所有权协议、物理协调者和状态目录；清理选择无需与协调者逐字相同。共享保留生产者仍使用其物理存储所有者的政策。调用方负责等待或排除旧程序、保留原状态、有序部署并核对切换；未知归属继续保护。migrate 采用目标自己的已提交输入，不清理，也不宣称完成。[v2 切换和租约限制](../../docs/worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)仍要求候选、支持平台和真实宿主验收。

输出保留采用 retention/v2。每个输出和真实使用方各有独立声明；普通发布、读取和完成不取得全库准入锁，也不扫描总清单。512 上限只约束实际关系集合，已删除全库数量和总字节准入；单输出及每轮节点、字节、25 ms 上限继续生效。显式维护以非阻塞方式处理实际关联对象；局部繁忙或损坏时保留错误，继续处理同级对象。没有遍历或无法计数时报告 unknown。

源与宿主政策同时变更。其他协调宿主仍在使用的总清单，须先由保管者排除旧使用方、保留原字节，再按精确声明切换；普通程序不兼容旧总清单。本仓原主检出已完成受保护切换、共享 owner 部署和普通 bootstrap 验收，也已验证：无关旧工作树持有 metadata 独占租约时，可以启动新工作树。原件和独有提交继续保留。覆盖表只记录固定交付范围，不凭局部成功宣称完整 SPEC 已完成。

## 按任务读取现有文档

| 当前任务 | 负责文档与适用条件 |
| --- | --- |
| 改登记、执行计划或测试 | [影响范围](../../docs/filemap-impact.md)、[执行、分组和语言](../../docs/execution.md)、[单元库存](../../docs/ci-units.md)：本宿主 cap=2，不越过依赖、共享 claim 或登记时限。跨调用隔离由调用方负责。分组、真实未过滤库存，以及原断言、线程和发布操作的保留要求，见单元说明。语言声明不能证明其余跨语言逻辑已迁移。 |
| 改输入或 full 采用 | [输入与同判条件](../../docs/inputs.md)、[Git facts](../../docs/git-facts.md)、[full CI](../../docs/full-ci.md)：快照不补依赖、不改预期摘要、不启用治理。候选解码器读取固定历史数据，不执行旧判官。full unscoped 需要原 origin receipt 和历史证据，缺失则失败。当前 projects 测试使用原未过滤 action；通用转换接口用自含合成测试数据验证，不获取旧仓库，也不提供已退役宿主的兼容入口。 |
| 改 CI 缓存 | [缓存说明](../../docs/ci-cache.md)：普通 plan／restore、producer-manifest 内容验证、失败恢复、原构建与测试、seal／save 和原始 outcome 报告均有当前使用方。专属远端激活层已退役，不增加替代入口。目前只支持 primary checkout，完整验收仍有缺口；具体观察和原件入口由缓存说明维护。缓存测试仅在失败时保留原测试数据、探测配置和报告。 |
| 改发布或消费关系 | [发布说明](../../docs/release-ci.md)：v5 明确登记 needs／rust_toolchain／verification_consumers，保留原未过滤操作、producer／run／attempt 和失败证据。declared 不证明必要性；本地、原生和公开采用分别验收。 |
| 读成本或排查失败 | [汇总观察](../../docs/ci-units.md#github-projection-and-transport)、[诊断范围](../../docs/diagnostics.md)：先读原件。当前 attempt 的 API 视图可能包含沿用结果，新 job ID 不证明任务重跑。canonical-check 包含构建、测试和汇总；读数不代表完整历史、重跑成本、计费或全流程总量，告警不能证明冗余或节省。 |

## 当前未完成边界

完整治理登记仍为 proposed，`enforcement=not-implemented`、`input_closure=incomplete`。Cargo、编译器、SDK、配置及其他适用输入和执行程序的绑定，仍有缺口。

宿主 Git facts 采用 `chrono-git-configs/v3` 精确摘要选择；实际映射和固定数据路径归[选择器](../config.json)，工作树接受的主身份及补充身份归[工作树政策](../worktree.json)。版本预期和完整验收仍须实际 bound probe 验证，未知摘要仍在执行前拒绝。主机、镜像、路径及工具观察直接查本轮 state／CI 原件，不在本上下文复制运行号或摘要。

政策与产品同时变更时，候选决定实际执行程序。固定端点先按候选程序身份读取明确登记；端点未登记该身份时，只读取候选声明的 registry_path 和端点原字节，不启动历史 Git，也不猜其他路径。额外读取和完整验证成本未测。

日常本机检查的 CARGO_HOME 使用当前检出的 `.chrono-harness/cache/bootstrap-owner-tooling/cargo`，与 bootstrap 的可重建工具缓存登记一致；HOME／TMPDIR 沿用继承环境。bootstrap 绑定所选 Rust 工具的版本和组件摘要；SDKROOT／DEVELOPER_DIR 等路径选择不等于 SDK 内容绑定，也不证明祖先 Cargo 配置及其余输入完整。当前声明与采用缺口归[输入说明](../../docs/inputs.md)，运行观察归原件。

普通 full check 在登记要求和所选义务通过后，只表示 DELTA 满足当前合同。`declared-complete` 是负责方的工程声明，registration 仍报告 `completeness_proven:false`。已知缺输入、必需依赖未解决、缺绑定或快照，以及漂移，都必须失败；不另要求普遍的隐藏输入证明或 VM。

同一命令已经采用；要主张同判，仍需完整相同的有效输入和确定性求值。当前程序不建立更强的输入完整性保证，因此报告保持“尚未建立 parity”。独立比较或改写命令、固定 `spec status` 已退役，也不另加普通 full 关卡。

缓存已有部分原生观察；坏内容恢复或替换，不等于修复后在原兼容域健康精确命中，该项仍未完整验收。linked transport 租约、其他使用方、并发及完整成本和生命周期合同仍有缺口，具体范围见[缓存说明](../../docs/ci-cache.md)。已退役的远端激活层不作为当前推进依赖。

当前 scoped 路径只采用 runner-startup 的版本化内部消费关系检查。full judge 的类型化 prelaunch 和报告诊断已有范围有限的实现。detector→startup 导入与安装→runner 检查→原件汇总链已有 Darwin 原生观察；Linux、外部及全部使用方采用仍未完成，具体限制见负责文档和[覆盖表](../../docs/spec-coverage.md)。旧实验、回退和测试读数直接查 Git、原 PR／CI，不在本上下文维护副本。

完整宿主启用、full 原生 CI、端到端交付和完整 SPEC 验收，仍未完成；阶段成功不是交付。成本仍有 unmeasured 项。来源和许可是可选阅读资料，不是执行政策。Git 生命周期沿用上文的自动交付授权和当前任务范围；不改全局配置、参考仓库或其他工作树，不把过程转录写入产品源。
