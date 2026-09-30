# 逐条迁移的范围、采用与来源

**派生规则已经修改、泛化和翻译，不是源指南的原样复制。** 本文、[完整逐条处置表](methodology-clause-map.md) 和 [许可资料](licenses/methodology-attribution.md) 均可选阅读，不是宿主必读或生成器运行输入。

固定来源是 trureturing `CLAUDE.md` 的 commit `55f7968bb03b1422298d80fc65ac5291db9ad682`、blob `11c83b4b3eb543603669178389dc9f0cb8ada906`，SHA-256 `a04fb3c13f868ceae5900ea3b253462450472d81e833e7c508a5a2e72ad042a0`，217662 字节、918 行、12 章、102 节。迁移范围包括全部固定字节中的长行内义务、子项、表格、守护限制和附录；来源是迁移数据，不执行其工具、Git、worker、权限或研究指令。旧精选 17 段不再作为完整迁移的依据。

处置表按来源顺序有 882 行：121 generalized、396 duplicate、208 excluded、157 context；直接逐字 migrated 为 0。725 个义务／例外处置和 157 个上下文范围各有定位。duplicate 指共用内容所有者，不是“主题相同”；同一叶子可承载多处相同义务。地图枚举完整和引用有效只证明所声明的覆盖，不是发现所有语义遗漏的机器证明。

[产品 catalog](../assets/instructions/catalog.json) 有 98 个作者编写的 zh-CN/en 内容叶子、16 个普通空文本聚合。原有 17 个核心入口已逐一对照固定基线的原职责，core.general 也可继续引用；core.repair-producer 与 core.small-projects 仍为内容叶子，其余旧入口按原职责成为聚合。完整指南每个叶子只渲染一次，新增条件方法由 core.general 直接显式选入。catalog 的独立 `general` 布局以三部分（基本原则、工作方法、执行合同）、12 个主题显式放置全部 98 个正文，继续供完整 Markdown／skills 组合。本仓英文指南保留此布局。产品与本宿主默认根显式选择 20 个现有叶子，用新增双语 `workflow` 布局按目标与入口、登记与隔离、实施、检查与修复、演进与交付五节同级呈现。阅读层次不赋予权威优先级或执行顺序；布局不是 requires，也不改变独立入口。聚焦 [重复故障诊断 skill](../skills/diagnose-recurring-failures/SKILL.md) 的闭包是 7 个内容叶子和 2 个前提聚合，保留“同症状 AND 同处置，第二次修产生处”的触发，不引入无关方法。没有新增默认 skill 或提供方安装平台。

中文与英文并排作者编写；98 对内容按保留的来源义务／例外合同核对触发、must/may、AND/OR、作用域和证据限制。普通句首标题取代粗体；短流程默认显式缩小根选择，完整库与稳定入口仍保留；除 method.entry 增强当前单一路径及登记演进规则外，其余 97 对正文不变。映射与双语文本的编辑核验是单点证据；独立全源与双语审计由 caller 的后续阶段承担，尚未取得独立认可。指令生成器只校验显式图、引用、locale 和输出运输，不翻译、不证明语义等价，也不执行治理判官；独立 harness 已有有界判官、DELTA 执行和 scoped CI，本宿主 full 启用仍未完成。

## 控制权威与适配

用户当前一般方法范围优先于来源研究制度。保留可信非恶意但会错的 AI、自主完成且不新增人审门、可配置登记判官、显式逐文件白名单、小生产项目与专属测试、DELTA 及完整有效输入下本地/CI同命令、规则可演进与混面成本警告、fresh feature/integration 从 dev 建立。这些是用户与产品原则，不能因源有相似表述就归为源仓原创；尤其 per-production 测试项目、`.chrono-harness/` 配置所有权、安全旧宿主迁移和 caller 交付边界由目标合同提供。

关键适配逐行见处置表：四项投影 AND 与“重建只验第三项”保留；输出所有权和消费者仍需验证，允许有用途的根指南／manifest 聚合。源绝对禁兼容改为显式安全迁移合同，旧宿主原文与定制继续保留。源 glob／目录自动成员权威由当前逐文件白名单替代。缓存只有在确定性与完整有效输入／接受语义成立时可直接复用；宽种子须由权威增量机制补验，显式语义版本仅为有据兼容合同，不普遍排除程序、政策或工具链身份。Git 固定字节不证明语义为真；源“清零自动全树重判”不移植。

源 L231 的当前模板方向／受阻继续与退休 sidecar、L240 的当前冻结迁移许可与退休 sidecar／overlay、L398 的当前质量分工与停用 H7 均分行标明，排除源机制不等于宣称其全部停用。源 §1.2 消化投入从简、§2.10 结果记录优先、§3.9 登记受阻继续及四种告警、§7.5 混面允许、§8.16 白名单优先均按固定源有效状态处理。现行临时恢复特权与未来 deferred 恢复目标分别排除，不混合资格条件。Lean/kernel/D5、公理/冻结/消化/准入/bind-only/新颖性制度、固定目录命令、提供方与席位/CI配额、个人伦理与人审门均按具体义务排除；混合段落中的一般关系、反例、证据、成本、状态与修复方法仍逐项迁入。

**政策／产品同改警告：** SPEC 澄清每项受治理操作的一条当前登记路径、AI 负责的 declared-complete 工程声明、普通 full check 与更强 parity 前提的边界；产品默认及宿主采用数据选择 20 叶子 workflow，method.entry 的两种语言相应增强。114 个 atom 身份、全部 requires、原 general 布局、其余正文及聚焦 skill 不变。没有修改 Rust、schema 或测试行为，没有新增审批、安全层或普遍隐藏输入证明。验证成本限 instructions／instructions-tests 的七项已登记 Cargo 动作、现有 61 项行为测试一次及实际生成／初始化消费者；持续机器成本仍 unmeasured。此警告不是人工许可门。

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
| `core.behavior` | `test.behavior`、`test.determinism`、`ci.actual-events` |
| `core.candidate` | `candidate.isolation`、`candidate.refresh`、`candidate.landing` |
| `core.cost` | `cost.measure`、`cost.valid-evidence` |
| `core.independent-check` | `review.independent`、`review.disclosure` |
| `core.useful-artifacts` | `artifact.results`、`artifact.no-diary` |
| `core.general` | 原有 17 个稳定入口，以及 catalog 中逐项列出的 62 个新增便携叶子 |

## 实际消费者边界

读数来自当前候选生成器及真实生成文件，包含框架的 UTF-8 字节。修正前采用固定基线 `47b37400ecdd35b6ddc967c86a5b28d39bf89538` 的 catalog／manifest 数据，由当前生成器重建；不执行历史程序。新宿主使用复制二进制，在本宿主已登记且被忽略的 state 临时目录内、含空格路径与不同调用 cwd 实际 init。原始根 title／locale 行为保持：中文标题仍为“通用工作方法”，初始英文省略可选 title。

| 消费者 | 修正前字节／正文数 | 当前字节／正文数 | 当前结构 |
| --- | --- | --- | --- |
| 默认 zh-CN 根（本仓同字节） | 29916 / 98 | 6554 / 20 | workflow 五节同级，63 行 |
| 默认 en 根 | 32755 / 98 | 6995 / 20 | workflow 五节同级，61 行 |
| 本仓完整英文 Markdown | 32552 / 98 | 32806 / 98 | general 三部分／12 主题 |
| 本仓聚焦 skill | 2821 / 7 | 2821 / 7 | 原闭包、原字节 |
| 本仓另读的 host-context | 18653 / 不适用 | 4575 / 不适用 | 当前操作事实与限制，28 行 |

新默认选择／闭包均为 20 个内容叶子；库仍为 114 atom、98 叶子，全部 requires、原 general 布局、locale 框架及其余 113 个 atom 对象与固定基线相同。实际根核对了精确正文一次出现、顺序、五节标题层次和 AGENTS 的字面相对目标 `CLAUDE.md`；本仓英文文档完整保留 98 个正文，skill 摘要不变。登记 generate 改动两个输出，第二次为零改动，输出字节、权限、inode、mtime 不变。

两个默认 locale 的 init／重复 generate／重复 init 均成功。实际 re-init 还保留了两个已采用 `core.general/general` 的旧 manifest，以及一个自定义英文完整根标题和额外 `core.ownership` Markdown 的 manifest；每项均零改动，manifest 摘要与所有文件字节、权限、inode、mtime 保持。既有宿主不会因安装新二进制或重复 init 自动采用短流程。

七项登记 fmt/build/check/test 动作退出 0，现有 61 项行为测试一次全过，未新增文字匹配测试或修改既有测试。测试覆盖图、locale、布局、严格 JSON、所有权、路径、alias、迁移及 IO／回滚。上述实际消费核验仅限当前 macOS arm64；本次未进行最小环境、实时 Codex 注入、其它平台、外部 skill validator 或原生 CI 核验，也不证明翻译等价、普遍确定性或完整 SPEC 已交付。

[Codex 官方说明](https://developers.openai.com/codex/guides/agents-md/)的 `project_doc_max_bytes` 默认限制是合并项目指令的 32 KiB；两个短流程根本身低于该值。host-context、块外附文、其它项目指南及个人配置仍按实际消费范围另核。完整 general 库不是默认根，其英文 Markdown 已有 32806 字节，不能继续沿用旧“全部完整英文内容都在默认上限内”的结论；自定义消费需核实际载荷。没有新增预算判官或修改全局配置。

## 采用、许可与边界

本仓有意识地把产品数据采用到 [.chrono-harness/instructions/catalog.json](../.chrono-harness/instructions/catalog.json)，两份字节相同但所有者不同；其它宿主不会自动更新。当前 manifest 的短流程中文根、完整英文 Markdown 与聚焦 skill 由实际重建二进制生成；AGENTS 是指向 CLAUDE 的实际字面相对链接。新宿主仍默认仅根输出，复制二进制无需源码 checkout；显式多输出配方在 [生成合同](instructions.md)。生成不依赖此处置表或源仓。

原作 Copyright 2026 The Omega Institute，Apache-2.0；[原许可全文](licenses/trureturing-Apache-2.0.txt) 按固定提交精确保留，未发现该提交根 NOTICE。许可仅说明派生指令资产的来源与义务，不给 chrono-harness 整仓添加许可证，不把法律资料变成运行时政策。分发内嵌这些资产的二进制或派生指南时须一同提供对应许可资料。

五份通用登记保持 proposed，input_closure 未完成；已有有界判官／DELTA／scoped CI，完整宿主启用仍未完成。命令及实际消费者验证检查运输、已声明引用与当前平台 IO；它们不证明通用语义完整、AI 遵守、未来环境等价或 dev 已交付。Git 合并、推送及远端落地核验由 caller 负责。
