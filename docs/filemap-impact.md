# 文件变更的影响范围

`chrono-judge-filemap --protocol chrono-judge/v1` 读取 registration 统一解释并绑定的登记，计算变更会影响哪些节点和测试，结果放在 `outputs.impact`。它使用 runner 已有的 Git 事实、协议身份和传输能力；需要 registration 的实际进程结果，不能用一次库调用冒充该程序运行过。

格式、准备状态、工作树和直接引用是否合法，仍由 registration 负责。它的不可变 `node_data()` 是完整格式节点清单的唯一来源。读到清单本身，不代表内容含义有效或输入已经齐全。

编译依赖方向是 `judge-ci → judge-filemap → judge-mixed → judge-registration → runner`，需要 Git 事实或传输的项目也直接依赖 runner。filemap 的测试项目独立拥有清单、锁文件和构建目录，没有根 workspace 或通用登记项目。

## 按配置选择依赖边

FILEMAP v2 可以通过 `configuration_edges` 选择启用 `chrono-filemap-configuration-edges/v1`，并提供非空 `configurations` 数组。每条记录以 `config` 点名一个已登记的直接政策文件，以非空 `edges` 列出已有格式的依赖边。

registration 的共用 `select_filemap` 只把匹配该端实际政策路径的记录加到公共 `project_edges`。没有匹配记录时继续使用公共图。配置上下文缺失、版本未知、政策路径未登记、配置重复或启用后的边重复都会失败；旧 v1 格式拒绝这个扩展。程序不会从目录、平台名或可执行文件观察推导依赖边。

保留快照保存源文件字节，图和记录的消费者在两端分别应用同一选择逻辑。因此增加、删除或修改启用边时，仍按两端合并图计算影响。

**产品与本仓政策曾同步修改：**独立 macOS 27 输入边从公共表移到这个明确范围，必需绑定和预期字节保留；固定的原生政策继续使用公共图。这类修改要求 Rust 格式／选择行为和两端影响测试、源码 bootstrap、标准变更检查，以及真实 CI 的 detector／消费者验证。新增构建、验证和维护成本未测；Git facts 的身份选择和工作树政策各由其负责处维护。

<a id="bounded-internal-causal-consumption"></a>

## 核对内部启动结果的实际用途

当前按范围检查的政策可通过 `causal_consumption.schema: chrono-internal-causal-consumption/v1` 启用这项检查。非空 `shared_startup` 数组中的记录各自包含唯一的 `{id, units, startup, consumer, producer, caller, evidence}`；其中 `evidence` 为 `{test, operation}`。路径必须指向已登记且存在的宿主文档，操作和测试身份由 projects 与执行计划解析。版本／字段未知、链条重复或身份为空都会失败。目前只检查内部共享启动结果被单元使用的关系。

本仓采用的 `runner-startup` 链是：detector 生产操作 → `host.bootstrap-shared-install-unit_runner` → 安装后的 runner／核心工具 → 当前 runner 单元检查。需求和产物用途写在所选单元配置里；配置、构建操作及安装源／目的归 bootstrap 登记；调用参数与负责人归 projects；构建产物声明和证据计划归 FILEMAP。不再建第二份清单，也不自动发现关系。

共享启动链可用 `data_outputs` 明确登记数据文件的用途：键为已安装目的路径，值为 `{producer, consumer}`。生产操作必须属于安装该文件的 bootstrap profile，使用操作必须属于该链 `preparation` 的正式操作，两者都须已登记。源和目的仍只由 install 清单负责，不从参数或文件名推测。漏登、未安装、错误生产者或未选使用操作均失败；普通工具仍核原输出占用和工具角色。本宿主据此登记 Verus 原压缩包的生产与单元解压，不把包登记为可执行工具。

**判官与宿主政策同时变更：**增加上述普通数据文件关系及成功／拒绝回归，保留当前工具角色、两端选择和真实消费证据要求。新增关系读取和完整验证成本未测。

`test.host-bootstrap-native-tests` 用 Rust mock 隔离外部命令，核对 bootstrap 调用、委托／拒绝、原报告与输出、阶段复用及源码变化时的拒绝。旧真实消费者测试入口已退役；mock 不证明真实工具或后续 runner 测试通过。实际启动清单、安装及 runner 检查仍由正式 CI 生产链提供原始结果，registration 核对声明，projects 与汇总核对实际执行。

**按新测试规则迁移：**消费证据测试在外部命令边界使用 mock。判官继续要求证据操作、测试配对、执行计划和输出声明完整；输出声明允许空数组，原始测试 stdout／stderr 由现有检查回执保留。缺失声明仍失败，mock 不证明生产工具行为。

标准检查在选单元前调用 registration。固定任一端的规则、点名源路径或影响到的操作负责人变化，都会选中这条链。删除或重新绑定后，仅旧端存在的引用仍会选中。选中链必须在候选中可解析；直接关闭这项检查属于未经证明的移除，会失败。

需求／用途、输入、调用方、配置、生产者声明、安装工具角色、当前检查绑定或必需证据缺失时返回 `E_CONSUMPTION`；已移除链使用 `E_CONSUMPTION_RETIRED`。选中链把明确登记的证据测试加到同一检查要求中，单元分配和原始证据规则不变。未受影响的链只作为历史上下文报告；未采用扩展的消费者保留原含义。

这里同时增加了产品的一致性规则和本仓对一条已有链的采用。验证成本包括额外读取固定输入、回归测试和运行已登记的证据测试；完整构建／CI／存储成本未知。程序不能证明需求有价值或实现忠于需求，这仍需要工程判断。外部和全部消费者采用、完整调用路径检查及全部 SPEC 启用仍未完成。

该内部启动链已有 Darwin 上的有限原生观察；适用输入、产物及原件直接查[原运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/39695762671)，不在本文维护运行和产物清单。它不能证明 Linux 或全部使用方已采用。

真实 CI 交接沿用指向 dev 的 PR 和 macos-26 父工作流：先取得 detector 当前共享产物、干净生产者的原报告和清单，再实际执行 `unit_runner` 安装／验证与 `.chrono-harness/bin/chrono-harness check --unit runner`，最后走已有汇总。

必须保留启动绑定、精确产物 ID、实际生产 run／attempt、`startup-install.json`、导入的 bootstrap 来源、清单及文件身份、封存的标准请求／报告、原 stdout／stderr 和上传单元产物绑定。各处检出同一个固定 PR 合并提交，GitHub 运行归属另按事件 head 核对，具体绑定见[合并方案](merging.md#检查与落地)。重跑时，沿用的 detector 结果要追溯到原生产者和产物 attempt，不能假定来自当前 attempt。

汇总必须读取实际选中单元的原件和成功业务响应。本地测试样本或生成工作流不能代替远端观察。产物沿用 provider 已有的有限保留期限；调用方须先等待消费者完成、收取必需证据，再走受保护的生命周期清理。

## 带版本的输出

`schema` 为 `chrono-filemap-impact/v2`。Rust 消费者可反序列化成 `chrono_judge_filemap::Impact`；JSON 消费者必须核对格式。外层 v1 请求／报告绑定两端提交、文件树和登记摘要。数组与映射按确定顺序输出，未知成本保留为 JSON `null`。

结果说明登记图中的影响；提供并验证实际环境变化时也将其纳入。它不证明操作已执行、移除已获准或成本已合格。

| 字段 | 含义 |
| --- | --- |
| `workflow` | 明确的稳定性／集成要求、共用分类、各端来源指针及转换后的测试边；只负责选测 |
| `delta` | 完整的新增／修改／删除路径、旧新 blob 和文件模式；不靠相似度猜重命名 |
| `records` | 稳定记录 ID → `{base, candidate}`，两值都可为空；每条保留登记 `path`、明确 `targets` 和标准化 `value` |
| `changes` | `{record, fields}`，用相对 JSON 指针列变化字段；空指针表示整条新增／删除，`/$registry_path` 表示移动登记位置 |
| `nodes` | 节点 ID → 可唯一解析的 `base`／`candidate`，另存完整 `{identity, value}` 数组 `base_definitions`／`candidate_definitions`，包括仅旧端存在和有歧义的定义 |
| `edges` | 两端全部明确边的平表 `{from, kind, to, origin}`；来源为 `base`、`candidate` 或 `both` |
| `seeds` | 排序去重的影响起点节点 ID |
| `seed_causes` | 可累加的 `{id, node, reference, reason}`；引用为实际变更路径、稳定记录 ID 或明确工作流要求指针；原因为 `delta-path`、`record-change` 或 `edge-change` |
| `closure.reached` | 每个到达节点及所有使它受影响的起点 ID |
| `closure.predecessors` | 起点 ID → 节点 → 一条最短前驱边；起点处为 `null`，可据此还原有限的连续原因链 |
| `closure.traversed` | 每条走过的边及所有相关起点；保留分叉后汇合的不同路径和环，不枚举无穷路径 |
| `tests` | 候选仍存在的已选测试 ID，排序去重，也包含歧义 ID；存在不代表唯一或已执行 |
| `retired_tests` | 候选已不存在的必需测试 ID，排序去重；只说明移除事实，不表示允许移除 |
| `required_tests` | 全部已选 ID、`candidate_present`、`candidate_ambiguous`、实际选测的 `execution_edges`，及可为空的旧新成本 `base_cost`／`candidate_cost`，含引用和值 |
| `judges` | 候选中每个每次变更必跑的检查程序只列一次，附受影响的 `judge-trigger` 边作为原因 |
| `historical_context` | 未受影响的边含义错误及所属端；不算本次问题 |
| `historical_ambiguities` | 未受影响的歧义节点、所属端和定义身份；只记录事实 |
| `limits` | 明确列出限制：不判断执行／移除／成本，未保留的外部字节，以及尚未证明的完整性和影响范围正确性 |

稳定记录 ID 以 `/records/<registry>/<collection>/<key>` 开头，key 使用 JSON Pointer 转义。它是 `records` 内的身份，不是源 JSON 的数组下标。

| 记录种类 | key |
| --- | --- |
| 文件 | 路径 |
| project／script／judge／input／tool／stability | ID |
| owner | 名称 |
| 测试成本 | 无前缀测试 ID |
| 成本模型 | 成本 ID |
| 项目边 | 紧凑 JSON 元组 `[from,kind,to]` |
| 其他单值字段，包括没有唯一身份的数组 | `fields/<field-name>` |

输入声明各用 `config/inputs/<id>`；环境变量各用 `config/environment/<name>`，并明确登记消费者边。身份数组按排序后的声明集合比较，重复声明仍留在标准化值中；`argv`、`version_argv` 和计划 `operations` 保留顺序。记录的 `targets` 本身不生成依赖边。

完整求值会把 registration 核验过的保留 `effective` 环境值加到两端记录。`null` 表示缺失，区别于空字符串；配置值覆盖继承值。`produce` 只读取声明；`produce_with_inputs` 通过 registration 输入 API 验证并使用实际保留事实。两者共用图、记录和历史解释，缺失或不一致的实际事实不能当作没有变化。

registration 清单保留每条定义及其身份，例如 `project:t` 与 `script:t` 都可能定义 `test:t`。只有恰好一个定义时，`NodeView::unique()` 才返回它。协议中的 `base`／`candidate` 为 `null` 时可能是缺失，也可能是歧义，要通过对应定义数组区分零个、一个和多个定义；冲突时不能任取一个可执行值。project 和 script 都在 `targets` 中保留测试别名，filemap 直接复用，不重新查找别名。

两端完整记录保留负责人、旧成本引用和模型值。已移除的必需测试仍在 `required_tests` 中，标为 `candidate_present:false`，并进入 `retired_tests`。`tests`／`retired_tests` 只按候选是否存在分组，不存放“已执行”或“允许移除”状态。进入 `retired_tests` 不能豁免 `E_REQUIRED_TEST_REMOVED`；后续 projects／workflow 仍须核对 SPEC 要求的移除／替代证据，缺少时必须失败。projects 验证替代项实际执行；工作流的移除证明仍有未完成项。缺少成本声明记为 `null`，不能写成零；后续程序各自作出判断。

## 影响怎么传播，原因怎么解释

文件新增／修改／删除以 `file:<path>` 为起点。标准化记录变化以两端 `targets` 为起点，包括 registration 为 project／script 提供的可执行测试别名。输入声明变化以 `input:<id>` 为起点。依赖边新增、删除或改目标时，两端受影响节点都成为起点；删除的边仍留在合并图中。

明确的工作流路径／测试和有条件集成测试先转成有类型的边，再进入同一计算，见[工作流选测](workflow-selection.md)。传播只沿登记边前进，直到不再出现新节点；不从配对、目录、import、Cargo 元数据或运行时发现中补依赖。

| 边类型 | 允许的起点类型 | 目标类型 |
| --- | --- | --- |
| `compile` | file、project | project |
| `build-input`／`runtime-input` | file、input、tool、environment、project、script | project、script |
| `test-execution` | file、input、tool、environment、project、script | test |
| `judge-trigger` | file、project | judge |

完整规则下，只有走过 `test-execution` 边才选中测试。测试本身是起点、成本改变或存在配对关系，都不意味着要执行。每次变更必跑的检查程序在空变更下也会选中；触发边只补原因，不重复选中。

比较外部输入声明不代表实际字节已保留。真实 registration 仍以 `E_EVIDENCE_UNRESOLVED` 拒绝非空却未保留的外部输入；直接测试 input 节点只覆盖声明层。

受影响或已变化的无效引用、错误节点类型分别报 `E_DANGLING_EDGE`、`E_EDGE_TYPE`。受影响 ID 有歧义时报 `E_NODE_AMBIGUOUS`，除非有限历史修复已说明全部旧定义及唯一当前身份；候选仍有歧义时一律失败。即使只由记录变化触发、还没走过执行边，也适用这条规则。未受影响的歧义只记事实，加载所有节点不等于全仓判定。

问题中的 `delta_refs` 指向起点路径或变化记录。`causes` 从该起点沿真实合并边连续走到错误端点或歧义 ID；直接记录起点可以只有一个节点。消息列出所属登记端及定义或边。即使报告选择一条确定的原因链，影响结果仍保留 `seed_causes` 中全部相关原因 ID。必需登记／对象缺失或无法解析、伪造空变更、摘要不符都返回错误，不能输出空成功结果。

## 图计算的范围

实际内核使用同源 Verus 保全类型边、两端来源、精确名称和槽位、FIFO 最短前驱、有限原因链、传播行、有序原因集合及逐 seed 流式状态／根对应；选测直接从真实遍历记录提取全部 test-execution 目标，排序去重。诊断按完整 seed ID 借用首个原记录，无匹配时返回 None，两处使用方直接消费同源证明。实际边原因更新直接在原有类型边 map 上证明精确并集、对应起点 ID 及无关记录保留。当前证明边界、标准库返回转换接口与执行入口见 [Verus 实施范围](verus.md)。节点原因关联、完整遍历／输出组合、seed-ID 生产及关联完整性、JSON 适配及额外选测仍待证明。

当前消费者只需要两端合并图和有限影响传播，保留有类型边、所有起点、不同路径及环。程序不再额外计算强连通分组、缩点图深度、关系数量、跨负责人排名或执行冲突汇总，因为这些结果没有现役生产消费者。

**产品与政策曾同步修改：**SPEC §5.2 不再要求每次影响计算都附这些观察，对应的报告自检和两项专用测试随之移除。已有环／分叉汇合／多起点回归仍检查真实边和原因链，runner 消费者仍核对影响和选测。执行 DAG、资源及输出占用声明归 routes，相关检查及执行回执自诊断仍为必需。普通影响格式与权威选测字段不变。

重构须对应真实消费者并保留明确测试义务。本次变更能使用已有图事实和负责人检查时直接复用；新增测量要有具体用途。删除带来的运行、报告体积和构建收益，在候选实测前均为未知。

## CI 适配器和退出条件

`judge-ci` 保留已有的按范围快照、首次采用／初始清单语义，以及操作、绑定、工具、环境、执行限制触发、验证、参数执行和退出码传递。遍历改用 filemap 的 `graph::union`、`graph::closure`、`Closure::selected_tests`，移除原私有实现。这个 API 接收明确边和起点，不加载或强制旧宿主采用完整登记格式。

CI 证据增加 `selection_explanation`，范围为 `chrono-ci-check/v1-adapter`，保存平表合并边、节点 ID `seeds`、可累加 `seed_causes`、传播结果、`extra_selections` 和 `legacy_only_selections`。额外原因明确说明操作、绑定、工具、环境或限制变化，但不能把原因文字当作白名单边或完整规则权威。原 CI 套件继续覆盖命令选择和执行语义；新增测试覆盖空图下旧版选择及执行限制变化。

当完整 routes／projects、保留输入和迁移证据能够替代实际本地／真实 CI 消费者后，这个适配器才能退出。共享执行和回执核对已归 routes／projects；有限输入和 ci-verify 迁移见[执行说明](execution.md)。工作流证明、分支检查、完整 Cargo／SDK 输入、全面启用及真实 CI 同判仍有后续工作，本仓仍为 `proposed`／输入不齐。

## 直接验证

专属 `impact` 测试独立定义小图，再核对原因链是否真的沿这些边前进。范围包括内容／模式／二进制变化、删除加新增式改名、非祖先和多提交比较、记录／字段／边变化、全部边来源、移除负责人／成本／测试、无变化／文档、有类型传播、仅配对不选测、检查触发、环／分叉汇合／多起点，以及数组与参数顺序的区别。

`consumer` 在已提交的临时仓库中，从其他工作目录实际运行构建后的 runner、registration、filemap；仓库路径含空格，不提供 HOME，也没有历史二进制绑定。它核对报告指针、缺失／畸形输入、伪造变更／对象／摘要及保留输入的边界。这些是单工作者直接检查，不算独立评审或真实 CI 证据。

共享清单回归直接核对不同定义和安全的唯一解析。真实消费者回归覆盖 project／script 同 ID、不同 ID 成功、受影响与无关历史冲突、空变更、仅别名变化、两端记录目标、仅基线歧义，以及 SPEC 要求的协议字段，包含尚未获准移除的必需测试。

先执行登记的 `verify.filemap-graph`、`verify.filemap-graph-tests`，完成整 crate 证明及编译。随后构建前置是登记的 `build.runner`、`build.judge-registration`、`build.judge-filemap`、`build.judge-routes`、`build.judge-projects`，之后运行 `cargo test --locked --manifest-path crates/judge-filemap-tests/Cargo.toml`。CI 绑定明确列出这些前置。测试使用本地合成提交，目标候选的生命周期仍由调用方负责。

impact v2 新增工作流选测，v1 不执行这些要求；消费者拒绝不支持的结果格式。Git 中 FILEMAP 登记仍为 schema 2，历史解码不变。共用 mixed 分类和固定对象读取在制定执行计划前完成，无效的已选含义输入会阻止后续操作。`workflow_selection`、`workflow_consumer` 通过真实进程验证这些要求。
