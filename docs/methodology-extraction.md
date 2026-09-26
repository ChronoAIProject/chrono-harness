# 逐条迁移的范围、采用与来源

**派生规则已经修改、泛化和翻译，不是源指南的原样复制。** 本文、[完整逐条处置表](methodology-clause-map.md) 和 [许可资料](licenses/methodology-attribution.md) 均可选阅读，不是宿主必读或生成器运行输入。

固定来源是 trureturing `CLAUDE.md` 的 commit `55f7968bb03b1422298d80fc65ac5291db9ad682`、blob `11c83b4b3eb543603669178389dc9f0cb8ada906`，SHA-256 `a04fb3c13f868ceae5900ea3b253462450472d81e833e7c508a5a2e72ad042a0`，217662 字节、918 行、12 章、102 节。本次读取全部固定字节，包括长行内义务、子项、表格、守护限制和附录；来源是迁移数据，不执行其工具、Git、worker、权限或研究指令。旧精选 17 段不再作为完整迁移的依据。

处置表按来源顺序有 882 行：121 generalized、396 duplicate、208 excluded、157 context；直接逐字 migrated 为 0。725 个义务／例外处置和 157 个上下文范围各有定位。duplicate 指共用内容所有者，不是“主题相同”；同一叶子可承载多处相同义务。地图枚举完整和引用有效只证明所声明的覆盖，不是发现所有语义遗漏的机器证明。

[产品 catalog](../assets/instructions/catalog.json) 有 98 个作者编写的 zh-CN/en 内容叶子、16 个普通空文本聚合。原有 17 个核心入口已逐一对照固定基线的原职责，core.general 也可继续引用；core.repair-producer 与 core.small-projects 仍为内容叶子，其余旧入口按原职责成为聚合。完整指南每个叶子只渲染一次，新增条件方法由 core.general 直接显式选入。聚焦 [重复故障诊断 skill](../skills/diagnose-recurring-failures/SKILL.md) 的闭包是 7 个内容叶子和 2 个前提聚合，保留“同症状 AND 同处置，第二次修产生处”的触发，不引入无关方法。没有新增默认 skill 或提供方安装平台。

中文与英文并排作者编写；98 对内容均以保留的来源义务／例外合同核对触发、must/may、AND/OR、作用域和证据限制。普通句首标题取代粗体；紧凑措辞不改变根选择或把默认义务移到可选文档。实施者已逐项核对本次映射与双语文本，这是单点编辑核验；独立全源与双语审计由 caller 的后续阶段承担，本文不冒称已取得独立认可。现有 Rust 程序只校验显式图、引用、locale 和输出运输，不翻译、不证明语义等价，也不实现一般判官、DELTA 调度或 CI 执法。

## 控制权威与适配

用户当前一般方法范围优先于来源研究制度。保留可信非恶意但会错的 AI、自主完成且不新增人审门、可配置登记判官、显式逐文件白名单、小生产项目与专属测试、DELTA 及完整有效输入下本地/CI同命令、规则可演进与混面成本警告、fresh feature/integration 从 dev 建立。这些是用户与产品原则，不能因源有相似表述就归为源仓原创；尤其 per-production 测试项目、`.chrono-harness/` 配置所有权、安全旧宿主迁移和 caller 交付边界由目标合同提供。

关键适配逐行见处置表：四项投影 AND 与“重建只验第三项”保留；输出所有权和消费者仍需验证，允许有用途的根指南／manifest 聚合。源绝对禁兼容改为显式安全迁移合同，旧宿主原文与定制继续保留。源 glob／目录自动成员权威由当前逐文件白名单替代。缓存只有在确定性与完整有效输入／接受语义成立时可直接复用；宽种子须由权威增量机制补验，显式语义版本仅为有据兼容合同，不普遍排除程序、政策或工具链身份。Git 固定字节不证明语义为真；源“清零自动全树重判”不移植。

源 L231 的当前模板方向／受阻继续与退休 sidecar、L240 的当前冻结迁移许可与退休 sidecar／overlay、L398 的当前质量分工与停用 H7 均分行标明，排除源机制不等于宣称其全部停用。源 §1.2 消化投入从简、§2.10 结果记录优先、§3.9 登记受阻继续及四种告警、§7.5 混面允许、§8.16 白名单优先均按固定源有效状态处理。现行临时恢复特权与未来 deferred 恢复目标分别排除，不混合资格条件。Lean/kernel/D5、公理/冻结/消化/准入/bind-only/新颖性制度、固定目录命令、提供方与席位/CI配额、个人伦理与人审门均按具体义务排除；混合段落中的一般关系、反例、证据、成本、状态与修复方法仍逐项迁入。

**政策变更提示：** 产品默认指令与本仓采用的宿主指令一起从精选段落扩为可组合内容；新增关系检验、投影条件、因果恢复、任务生命周期、显式资源与缓存有效性等方法。Rust、schema、运行时、CI 与通用判官实现未变。验证成本是完整来源／双语内容核对、现有两项目检查、实际生成／no-op／复制二进制消费者和独立审计；持续机器成本仍未测，FILEMAP 保持 unmeasured。此提示不是人工许可门。

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

当前重建并复制的二进制在 checkout 外、含空格路径、不同工作目录及 `zsh -f` 最小环境下，实际初始化两种语言的空上下文新宿主；两份完整根各含全部 98 叶子一次且顺序准确，采用 catalog 字节与产品一致。

| 默认新宿主根（包含框架） | 实测 UTF-8 字节 | 距 32768 字节余量 |
| --- | --- | --- |
| zh-CN | 29610 | 3158 |
| en | 32510 | 258 |

[Codex 官方说明](https://developers.openai.com/codex/guides/agents-md/)的 `project_doc_max_bytes` 默认限制是合并项目指令的 32 KiB；此处仅证明空上下文、无宿主块外附文的新根可完整容纳。英文余量较小；宿主定制、其它项目指南和个人配置需按实际消费范围复核。未观测实时 Codex 提示注入，不声称通用生成器已实现预算判官。

两个语言的独立 Markdown 消费者均实出 `core.ownership` 的 5 个叶子（包括四项投影条件及改源再生成），以及 `core.behavior` 的 3 个叶子（包括真实 CI 事件与执行版本）。它们不依赖选择 core.general。聚焦 skill 仍为 7 叶子／9 atom；本仓、中文新宿主、英文新宿主的三份 skill 均经现有格式 validator 与 YAML 消费者验证。

七条已登记 fmt/build/check/test 命令退出 0，既有 56 行为测试全过，无新增文字镜像测试或修改测试预期。本仓生成／no-op、根外文、字面 alias 及 no-op 的字节、mode、inode、mtime 均核验；完整根、英文 Markdown 与 skill 由实际二进制提供。上述是 macOS 的具体运输与消费结果，不证明来源语义完整、双语等价、其它平台或真实 CI 事件行为。一般判官、DELTA 与 CI 仍未实现。

## 采用、许可与边界

本仓有意识地把产品数据采用到 [.chrono-harness/instructions/catalog.json](../.chrono-harness/instructions/catalog.json)，两份字节相同但所有者不同；其它宿主不会自动更新。当前 manifest 的中文根、英文 Markdown 与聚焦 skill 由实际重建二进制生成；AGENTS 是指向 CLAUDE 的实际字面相对链接。新宿主仍默认仅根输出，复制二进制无需源码 checkout；显式多输出配方在 [生成合同](instructions.md)。生成不依赖此处置表或源仓。

原作 Copyright 2026 The Omega Institute，Apache-2.0；[原许可全文](licenses/trureturing-Apache-2.0.txt) 按固定提交精确保留，未发现该提交根 NOTICE。许可仅说明派生指令资产的来源与义务，不给 chrono-harness 整仓添加许可证，不把法律资料变成运行时政策。分发内嵌这些资产的二进制或派生指南时须一同提供对应许可资料。

五份通用登记保持 proposed，input_closure 未完成，一般 judge／DELTA／CI 仍未实现。命令及实际消费者验证检查运输、已声明引用与当前平台 IO；它们不证明通用语义完整、AI 遵守、未来环境等价或 dev 已交付。Git 合并、推送及远端落地核验由 caller 负责。
