# 宿主指令生成：atomic-rules/relative-alias/v3

`chrono-instructions` 把登记的规则生成工作指南。它是独立 Rust 库，CLI 只负责参数和调用；生产项目与专属测试项目为 instructions ↔ instructions-tests，没有根 workspace，也不依赖 runner。

专属测试自行编译真实库和 CLI 源码，只消费测试项目的 target；测试计划和单元缓存不再包含未被读取的生产 target。日常构建、生成及发布仍保留各自的正式入口；发布所用生产缓存不受此删减影响。

生成器校验明确登记的数据，解析内容依赖，再用同一个普通 IO 发布器写出生成副本。它不执行判官，不猜语义依赖、项目、语言或政策，不翻译，也不联网解析包。

## 调用与唯一编辑源

```sh
cargo install --locked --path crates/instructions
chrono-instructions init --host-root "/path/to/existing host"
# 新宿主可以明确选择已有英文规则；程序不会翻译
chrono-instructions init --host-root "/path/to/another host" --locale en
# 修改宿主的 catalog、manifest 或登记的 file 源后，重新生成
chrono-instructions generate --host-root "/path/to/existing host"
```

默认 init 使用编译时内嵌的 `assets/instructions/catalog.json` 和只含根输出的 `default-manifest.json`，在新宿主创建独立的 `.chrono-harness/instructions/catalog.json`、`manifest.json` 和空 `host-context.md`。复制后的二进制无需源码 checkout。产品资产和宿主采用数据分别维护；升级二进制不会覆盖已经采用的数据。要采用新默认，必须先比较差异、编辑宿主数据，再 generate。

默认根从已有双语规则中明确选择内容，按 `workflow` 的“需求→审计→重构→开发”四阶段组织；验证与 SPEC 迭代、交付与清理归开发阶段。它采用成熟的工程实践，以现有负责位置和复用落实 KISS、YAGNI、DRY：先核对实际需求、使用方、当前 SPEC、已有能力、唯一职责归属和明确依赖，再按产品价值、交付时间和全生命周期成本，选择最简单且足够的方案。成本包括实现、构建、验证、CI、存储、维护和复杂度。现有工具或不改动就足够时，直接采用；未知收益或成本不能成为扩大范围的理由。

新增依赖、原生工具链、自研或防御机制、兼容层和验证路径，必须说明当前需要，以及成熟方案为何不足。够用就复用，有证据才在负责位置重构，开发只补真实缺口。审查和测试按实际风险及明确依赖收窄，不要求每次全仓审查、全测或无条件重构。每项受治理操作只提供一个当前登记路径；定制先登记，方法变更同步更新登记并验证。完整规则库、`core.general` 和 `general` 布局仍可用于自定义 Markdown／skill，不自动覆盖已有宿主的 manifest。

CLI 接受 `init --host-root H [--methodology M] [--host-context C] [--locale L]` 和 `generate --host-root H`。H 必须已经存在，各选项只能出现一次，路径按操作系统路径传递。新宿主省略 locale 时选择 zh-CN。明确给出 methodology 时，程序把文件作为不解析内容的 file atom 保留；未指定 locale 就绑定 und，指定后就绑定该语言，不做翻译。初始 locale 必须已在内嵌 catalog 声明。

默认根标题只属于默认 zh-CN 绑定；换其他 locale 时省略这个可选标题，作者可以在 manifest 中明确添加对应语言的标题。context 可独立选择。外部 M／C 必须是可读的 UTF-8 普通文件，调用方可以明确选择链接。

对已登记宿主，init 中省略的选项保留原数据。明确指定不同的 context、原始方法或根 locale 会被拒绝，提示编辑源和输出计划后再 generate。已经拆成单条规则的根，不能由原始 methodology 替换。源缺失或登记损坏时，不补默认；没有登记、却已有保留源或控制文件时，拒绝碰撞。公共接口 `init`、`init_with_defaults`、`generate`、`dispatch` 继续可用。

退出码 0 表示完成、无需写入、帮助或版本；2 表示 CLI 用法或日志错误；1 表示输入、生成、业务 IO 或平台错误。成功时报告实际改动数和 `no judges executed`。独立的 `chrono-harness check` 已有 [CI 范围检查](ci.md) 和有限范围的 full 判官链，但完整宿主启用仍未完成。指令生成不能证明 AI 已阅读、遵守、执行判官或通过检查。

CLI 复用标准诊断库，按明确 `--host-root`（省略时为当前目录）读取宿主 `.chrono-harness/logging.json`，记录开始、返回和原诊断文本，stdout 保持原业务输出。日志初始化失败时不启动业务；写入失败时保留业务输出及产物，最终退出 2，并向 stderr 尝试发布日志错误及原业务返回码。日志格式、保留上限和无配置时的行为归[诊断说明](diagnostics.md)，库接口 `dispatch` 仍只返回调用结果。

**产品与宿主登记同步修改：**instructions 入口接入现有 diagnostics，FILEMAP 登记编译及日志输入关系，专属 Rust 测试项目编译真实 main 源码，覆盖显式宿主、成功、生成／用法失败和日志初始化／写入失败。沿用原构建、未过滤测试及 DELTA 检查入口，不增加日志机制或检查路径。局部测试已通过；完整 CI 和生命周期成本仍以实际验收为准。

## 新默认的宿主采用边界

**同时修改产品默认与宿主政策：**本轮整理两份 catalog 的中文表达，保留稳定 ID、依赖、结构和英文内容；本仓 manifest 改为生成完整中文指南与中文诊断 skill，产品默认仍只生成根，英文输出能力保留。输出路径和 FILEMAP 同步迁移。根、完整指南和技能都从源生成，不新增措辞形状测试。生成与适用验证使用现有入口；实测成本以实际结果为准，总生命周期成本仍未知。这是可见提示，不增加人工确认关卡，也不豁免适用检查。

新宿主 init 采用二进制内嵌默认；已有宿主升级二进制或重复 init，仍保留自己的 catalog、manifest、context 和根块外内容。没有自动升级、合并或更新子系统。

**必要性规则同时更新产品默认与宿主采用：**现有 `goal.deliverable` 的中英文正文加入 SPEC §1.1 的逐跳“为什么”要求，适用于所有受跟踪或受执行对象。它明确当前终点、每跳证据、共享分支、unknown、禁止新增无必要对象，以及已有对象的补证／迁移／退役；需求或使用方变化后重新核对。默认根和本仓根／完整指南都从现有选择消费这条规则，不新增 atom、schema、判官或人工确认关卡。生成只发布方法文字，不能证明 AI 遵守或全仓审计已完成。必要性由 AI 基于当前事实判断；现有判官按明确合同检查关系和原件，不另建价值机器门或重复对象账本。验证复用现有生成与 DELTA 检查；完整成本仍未知。

**退役判据同时修正产品默认与宿主采用：**同一规则明确保留有具体需求、使用方和验收目标的在研功能；删除前查原始需求、引入 PR 及当前接入工作。未调用、未发布或证据缺失不能单独推出“无需求”；替代工具须覆盖具体接口和行为合同，不能只因功能名称相似就视作替代。退役须有无当前需求（含取消／过期）或替代迁移已验收的证据，不能通过先删改 SPEC 目标为删除自证。根与完整指南从登记源重新生成，中英文含义同步；沿用生成、现有行为测试和 DELTA 检查，实测验证成本以原件为准。文字发布不证明 AI 已遵守这些判据。

已有宿主采用新默认时，以所选产品源码版本的 `assets/instructions/catalog.json` 和 `default-manifest.json` 为比较输入。在宿主自己的 catalog 中合入所需规则正文和 `workflow` 布局，再在 manifest 根输出选择 roots／layout。保留自定义 atoms、locale、file 源、其他输出、上下文和块外内容，不整份覆盖 catalog／manifest。已有不解析内容的 file 方法仍由原文件负责方编辑，不用 init 替换。

然后走宿主登记的 `instructions.generate` 入口，核对真实根、AGENTS 字面链接和全部选中 Markdown／skill 使用方；重复生成确认无需写入。本仓使用自己的源和三个输出，产品默认仍只生成根。此路径只更新可读方法，不激活宿主判官，不证明 AI 遵守，也不建立 full／native 验收。

当前 `method.host-tools`、`owner.migration`、`action.autonomy`、`action.blocked` 和 `cost.measure` 等规则保留以下要求：以当前规范为准，不再为历史版本、格式、接口或路径保留执行分支、适配器和测试；变更时迁移当前使用方与登记。固定基线事实、仍有用途的源码和失败原件保留为数据与证据，不让旧合同成为运行时权威。局部失败只阻断明确依赖的工作，其他已授权工作继续；不申请额外权限，移除无必要使用方的特殊权限功能，不建替代激活层。普通必要流程保留已有权限，CI 缓存的普通内容验证、恢复、构建和保存仍由缓存负责方维护。

性能和保留要求也继续适用。只保留必要使用方、IO 和进程，验证与 DELTA 影响相称；成本比较保持相同输入、单位和窗口。运行结果与证据要有用途、负责方、有界期限或容量、清理条件。消费结束且条件满足后，回收可重建临时产物；失败原件保留至调试和验收义务完成，保护源码与尚未保全的独有工作。TTL 须先明确资格并检查活跃使用，压缩不能代替回收。交付要完整小步、限制同时进行的工作、及时合并，并按实际落地与验收范围报告。

这些要求复用现有规则和生成器，不增加保留平台或自动证据清理实现。本仓 automatic_cleanup/v2 的操作、租约、回执、兼容切换和保留条件，仍由 host_context 和[工作树说明](worktree.md#kernel-ownership-and-unfinished-cache-recovery-v2)维护。没有新增判官或性能基准套件；生成与适用检查的成本按实际保留结果报告，完整 CI、存储和生命周期成本未知。生成前的旧副本不代表新规则；生成后也不证明遵守、翻译等价或完整验收。

`candidate.landing` 还为新宿主提供[默认合并方案](merging.md)：独立 PR 并行检查、关闭 GitHub strict、验证固定合并树并在目标 push 检查实际落地变化。原有评审及必需检查保留；实际冲突、输入和依赖决定补验范围。产品默认与本仓 catalog 同步采用，完整治理的精确凭据和严格同树操作另按其明确合同验证。生成指南不修改远端规则，也不能证明原生事件或保护配置已生效。

## Schema 与可复制组合配方

默认根明确选择 `job.wait`，本仓也在自己的 catalog／manifest 中采用。任务优先使用一个真实完成通知或原生阻塞等待通道，完成后一次核对原 stdout／stderr、退出码或 signal 和任务身份。产生结果的程序须等待自己的子孙及后台任务，调用方也须等待另行启动的任务。禁止用短周期状态、进程、产物查询，或反复空写、sleep 后查询、改名 wait/watch 的循环代替等待；允许授权内对原任务作长周期核实，不另开并行完成通道或新增确认关卡。

原生阻塞工具可以内部查询，但名称、默认间隔或包装程序退出 0，都不能证明等待合规或任务成功。超时、缺通知、通道丢失或结果不可取得时，保留句柄、已有结果、未知状态和保护，不重复启动；继续独立工作并报告缺少的能力。这些文字不新增轮询器、调度器或进程引擎改动。此前双语 `job.wait` 已从“禁止一切定时查询”改为以上要求；生成本身不能证明 §7.4 或覆盖表 69–71 的运行行为已验收。

manifest 使用 schema_version=2，producer 固定为 `chrono-instructions`，与包版本无关；render 固定为 `atomic-rules/relative-alias/v3`。下面是完整示例。默认宿主只有第一项，增加后两项即可生成中文完整指南和聚焦 skill，本仓也采用这组输出。

```json
{
  "schema_version": 2,
  "producer": "chrono-instructions",
  "render": "atomic-rules/relative-alias/v3",
  "catalog": ".chrono-harness/instructions/catalog.json",
  "host_context": ".chrono-harness/instructions/host-context.md",
  "outputs": [
    {"id":"root","path":"CLAUDE.md","format":"root-guide","locale":"zh-CN","roots":["goal.deliverable","action.autonomy","candidate.isolation","reuse.search","registry.explicit","evidence.failure","cost.measure","owner.canonical","core.small-projects","method.host-tools","method.entry","judge.register","policy.evolution","policy.mixed-warning","delta.selection","delta.candidate-judge","delta.local-ci","test.behavior","test.same-language","evidence.program-state","job.wait","candidate.landing","artifact.results","artifact.cleanup"],"title":"通用工作方法","layout":"workflow"},
    {"id":"general-zh","path":"docs/generated/general-methods.zh-CN.md","format":"markdown","locale":"zh-CN","roots":["core.general"],"title":"通用工作方法：完整指南","layout":"general"},
    {"id":"repair-skill","path":"skills/diagnose-recurring-failures/SKILL.md","format":"skill","locale":"zh-CN","roots":["core.repair-producer"],"skill":{"name":"diagnose-recurring-failures","description":"诊断反复出现的失败，先检查实际证据，再修复产生问题的程序、工具或规则。同一症状第二次需要相同处理动作时使用。"}}
  ]
}
```

恰好有一个 root-guide，路径必须是 `CLAUDE.md`；`AGENTS.md` 只用于目标字面值为 `CLAUDE.md` 的相对链接。发布时先处理根与别名，且 CLAUDE 先于所需别名，再按 outputs 顺序发布额外输出；root-guide 在数组中的位置不改变此顺序。ID 和路径唯一。每项必填 id／path／format／locale／roots；title 可选，必须单行、非空且无控制字符。只有 skill 格式必须且允许提供 skill 元数据。

catalog 使用 schema_version=1，必填 locales 和 atoms 数组。下面是合成示例；替换宿主 catalog 时，要同步修改 manifest 的 roots 和 locale。

```json
{
  "schema_version": 1,
  "locales": [{"id":"zh-CN","root_frame":"修改登记源后重新生成；工作前读取登记的宿主上下文。生成不证明遵守规则。","projection_notice":"本文件由登记源生成。修改源文件后重新生成。"}],
  "atoms": [
    {"id":"example.evidence","requires":[],"variants":[{"locale":"zh-CN","source":{"type":"inline","text":"判断失败原因前，先检查实际错误。"}}]},
    {"id":"example.repair","requires":["example.evidence"],"variants":[{"locale":"zh-CN","source":{"type":"file","path":".chrono-harness/instructions/repair.zh-CN.md"}}]},
    {"id":"example.bundle","requires":["example.repair"],"variants":[{"locale":"zh-CN","source":{"type":"inline","text":""}}]}
  ]
}
```

locale id 必须明确、唯一，可以追加语言；程序不猜语言标签，也不自动回退。atom id 是稳定引用身份，只含 ASCII 字母、数字和 `._-`，不能为空。requires 有序，variants 中每种 locale 唯一；source 严格选择带类型标签的 inline{text} 或 file{path}。拒绝 JSON 重复字段和未知字段。根引用及依赖引用必须存在；全部已登记依赖都检查循环，并报告依赖链。

所有 file variant 都是完整的明确输入，即使当前输出没选中，也检查存在、UTF-8、普通文件类型和路径。只有选中的依赖范围需要对应 locale variant；未选中 atom 可以缺翻译。

没有 layout 时，按 roots／requires 的声明顺序做 DFS，先依赖、后使用方，每个输出中的共享 atom 只渲染一次，不按文字猜依赖边。`core.general` 是 zh-CN／en 正文均为空的普通聚合，明确选择原有 17 个稳定入口和新增的便携要求。当前有 100 条内容规则、16 个普通聚合；原主题 ID 只组合自己的职责，不扩成全部章节。`core.repair-producer` 是内容规则，仅依赖 `core.evidence` 的 3 条和 `core.reuse` 的 3 条，skill 共呈现 7 条，不引入容量、缓存、CI 或研究路线。

空 inline 可仅用于组合；非空段按顺序用两个 LF 分隔，段内字节不裁剪、不正规化。file 的 UTF-8 字节和 JSON 解码后的 inline UTF-8 精确保留，支持 BOM、CRLF、空内容和无末尾换行。标题、框架、输入引用、分隔 LF 和所有权标记属于输出格式。选中内容与框架不能含保留前缀 `<!-- chrono-instructions`；context 不嵌入正文，因此不受此限制。

每种 locale 提供 root_frame 和 projection_notice，框架与正文都使用所选语言的登记数据。根内列出的 manifest／catalog／host_context 路径来自实际绑定。正文直接放在根受管块中，无需重复读取源。翻译由作者提供，程序不证明语义等价或内容完整。

## 可选文档布局

catalog 可选 `layouts`，省略时为空数组；output 可选 `layout`，引用具名布局。catalog schema 1、manifest schema 2 和 render v3 的原字段含义不变。不选 layout 时仍按 DFS 顺序逐字节输出；仅登记布局不会自动采用。旧二进制会拒绝未知字段，不会静默忽略；升级生成器后可读取，但已有宿主仍须明确选择。原始方法 init 不带布局和选择器。

布局包含有序 `sections`；每节必填相对 `depth`、`titles` 和明确的 `atoms`。下面片段可加入前面的合成 catalog，并在输出选择 `"layout":"example"`。

```json
"layouts": [{"id":"example","sections":[
  {"depth":1,"titles":[{"locale":"zh-CN","text":"工作方法"}],"atoms":[]},
  {"depth":2,"titles":[{"locale":"zh-CN","text":"证据与修复"}],"atoms":["example.evidence","example.repair"]}
]}]
```

布局 ID 唯一，引用的 atom 必须已登记，同一布局不能重复放置。depth 为 1–6 的整数，从 1 起；相邻章节最多加深一级，可以任意回浅。output 有 title 时，渲染深度加 1，不能超过 Markdown 的 6 级。每节标题按 locale 唯一，文本非空、单行，没有控制字符或保留标记；拒绝未知字段和重复 JSON 字段。

所有布局都检查结构与引用。输出选中布局后，每节必须有对应语言标题，不自动回退。程序先按 roots／requires 求出完整依赖范围，读取并验证所选 variant，再检查布局是否把全部非空正文各放置一次。是否为空按 UTF-8 字节长度判断，不裁剪；空聚合不放入布局。缺依赖正文、重复、未知或范围外放置都会报错，不能借布局扩大选择。

每节先输出标题，再按明确顺序输出正文，以两个 LF 分隔，正文原字节不变。空 atoms 可用于上层标题。全部输出预检和渲染完成后，才交给原发布器；此前失败不写源、输出、别名或目录。

布局只组织阅读，不决定权威优先级，也不承诺执行顺序；requires 仍只表示内容依赖。产品 `workflow` 明确放置默认规则：四阶段 depth=1，开发下的验证和交付 depth=2。中文保留根标题，因此阶段实际为二级，其下为三级；初始英文没有标题时，分别为一级和二级，语言选择行为不变。

`general` 明确列出基本原则、工作方法、执行约定三部分和 12 个主题，覆盖全部 100 条内容规则。本仓中文完整指南选择 `core.general/general`。主题成员来自明确列表，不由扫描、ID 前缀或正文推断，也不是 header atom。宿主采用数据独立维护。聚焦 skill 不选择通用布局，仍平铺原有 7 条依赖规则。

## 所有权、路径与退休

根只替换 `<!-- chrono-instructions:begin -->` 到 `<!-- chrono-instructions:end -->` 的一个受管块。标记必须唯一、有序、各自独占一行；其他保留前缀会被拒绝。块外原字节全部保留，即使不是 UTF-8。没有受管块时追加，必要时补 LF。

如果只有 AGENTS 普通文件，就以它作为原文来源，继承原文和普通权限。两个根都是普通文件时，分别渲染；只有整份预期字节相同，才把 AGENTS 转换成链接。原文不同时明确报告冲突，由已获授权的 AI 保全并整合后重试。已有有效别名的 inode 不变。

额外 Markdown／skill 整文件都是生成副本。文件不存在时可创建；已有文件必须具备 producer、output ID 和 format 都相符的完整 begin／end 所有权标记。没有标记、身份错误或标记畸形，都会在写入前失败，不会静默覆盖宿主文件。

skill 的 `---` 必须从文件第一个字节开始；name／description 用 JSON 字符串形式的合法 YAML 双引号标量表示，所有权标记位于 front matter 关闭之后。name 为 1–63 位小写 ASCII 字母、数字或连字符，首尾不能是连字符，也不能连续使用；父目录须同名，文件名恰为 SKILL.md。description 非空、单行、无控制字符，最多 1024 个字符且不含尖括号，供当前外部 validator 使用。标题和元数据不自动决定使用范围，作者负责让内容聚焦且实用。

只有当前 manifest 的输出受管理。删除条目或改名都会原样保留旧文件；旧标记只记录来源，不代表当前仍受管理。旧 skill 可能继续被发现，已获授权的 AI 必须明确退役旧文件。没有 previous-manifest、history ledger、glob、扫描卸载或自动删除。

全部登记路径必须是规范的相对 `/` 路径，不得含空段、`.`、`..`、反斜杠、控制字符或绝对路径。catalog／context／file 源须位于宿主 `.chrono-harness/` 下。控制文件、源、输出与别名不能相互覆盖，也不能让文件成为另一文件路径的祖先。可以重复引用同一 file variant 输入，不能重复输出路径。

已有祖先必须是真实目录。保留文件、控制文件和输出拒绝链接、硬链接及特殊类型，AGENTS 固定别名是唯一例外。已有普通文件还核对设备和 inode，防止别名。当前 macOS 会保守拒绝各路径段经过 Unicode 大小写折叠和文件系统表示转换后相同的不同拼写，如 `straße`／`STRASSE`，即使卷区分大小写也拒绝。

比较键先由现有 CoreFoundation 的 `CFStringFold` 使用 `kCFCompareCaseInsensitive` 和规范系统 locale 生成，再经文件系统表示 API 转换。此过程不改登记原字节，不增加 Cargo 依赖，也不写探测文件，用来避免新文件或目录产生别名。程序先验证完整计划、渲染全部输出，再写任何内容；目录去重后，按父先子后创建。

## 当前输入合同

init／generate 只接受 schema 2、producer=`chrono-instructions`、render=`atomic-rules/relative-alias/v3` 的 manifest。不支持格式、未知字段和重复字段都在写入前报错，源、manifest、根和链接保持原样；不自动转换旧 manifest，也不补用默认数据。

新宿主的原始方法 init 把明确选择的方法登记为 `legacy.method` file atom。未声明语言时为 und，明确给出 --locale 时则采用作者声明。und 仅表示未指定语言，框架不冒称中文或英文。方法和上下文保留精确 UTF-8 字节；刷新时保留已有文件普通权限和根块外原文。没有改变绑定时，保留当前 manifest 排版。

本仓将产品双语 catalog 采用到独立宿主 catalog，从源构建二进制并生成 manifest 选择的输出。已有宿主保留自己的 catalog 或不解析内容的方法文件；采用新默认时，比较所选产品版本、保留定制并编辑宿主数据，不自动覆盖。[来源与采用说明](methodology-extraction.md)及[许可资料](licenses/methodology-attribution.md)不进入生成依赖图；历史迁移过程归 Git／PR。

## IO、边界与验证

生成器复用同一发布器。在同目录暂存普通新文件和原文件备份，执行 flush／sync；新文件使用 tempfile 默认私有权限，已有普通权限保留。写入顺序为源先于输出、CLAUDE 先于别名、新建 manifest 最后。用户已经编辑的 manifest，不是程序保存的历史版本。全部内容相等时，不创建临时文件，也不改 inode／mtime。

普通发布失败时，按相反顺序恢复字节、普通类型和权限，删除本次新建文件或链接，再按子先父后清理空目录。错误列出 published、unrestored、recovery、cleanup；published 不表示文件仍处于改变状态。回滚失败时，保留原文件备份并报告路径；没有原文件时，报告未移除目标。恢复实际异常状态后再重试。清理失败也退出非零；若发布已完成但清理失败，会明确说明。

支持范围限于 Unix 和当前 macOS 普通 IO，不保证崩溃原子性、并发写入、跨平台行为，或恢复 ACL、扩展属性、原 inode、时间戳及所有者。没有针对恶意 AI 的关卡，也不假定全局安装或 provider。

本仓验证走[登记的统一入口](ci.md#this-repository)，由明确依赖选择格式检查、生产构建、专属行为测试及适用的同源 Verus 证明。具体参数归 projects／FILEMAP 登记，本文引用该入口。

产品默认和本宿主已采用“测试中的具体外部可执行文件调用一律使用 mock”：在命令执行边界核对调用和结果，不以集成或真实消费者测试为名例外执行。真实构建、生成、检查和交付仍使用正式生产入口，mock 结果不证明外部程序的实际行为。规则采用不证明历史测试已全部迁移；当前迁移范围见 [CI 测试说明](ci.md)。本次同时修改产品默认规则和宿主采用数据，按登记生成副本；受影响验证及总成本分别核实，尚未测量的成本保持未知。

产品默认根包含同语言测试规则，以及 sh／bash 可以用 Python 测试的例外。[实际消费者边界](methodology-extraction.md#实际消费者边界)说明本仓根、上下文和 skill 的内容范围与测量口径。编辑内容须复核完整载荷，不截尾，也不靠全局配置扩限。单独选择 `core.behavior` 也包含同语言要求；生成指南不代表语言配对判官已执行。

专属行为测试覆盖依赖图顺序、去重和错误，明确语言选择，精确字节，布局重排、双语复用、深度和完整覆盖，写入前拒绝，所有权和路径预检，不支持格式时保护原件，原文、别名和无改动生成，以及普通与注入失败恢复。test-support 没有生产开关。测试真实复制二进制后验证默认 init，再明确添加多输出配方；内容测量和外部 skill 格式检查的适用范围见[迁移说明](methodology-extraction.md#实际消费者边界)。它们不增加生产依赖，也不证明语义。

FILEMAP 的 proposed projection 元数据使用明确的 `sources` 数组，取代单一 `source`，列出 catalog、manifest 和被引用的 file 输入；producer／scope 不变。根框架中的路径引用也由 manifest 决定。产品资产属于 build-input／test-execution；宿主数据和现有生成副本属于 runtime-input。五份完整治理登记仍为 proposed，成本仍有 unmeasured 项。指令专用 schema 校验不能冒称治理判官已执行。独立 CI 范围检查的实际 DELTA、协议和生成范围见 [CI 说明](ci.md)。
