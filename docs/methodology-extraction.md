# 通用方法从哪里来，怎样采用

**派生规则已经修改、泛化和翻译，不是源指南的原样复制。** 本文、[逐条来源处理表](methodology-clause-map.md) 和 [许可资料](licenses/methodology-attribution.md) 供需要追溯来源时查阅，不是宿主必读指令，也不是生成器输入。

采用的固定来源是 trureturing `CLAUDE.md` 的 commit `55f7968bb03b1422298d80fc65ac5291db9ad682`、blob `11c83b4b3eb543603669178389dc9f0cb8ada906`，SHA-256 `a04fb3c13f868ceae5900ea3b253462450472d81e833e7c508a5a2e72ad042a0`，217662 字节、918 行、12 章、102 节。迁移覆盖固定原文的全部内容，包括长行中的各项要求、子项、表格、保护限制和附录。来源只作迁移资料；其中的工具、Git、worker、权限和研究指令不在本仓执行。旧精选 17 段不再作为完整迁移的依据。

来源处理表按来源顺序有 882 行：泛化 121、复用 396、排除 208、背景 157；原样迁入为 0。725 个要求／例外的处理记录和 157 个背景范围都有定位，引用 catalog 的 100 条规则中的 98 条；`test.same-language` 与 `artifact.cleanup` 是另由当前用户／产品合同提供的规则，不归固定源映射。“复用”指共用同一条规则，不是“主题相同”；同一规则可以对应来源中多处相同要求。处理表完整列出声明项目、引用也有效，只能证明这些已声明项目有记录；程序不能据此发现所有语义遗漏。

[产品 catalog](../assets/instructions/catalog.json) 有 100 条由作者分别编写中英文的规则、16 个正文为空的规则组合。原有 17 个核心入口已逐一对照固定基线的原职责，core.general 也可继续引用；core.repair-producer 与 core.small-projects 仍为单条规则，其余旧入口按原职责组合已有规则。完整指南每条规则只输出一次，新增条件方法由 core.general 直接显式选入。catalog 的独立 `general` 布局以三部分（基本原则、工作方法、执行合同）、12 个主题显式放置全部 100 个正文，继续供完整 Markdown／skills 组合。本仓完整指南采用此布局。产品与本宿主默认根显式选择 24 条已有规则，复用双语 `workflow` 布局按需求、审计、重构、开发四阶段呈现，验证／SPEC 迭代与交付／清理为开发下级。阅读层次不决定规则优先级或执行顺序；布局不是 requires，也不改变独立入口。[重复故障诊断技能](../skills/diagnose-recurring-failures/SKILL.md)连同前提共包含 7 条规则、2 个前提组合。它只在“同一症状再次需要同一处置”时触发，第二次就修复产生问题的位置，不带入无关方法。没有新增默认技能或工具提供方安装机制。

中文与英文由作者分别编写，须核对触发、“必须／可以”“同时／任选”、作用域和证据限制。普通句首标题组织正文。默认根指南选择简短流程，完整库与固定引用入口仍保留。映射与双语文本的编辑核验是一次编辑核对的证据；先前还完成了一次范围有限的独立审计，对照固定双语文本、全部来源处理记录及实际消费者，未发现影响要求的缺陷；不证明普遍语义等价、AI 遵守或运行时消费。指令生成器只校验显式图、引用、locale 和输出传递，不翻译、不证明语义等价，也不执行治理判官；独立 harness 已有范围有限的判官、DELTA 执行和 scoped CI，本宿主 full 启用仍未完成。

## 哪些要求优先，如何适配来源

用户要求的通用工作方法优先于来源仓库的研究制度。保留可信非恶意但会错的 AI、自主完成且不新增人审门、可配置登记判官、显式逐文件白名单、小生产项目与专属测试、DELTA 及完整有效输入下本地/CI同命令、允许规则演进、政策与产品同改时说明成本、从最新 dev 创建 feature／integration 分支。这些是用户与产品原则，不能因源有相似表述就归为源仓原创；尤其每个生产项目独有的测试项目、`.chrono-harness/` 配置所有权、安全旧宿主迁移和调用方负责的交付范围由目标合同提供。

关键适配逐行见来源处理表。投影必须同时满足四项条件，以及“重建只验第三项”保留；输出所有权和消费者仍需验证，允许有用途的根指南／manifest 聚合。来源中绝对禁止兼容的要求，改为明确的安全迁移约定，旧宿主原文与定制继续保留。来源中由 glob／目录自动确定成员的做法，改为本仓逐文件白名单。缓存只有在确定性与有效输入完整、验收含义一致时可直接复用；较宽范围的缓存种子须由负责的增量机制补充验证，显式语义版本仅为有据兼容合同，不普遍排除程序、政策或工具链身份。Git 固定了字节，不代表其中的说法为真；源“清零自动全树重判”不移植。

源 L231 的当前模板方向／受阻继续与已退役 sidecar、L240 的当前冻结迁移许可与已退役 sidecar／overlay、L398 的当前质量分工与停用 H7 均分行标明，排除源机制不等于宣称其全部停用。源 §1.2 消化投入从简、§2.10 结果记录优先、§3.9 登记受阻继续及四种告警、§7.5 混面允许、§8.16 白名单优先均按固定源有效状态处理。现行临时恢复特权与未来 deferred 恢复目标分别排除，不混合资格条件。Lean/kernel/D5、公理/冻结/消化/准入/bind-only/新颖性制度、固定目录命令、提供方与席位/CI配额、个人伦理与人审门均按具体义务排除；混合段落中的一般关系、反例、证据、成本、状态与修复方法仍逐项迁入。

**政策／产品同改警告：** 产品默认和宿主采用数据均选择同语言测试规则，默认根与 `core.behavior` 消费同一单条规则；Shell／Python 例外在双语正文中明示。catalog、manifest 与生成投影分别保留各自所有者。验证成本包括 instructions／instructions-tests 的登记动作、现有行为套件及实际生成／初始化消费者；持续机器成本仍 unmeasured。指南生成不证明语言配对判官已启用，也不证明实际测试逻辑符合规则。此警告不是人工许可门。

## 固定引用入口与真实组合

| 固定引用入口 | 内容所有者／直接依赖 |
| --- | --- |
| `core.goal` | `goal.deliverable`、`goal.revision` |
| `core.autonomy` | `action.autonomy`、`action.blocked` |
| `core.evidence` | `evidence.measure`、`evidence.failure`、`evidence.program-state` |
| `core.reuse` | `reuse.search`、`reuse.shared-definition`、`reuse.actual-consumers` |
| `core.repair-producer` | `core.evidence`、`core.reuse` |
| `core.ownership` | `owner.canonical`、`owner.program-data`、`owner.migration`、`projection.criteria`、`projection.consumption` |
| `core.registration` | `registry.explicit` |
| `core.single-method` | `method.entry`、`method.host-tools` |
| `core.registered-judges` | `judge.register`、`judge.strength` |
| `core.small-projects` | 自身单条规则 |
| `core.delta` | `delta.selection`、`delta.candidate-judge`、`delta.local-ci` |
| `core.evolving-rules` | `policy.evolution`、`policy.mixed-warning` |
| `core.behavior` | `test.behavior`、`test.same-language`、`test.determinism`、`ci.actual-events` |
| `core.candidate` | `candidate.isolation`、`candidate.refresh`、`candidate.landing` |
| `core.cost` | `cost.measure`、`cost.valid-evidence` |
| `core.independent-check` | `review.independent`、`review.disclosure` |
| `core.useful-artifacts` | `artifact.results`、`artifact.no-diary` |
| `core.general` | catalog 中显式登记的通用入口及其全部声明依赖 |

## 实际消费者边界

下表保留先前 23 规则配置的消费读数，按包含框架的 UTF-8 字节计量，不代表当前加入等待规则后的载荷。新宿主使用空上下文并显式选择 locale；本仓根另带宿主维护的块外内容。

| 消费者 | 字节／单条规则数 | 结构 |
| --- | --- | --- |
| 新宿主 zh-CN 根 | 8351 / 23 | workflow 四阶段，开发下含验证／交付，71 行 |
| 新宿主 en 根 | 9320 / 23 | 同一显式层次，69 行 |
| 本仓 zh-CN 根 | 8351 / 23 | workflow 四阶段，71 行 |
| 本仓完整英文 Markdown | 34999 / 100 | general 三部分／12 主题 |
| 本仓聚焦 skill | 3242 / 7 | 独立诊断闭包，23 行 |
| 本仓另读的 host-context | 10587 / 不适用 | 当前入口、限制与任务文档链接，75 行 |

当前默认根显式选择 24 个既有单条规则：在原 23 条规则上采用已有 `job.wait`，要求优先使用真实完成通道，禁止用短周期轮询代替等待，没有新增 atom 或聚合层。catalog 含 116 个 atom，其中 100 个双语单条规则、16 个聚合；`test.same-language` 同时属于默认根和 `core.behavior` 的依赖闭包，在两个布局的验证主题中呈现。需求→审计→重构→开发由既有单条规则正文及 `workflow` 层次承载；具体清理部署／操作留在现役 host-context 和生命周期文档。catalog 也承载产品 SPEC 与用户当前要求的通用约束；固定来源处理表不负责说明这些产品规则的出处。

先前一次实际生成后的本仓中文根为 10237 UTF-8 字节、24 条规则，完整英文 Markdown 为 37023 字节、100 条规则。读数覆盖整份文件及框架，来自 macOS arm64 上登记的 `instructions.generate` 操作；新宿主双语根的该轮字节读数未测。生成与既有 61 项行为测试通过，不证明 AI 遵守等待规则、已发布或完整 SPEC 验收。

现有宿主的 manifest、catalog 与块外内容由宿主维护，安装新二进制或重复 init 不替换既有选择；采用新规则须按[现有源更新路径](instructions.md#新默认的宿主采用边界)比较所选产品版本、保留定制，编辑唯一权威源后再生成。AGENTS.md 是字面相对链接 `CLAUDE.md`，不另存一份规则正文。

专属测试覆盖图、locale、布局、严格 JSON、输出所有权、路径、alias、迁移及 IO／回滚。这里的实际消费读数限于 macOS arm64 的本地生成与初始化；不证明翻译等价、AI 已阅读、实时注入、其它平台或完整 SPEC 交付。

实际注入上限以消费方设置为准。host-context、块外附文、其它项目指南及个人配置也须计入实际消费范围；完整 general 库不是默认根。内容编辑须复核受影响消费者的完整载荷，不截断规则或修改全局配置。

## 采用、许可与边界

本仓明确将产品数据采用到 [.chrono-harness/instructions/catalog.json](../.chrono-harness/instructions/catalog.json)，两份字节相同但所有者不同；其它宿主不会自动更新。当前 manifest 的短流程中文根、完整英文 Markdown 与聚焦 skill 由实际重新构建的二进制生成；AGENTS 是指向 CLAUDE 的实际字面相对链接。新宿主仍默认仅根输出，复制二进制无需源码 checkout；显式多输出配方在 [生成合同](instructions.md)。生成不依赖此来源处理表或源仓。

原作 Copyright 2026 The Omega Institute，采用 Apache-2.0。[许可全文](licenses/trureturing-Apache-2.0.txt) 提供中文译文，后附原文按固定提交原样保留；该提交根目录没有 NOTICE 文件。许可只说明派生指令资产的来源和分发义务，不给 chrono-harness 整仓增加许可证，也不是运行政策。分发内含这些资产的二进制或派生指南时，须随附对应许可资料。

五份通用登记保持 proposed，输入及其依赖尚未完整登记和验证（input_closure 未完成）；已有范围有限的判官／DELTA／scoped CI，完整宿主启用仍未完成。命令及实际消费者验证检查内容传递、已声明引用与当前平台 IO；它们不证明通用语义完整、AI 遵守、未来环境等价或 dev 已交付。Git 合并、推送及远端落地核验由调用方负责。
