# 逐条迁移的范围、采用与来源

**派生规则已经修改、泛化和翻译，不是源指南的原样复制。** 本文、[完整逐条处置表](methodology-clause-map.md) 和 [许可资料](licenses/methodology-attribution.md) 均可选阅读，不是宿主必读或生成器运行输入。

固定来源是 trureturing `CLAUDE.md` 的 commit `55f7968bb03b1422298d80fc65ac5291db9ad682`、blob `11c83b4b3eb543603669178389dc9f0cb8ada906`，SHA-256 `a04fb3c13f868ceae5900ea3b253462450472d81e833e7c508a5a2e72ad042a0`，217662 字节、918 行、12 章、102 节。迁移范围包括全部固定字节中的长行内义务、子项、表格、守护限制和附录；来源是迁移数据，不执行其工具、Git、worker、权限或研究指令。旧精选 17 段不再作为完整迁移的依据。

处置表按来源顺序有 882 行：121 generalized、396 duplicate、208 excluded、157 context；直接逐字 migrated 为 0。725 个义务／例外处置和 157 个上下文范围各有定位。duplicate 指共用内容所有者，不是“主题相同”；同一叶子可承载多处相同义务。地图枚举完整和引用有效只证明所声明的覆盖，不是发现所有语义遗漏的机器证明。

[产品 catalog](../assets/instructions/catalog.json) 有 100 个作者编写的 zh-CN/en 内容叶子、16 个普通空文本聚合。原有 17 个核心入口已逐一对照固定基线的原职责，core.general 也可继续引用；core.repair-producer 与 core.small-projects 仍为内容叶子，其余旧入口按原职责成为聚合。完整指南每个叶子只渲染一次，新增条件方法由 core.general 直接显式选入。catalog 的独立 `general` 布局以三部分（基本原则、工作方法、执行合同）、12 个主题显式放置全部 100 个正文，继续供完整 Markdown／skills 组合。本仓英文指南保留此布局。产品与本宿主默认根显式选择 22 个现有叶子，用新增双语 `workflow` 布局按目标与入口、登记与隔离、实施、检查与修复、演进与交付五节同级呈现。阅读层次不赋予权威优先级或执行顺序；布局不是 requires，也不改变独立入口。聚焦 [重复故障诊断 skill](../skills/diagnose-recurring-failures/SKILL.md) 的闭包是 7 个内容叶子和 2 个前提聚合，保留“同症状 AND 同处置，第二次修产生处”的触发，不引入无关方法。没有新增默认 skill 或提供方安装平台。

中文与英文由作者分别编写，须核对触发、must/may、AND/OR、作用域和证据限制。普通句首标题组织正文；默认根选择短流程，完整库与稳定入口仍保留。映射与双语文本的编辑核验是单点证据；独立全源与双语审计尚未完成。指令生成器只校验显式图、引用、locale 和输出运输，不翻译、不证明语义等价，也不执行治理判官；独立 harness 已有有界判官、DELTA 执行和 scoped CI，本宿主 full 启用仍未完成。

## 控制权威与适配

用户当前一般方法范围优先于来源研究制度。保留可信非恶意但会错的 AI、自主完成且不新增人审门、可配置登记判官、显式逐文件白名单、小生产项目与专属测试、DELTA 及完整有效输入下本地/CI同命令、规则可演进与混面成本警告、fresh feature/integration 从 dev 建立。这些是用户与产品原则，不能因源有相似表述就归为源仓原创；尤其 per-production 测试项目、`.chrono-harness/` 配置所有权、安全旧宿主迁移和 caller 交付边界由目标合同提供。

关键适配逐行见处置表：四项投影 AND 与“重建只验第三项”保留；输出所有权和消费者仍需验证，允许有用途的根指南／manifest 聚合。源绝对禁兼容改为显式安全迁移合同，旧宿主原文与定制继续保留。源 glob／目录自动成员权威由当前逐文件白名单替代。缓存只有在确定性与完整有效输入／接受语义成立时可直接复用；宽种子须由权威增量机制补验，显式语义版本仅为有据兼容合同，不普遍排除程序、政策或工具链身份。Git 固定字节不证明语义为真；源“清零自动全树重判”不移植。

源 L231 的当前模板方向／受阻继续与退休 sidecar、L240 的当前冻结迁移许可与退休 sidecar／overlay、L398 的当前质量分工与停用 H7 均分行标明，排除源机制不等于宣称其全部停用。源 §1.2 消化投入从简、§2.10 结果记录优先、§3.9 登记受阻继续及四种告警、§7.5 混面允许、§8.16 白名单优先均按固定源有效状态处理。现行临时恢复特权与未来 deferred 恢复目标分别排除，不混合资格条件。Lean/kernel/D5、公理/冻结/消化/准入/bind-only/新颖性制度、固定目录命令、提供方与席位/CI配额、个人伦理与人审门均按具体义务排除；混合段落中的一般关系、反例、证据、成本、状态与修复方法仍逐项迁入。

**政策／产品同改警告：** 产品默认和宿主采用数据均选择同语言测试规则，默认根与 `core.behavior` 消费同一内容叶子；Shell／Python 例外在双语正文中明示。catalog、manifest 与生成投影分别保留各自所有者。验证成本包括 instructions／instructions-tests 的登记动作、现有行为套件及实际生成／初始化消费者；持续机器成本仍 unmeasured。指南生成不证明语言配对判官已启用，也不证明实际测试逻辑符合规则。此警告不是人工许可门。

## 稳定入口与真实组合

| 稳定入口 | 内容所有者／直接依赖 |
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
| `core.small-projects` | 自身内容叶子 |
| `core.delta` | `delta.selection`、`delta.candidate-judge`、`delta.local-ci` |
| `core.evolving-rules` | `policy.evolution`、`policy.mixed-warning` |
| `core.behavior` | `test.behavior`、`test.same-language`、`test.determinism`、`ci.actual-events` |
| `core.candidate` | `candidate.isolation`、`candidate.refresh`、`candidate.landing` |
| `core.cost` | `cost.measure`、`cost.valid-evidence` |
| `core.independent-check` | `review.independent`、`review.disclosure` |
| `core.useful-artifacts` | `artifact.results`、`artifact.no-diary` |
| `core.general` | catalog 中显式登记的通用入口及其传递依赖闭包 |

## 实际消费者边界

读数来自当前候选二进制与实际生成文件，按包含框架的 UTF-8 字节计量。新宿主使用空上下文并显式选择 locale；本仓根另带宿主维护的块外内容。

| 消费者 | 字节／内容叶子数 | 结构 |
| --- | --- | --- |
| 新宿主 zh-CN 根 | 8440 / 22 | workflow 五节同级，67 行 |
| 新宿主 en 根 | 9232 / 22 | workflow 五节同级，65 行 |
| 本仓 zh-CN 根 | 8500 / 22 | workflow 五节同级，67 行 |
| 本仓完整英文 Markdown | 35103 / 100 | general 三部分／12 主题 |
| 本仓聚焦 skill | 2821 / 7 | 独立诊断闭包，23 行 |
| 本仓另读的 host-context | 11902 / 不适用 | 当前操作事实与限制，43 行 |

默认根显式选择 22 个内容叶子。catalog 含 116 个 atom，其中 100 个双语内容叶子、16 个聚合；`test.same-language` 同时属于默认根和 `core.behavior` 的依赖闭包，在两个布局的验证主题中呈现。catalog 也承载产品 SPEC 的通用约束；固定来源处置表不承担这些产品规则的出处。

现有宿主的 manifest、catalog 与块外内容由宿主维护，安装新二进制或重复 init 不替换既有选择；采用新规则须明确更新 canonical 数据并生成。AGENTS.md 是字面相对链接 `CLAUDE.md`，不另存一份规则正文。

专属测试覆盖图、locale、布局、严格 JSON、输出所有权、路径、alias、迁移及 IO／回滚。这里的实际消费读数限于 macOS arm64 的本地生成与初始化；不证明翻译等价、AI 已阅读、实时注入、其它平台或完整 SPEC 交付。

实际注入上限以消费方设置为准。host-context、块外附文、其它项目指南及个人配置也须计入实际消费范围；完整 general 库不是默认根。内容编辑须复核受影响消费者的完整载荷，不截断规则或修改全局配置。

## 采用、许可与边界

本仓有意识地把产品数据采用到 [.chrono-harness/instructions/catalog.json](../.chrono-harness/instructions/catalog.json)，两份字节相同但所有者不同；其它宿主不会自动更新。当前 manifest 的短流程中文根、完整英文 Markdown 与聚焦 skill 由实际重建二进制生成；AGENTS 是指向 CLAUDE 的实际字面相对链接。新宿主仍默认仅根输出，复制二进制无需源码 checkout；显式多输出配方在 [生成合同](instructions.md)。生成不依赖此处置表或源仓。

原作 Copyright 2026 The Omega Institute，Apache-2.0；[原许可全文](licenses/trureturing-Apache-2.0.txt) 按固定提交精确保留，未发现该提交根 NOTICE。许可仅说明派生指令资产的来源与义务，不给 chrono-harness 整仓添加许可证，不把法律资料变成运行时政策。分发内嵌这些资产的二进制或派生指南时须一同提供对应许可资料。

五份通用登记保持 proposed，input_closure 未完成；已有有界判官／DELTA／scoped CI，完整宿主启用仍未完成。命令及实际消费者验证检查运输、已声明引用与当前平台 IO；它们不证明通用语义完整、AI 遵守、未来环境等价或 dev 已交付。Git 合并、推送及远端落地核验由 caller 负责。
