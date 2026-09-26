# 通用方法逐条来源处置表

本表是可选来源与最终处置结果，不是宿主必读政策、生成输入或运行时登记。派生指令已修改、泛化并翻译；原作 Copyright 2026 The Omega Institute，Apache-2.0，见 [许可与改动说明](licenses/methodology-attribution.md)。

固定来源：[trureturing/CLAUDE.md](https://github.com/the-omega-institute/trureturing/blob/55f7968bb03b1422298d80fc65ac5291db9ad682/CLAUDE.md)，commit `55f7968bb03b1422298d80fc65ac5291db9ad682`，blob `11c83b4b3eb543603669178389dc9f0cb8ada906`，SHA-256 `a04fb3c13f868ceae5900ea3b253462450472d81e833e7c508a5a2e72ad042a0`，217662 字节、918 行、12 章、102 节。来源仓 HEAD 后续移动不改变此 pin。未复制整份来源归档。

表内定位按固定源的节号与实际行号；同一行的 `a`、`b` 等后缀区分该行内的独立义务／例外；已分配定位保留，补拆项追加后缀，短锚为该行的“实际义务”栏。一个 AND 判据在一行中列全条件，分行表示可独立评估的义务；不是按章或按源码行数宣称完成。标题、空行、图表与背景也有明确上下文范围。源文本已连续读取 L1–45、46–105、106–165、166–215、216–265、266–315、316–375、376–425、426–463、464–515、516–555、556–605、606–655、656–705、706–745、746–785、786–835、836–880、881–918，无未读截断。

`generalized` 表示本次抽取／适配为通用义务；`duplicate` 表示与所指叶子的触发、动作和边界共用内容所有者（包括本次先行建立的叶子），不是只因主题相近；`migrated` 保留给无适配的直接迁入，本表未靠逐字复制制造这类计数。`excluded` 的原因在该行明确；`context` 不计作义务。目标栏均指 [产品 catalog](../assets/instructions/catalog.json) 的内容叶子，绝不用 core.general 代替具体处置。聚合入口与用户权威另见 [迁移说明](methodology-extraction.md)。

源状态栏的“现行”只表示固定源中的有效文字，绝不表示 chrono-harness 已实现该规则。全表记录形式受源 §2.10 当前“只留结果”修订控制；消化投入受 §1.2 限定，源 §3.9 受阻继续和告警修订、§7.5 混面允许、§8.16 白名单优先均保留。目标以用户当前一般方法、非恶意而会错、自主无新人审、显式逐文件登记、完整有效输入、DELTA 与安全宿主迁移合同为控制权威，任何与之冲突的来源绝对要求在对应行适配或排除。

机械定位／引用检查只验证声明枚举和运输，不能发现所有潜藏语义义务或证明双语等价。此表与叶子经过实施者逐条编辑核对；独立全源／双语审计属于 caller 的后续独立核验，不能由本表或生成成功替代。

实测表项：882 行；migrated 0，generalized 121，duplicate 396，excluded 208，context 157。目标含 98 个内容叶子、16 个普通聚合，表内引用 98 个叶子。计数不证明语义完整。

| 固定源定位 | 实际义务／短锚 | 有效源状态 | 处置 | 内容所有者或具体排除原因 | 适配／保留边界 |
| --- | --- | --- | --- | --- | --- |
| §preamble L1–2:a | trureturing — 不动点(所有 agent 的必读标架) | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §preamble L3:a | 默认使用指定思考／回答 skill | 现行 | excluded | 源宿主默认技能与编译工作流 | 固定技能不是目标的依赖 |
| §preamble L3:b | 默认普通语言回答，保留影响结论的条件与未决边界 | 现行 | generalized | `artifact.no-diary` | 实际默认沟通义务；必要细节仍保留，不带指定技能或形式化工作流 |
| §preamble L4–8:a | 1. 权威、本体与不可逆真值 DAG；1.1 权威原文与守护强度 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.1 L9:a | 定理比喻、项目上下文与成熟理论归属 | 上下文／有界历史样本 | context | 本体介绍与来源定位 | 不是工程执行规则 |
| §1.1 L10:a | 硬软分列且不得超报守护强度 | 现行 | generalized | `judge.strength` | 人工门由当前用户无新增人审原则替代 |
| §1.1 L10:b | 不可机器判者须对手官与人类门 | 现行 | excluded | 人类门及强制席位 | 保留自主核验，不移植审批 |
| §1.1 L11:a | 权威原文唯一且 AGENTS 只指向它 | 现行 | duplicate | `owner.canonical` | 保留目标 literal alias 合同 |
| §1.1 L11:b | 别名只在登记声明并保持同仓同快照真实源 | 现行 | generalized | `owner.canonical` | 目标仅实现根文件字面别名，不导入目录别名制度 |
| §1.1 L11:c | 保留链接原字节、拒绝不符／越界／链／循环／重复收文 | 现行 | excluded | 源 FILEMAP 目录链接详细策略 | 目标已有专用输出路径与 alias 校验；不扩 runtime |
| §1.1 L11:d | 历史采用自身 FILEMAP；各 worktree 指向本地源 | 现行 | generalized | `delta.candidate-judge`、`owner.canonical` | 固定修订角色，目标不新增历史链接引擎 |
| §1.1 L11:e | 禁 .git/.lake 链接及常规指针文件许可 | 现行 | excluded | 源目录与路径政策 | 不移植固定目录名单 |
| §1.1 L12–14:a | 1.2 架构三分与唯一真源 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.2 L15:a | Lean 声明／证明项／axiom 闭包是数学真值且优先扩 DAG | 现行 | excluded | 数学真值与形式化优先制度 | 不移植内核／公理制度 |
| §1.2 L15:b | 系统内已验证不涵盖散文与解释 | 现行 | generalized | `evidence.scope`、`judge.strength` | 改为实际检查范围限制 |
| §1.2 L16:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.2 L17:a | 消化偏差简注、允许暂存、不追账本完美 | 现行 | excluded | 源消化投资优先例外 | §1.2 控制源记账义务，不移植账本制度 |
| §1.2 L17:b | 不为辅助追踪扩工具或重复验证主线 | 现行 | generalized | `cost.valid-evidence`、`structure.pressure` | 一般化为实际用途与充分验证边界 |
| §1.2 L17:c | 现役门阻塞时只作恢复所需最小处理 | 现行 | generalized | `recovery.validate`、`action.blocked` | 保留适用验证，不导入门旁路 |
| §1.2 L17:d | 不得伪报覆盖／证明／检查通过 | 现行 | duplicate | `evidence.program-state` | 不要求目标证明制 |
| §1.2 L17:e | 内核、公理、冻结严格执行及本款优先 | 现行 | excluded | 研究保护面与消化修订优先级 | 本行一般诚实义务已另映射 |
| §1.2 L18:a | 文字修订不升级收据为证明或改变机器检查 | 现行 | duplicate | `judge.strength` | 守护强度限制 |
| §1.2 L19:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.2 L20–25:a | 不可逆真值 DAG 的三层图示 | 上下文／有界历史样本 | context | 研究架构背景 | 程序／数据义务在 L28 独立处理 |
| §1.2 L26:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.2 L27:a | Lean声明/证明项/公理闭包为真值，注释不参与 | 现行 | excluded | 数学真值权威 | 不移植内核真值制度 |
| §1.2 L27:b | D5 TASK地址是冻结门读取地址，其余工单散文不承重 | 现行 | excluded | 源研究治理地址 | 不移植固定D5/SL消费者 |
| §1.2 L27:c | SL-013 deferred且不执法散文形状 | 现行 | duplicate | `judge.strength` | 实际检查状态不由文字承诺改变 |
| §1.2 L28:a | 程序集只放程序，声明实例在外或测试夹具 | 现行 | duplicate | `owner.program-data` | 去固定 C# 与目录 |
| §1.2 L29:a | 理论是参考输入且程序不依其定位 | 现行 | generalized | `owner.canonical`、`artifact.substance` | 来源仅可选 provenance，不导入 TheoryIsolation |
| §1.2 L29:b | 理论 PR 可不摄入、atoms 不删、勘误追加 | 现行 | excluded | 源理论／消化／CAS 生命周期 | 由 §1.2、§3.8 的有效例外解释 |
| §1.2 L29:c | 已完成者不重复，历史由 Git，程序只证当前态 | 现行 | generalized | `reuse.search`、`evidence.program-state`、`delta.candidate-judge` | Git 固定字节不自动保证语义 |
| §1.2 L29:d | 回滚只删新建未入账 blob | 现行 | excluded | 源 CAS 写者具体回滚制度 | 目标普通 IO 回滚已有独立合同 |
| §1.2 L30–32:a | 1.3 真值图、冻结与两个偏序 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.3 L33:a | 节点四态与证明边、良基无环分层 | 现行 | excluded | 研究真值图定义 | 不导入closed/open/tail/semantic |
| §1.3 L34:a | kernel-verified即冻结、成员按state且永不解冻 | 现行 | excluded | 数学冻结成员与不可逆制度 | 不迁入目标状态机 |
| §1.3 L34:b | 当前检查只核成员与当前树，历史归Git | 现行 | generalized | `evidence.program-state`、`judge.strength` | 不以历史字节证明当前语义 |
| §1.3 L35:a | 图深度与信任层次两偏序不得混用 | 现行 | generalized | `reason.comparison` | 保留比较坐标不得混用；不导入深度／τ 定义 |
| §1.3 L36:a | 核心变更影响后代且成本以去重向量审计 | 现行 | generalized | `cost.measure`、`policy.evolution` | 去比例定理，按显式影响与读数 |
| §1.3 L37:a | 新规则不能推翻已冻结admit | 现行 | excluded | 数学保守扩展制度 | 用户允许有据演进政策 |
| §1.3 L38:a | 模型唯一作用是frontier证明open后冻结 | 现行 | excluded | 研究方向与模型角色 | 不限定一般工程任务 |
| §1.3 L39:a | 误判撤销自身与后代但不改变kernel真值 | 现行 | excluded | 数学冻结勘误制度 | 一般因果恢复另映射§7.14 |
| §1.3 L40:a | 人无位置且四态穷尽 | 现行 | excluded | 源真值角色本体 | 目标自主来自用户授权 |
| §1.3 L41:a | harness维护admission/SL-008/τ/成本/frontier图 | 现行 | excluded | 源数学治理执行体系 | 不宣称目标实现同构 |
| §1.3 L42–44:a | 1.4 素数、因陀罗网与罗盘边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §1.4 L45–48:a | 素数／因陀罗网／Merkle／Gödel 的本体比喻 | 上下文／有界历史样本 | context | 本体罗盘及历史机制说明 | 不是移植执行政策 |
| §1.4 L48:a | 罗盘非证明，停用机制不得报现役 | 现行 | duplicate | `judge.strength`、`review.independent` | 不引定理背书 |
| §1.4 L49–53:a | 2. 认识论与科学方法；2.1 账必须平 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.1 L54:a | 异常有读数并显式记录未验／延后，不静默遗漏 | 现行 | generalized | `evidence.anomaly` | §2.10 只留结果，取消强制永久案号与浮账清零 |
| §2.1 L55:a | 源 SL 检查与散文评审的不同保证 | 现行 | duplicate | `judge.strength` | 具名检查不带入 |
| §2.1 L56–58:a | 2.2 机器判对错与不可判 open | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.2 L59:a | 形式不可判诚实未决而非由人判真 | 现行 | generalized | `evidence.anomaly`、`judge.strength` | 未验证不冒称逻辑不可判；源 Gödel 制度排除 |
| §2.2 L59:b | 自信仍须独立核验 | 现行 | duplicate | `review.independent` | 不引入人类动作授权门 |
| §2.2 L59–60:a | Lean 无 sorry／私 axiom，sshx 三席等验证配置 | 现行 | excluded | 源研究准入与固定席位 | 一般独立核验另映射 |
| §2.2 L61–63:a | 2.3 美与逻辑 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.3 L64:a | 审美启发不能当证明，直觉交实际证据 | 现行 | duplicate | `review.independent` | 可用消费者／反例／独立视角 |
| §2.3 L65:a | 软性美学不可 lint | 现行 | duplicate | `judge.strength` | 守护限制 |
| §2.3 L66–68:a | 2.4 状态语法与不冒领 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.4 L69:a | 可验须验；未验／延后明说且不得冒称现役 | 现行 | duplicate | `evidence.measure`、`judge.strength` | 源状态词／案号非必需 |
| §2.4 L70:a | 状态禁止手写且 active/deferred 分列 | 现行 | duplicate | `evidence.program-state`、`judge.strength` | 去具名 SL |
| §2.4 L71–73:a | 2.5 门槛与最小权威 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.5 L74:a | 门控与实际风险相称不过度 | 现行 | generalized | `policy.proportional` | 用户可信非恶意 AI；不导入权限门 |
| §2.5 L74–75:a | 指定数学／发布／元层人类审批与工具名单 | 现行 | excluded | 源人工门和研究面 | 当前用户明确冲突 |
| §2.5 L76–78:a | 2.6 预测检验与适应 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.6 L79:a | 预先登记可证伪预测、未来检验而非事后拟合 | 现行 | generalized | `evidence.predeclare`、`evidence.prediction` | golden／break_eta／新颖性账本不移植 |
| §2.6 L80:a | 方法可适应变更、显式登记、版本化迁移 | 现行 | generalized | `owner.migration`、`registry.explicit` | 保持定制，不能以源码永久真值解释 |
| §2.6 L80–81:a | 真值单调与冻结内核／epoch 义务会计 | 现行 | excluded | 研究保守扩展制度 | 不是目标升级合同 |
| §2.6 L81:a | 选择预测仍需判断，硬锚不涵盖软纪律 | 现行 | duplicate | `judge.strength` | 机器边界 |
| §2.6 L82–84:a | 2.7 科学方法的自反回灌 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.7 L85:a | 所有 agent 默认用本库定理为罗盘 | 现行 | excluded | 来源定理默认推理权威 | 源数据不是运行指令 |
| §2.7 L85:b | 域内定理外推仍须独立预测检验 | 现行 | duplicate | `reason.transfer`、`evidence.prediction` | 不引 theorem 保证 |
| §2.7 L86:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.7 L87:a | 跑前明确成功／推翻判据，仅按预定结算 | 现行 | duplicate | `evidence.predeclare` | 结果记录受 §2.10 控制 |
| §2.7 L88:a | 见过数据全对不等于预测力 | 现行 | duplicate | `evidence.prediction` | 不把“零信息”作普遍形式结论 |
| §2.7 L89:a | 区分可消混淆与结构盲区，后者换语言框架 | 现行 | duplicate | `reason.representation` | 去定理专名 |
| §2.7 L90:a | 无边际改进换表述／工具／层级而非加预算 | 现行 | duplicate | `reason.representation` | 不设固定轮数 |
| §2.7 L91:a | 改假设／目标／规则显式列旧结论有效与作废 | 现行 | duplicate | `goal.revision` | 不保存结算日记 |
| §2.7 L92:a | 逐坐标比较且不同叙事路径收益须一致 | 现行 | duplicate | `reason.comparison` | 不用未经论证标量 |
| §2.7 L93:a | 陈旧／相关证据限制自判，取独立证据或标未验 | 现行 | duplicate | `evidence.source-quality` | 无强制席位或利益审批 |
| §2.7 L94:a | 预定截止与判据，到期真实状态不拖延 | 现行 | duplicate | `evidence.predeclare` | 不引五态终局制度 |
| §2.7 L95:a | 类比持续误判修外推面；定理冻结另守 | 现行 | generalized | `reason.transfer`、`reason.representation` | 冻结部分排除 |
| §2.7 L95:b | 推理罗盘软纪律不因引用定理升硬 | 现行 | duplicate | `judge.strength` | 守护限制 |
| §2.7 L96–98:a | 2.8 成本感与数据核验 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.8 L99:a | 觉得贵先取当前计数、diff、日志与原判词 | 现行 | duplicate | `cost.measure` | 不拿旧面板／记忆代实测 |
| §2.8 L99:b | 先去非必要工作或错误形态，再量；贵不降质 | 现行 | duplicate | `cost.measure` | 不造强制重命名迁移 |
| §2.8 L99:c | 新读数修改结论须显式说明 | 现行 | duplicate | `goal.revision` | §2.10 取消过程留存 |
| §2.8 L100:a | 成本导致改计划须读数、采集方法、窗口口径 | 现行 | duplicate | `cost.measure`、`signal.meaning` | 不机械引用 PR 格式 |
| §2.8 L101–103:a | 2.9 测量、未知与排除项 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.9 L104:a | 模糊词不替测量，未测需说明原因 | 现行 | duplicate | `evidence.measure` | 不是禁某些自然语言词 |
| §2.9 L105:a | 未知标对象原因并列已排除／仍存解释 | 现行 | duplicate | `evidence.anomaly` | 不强制源状态机或完成时间承诺 |
| §2.9 L105:b | 转述同样保留核验责任，绿只认真实退出 | 现行 | duplicate | `review.disclosure`、`evidence.measure` | 时间不替代退出码 |
| §2.9 L106:a | 行为强制与自然语言无 lint 分列 | 现行 | duplicate | `judge.strength` | 去源检查名 |
| §2.9 L107–109:a | 2.10 记录律：整个系统只留结果，不留过程 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.10 L110:a | 只留正式成果、必要程序／数据／来源／许可证和机器状态 | 现行 | duplicate | `artifact.results`、`evidence.program-state` | 本条控制早先及后续留痕义务 |
| §2.10 L111:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.10 L112:a | 成果各归对应位置且实验材料有用途与登记 | 现行 | duplicate | `artifact.results`、`registry.explicit` | 目标宿主登记 |
| §2.10 L113:a | 搜索／枚举／反例／边界程序无正文引用仍可有用 | 现行 | duplicate | `artifact.results` | 不能按主构建成员身份删有用材料 |
| §2.10 L114:a | 禁过程转录、评审对话、命令流水、回执／快照及链接归档 | 现行 | duplicate | `artifact.no-diary` | §2.10 优先 |
| §2.10 L114:b | 会话 ID 恢复指针例外 | 现行 | excluded | 源会话追踪要求 | 目标无此默认字段 |
| §2.10 L115:a | 正文仅当前结论读数状态边界，失败留回归 | 现行 | duplicate | `artifact.results`、`artifact.no-diary` | 不复述机器状态或为记录派席 |
| §2.10 L116–117:a | 结果约束优先而不免检索／评审／异常处理；软边界 | 现行 | duplicate | `artifact.no-diary`、`judge.strength` | 不把登记称机器执行 |
| §2.10 L118–120:a | 2.11 关系优先的研究与类比迁移 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.11 L121:a | 关系组织问题、定位缺口并以证据验证类比 | 现行 | duplicate | `reason.goal-relations`、`reason.transfer` | 去研究／消化制度 |
| §2.11 L122:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.11 L123:a | 核已有成果版本假设范围状态，连接结论到假设 | 现行 | duplicate | `reuse.search`、`reason.goal-relations` | 研究真源附加制度另排除 |
| §2.11 L123:b | 共享记录不是自动证明 | 现行 | duplicate | `evidence.scope` | 去 Lean／TheoryIsolation／bind-only 引用 |
| §2.11 L124:a | 说明对象生成、共同来源、观察、操作、量词精度资源 | 现行 | duplicate | `reason.goal-relations` | 保留各条件，适用时使用 |
| §2.11 L124:b | 区分存在／可识别／可取得／可认证，倒推缺口 | 现行 | duplicate | `reason.goal-relations` | 一般工程对象而非数学制度 |
| §2.11 L124:c | 同目标范围资源比较足够辅助结构 | 现行 | duplicate | `reason.comparison`、`reason.sufficient-condition` | 保留减少重复与取得验证成本 |
| §2.11 L125:a | 同观察但异答案或合法性的实际对例推翻充分性 | 现行 | duplicate | `reason.insufficiency` | 成对条件一起保留 |
| §2.11 L125:b | 未找到对例不证明充分；须所有同观察实现不变 | 现行 | duplicate | `reason.insufficiency` | 保留充分性证据义务 |
| §2.11 L125:c | 后处理不能恢复合并区别，增加关系并说明回接缺口 | 现行 | duplicate | `reason.insufficiency`、`reason.representation` | 去形式词汇强制 |
| §2.11 L126:a | 启动／新缺口／停滞主动找直接可接结果 | 现行 | duplicate | `search.routes` | 指定 GPT Pro/Nyx/§5.11 排除 |
| §2.11 L126:b | 逐级退求辅助／受限／估计／反例，原标准不变 | 现行 | duplicate | `search.routes` | 部分进展不是目标完成 |
| §2.11 L127:a | 沿论文引用及结构找工具和反例，核原始来源条件 | 现行 | duplicate | `search.routes`、`search.bounded` | 不强制论文检索或指定提供方 |
| §2.11 L127:b | 无命中不证明不存在或原创 | 现行 | duplicate | `search.bounded` | 负证据限定范围 |
| §2.11 L128:a | 类比产生表示与搜索词，迁移核对象假设操作性质等 | 现行 | duplicate | `reason.transfer` | 量词尺度方向误差成本均保留 |
| §2.11 L128:b | 可表达／可逆不等于可操作／可验／等成本 | 现行 | duplicate | `reason.transfer` | 条件不能省略 |
| §2.11 L128:c | 定位失配和未证桥梁，同名数值术语不替有效对应 | 现行 | duplicate | `reason.transfer` | 不移植形式化证明制度 |
| §2.11 L129:a | 授权隔离下并行、同目标假设版本；盲评不读同轮结果 | 现行 | duplicate | `collaboration.combine` | 条件化，不自行授权派席 |
| §2.11 L129:b | 汇合核相容重复遗漏和用户限定，收益看缺口 | 现行 | duplicate | `collaboration.combine` | 席位数不算进展 |
| §2.11 L130:a | 共同对象／历史／条件或有效映射方可联合 | 现行 | duplicate | `reason.joint-evidence` | 数学概率实例改一般证据 |
| §2.11 L130:b | 分别最优不同时可达、边缘不等联合、界方向正确 | 现行 | duplicate | `reason.joint-evidence` | 总量上界不是部分下界 |
| §2.11 L131:a | 倒推联合充分条件，允许互补且区分必要与强条件 | 现行 | duplicate | `reason.sufficient-condition` | 强条件失败不反驳原目标 |
| §2.11 L132:a | 预定判据、小／退化／极端样本及独立核验 | 现行 | duplicate | `evidence.predeclare`、`evidence.scope` | 按适用范围 |
| §2.11 L132:b | 分列数值／松弛／实现／有限／统一及近似方向 | 现行 | duplicate | `evidence.scope` | 不混方法反例与命题反例 |
| §2.11 L132:c | 结构不足换法，结果与未解边界保留 | 现行 | duplicate | `reason.representation`、`artifact.results` | §2.10 结果形式 |
| §2.11 L133:a | 成果集中且保留实质定义假设推导与边界 | 现行 | duplicate | `artifact.substance` | 不以短摘要代成果 |
| §2.11 L133:b | 应用／综合／原创区分，改表述不自动新颖 | 现行 | duplicate | `artifact.substance`、`search.bounded` | 研究原创准入／新增 Lean bind-only 排除 |
| §2.11 L134:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §2.11 L135:a | 对应与条件靠实际核验，不新增判官或把实验当形式证明 | 现行 | duplicate | `judge.strength`、`evidence.scope` | 守护边界 |
| §2.11 L136–140:a | 3. 形式化、逃逸内容、用途与研究；3.1 先库后证与宿主搜索能力 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.1 L141:a | 搜索能力属宿主，禁止改全局配置 | 现行 | duplicate | `method.host-tools` | 固定搜索工具与离线研究门排除 |
| §3.1 L142:a | 本仓→钉版依赖→允许外部，精确命中直接应用 | 现行 | duplicate | `reuse.search`、`reuse.actual-consumers` | 源 axiom debt／open 制度排除 |
| §3.1 L142:b | 只留搜索结果范围与命中，不存过程 | 现行 | duplicate | `search.bounded`、`artifact.no-diary` | §2.10 |
| §3.1 L143:a | 私有命中公开原定义，不视为缺失重做 | 现行 | duplicate | `reuse.shared-definition` | 去 theorem 特定规则 |
| §3.1 L144:a | 实测搜索可用性，缺失不报完成，恢复后补检索 | 现行 | duplicate | `action.blocked`、`search.bounded` | 不继承源 worker 调度 |
| §3.1 L145–147:a | 3.2 bind-only 禁令、逃逸见证与 atom 终结 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.2 L148:a | 逐声明禁止 bind-only、全绑定零新增与外部问题例外 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 数学新颖性准入制度 | §1.2 消化从简与 L155 例外有效 |
| §3.2 L149:a | proof_shape 与 admission_basis 以及消化分开 | 现行 | excluded | 研究专用分类与状态轴 | 一般不同状态见 state.domain-workflow |
| §3.2 L150:a | 按内联、冻结／上游边界与实例化投影规范化判形 | 现行 | excluded | Lean 证明形状判定 | 全步骤、tactic 与局部包装限定属于研究制度 |
| §3.2 L150:b | 仅 tactic 名不能决定语义分类 | 现行 | generalized | `tool.semantic` | 一般语义结论不靠关键词 |
| §3.2 L151:a | 逃逸见证四项 AND：闭包内、非规范化可得、非等价重述、活路径 | 现行 | excluded | 新数学内容见证制度 | 四项整体及不用也可得即无见证一并排除 |
| §3.2 L152:a | 独立中间命题或公开结论可作见证，不人为造中间项 | 现行 | excluded | 数学见证形态 | 不得移植为目标产物准入 |
| §3.2 L152:b | 行数／声明数／参数数不作内容判据 | 现行 | generalized | `artifact.substance` | 保持用途胜于数量，不移植 statement identity |
| §3.2 L153:a | 首次冻结域与 immutable base pin；不能伪候选历史身份 | 现行 | excluded | 数学冻结范围 | 通用固定候选身份已在 delta.candidate-judge |
| §3.2 L153:b | 私有化／拆名／anchor 不得绕且既冻不撤 | 现行 | excluded | 研究冻结／判形反绕过 | 不引追溯真值义务 |
| §3.2 L154:a | escape-witness 准入逐声明，任一绑定拒；全绑定停实施 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 数学准入及停止条件 | 不作为通用研发判断 |
| §3.2 L155:a | 外部具名开放问题预登记后 bind-only 可首次冻结 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 源 2026-09-15 开放问题例外 | 保留来源有效状态，未泛化新颖性制度 |
| §3.2 L155:b | 例外四项 AND：最小公开声明、如实判形、Scribe/Problems、用途仍判 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 研究例外的完整条件 | 仓内 atom 不适用，探针后继续实施均属同制度 |
| §3.2 L156:a | 连读全部子句、参数假设与规范化才能判全 bind-only | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 数学内容完整性结算 | 一般全项完成限制另映射 L157 |
| §3.2 L156:b | 确认绑定停新增，低成本覆盖／decompose 与父链闭合 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 消化 writer 与终态合同 | §1.2 从简优先，不引强制结算 |
| §3.2 L157:a | 混合项只保留缺口，全部子项义务完成才报整项 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | generalized | `state.domain-workflow` | 一般完成完整性，不移植数学分类 |
| §3.2 L157:b | 前提未核实不能先称全部已满足 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | duplicate | `evidence.anomaly` | 具体 bind-only 声称不移植 |
| §3.2 L158:a | 终态由现役 writer 产生，失败回执／目录不冒成功 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | duplicate | `evidence.program-state`、`state.domain-workflow` | 源 settle/GID/收据字段制度排除 |
| §3.2 L158:b | 仅上游闭合用 settle，不新 writer/状态/机器判形 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 消化源专用特例 | 不引入“无人消费状态”普遍规则 |
| §3.2 L159:a | 已有上游直接复用，重复绑定两次换路线 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | generalized | `reuse.search`、`reason.representation` | 研究两次配额排除，保留证据驱动转向 |
| §3.2 L159:b | 空闲席位不构成派题理由 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | duplicate | `collaboration.combine` | 不自行授权派席 |
| §3.2 L160:a | 拟议逃逸须探针前预登记，变见证重登记，逐声明报告 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 研究见证预登记及字段要求 | 一般预定判据见 evidence.predeclare |
| §3.2 L160:b | 判形评审不能冒充机器已分类 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | duplicate | `judge.strength`、`tool.semantic` | 源 reject 清单不导入 |
| §3.2 L161–163:a | 3.3 计算性内容的用途准入 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.3 L164:a | 用途是独立第三合取，delta 范围逐声明评审 | 现行 | excluded | 数学用途准入分类 | 不将源独立维度混为一个工程门 |
| §3.3 L164:b | 无关反例不能掩盖未履行用途 | 现行 | generalized | `evidence.scope`、`artifact.substance` | 实际用途证据不靠同模块装饰 |
| §3.3 L165:a | 计算内容四类按交付语义而非文件／tactic 名 | 现行 | excluded | 数学分类四型 | 一般语义工具规则另保留 |
| §3.3 L166:a | 预登记独立问题；普通正向有限实例禁准入 | 现行 | excluded | 计算性数学准入禁令 | 不移植对工程有限实例的禁令 |
| §3.3 L166:b | 有效反驳免消费者但不免判形；无关反例不能夹带 | 现行 | excluded | refutes/escape/open-problem 制度 | 全部例外仍属数学准入 |
| §3.3 L166:c | checker/numeric-reduction 的活路径消费者或独立终点 | 现行 | generalized | `reuse.actual-consumers`、`structure.pressure` | 仅迁移真实首个消费，不导入数学分类 |
| §3.3 L166:d | 最严多类判定及 consumer/terminal 旧许可撤销 | 现行 | excluded | 源数学分类与撤销许可 | 旧许可 inactive，现行 refutes 生效 |
| §3.3 L167:a | 一般结果不加强假设可复用，有限枚举不证明无界全称 | 现行 | duplicate | `reuse.search`、`evidence.scope` | 不导入禁止冻结 |
| §3.3 L168:a | 首个实例四项：具体输入、证书或成功证据、soundness应用、所解问题 | 现行 | generalized | `test.first-useful-case` | 保留四项合取；形式soundness适配为实际正确性合同应用，不准空接口/未履行假设/无关玩具 |
| §3.3 L168:b | 归约前提同函数参数域方向，未履行只报条件结果 | 现行 | generalized | `reason.goal-relations`、`reason.joint-evidence`、`evidence.scope` | 数学用途／冻结准入部分排除 |
| §3.3 L168:c | 定义被用不等于结论被消费；源句重算不算新增成果 | 现行 | duplicate | `reuse.actual-consumers`、`artifact.substance` | 不引禁止认证实例 |
| §3.3 L169:a | utility 头部字段／GID 方向／formal claim 与 result 文法 | 现行 | excluded | Lean 源绑定用途 schema | 不复制 schema 或分类器 |
| §3.3 L169:b | 结构引用与源断言忠实性分开核 | 现行 | duplicate | `reference.resolve`、`judge.strength` | 一般结构／语义界限 |
| §3.3 L170:a | none 须全部声明皆不命中、逐声明理由、与 G/I/E 正交 | 现行 | excluded | 数学用途 none 例外合同 | 不泛化形式分类政策 |
| §3.3 L171:a | 必填 PR 字段及可选 build_seconds | 现行 | excluded | 源用途报告格式 | 一般读数口径见 cost.measure |
| §3.3 L171:b | 可变正文无消费者，结构硬门不使语义变硬 | 现行 | duplicate | `judge.strength`、`evidence.mutable-snapshot` | 保留证据时点限制 |
| §3.3 L172–174:a | 3.4 用途机器面、停用机制与软边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.4 L175:a | P-B0/J5/D2 旧机器与升级触发退役 | INACTIVE 的现行说明；不执行旧机制 | excluded | 源已停用准入机器 | 停用 |
| §3.4 L175:b | 零消费者边数不是用途判词，闭包不证全部语义 | 现行 | duplicate | `tool.semantic`、`judge.strength` | 防工程引用数冒语义 |
| §3.4 L176:a | SL-031 选择 changed-unfrozen/first-pin 与具体 Block 代码 | 现行 | excluded | 源 Lean 准入执行配置 | 不导入模块选择器或棘轮 |
| §3.4 L177:a | typed refutation 闭合 Prop、Not claim、许可公理及拒绝列表 | 现行 | excluded | Lean 内核反驳合同 | 不移植形式真值保证 |
| §3.4 L177:b | legacy report 能读取不等于可用于新准入 | 现行 | generalized | `owner.migration`、`evidence.scope` | 保留迁移兼容与使用资格分界 |
| §3.4 L178:a | ChangedContent/PreDeposit/FirstFreeze 字段与 coverage/import | 现行 | excluded | 源阶段准入合同 | 不引证明管线 |
| §3.4 L178:b | BASE 固定 commit，候选新增 pin 无历史豁免 | 现行 | generalized | `delta.candidate-judge`、`query.identity` | 不执行基线代码 |
| §3.4 L179:a | 实际输入即使不在 import 闭包仍须明确登记使其变化失效 | 现行 | generalized | `registry.explicit`、`cache.memoization` | 人工显式登记，不推断图 |
| §3.4 L179:b | 不得拼当前输入与旧证据，失效依赖不是语义分类器 | 现行 | duplicate | `evidence.mutable-snapshot`、`tool.semantic` | 去 claim report 名 |
| §3.4 L180:a | 分类、源映射、活路径、前提与支配仍须语义评审 | 现行 | duplicate | `tool.semantic`、`judge.strength` | 不引数学用途审判 |
| §3.4 L180:b | fresh none／改标签不是机器证明；关键词／依赖图不补语义 | 现行 | duplicate | `judge.strength`、`tool.semantic` | 保留已核 typed 关系的局部性 |
| §3.4 L181:a | 须解释解决问题和用途，引用覆盖数不能代替 | 现行 | duplicate | `artifact.substance`、`reuse.actual-consumers` | 不造无用消费者 |
| §3.4 L182–184:a | 3.5 用途与黎曼路线的证据范围 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.5 L185:a | 样本读数只属固定日期树与口径，不能外推当前全库 | 现行 | duplicate | `signal.meaning`、`evidence.scope` | 历史数据不复制为目标读数 |
| §3.5 L186:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.5 L187–192:a | 用途样本表各列与四行历史统计 | 上下文／有界历史样本 | context | 历史研究样本数据 | 保留方法，不迁移整表 |
| §3.5 L193:a | 族内引用不证明外部用途；未复算／集合不一致标未验 | 现行 | duplicate | `reuse.actual-consumers`、`evidence.anomaly`、`reason.comparison` | 具体模块／数字属样本 |
| §3.5 L194:a | 未履行前提的条件结果不得当已实现用途 | 现行 | duplicate | `evidence.scope`、`reason.goal-relations` | 源模块细节不导入 |
| §3.5 L195:a | 保留实际有用探针而非整个重复塔 | 现行 | duplicate | `artifact.results`、`cost.measure` | bind-only 冻结规则排除 |
| §3.5 L196:a | 黎曼各模块判形与冻结保留，空闲不再派无收益任务 | 现行 | excluded | 研究样本处置 | 空闲／边际收益的一般义务已映射 L159/L90 |
| §3.5 L197:a | 成熟理论引用 | 上下文／有界历史样本 | context | 方法背景 | 不赋予目标运行权威 |
| §3.5 L197:b | DECT无消费者淘汰与C8禁枚举须按范围及§3.3例外 | 现行 | excluded | 源理论用途准入及例外 | 不泛化为工程无消费者一律删除 |
| §3.5 L198:a | 无 fail-closed 消费者的判形／PR 正文不得称硬 | 现行 | duplicate | `judge.strength` | 源 SL/目录与未来机器形态不移植 |
| §3.5 L198:b | 若升硬须不可变绑定候选输入及变异击中具名检查 | 现行 | generalized | `evidence.mutable-snapshot`、`test.mutation`、`reference.resolve` | 不造新 judge 框架 |
| §3.5 L198:c | 报告通过不等于语义通过、无 lint 不免现行义务 | 现行 | duplicate | `judge.strength` | 源逐声明 reject 清单排除 |
| §3.5 L199–201:a | 3.6 开放问题三档与研究线 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.6 L202:a | 选题按实际推进，已知不重做，席位空闲不算理由 | 现行 | duplicate | `reuse.search`、`collaboration.combine` | 三档研究目标另排除 |
| §3.6 L203:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.6 L204–206:a | 三档开放问题／计算前沿／核心问题的提供方与节奏 | 现行 | excluded | 研究选题及模型角色制度 | 不导入固定人选、时长和人类点题 |
| §3.6 L207:a | 长期线保存障碍结果，预定停止条件，无进展换法 | 现行 | generalized | `evidence.predeclare`、`reason.representation`、`artifact.results` | 源两周／五态／子引理工作流排除 |
| §3.6 L207:b | 长期与短任务不能盲套相同节奏 | 现行 | generalized | `reason.comparison` | 比较要对齐范围资源，不导入研究时间配额 |
| §3.6 L208:a | 核文献已有、前两档特殊准入与探针流程 | 现行 | excluded | 数学研究准入 | 一般搜索范围见 search.bounded |
| §3.6 L208:b | 进展按解决缺口而非模块／席位／行数 | 现行 | duplicate | `collaboration.combine`、`artifact.substance` | 不导入有限证书资格 |
| §3.6 L208:c | 第三次非数学缺陷停、第四次 deposit 禁止 | 现行 | excluded | 源 lane/deposit 次数制度 | 同症状且同处置的一般触发保留 core.repair-producer |
| §3.6 L209:a | 选题判据与硬投影分列，不可 lint 不豁免 | 现行 | duplicate | `judge.strength` | 源档位制度排除 |
| §3.6 L210–212:a | 3.7 写作尽调与文献状态 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.7 L213:a | 正式数学写作须文献三态与 note/DOI/离线门 | 现行 | excluded | 研究尽调与来源准入制度 | 来源归属一般义务另映射 |
| §3.7 L213:b | 准确承认前人成果与自行推导，检索无命中有范围 | 现行 | duplicate | `artifact.substance`、`search.bounded` | 不强制学术写作流程 |
| §3.7 L213:c | 字段链检测不证明真已知，线上 Observe 非离线门 | 现行 | generalized | `reference.resolve`、`judge.strength` | 不导入特定在线／离线架构 |
| §3.7 L214–216:a | 3.8 理论正文与追加纪律 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.8 L217:a | 缺口在当前授权目标内自主补足，不等用户重写 | 现行 | duplicate | `action.autonomy`、`action.blocked` | 保持原目标假设；无 Lean 真值制度 |
| §3.8 L217:b | 尚未证成不得冒定理，辅助摄入不必阻塞主工作 | 现行 | generalized | `evidence.scope`、`action.blocked` | 源理论／消化的优先例外不泛化为免检查 |
| §3.8 L218:a | 纯新增理论 PR 可不 ingest，含工件须对应链核验 | 现行 | excluded | 源理论 PR 类型与消化例外 | 不移植此链 |
| §3.8 L219:a | 理论正文纯数学，其余成果／过程分别放置 | 现行 | excluded | 源数学正文类型制度 | 成果归位与禁过程已在 artifact.results/no-diary |
| §3.8 L220:a | 合 dev 后理论只追加新编号，更正另加命题，草稿可改 | 现行 | excluded | 源理论追加与编号制度 | 目标当前规范可原位演进 |
| §3.8 L220:b | 更正明确替代关系，旧结论不得继续作有效前提 | 现行 | duplicate | `goal.revision`、`collaboration.current-result` | 不导入禁止原位编辑 |
| §3.8 L221:a | 禁已有理论成为新卷／章／定理，只用 Library 中间引用 | 现行 | excluded | 数学新颖性与文献纳入制度 | 一般准确归属见 artifact.substance |
| §3.8 L222:a | 作者／独立评审核语义，不冒称机器强制 | 现行 | duplicate | `judge.strength`、`review.independent` | 研究正文义务排除 |
| §3.8 L223–225:a | 3.9 登记即声明模板与 delta 判官 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.9 L226:a | 登记明确模板与合法来源，不让判官猜模板 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `registry.explicit`、`judge.register` | 不导入 Lean enrollment |
| §3.9 L226:b | 无法表达不能硬套错误模板或为语料放宽判官 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `reason.representation`、`policy.evolution` | 规则可有据自主演进，不绝对禁止修改 |
| §3.9 L227:a | DTR 四个 Observe、delta/first-pin 选择、judge ownership | 现行；§3.9 告警及登记受阻继续例外 | excluded | 源逃逸登记判官制度 | 现行 2026-09-20 告警；旧阻断停用 |
| §3.9 L227:b | 全工件完整性检查与单条语义裁决范围分开 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `delta.indexing`、`judge.strength` | 目标只声明适用输入，不导入全局门 |
| §3.9 L228:a | 新增 D5 公开定理同交付尽力登记镜像／四槽／证明 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 数学逃逸作者义务 | 当前告警不免作者义务；L229 例外有效 |
| §3.9 L228:b | CI 绿／警告收据不能当实际义务完成 | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `evidence.program-state`、`judge.strength` | 不导入审计状态 |
| §3.9 L228:c | helper 不递归产生人工审计目标，历史不全回填 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 研究包角色豁免 | 目标 per-production test 另属用户权威 |
| §3.9 L229:a | 登记受阻 issue 后继续数学交付，优先于立即 hotfix | 现行；§3.9 告警及登记受阻继续例外 | excluded | 源 owner 特定审计受阻例外 | 完整保留例外范围；不泛化为跳过工程验收 |
| §3.9 L229:b | 报告原目标、实际失败、缺证且不能拿 issue 当完成 | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `action.blocked`、`evidence.anomaly` | 不新建强制外部消息 |
| §3.9 L229:c | 不硬套／削弱原命题／造包装，其余检查仍须通过 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `reason.representation`、`test.behavior` | 具体 Lean 准入不移植 |
| §3.9 L230:a | DependentFamily API／选择器／bridge／variation 语义 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 研究登记 API 合同 | 不构建对应 runtime |
| §3.9 L230:b | API 存在不证明未编译案例成立 | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `judge.strength`、`evidence.scope` | 守护限制 |
| §3.9 L231:a | 无合法模板归属时由内容所有者加有意义模板并共享 enrollment | 现行；2026-09-18 默认方向，受 L229 继续例外限定 | excluded | 研究内容模板与 Reg 镜像登记制度 | 新模板仍是默认方向；冻结所有者仍走镜像，不要求历史回填或另建债务／兼容／宽限机制，不造 bind-only 包装或给无新声明的复用另加审计 |
| §3.9 L231:b | 具体登记障碍可继续，不以新增模板为唯一继续路线 | 现行；L229 最新 owner 修订优先于 §5.4／§5.5，仅限登记受阻 | excluded | 源数学逃逸审计受阻特例 | issue 后继续不代表审计完成，不泛化为跳过工程验收；通用阻塞诚实义务见 L229:b |
| §3.9 L231:c | 旧 sidecar 命令已退役 | INACTIVE；L231 现行退役声明，L240 再确认 | excluded | 已退休的源登记机制 | 不将当前模板方向或受阻继续许可标成停用，也不复建 sidecar |
| §3.9 L232:a | 新定理至少一条四槽，多舞台不判完备自然性 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 逃逸分类及登记标准 | 选中域与 helper 豁免属研究 |
| §3.9 L233:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.9 L234:a | 原逃逸常量／binder 在陈述中与 State 身份匹配 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 逃逸槽一与编译器检查 | 不移植研究 schema |
| §3.9 L235:a | 处理逃逸须已 enroll 模板及 E1–E8 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 逃逸槽二 | 不移植研究 schema |
| §3.9 L236:a | 新信息桥／forward sensitivity／反例 bridge 细则 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 逃逸槽三 | 各合取均属形式化合同 |
| §3.9 L237:a | 残余 witness/empty/open 证书类型与舞台 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 逃逸槽四 | 不移植研究 schema |
| §3.9 L237:b | open 只表示未知，不证明不可判或无穷，也不替必要证据 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `evidence.anomaly`、`evidence.scope` | 一般未知诚实界限 |
| §3.9 L238:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §3.9 L239:a | Reg 独占声明、旧文法读取、四 Observe 与报告限制 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 研究声明格式及判词制度 | 现行兼容许可说明源禁兼容并非可直接普化 |
| §3.9 L239:b | 报告复用用 trace 与语义版本且仅检结构 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `cache.compatibility`、`cache.incremental-seed` | 仅有据合同，接受语义及有效输入不可省 |
| §3.9 L239:c | 新旧身份按固定 base 字节，读数据不执行 base | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `delta.candidate-judge` | 不移植 theorem 公开性豁免 |
| §3.9 L240:a | 数学／判官接口实现／登记独立包及单向依赖 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `owner.program-data`、`reuse.shared-definition`、`core.small-projects` | 保留真实独立消费者，不导入 D5/Reg 路径 |
| §3.9 L240:b | 判官只影响显式依赖者，复用现有缓存不造专属平台 | 现行；§3.9 告警及登记受阻继续例外 | generalized | `registry.explicit`、`method.host-tools`、`cache.incremental-seed` | 不按 import 动态推断测试图 |
| §3.9 L240:c | 冻结模块可为迁出声明编辑／搬迁，数学陈述证明不变，writer 重钉 | 现行；2026-09-20 owner 迁移许可 | excluded | 源数学冻结与 canonical writer 权限合同 | 当前许可不是退休机制；不改变目标旧宿主定制保留合同 |
| §3.9 L240:d | 迁移债务只能缩不换、触债严格减少 | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `change.debt` | 清零后全树判的源要求按用户 DELTA 排除 |
| §3.9 L240:e | sidecar 命令及跨模块 overlay 读取已退役 | INACTIVE；L240 当前迁移完成声明 | excluded | 已退休的源命令与覆盖读取路径 | 与当前冻结模块迁移许可分列，不复建旧路径 |
| §3.9 L241:a | Reg 包不承担 GID/Scribe/deposit/utility 义务 | 现行；§3.9 告警及登记受阻继续例外 | excluded | 研究项目角色豁免 | 不引固定包名 |
| §3.9 L242:a | DTR 硬告警与忠实性软评审分列 | 现行；§3.9 告警及登记受阻继续例外 | duplicate | `judge.strength` | 仅源状态事实，不宣称目标判官 |
| §3.9 L243–247:a | 4. 结构、递归归属、投影与消化；4.1 程序与数据的递归归属 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.1 L248:a | 每物唯一权威、程序治理声明实例与投影 | 现行 | duplicate | `owner.canonical`、`owner.program-data` | 去数学／τ 递归本体 |
| §4.1 L248:b | 改真源按层成本、改投影近免费 | 现行 | generalized | `cost.measure`、`projection.consumption` | 成本未知不能算零，不引定理比例 |
| §4.1 L248:c | 顶端人类 bootstrap 与 Gödel open | 现行 | excluded | 源信任塔本体 | 不作为目标执行结构 |
| §4.1 L249–251:a | 4.2 算法地址、当前形态与数据居所 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.2 L252:a | 算法 GID 地址、桶裂不迁、历史只追加 | 现行 | excluded | 源地址代数与容量制度 | 目标文件成员显式白名单 |
| §4.2 L253:a | 单 PR 全迁、不许 legacy/双读/grandfather 的绝对禁令 | 现行 | excluded | 与目标安全迁移和宿主定制合同冲突 | 保留声明兼容，完成后收尾见 owner.migration |
| §4.2 L253:b | 当前唯一权威、历史归 Git、过程不留 | 现行 | duplicate | `owner.migration`、`artifact.no-diary` | 不保存第二政策 |
| §4.2 L254:a | 类型与逻辑在程序、声明实例在外、测试 fixture 非 canonical | 现行 | duplicate | `owner.program-data` | 固定 tools/Blueprint 等路径排除 |
| §4.2 L254:b | 类型内封闭词表可写程序，loader 校验 schema | 现行 | duplicate | `owner.program-data`、`reference.resolve` | 不制造重复锚定测试 |
| §4.2 L254:c | 重要内容不因此进入保护面，golden 目录由 strict loader | 现行 | generalized | `owner.program-data`、`judge.register` | 固定目录/SL 名排除 |
| §4.2 L255:a | 旧目录、保守重放/C0/证书退役；SL-008 当前态 | INACTIVE 的现行说明；不执行旧机制 | excluded | 源停用机制与历史执法归属 | 停用／历史样本，不复建 |
| §4.2 L256:a | 硬路径检查与软兼容评审分开 | 现行 | duplicate | `judge.strength` | 不移植 blanket 禁兼容 |
| §4.2 L257–259:a | 4.3 依赖骨骼与叙事真源 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.3 L260:a | D5→judge/Reg 债务只缩不换，清零全树 | 现行 | generalized | `change.debt` | 保留集合约束；排除自动全量裁决 |
| §4.3 L261:a | 依赖无环、叙事不能反定结构 | 现行 | duplicate | `reuse.shared-definition`、`owner.canonical` | 目标允许正常文档互引 |
| §4.3 L261:b | Scribe AST/GID 构造期解析与生成编号 | 现行 | excluded | 源叙事生产管线 | 目标复用现有 Rust generator |
| §4.3 L262:a | 新增前核已有类型函数定理及接口条件，直接复用 | 现行 | duplicate | `reuse.search` | 去数学制度 |
| §4.3 L263:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.3 L264:a | private 挡复用时改原定义及必要类型，不复制／包装 | 现行 | duplicate | `reuse.shared-definition` | 保持目标语言中实际接口 |
| §4.3 L265:a | 共同依赖抽取，原方同用，不反向／循环 | 现行 | duplicate | `reuse.shared-definition` | 去源架构名 |
| §4.3 L266:a | 依赖在活路径上实际使用，未用 import／别名不算 | 现行 | duplicate | `reuse.actual-consumers` | 不引公开化新颖性义务 |
| §4.3 L267:a | 公开化保持原语义假设并验证消费者 | 现行 | duplicate | `reuse.actual-consumers` | 冻结身份/writer 专项排除 |
| §4.3 L268:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.3 L269:a | 来源编号不反绑程序、引用仅 provenance | 现行 | generalized | `owner.canonical`、`artifact.substance` | 目标可选源行定位不成 runtime 权威 |
| §4.3 L269:b | 形式 GID 唯一真值且 Scribe 自动编号 | 现行 | excluded | 研究形式化地址权威 | 不移植 |
| §4.3 L270:a | 结构工具不能保证活路径与复用真实性 | 现行 | duplicate | `judge.strength`、`tool.semantic` | 守护限制 |
| §4.3 L271–273:a | 4.4 真源、生成程序与投影判据 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.4 L274:a | 治理真源与 producer，不重复把投影当政策 | 现行 | generalized | `projection.consumption` | 输出所有权和消费者仍验证 |
| §4.4 L275:a | 四项 AND：受管 producer；完整受管且保留输入；逐字节重建；无独立 policy/oracle/history | 现行 | duplicate | `projection.criteria` | 全部保留，不因重建成功省条件 |
| §4.4 L275:b | 未知／外部闭包不能假定纯投影 | 现行 | duplicate | `projection.criteria` | 明确登记实际输入与边界 |
| §4.4 L275:c | 重算只验第三项，不证明闭包或无权威 | 现行 | duplicate | `projection.criteria` | 严格保留限制 |
| §4.4 L275:d | ledger/certificate/source 可有独立权威需治理 | 现行 | duplicate | `projection.criteria` | 不导入研究信任根实例 |
| §4.4 L275:e | 满足四项即不得保护或触保守门 | 现行 | excluded | 源禁止治理投影的绝对结论 | 目标需输出所有权、路径和消费校验 |
| §4.4 L276:a | 补偿源自误守投影时消因并移除多余机制 | 现行 | duplicate | `projection.consumption` | 不按类别盲删目标安全检查 |
| §4.4 L276:b | tracked 快照可供人读，消费者应取有效 producer 产物 | 现行 | duplicate | `projection.consumption` | 目标聚合文档为明确消费者 |
| §4.4 L276:c | 必须删除所有重算链／租约／freshness／emit-check 的清单 | 现行 | excluded | 源投影禁令的固定实现处方 | 目标真实 byte/noop/ownership 检查保留 |
| §4.4 L276:d | 消费数据与存在性断言不是内容政策守卫 | 现行 | duplicate | `projection.consumption` | 保留消费者校验 |
| §4.4 L277:a | 分类器／重建／FILEMAP 只各证其范围 | 现行 | duplicate | `projection.criteria`、`judge.strength` | 目标无通用分类器，不冒实现 |
| §4.4 L278–280:a | 4.5 投影分区与变更单元 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.5 L281:a | 聚合独立写集合制造冲突，按变更单元衡量 | 现行 | generalized | `projection.partition` | 不是强制一输入一输出 |
| §4.5 L282:a | 禁止所有 tracked 跨分区聚合，一律 stdout | 现行；§8.16 登记权威优先，目标逐文件白名单 | excluded | 与目标根指南/manifest 聚合合同冲突 | 有真实消费者可保留，量写成本 |
| §4.5 L282:b | 每片独立输入；身份绑定分区；键从稳定源键派生 | 现行；§8.16 登记权威优先，目标逐文件白名单 | generalized | `projection.partition` | 完整必要依赖不省略，成员仍显式登记 |
| §4.5 L282:c | 禁止手写成员清单、由 producer 自动发现分片 | 现行；§8.16 登记权威优先，目标逐文件白名单 | excluded | 与当前逐文件显式白名单冲突 | 可派生身份不可推断成员权威 |
| §4.5 L283:a | 独立可接受变更单元不等于文件数 | 现行 | duplicate | `projection.partition` | 保留判别依据 |
| §4.5 L284:a | 单项或跨键原子约束才许可单文件、changed paths 恒为1 | 现行 | excluded | 源严格单片形状准入 | 目标聚合合同允许；实际影响用读数评估 |
| §4.5 L285:a | 全局 attestation 使名拆实聚，须量跨片重写 | 现行 | duplicate | `projection.partition` | 不以“最小闭包”删必要输入 |
| §4.5 L286:a | 样本键数不作永久豁免，不写具体路径赦免 | 现行 | duplicate | `projection.partition` | 保留样本边界 |
| §4.5 L287:a | values.json 十四条历史违例和 D5-T0031 | 上下文／有界历史样本 | context | 源开放违例样本 | 不复制数据或判断目标违规 |
| §4.5 L288:a | 造冲突补偿前先查合并单元是否过粗 | 现行 | duplicate | `projection.partition`、`projection.consumption` | 量原因再选结构 |
| §4.5 L289:a | 分片／CRDT 等成熟锚 | 上下文／有界历史样本 | context | 方法背景 | 无新增规则 |
| §4.5 L290:a | 分区语义软；glob／目录闭包机器权威 | 现行；§8.16 登记权威优先，目标逐文件白名单 | generalized | `judge.strength` | glob 权威由 §8.16 和用户显式白名单取代 |
| §4.5 L291–293:a | 4.6 run-local 投影的归宿 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.6 L294:a | 声明 run-local 产物不入 Git，分片仅减冲突 | 现行 | duplicate | `artifact.temporary` | 不导入 Generated 目录与具名 checker |
| §4.6 L295–297:a | 4.7 生产链、冻结与消化状态 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L298–299:a | 单向产出链、来源／PR 面／状态分别说明 | 现行 | generalized | `state.domain-workflow`、`registry.explicit` | 纯理论与消化从简例外仍仅源适用 |
| §4.7 L300:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L301–320:a | ingest→Lean→Scribe→deposit→cover 生产图 | 现行 | excluded | 源数学／消化工作流 | 无目标同构，图中一般状态义务另保留 |
| §4.7 L321:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L322:a | GID 三处路径同形，错位是地址错 | 现行 | excluded | 源数学路径代数 | 目标已有显式输出计划 |
| §4.7 L323:a | 纯理论正文 PR 不进入消化、不报终态 | 现行 | excluded | 源理论 PR 特例 | 一般未执行不报完成已保留 |
| §4.7 L324:a | deposit/cover 可顺序同 PR 同树 | 现行 | excluded | 源交付组合许可 | 不覆盖本次 caller 生命周期 |
| §4.7 L325:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L326–328:a | deposit/cover/ingest 三类封闭改动面 | 现行 | excluded | 源专用 writer 产物契约 | 不移植目录清单 |
| §4.7 L329:a | 两 PR 律和 formalization 收据退役，不复建 | INACTIVE 的现行说明；不执行旧机制 | excluded | 源停用工作流 | 停用 |
| §4.7 L330:a | coverage_gids 对象 schema、loader/writer/align/未解析 Open | 现行 | excluded | 源消化引用 schema | 不引第二账本或 runtime |
| §4.7 L330:b | 旧三步迁移已完成不是当前兼容流程 | 现行 | generalized | `owner.migration`、`judge.strength` | 目标仍保留自身显式旧宿主合同 |
| §4.7 L331:a | 领域／工作流正交状态禁止相互冒充 | 现行 | duplicate | `state.domain-workflow` | 不导入两源状态机 |
| §4.7 L332:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L333–334:a | 冻结成员 state 判据、Observe pin、旧事件字段 | 现行 | excluded | 数学冻结当前／历史合同 | 旧事件为历史，不替当前状态 |
| §4.7 L335:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.7 L336:a | 四种消化态、settle writer、邻接检查、clear及链闭合 | 现行 | excluded | 源消化状态机完整合同 | 不移植 atom 路径/分母/收据 |
| §4.7 L337:a | bind-only 用覆盖或 settle 终结、无新增冻结 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 数学消化结算 | §1.2 低投入例外优先 |
| §4.7 L338:a | 文献已发表无 GID 保留 open，quarantine 和失败回执非终态 | 现行；§1.2 消化从简、§3.2 外部问题例外适用 | excluded | 源 GID/消化专门状态与例外 | 一般失败回执不是完成已保留 |
| §4.7 L339:a | 冻结和消化不同构，不能混报 | 现行 | duplicate | `state.domain-workflow` | 具体单向蕴含属于研究制度 |
| §4.7 L340:a | 含研究工件 PR 三项形态／链位置／落地状态 | 现行 | excluded | 源研究交付报告格式 | 可定位当前变更的一般义务见 candidate.landing |
| §4.7 L340:b | 判据看真实定位，不是有没有出现术语 | 现行 | generalized | `evidence.scope`、`tool.semantic` | 不造关键词测试 |
| §4.7 L341:a | 生成 Markdown 改源再生成 | 现行 | duplicate | `projection.consumption` | 目标实际 Rust 生成器 |
| §4.7 L341:b | CAS 只能追加／冻结源码改后弃分支 deposit | 现行 | excluded | 源数学不可变物与修复处方 | 目标保护用户已有工作 |
| §4.7 L342:a | 并行不需领域分配，开工／PR 前查重复证明 | 现行 | generalized | `reuse.search`、`collaboration.combine` | 不导入固定频次或 GID 算法 |
| §4.7 L343:a | 产者唯一与血缘、工作流非领域状态 | 现行 | duplicate | `owner.canonical`、`state.domain-workflow` | 成熟锚中的实际义务 |
| §4.7 L344:a | 源实际硬门／未实现 changed-path／可变 PR 说明分列 | 现行 | duplicate | `judge.strength`、`evidence.mutable-snapshot` | 不称正文存在性硬保证 |
| §4.7 L345–347:a | 4.8 生长、抽象与容量压力 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.8 L348:a | 结构随实际压力生长 | 现行 | duplicate | `structure.pressure` | 不引自相似数学本体 |
| §4.8 L349:a | 第二实例或实际压力才抽象，首工件才建目录 | 现行 | duplicate | `structure.pressure` | 去数学模块与桶配额 |
| §4.8 L350:a | 特定数据扩展名豁免 SL-003 行数，目录容量不豁免 | 现行 | excluded | 源行数／容量配额政策 | 目标不引任意字数或行数限额 |
| §4.8 L351–352:a | 不把局部事实普遍化，容量硬与不预建软分列 | 现行 | duplicate | `evidence.scope`、`judge.strength` | 去源阈值 |
| §4.8 L353–355:a | 4.9 商余结构与代表元 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §4.9 L356:a | harness 存共性，改实例不该改运行时 | 现行；§8.16 登记权威优先，目标逐文件白名单 | duplicate | `structure.pressure`、`owner.program-data` | 实例明确登记而非自动扫描 |
| §4.9 L356:b | 成员用目录闭包与派生 digest，禁止地址清单 | 现行；§8.16 登记权威优先，目标逐文件白名单 | excluded | 与用户显式逐文件白名单冲突 | §8.16 优先，目标绝不推断依赖 |
| §4.9 L356:c | 旧手列 TOWER/C0 inactive，无共性不造 harness | 现行；§8.16 登记权威优先，目标逐文件白名单 | generalized | `structure.pressure` | 仅迁移无共性不造机器；旧系统不复建 |
| §4.9 L357:a | schema 形状不证明成员规则适切 | 现行；§8.16 登记权威优先，目标逐文件白名单 | duplicate | `judge.strength` | 不复制 glob 构建权威 |
| §4.9 L358–362:a | 5. 协作、产地、独立性与自主推进；5.1 通信即工件 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.1 L363:a | 通信必须全部经仓内工件、未记录视为未发生 | 现行 | excluded | 源通信渠道与记录绝对政策 | 当前仅授权外部消息才可发；不造第二记录义务 |
| §5.1 L363:b | 协调工件只留结果，不留过程 | 现行 | duplicate | `artifact.no-diary` | §2.10 控制 |
| §5.1 L364:a | 源 H9 与库外信道禁令守护 | 现行 | excluded | 源通信门配置 | 无目标渠道强制 |
| §5.1 L365–367:a | 5.2 工件产地与独立性披露 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.2 L368:a | 实际生产者、技能与混合方式须披露，独立性不凭空 | 现行 | generalized | `review.disclosure` | 不复制 PR/issue 顶部必填格式 |
| §5.2 L369:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.2 L370:a | 实际使用 skill 名或无 skill | 现行 | generalized | `review.disclosure` | 适用方法／评审来源说明中列实际 skill／方法或未用；无固定 PR 标头、提供方格式或自动技能调用 |
| §5.2 L371:a | 承重生产者、实现／评审身份和同族关系 | 现行 | duplicate | `review.disclosure` | 去固定席布局 |
| §5.2 L372:a | 并发盲评／串行、裁决、亲验与自报区分 | 现行 | duplicate | `review.disclosure`、`collaboration.combine` | 票数仅事实不作证明，不留对话 |
| §5.2 L373:a | footer 不抵产地，未亲验标转述；失败回退缺席披露 | 现行 | duplicate | `review.disclosure` | 单点合法，不能假多样性 |
| §5.2 L374:a | 强制宿主会话 ID/恢复命令及追加规则 | 现行 | excluded | 源宿主追踪格式 | 不读 rollout 或导入全局配置 |
| §5.2 L374:b | 恢复指针非内容，正文须自足 | 现行 | duplicate | `artifact.no-diary` | 无强制指针字段 |
| §5.2 L375:a | 可编辑正文无硬保证，绑定不可变快照才可能加强 | 现行 | duplicate | `evidence.mutable-snapshot`、`judge.strength` | 不造新 reviewer/CI 门 |
| §5.2 L375:b | 旧存在性硬投影承诺撤销与不建机制 | 现行 | excluded | 源停用产地门承诺 | 停用 |
| §5.2 L376–378:a | 5.3 issue 留痕的判决节点 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.3 L379:a | 接手更新当前方向、推翻读数优先纠正、根因处置落地更新 | 现行 | duplicate | `collaboration.current-result` | 条件为已授权维护表面 |
| §5.3 L379:b | 在不可逆研究物前更新及结案 MERGED/SHA | 现行 | generalized | `candidate.landing` | 研究对象清单排除；不自行授权发布 |
| §5.3 L380:a | 只发新读数／判断，可复核且不等回复当请示 | 现行 | generalized | `collaboration.current-result`、`action.autonomy` | 不无条件发外部消息 |
| §5.3 L381:a | 可变评论无 check-time 保证、义务仍在 | 现行 | duplicate | `judge.strength` | 守护边界 |
| §5.3 L382–384:a | 5.4 遇题即解与实施让渡 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.4 L385:a | 立案后推进修复，issue/等回应不算完成 | 现行 | duplicate | `action.blocked` | 纯消化例外按源 §1.2 排除 |
| §5.4 L385:b | 所属 session worktree、sshx 与 PR 落地流程 | 现行 | excluded | 源固定实施渠道 | 本次严格按 caller dispatch |
| §5.4 L386:a | 让渡仅活动认领有证据或对方唯一真源，写恢复动作 | 现行 | duplicate | `collaboration.handoff` | 两分支 OR 保留；第二分支不要求对方在飞 |
| §5.4 L386:b | 停滞收回、不预设等待窗 | 现行 | duplicate | `collaboration.handoff` | 限授权内，不侵犯唯一所有权 |
| §5.4 L387:a | 让渡／推进属纪律，工件存在不等真实推进 | 现行 | duplicate | `judge.strength`、`collaboration.handoff` | 去硬投影格式宣称 |
| §5.4 L388–390:a | 5.5 目标阻塞与最小 hotfix | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.5 L391:a | 目标阻塞及时修最小可达根因，不以归属/等待搪塞 | 现行 | generalized | `action.blocked`、`recovery.validate` | 按当前授权与唯一所有权；不扩语义权 |
| §5.5 L392:a | 四边界的 session/dev/三门/两PR/τ=0限定 | 现行 | excluded | 源 hotfix 特殊流程与权限制度 | 不导入固定数量／目录／人类门 |
| §5.5 L392:b | 修复仍不抬预算／降检测，结果与读数保留 | 现行 | duplicate | `cost.measure`、`artifact.results` | 不留过程 |
| §5.5 L393:a | 本 lane 绕开不等共享阻塞已修 | 现行 | duplicate | `collaboration.handoff`、`recovery.validate` | 不把工作票当修复 |
| §5.5 L394–396:a | 5.6 利益回避与旗判分离 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.6 L397:a | 实现与评审独立、提问者不自证 | 现行 | generalized | `review.independent`、`review.disclosure` | 可用独立证据，不造必设席位 |
| §5.6 L398:a | sshx 实施／评审分离、异 bias 隔离属质量层，准入不依赖席位 | 现行；L397 分工纪律与 §7.4 机器准入分列 | excluded | 源 sshx 席位与准入层配置 | 当前质量纪律不是 inactive；一般独立证据与产地义务由 L397:a 迁移，不导入固定布局或研究准入 |
| §5.6 L398:b | 旧禁自并与人审 H7 已停用 | INACTIVE；L398 明示旧规则停用 | excluded | 已退休的人审／自并禁令 | 不恢复人类门，也不将当前实现／评审分离误标为停用 |
| §5.6 L399–401:a | 5.7 单点错误与独立核验 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.7 L402:a | 单点再聪明也会错，需要实际与独立证据 | 现行 | duplicate | `review.independent` | 不称独立收敛必正确 |
| §5.7 L403:a | 三席异模型由 sshx 编排 | 现行 | excluded | 源提供方与席位配额 | 不影响本 dispatch |
| §5.7 L404–406:a | 5.8 模型多样性与诚实声明 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.8 L407:a | 同族共享盲点，可用不同视角就核，单点如实说 | 现行 | duplicate | `review.disclosure`、`review.independent` | 多样性不等保证 |
| §5.8 L408:a | 能力受限披露实际布局 | 现行 | duplicate | `review.disclosure` | 不引固定混排 |
| §5.8 L409–411:a | 5.9 自主推进与四态归位 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.9 L412:a | 授权可逆工作自主推进不逐步确认 | 现行 | duplicate | `action.autonomy` | 禁止工具弹窗的源绝对规则不覆盖上层约束 |
| §5.9 L413:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.9 L414:a | 机器可判者实际判断再推进 | 现行 | duplicate | `judge.register`、`evidence.program-state` | 不声称所有判断已有机器 |
| §5.9 L415:a | 形式不可判为 Gödel open | 现行 | excluded | 源逻辑不可判分类 | 普通未测不是不可判 |
| §5.9 L416:a | 能力授权资源缺口说明条件，其余工作继续 | 现行 | duplicate | `action.blocked` | 去超仪状态机与源 τ 语义门 |
| §5.9 L417:a | 规则／harness bug 自主修复 | 现行 | duplicate | `action.autonomy`、`policy.evolution` | 不许以文字宣布完备 |
| §5.9 L418:a | 新用户输入更新，完成汇报非再次请示 | 现行 | duplicate | `goal.revision`、`action.autonomy` | 无全局禁提问扩展 |
| §5.9 L419–421:a | 5.10 失败战史与重开工单 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.10 L422:a | 先查有效失败结论、留回归，不重走死路 | 现行 | duplicate | `reuse.search`、`artifact.results` | 持久性限真实消费者，不冒 SL-013 |
| §5.10 L423:a | 所有死工单关闭重开新号、永不复活 | 现行 | excluded | 源工单不可变生命周期绝对规则 | 适配为 state.recovery-context：先核状态，有效可恢复 |
| §5.10 L423:b | 陈旧标签／重试耗尽／过期上下文不能盲继承 | 现行 | duplicate | `state.recovery-context` | 保留有用结果，不留过程 |
| §5.10 L424:a | deferred 检查不守散文，恢复纪律需实际核验 | 现行 | duplicate | `judge.strength` | 源 SL 不导入 |
| §5.10 L425–427:a | 5.11 sshx 载体、公开证据与运行契约 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.11 L428:a | sshx 指定插件／载体部署 | 现行 | excluded | 源工作流工具配置 | 源为数据，不覆盖本次 worker |
| §5.11 L429:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §5.11 L430:a | 覆盖 skill pass_budget、目标无限轮、审计/MERGED硬要求 | 现行 | excluded | 源 sshx 调度覆盖指令 | 本 dispatch 与 caller 所有权优先 |
| §5.11 L430:b | 仍可推进不原样空转，按证据修缺口、无进展换法 | 现行 | duplicate | `action.blocked`、`reason.representation` | 不导入无限资源或轮数要求 |
| §5.11 L431:a | 禁 subagent、特定回退布局与 abstain 规则 | 现行 | excluded | 源提供方／seat 配置 | 不是目标默认政策 |
| §5.11 L432:a | tests 只能 codex-cli，oracle 无树不能自报跑过 | 现行 | generalized | `evidence.measure`、`review.disclosure` | 一般实际执行边界；排除指定载体 |
| §5.11 L433:a | brief 固定 GitHub URL、必须先测 public 可达性 | 现行 | excluded | 源仓地址与公开信息配置 | 不迁移固定网址；能力边界见 action.blocked |
| §5.11 L434:a | 公开载体看不到未发布本地状态，不能假装亲验 | 现行 | duplicate | `review.disclosure`、`evidence.scope` | 源“能推先推”不是本 worker 权限 |
| §5.11 L435:a | 池名先 list 与具体 slug/403 样本 | 现行 | excluded | 源服务配置与诊断数据 | 通用工具输入校验已保留 |
| §5.11 L436:a | prompt 文件 stdin 避免 shell 改美元号／引号／换行 | 现行 | generalized | `transport.literal` | 不强制某 CLI 参数形式 |
| §5.11 L437:a | flight 可临时变异，固定快照或前后哈希核读数 | 现行 | duplicate | `evidence.mutable-snapshot` | 临时快照不归档 |
| §5.11 L438–442:a | 6. 工作树、PR 分层、债务收缩与完成；6.1 独立 worktree 与 MERGED 完成态 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §6.1 L443:a | 独立 worktree 保护主干，代码文档都隔离 | 现行 | generalized | `candidate.isolation` | 当前 caller 指定 integration 可用；不改主检出 |
| §6.1 L443:b | 固定 make/session/每任务 PR/机器保证 | 现行 | excluded | 源 Git 流程配置 | caller 负责 Git |
| §6.1 L444:a | 逻辑单元固定提交锚，未提交／stash 可能失配 | 现行 | generalized | `candidate.isolation`、`evidence.mutable-snapshot` | 不授权 worker commit/push |
| §6.1 L444:b | 具体工具 ReadCurrent/deposit/cover 无自动提交 | 现行 | excluded | 源工具生命周期事实 | 无目标 runtime 对应 |
| §6.1 L445:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §6.1 L446:a | 按完整 session ID/固定同级路径创建 worktree | 现行 | excluded | 源宿主目录命名和 make 参数 | 不移植个人宿主策略 |
| §6.1 L447:a | 同 session 强制复用树，切分支前提交干净 | 现行 | excluded | 源 session 工作树复用制度 | 保留已有工作的通用义务 candidate.isolation |
| §6.1 L448:a | 目录 session 与 branch grammar 分离 | 现行 | excluded | 源分支/目录命名合同 | 不导入固定 grammar |
| §6.1 L449:a | 主检出只 dev，开工/合后/派席前 pull | 现行 | excluded | 源 Git 操作及频次 | 本 caller 已固定基线 |
| §6.1 L450:a | 24小时且落后300提交强制回收，不顾未提交/占用 | 现行 | excluded | 源破坏性清理策略及配额 | 与保护已有工作冲突 |
| §6.1 L450:b | 删除前重验目标身份、锁和适用清理条件 | 现行 | generalized | `candidate.isolation` | 仅授权删除前立即复核声明的可删除条件；不引 24 小时／300 提交配额或无条件破坏性回收 |
| §6.1 L451:a | push/pr-open/三门/auto-merge/同步主检出链 | 现行 | excluded | 源交付命令与门数 | caller 管 Git |
| §6.1 L451:b | 开PR/绿/CLOSED不等MERGED，核dev实际状态 | 现行 | duplicate | `candidate.landing` | 按实际交付合同核落地 |
| §6.1 L452:a | 完成须实际落地身份，纪律不冒机器保证 | 现行 | duplicate | `candidate.landing`、`judge.strength` | 源地址算法和检查名排除 |
| §6.1 L453–455:a | 6.2 PR 层序、delta 门与债务收缩 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §6.2 L456:a | 一单职责、可独立落地的小变更控制漂移 | 现行 | generalized | `change.coherent` | 不设一层一 PR 绝对配方 |
| §6.2 L457:a | 先约束新增、再补存量 | 现行 | duplicate | `change.debt` | 条件为选择渐进迁移 |
| §6.2 L457:b | 债清零后同门自动判全树 | 现行 | excluded | 与当前 DELTA-only 原则冲突 | 不改为全量回退 |
| §6.2 L458:a | delta 定义域不等 grandfather 运行时兼容 | 现行 | generalized | `change.debt`、`owner.migration` | 目标合法兼容保留 |
| §6.2 L459:a | 债务 AND：禁新身份、集合子集、触债严格减、守收缩机制 | 现行 | generalized | `change.debt` | 适用已声明的债务收缩／迁移合同时保留①②③⑤；自动全树第四项明确排除 |
| §6.2 L459:b | 计数不增不能防换债，blob ratchet不证全条件，收缩不保证期限 | 现行 | duplicate | `change.debt` | 不把局部证据当全部保证 |
| §6.2 L460:a | 旧删机制判兼容与计数判据 inactive | INACTIVE 的现行说明；不执行旧机制 | excluded | 源停用债务分类判据 | 停用 |
| §6.2 L461:a | PR规模对齐p75和历史阈值；超出须说明 | 现行 | excluded | 源 PR 数量／分布配额 | 真实冲突与判词分区替代 |
| §6.2 L461:b | 一句话职责、独立验收、不依后续，不留永久双轨 | 现行 | duplicate | `change.coherent` | 去三门数量 |
| §6.2 L461:c | 落地前试合最新基线，已退休路径不要搬回 | 现行 | generalized | `candidate.refresh`、`change.coherent` | caller 生命周期，不强制特定 git 命令 |
| §6.2 L462:a | 规模说明可编辑不升硬，真实独立职责要评审 | 现行 | duplicate | `judge.strength`、`change.coherent` | 源 p75门排除 |
| §6.2 L463–465:a | 6.3 冲突面、判词分区与拆分成本 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §6.3 L466:a | 不拆要同时核冲突与整批判词，阈值只是代理 | 现行 | duplicate | `change.partition` | 保留双维，不绝对宣称任何非零风险必须拆 |
| §6.3 L467:a | 零冲突五项 AND：仅新增；不交路径/稳定键；无聚合；单producer确定；无后续依赖 | 现行 | duplicate | `change.negligible-conflict` | 五项完整，不能单由文件多寡决定 |
| §6.3 L467:b | 五项只消文件数理由，不免不同判词分区 | 现行 | duplicate | `change.partition`、`change.negligible-conflict` | 不宣称机器零冲突规则已实现 |
| §6.3 L468:a | 先量判词分布，再按差异及依赖闭包定边界 | 现行 | duplicate | `change.partition` | 图来自显式登记，不造求解器 |
| §6.3 L469:a | 拆分成本并列，重复正文不证明全批同判或零收益 | 现行 | duplicate | `change.partition`、`cost.measure` | 固定CI轮数/session数量只是源例子 |
| §6.3 L469:b | 无按文件数机器红就给真实理由，不虚报机器要求 | 现行 | duplicate | `change.partition`、`judge.strength` | 保留因果依据 |
| §6.3 L470:a | 37模块/74文件/6 PR/7可合的有界历史反例 | 上下文／有界历史样本 | context | 源分区历史读数 | 不迁移具体数字为当前事实 |
| §6.3 L470:b | 缓存成本未测，混合判词有收益但不验证盲分区 | 现行 | duplicate | `cost.measure`、`change.partition` | 有界证据保留 |
| §6.3 L471:a | 唤醒依赖被阻动作会漏输入，补实际输入域 | 现行 | generalized | `registry.explicit`、`test.selection` | 不新增自动发现或全仓检查 |
| §6.3 L471:b | 不同计数口径不能混，结构保证不证语义 | 现行 | duplicate | `signal.meaning`、`judge.strength` | 旧源 wakeup open 已修状态不移植 |
| §6.3 L472:a | 引用解析用实际 loader，不凭文件名扫描推断闭合 | 现行 | duplicate | `reference.resolve`、`tool.semantic` | 去 FrozenNodeId 源格式 |
| §6.3 L472:b | 硬配对不等软依赖成组，软硬不能互冒 | 现行 | duplicate | `judge.strength` | 一般守护边界 |
| §6.3 L473:a | 零冲突尚无规则，全批同判须实际读数 | 现行 | duplicate | `change.negligible-conflict`、`judge.strength` | 不将本表映射变机器证明 |
| §6.3 L474–476:a | 6.4 运行中的不变量先立门后补账 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §6.4 L477:a | 运动中先约束新增再清既有，免不断添债 | 现行 | duplicate | `change.debt` | 不全树即时红，不豁免明确受影响旧项 |
| §6.4 L478:a | 新增域可测但实施次序仍需判断 | 现行 | duplicate | `judge.strength` | 不声称已实施通用门 |
| §6.4 L479–483:a | 7. 治理、准入、成本与因果恢复；7.1 无外部特权与机器保证的 open | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.1 L484:a | 身份不使人/AI有绕检查特权 | 现行 | generalized | `action.autonomy`、`judge.register` | 信任非恶意且会错，不引零信任威胁模型 |
| §7.1 L484–485:a | enforce_admins/rulesets读数与管理员可修改保护 | 现行 | excluded | 源平台权限配置与样本 | 不建设目标权限平台 |
| §7.1 L485:a | 配置开关不自动提供实际机器保证 | 现行 | duplicate | `judge.strength` | 不引人类审批或组织级 gate |
| §7.1 L485:b | 自修仍付 τ/W/D/E 成本不可廉价特权绕过 | 现行 | excluded | 源信任塔成本制度 | 一般修复验证与成本已保留 |
| §7.1 L486–488:a | 7.2 真值机器、自建与保守扩展 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.2 L489:a | 形式对错机器判、单调不翻旧真、内化塔无限 | 现行 | excluded | 数学保守扩展与 Gödel 治理 | 用户允许自主改政策与裁决 |
| §7.2 L490:a | 停用重放/C0/证书不冒现役，成本机制未实现 | 现行 | duplicate | `judge.strength` | 不重建机器 |
| §7.2 L491–493:a | 7.3 分类与规范先于实例 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.3 L494:a | 新类先明确规范处理方式再处理实例 | 现行 | duplicate | `judge.register` | 当前授权自主登记，不新增评审/元门许可 |
| §7.3 L495:a | 未定义须指明缺口，不临时猜值 | 现行 | generalized | `action.blocked`、`registry.explicit` | 不强制“未实例化”源状态名 |
| §7.3 L496–498:a | 7.4 零信任准入与三 required check | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.4 L499–500:a | 所有提交机械准入、身份无关、评审只是质量 | 现行 | generalized | `judge.register`、`review.independent` | 用户可信非恶意模型优先，不声明现有机械准入 |
| §7.4 L501:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.4 L502:a | dev/main/releasePR/tag E<n> 分支发布体系 | 现行 | excluded | 源 Git 发布策略 | caller 交付合同独立 |
| §7.4 L503:a | creation/lifecycle namespace及固定 WorktreeCommand词表 | 现行 | excluded | 源工作树创建和清理实现合同 | 不构建新的分支平台 |
| §7.4 L503:b | 新建格式不能意外缩窄已有受管生命周期 | 现行 | generalized | `owner.migration` | 声明迁移范围，保留宿主定制 |
| §7.4 L504:a | 三 required checks、push 两门、PR merge-ref 缓存权限 | 现行 | excluded | 源 CI/检查数量与执行架构 | 目标通用 CI 仍未实现 |
| §7.4 L504:b | 基线数据不执行，候选判官及实际合并并集证据 | 现行 | duplicate | `delta.candidate-judge`、`candidate.landing` | 有效输入限制保留 |
| §7.4 L504:c | 文档名称不证明已部署，红修根因 | 现行 | duplicate | `ci.actual-events`、`action.blocked` | 不移植 auto-merge 默认 |
| §7.4 L505:a | 必要来源披露非准入，过程不留 | 现行 | duplicate | `review.disclosure`、`artifact.no-diary` | 不复制 H9 表单 |
| §7.4 L506–507:a | 零信任成熟锚与三门/admin/exit3守护 | 现行 | excluded | 源信任假设及具体门 | 目标明确现役／未实现，不继承保证 |
| §7.4 L508–510:a | 7.5 base 判官永久禁令与 SL-030 边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.5 L511:a | base判官永久禁令；判官内容不可同PR已撤 | 现行；2026-09-24 混面允许覆盖旧禁令 | generalized | `delta.candidate-judge`、`policy.mixed-warning` | 2026-09-24 新例覆盖旧隔离禁令 |
| §7.5 L512:a | 判候选执行任何非候选修订代码为base判官，读base数据允许 | 现行 | duplicate | `delta.candidate-judge` | 目标只承诺此判断角色，不禁合法读源数据 |
| §7.5 L513:a | 禁止工作流/脚本复建base执行地板；候选代码路径约束 | 现行；2026-09-24 混面允许覆盖旧禁令 | generalized | `delta.candidate-judge` | 源 wrapper 路径实现排除 |
| §7.5 L513:b | 候选可削弱自身，标注评审后检不构独立准入保证 | 现行；2026-09-24 混面允许覆盖旧禁令 | duplicate | `policy.mixed-warning`、`judge.strength` | 可信会错模型，不导入恶意比喻 |
| §7.5 L514:a | 允许混面，显示旧新影响警告但普通验证仍执行 | 现行；2026-09-24 混面允许覆盖旧禁令 | duplicate | `policy.mixed-warning` | 去 SL-029 名；不新增人工许可 |
| §7.5 L514:b | 缺失/错误端点登记仍失败，警告不降低验证 | 现行；2026-09-24 混面允许覆盖旧禁令 | duplicate | `registry.explicit`、`policy.mixed-warning` | 实际后端未实现须明说 |
| §7.5 L515:a | 旧base防篡改/纯revert分类器等停用 | INACTIVE 的现行说明；不执行旧机制 | excluded | 源已退役机器与事故细节 | 停用 |
| §7.5 L515:b | 字面 HEAD^1 搜索漏变量，须按实际代码/数据用途判断 | 现行 | duplicate | `tool.semantic`、`delta.candidate-judge` | 不造新 shell 扫描器 |
| §7.5 L516:a | hermetic CI/单写者/Andon成熟锚 | 上下文／有界历史样本 | context | 方法背景 | 一般义务已在相应条款映射 |
| §7.5 L517:a | SL-030扫描语法、Git选项、YamlDotNet支持/不支持完整清单 | 现行 | excluded | 源扫描器能力与实现细则 | 不移植语法平台或测试框架 |
| §7.5 L517:b | 四个历史形态红只证样本，面外/跳过/被改弱仍可能漏 | 现行 | duplicate | `judge.counterexamples`、`judge.strength` | 保留两维与有界早反馈 |
| §7.5 L517:c | 消费workflow的生产逻辑夹具与workflow本身不同 | 现行 | generalized | `test.behavior`、`ci.actual-events` | 源 blanket 测试禁令不移植 |
| §7.5 L517:d | 实际执行暴露自锁，尚未lint不豁免义务 | 现行 | duplicate | `ci.actual-events`、`judge.strength` | 不把文字要求当执法 |
| §7.5 L518–520:a | 7.6 退出语义与 bootstrap 脚手架 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.6 L521:a | 源 baseline 0/1/2/3 与强制 Lean 地板/退役组件C | INACTIVE 的现行说明；不执行旧机制 | excluded | 源退出码与bootstrap脚手架 | 目标现有退出合同不变 |
| §7.6 L522–524:a | 7.7 strict 禁令与并集保证 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.7 L525:a | strict 永久禁令及历史吞吐统计 | 现行 | excluded | 源 Git branch protection 政策 | 用户最新 fresh dev 原则优先 |
| §7.7 L526:a | 被验树与落地树需对应，M1/M2可异不可冒同树 | 现行 | duplicate | `candidate.landing`、`ci.actual-events` | 源 PR/push M/B 名称只是案例 |
| §7.7 L527:a | 预留一格／全仓容量检测／分片／push恢复四处方 | 现行 | excluded | 源容量并集与全仓检测配置 | 保留一般落地校验和实际影响，不引全仓兜底 |
| §7.7 L528:a | 提高并集保证需批量>1合并执行者与merge_group核验 | 现行 | excluded | 源合并执行器具体设计 | 一般真实事件核验由 ci.actual-events |
| §7.7 L529–531:a | 7.8 可逆性、风险与检测红线 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.8 L532:a | 按可逆性与风险区分控制，异常可检出可修 | 现行 | duplicate | `policy.proportional` | 不复制固定四态与申诉系统 |
| §7.8 L533:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.8 L534:a | 不可逆/信任根/检测本身硬门清单 | 现行 | generalized | `policy.proportional`、`judge.register` | 宿主按实际约束登记；无新增人类门 |
| §7.8 L535:a | 可逆低风险允许错而须检测／快速修复 | 现行 | duplicate | `policy.proportional`、`recovery.validate` | 无强制案号/canary平台 |
| §7.8 L536:a | 不降检测掩盖失败，有限防御与可观测恢复 | 现行 | duplicate | `policy.proportional` | 不把错误无限穷举变新平台 |
| §7.8 L537:a | 源等级守护不是全覆盖保证 | 现行 | duplicate | `judge.strength` | 守护限制 |
| §7.8 L538–540:a | 7.9 错误驱动的制度生成 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.9 L541:a | 真实错误检出保留结果、按需定义规则并验证 | 现行 | generalized | `policy.proportional`、`judge.register`、`artifact.results` | 不每个错误强造新门 |
| §7.9 L541:b | 先delta约束新增再补账，清零全树 | 现行 | generalized | `change.debt` | 自动全量结论排除 |
| §7.9 L542:a | 同类第二次修器，不做逐实例过窄或过度门控 | 现行 | duplicate | `core.repair-producer`、`policy.proportional` | 同类须 §7.11 症状 AND 处置，非所有错误自动同类 |
| §7.9 L543–545:a | 7.10 有案防御与成本边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.10 L546:a | 防御须有实际缺陷/有据案例且承担自身成本 | 现行 | duplicate | `policy.proportional`、`cost.measure` | 当前用户信任非恶意前提 |
| §7.10 L546:b | 没出过错不许立制／恶意防御须实际攻击者 | 现行 | excluded | 源绝对事故前置与威胁模型 | 不把尚无事故变禁止合理前瞻规则 |
| §7.10 L547:a | 新增防御强制案号与永久结论引用 | 现行 | excluded | 源记录格式门 | 保留事实依据与结果，不强制 issue |
| §7.10 L548–550:a | 7.11 同症状停手与根因处置 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.11 L551–552:a | 同类必须症状相同且处置相同，局部解释不消系统性 | 现行 | duplicate | `core.repair-producer` | 触发合取原样保留 |
| §7.11 L553:a | 第二次停止实例处理先答为何重现，权限内修器优先 | 现行 | duplicate | `core.repair-producer` | 停止形态不停止目标 |
| §7.11 L553:b | 真能力/所有权边界才停同形，报次数单次总量并转向 | 现行 | duplicate | `core.repair-producer`、`action.blocked` | 不导入 τ=0 等人制度 |
| §7.11 L553:c | 投入前查既有问题，不边等边撞 | 现行 | duplicate | `core.repair-producer`、`reuse.search` | 不强制外部工单消息 |
| §7.11 L554:a | 不降检测不弃目标，零产出如实；禁第三盲试 | 现行 | duplicate | `core.repair-producer` | 不以忙碌掩盖无进展 |
| §7.11 L555:a | 同类判定是软，读数支持例外原因非过程流水 | 现行 | duplicate | `judge.strength`、`artifact.no-diary` | 源第三轮 PR 格式不移植 |
| §7.11 L556–558:a | 7.12 信任地层与成本递归塔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.12 L559:a | harness 可改但广影响提高验证成本 | 现行 | duplicate | `policy.evolution`、`cost.measure` | 不导入机器成本市场 |
| §7.12 L560:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.12 L561–565:a | τ分层、指数成本、W/D/E、保守扩展、bootstrap与Benefit阈值 | 现行 | excluded | 源信任塔及定理/货币制度 | 一般影响和验证成本已保留 |
| §7.12 L566:a | 数学可算不等于已有执行器，退役和open明示 | 现行 | duplicate | `judge.strength` | 不立永久工单假装完成 |
| §7.12 L567–569:a | 7.13 自主解锁与停用成本形 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.13 L570:a | 解锁付成本、有结果、错误可检测，自主修复 | 现行 | generalized | `recovery.validate`、`action.autonomy`、`policy.proportional` | 无自授admin或新账本 |
| §7.13 L571:a | base验证/证书/noop分类旧成本形均inactive | INACTIVE 的现行说明；不执行旧机制 | excluded | 源停用恢复机器 | 停用 |
| §7.13 L572:a | 修复保留论证结果并恢复实际检测，不存流水 | 现行 | duplicate | `recovery.validate`、`artifact.results` | 不导入 ceremony |
| §7.13 L573:a | 区分工具自锁与实际拒绝坏候选，不能以恢复名削检测 | 现行 | generalized | `recovery.causal`、`recovery.validate` | 不采用 noop/revert 单独足以归因或绕过 |
| §7.13 L574–576:a | 7.14 现役因果撤因与四条件恢复 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.14 L577:a | exact inverse 可admin合且不等自身CI临时权限 | TEMPORARY；§7.14 当前源特权，目标排除 | excluded | 源特定当前恢复特权 | 不导入admin/跳检查许可；源状态TEMPORARY |
| §7.14 L578:a | 四项AND：可逆且不触冻结、具名红与merge、首push验证、仍红不二绕 | TEMPORARY；§7.14 当前源特权，目标排除 | excluded | 临时admin通道资格条件 | 整体特权不移植；实际后验证另保留 |
| §7.14 L579:a | revert 保后续提交，落地不必等历史整树 | 现行 | duplicate | `recovery.validate`、`candidate.landing` | 不指令 worker 执行 Git |
| §7.14 L580:a | 固定last-green/first-red/parent/具名红，edge只候选 | 现行 | duplicate | `recovery.causal` | 保留时序非因果限制 |
| §7.14 L580:b | 固定其它输入做反事实，核BaseSnapshot错位，不猜撤 | 现行 | duplicate | `recovery.causal` | 归因未闭合显式未验 |
| §7.14 L581:a | 归因后及时恢复，merge不是恢复；实际落地检查仍须绿 | 现行 | duplicate | `recovery.validate`、`candidate.landing` | 源admin/冻结撤销路径排除 |
| §7.14 L582:a | 撤因修器必须两PR、改投integration-theory | 现行 | excluded | 源具体Git恢复流程 | 一般局部绕开不是共享恢复已保留 |
| §7.14 L583:a | #4198真revert仍失败/#4305滞后基线因果反例 | 上下文／有界历史样本 | context | 源恢复案例数据 | 不把样本推广所有 open PR |
| §7.14 L583:b | 单一dev红不推出全部openPR红 | 现行 | duplicate | `evidence.scope` | 保留范围界限 |
| §7.14 L584:a | 恢复当前无机器消费者，可变正文非硬投影 | 现行 | duplicate | `judge.strength`、`evidence.mutable-snapshot` | 未来字段/自动放行平台不移植 |
| §7.14 L585–587:a | 7.15 deferred 全路径精确逆目标 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.15 L588:a | deferred 自动识别revert跳重步骤产绿合并 | DEFERRED；不覆盖 §7.14 现行临时通道 | excluded | 尚未现役恢复权限目标 | 延后；不得作为本次授权 |
| §7.15 L589:a | deferred 精确逆/first-parent merge/不触逃生舱三项资格 | DEFERRED；不覆盖 §7.14 现行临时通道 | excluded | 源未来恢复通道完整合取 | 延后；不与现行四项合成七项 |
| §7.15 L590:a | deferred 全路径含冻结逆撤且列全删条目 | DEFERRED；不覆盖 §7.14 现行临时通道 | excluded | 源未来数学冻结恢复例外 | 延后；不带入当前目标 |
| §7.15 L591:a | 现行四条件与未来三条件尚未统一，不可互替 | DEFERRED；不覆盖 §7.14 现行临时通道 | generalized | `judge.strength` | 保留区分状态，不移植任一通道 |
| §7.15 L592–594:a | 7.16 稳态全自主与无须人审铁律 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §7.16 L595:a | 稳态自主、方向open驱动、bootstrap/Gödel塔 | 现行 | excluded | 源自治数学本体 | 用户自主原则单独生效 |
| §7.16 L596:a | 完全禁止新增“需人审”门 | 现行 | duplicate | `action.autonomy` | 按用户授权，不引永久不可修订标签 |
| §7.16 L597:a | 全部门必须机器可判，人工结果一律bug | 现行 | generalized | `action.autonomy`、`judge.register`、`judge.strength` | 不声称所有软义务可机器决定 |
| §7.16 L598:a | 人仅bootstrap/不可逆物理授权两角色 | 现行 | excluded | 源人类角色与资源制度 | 不覆盖当前系统／用户授权边界 |
| §7.16 L599:a | 固定candidate/Lean/账本检查和rc=3语义 | 现行 | excluded | 源具体机器手段 | 目标 chrono-harness check=3 未实现不等内容通过 |
| §7.16 L600–601:a | 终态无人、方向选择器open、四态穷尽/超仪 | 现行 | excluded | 源自治状态／方向／权限制度 | 不导入人类价值/思想体制 |
| §7.16 L602:a | 拆脚手架先有有效替代检测，配置开关不等保证 | 现行 | generalized | `policy.evolution`、`judge.strength` | 规则可修但需适用验证，不建权限塔 |
| §7.16 L603–604:a | 理论成熟锚与四态元守护 | 上下文／有界历史样本 | context | 源制度背景及导航 | 一般自主/保证边界已逐项处理 |
| §7.16 L605–609:a | 8. 工具、宿主作业、等待与 CI 真跑；8.1 分层 make 入口与器谱 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.1 L610:a | 各层每活动唯一入口，层内委托不复制配方 | 现行 | duplicate | `method.entry` | 固定 Makefile/数学门不移植 |
| §8.1 L611:a | 已有入口就用，前置失败契约明确 | 现行 | duplicate | `method.entry`、`tool.glue` | 不复制源 make 目标清单 |
| §8.1 L611:b | PR工具自动等检查/隔离token/显式auto-merge | 现行 | excluded | 源工具专用工作流 | caller 负责 Git 和认证 |
| §8.1 L611:c | 重复操作先修可复用工具，不留scratchpad | 现行 | generalized | `tool.maintainable-owner`、`core.repair-producer` | 不机械套第三次配额或重复造平台 |
| §8.1 L611:d | 等待看原语与真实完成，不认耗时 | 现行 | duplicate | `job.wait`、`evidence.measure` | 守护能力可用性仍需实测 |
| §8.1 L612–614:a | 8.2 本地早反馈与远端 CI 并行 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.2 L615:a | 本地/CI共用检查器，模式显式且缺参前置失败 | 现行 | generalized | `method.entry`、`delta.local-ci` | 不导入源 fast/push/pr/full 固定模式 |
| §8.2 L615:b | 完整范围含多提交、暂存/未暂存/未跟踪/删除 | 现行 | generalized | `delta.range` | 目标runner当前仅不可变干净快照，不声称dirty支持 |
| §8.2 L615:c | fast/full与BASE隔离，拒继承旧事件或陈旧候选 | 现行 | generalized | `query.identity`、`delta.candidate-judge` | 目标无模式推断或隐式基线 |
| §8.2 L615:d | 相同候选范围语义环境才等价，fast/本机噪声不冒CI权威 | 现行 | duplicate | `delta.local-ci`、`evidence.scope` | 不把本地绿当远端行为 |
| §8.2 L616:a | 源三门唯一完成权威及本地绿远端红一律器bug | 现行 | excluded | 源CI裁决与绝对归因 | 目标先查实际差异，不能未测归因 |
| §8.2 L616:b | 改CI本身须实际事件验证 | 现行 | duplicate | `ci.actual-events` | 保留真实消费者 |
| §8.2 L617:a | 本地验证与CI强制并行、不得本地绿再push | 现行 | excluded | 源提交时序政策 | caller 生命周期和目标适用验证优先 |
| §8.2 L618–620:a | 8.3 worktree 与 Lean 缓存入口 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.3 L621:a | 按session用固定make/worktree/Lean缓存，禁冷裸lake | 现行 | excluded | 源宿主与Lean缓存运行配置 | 不作为当前worker工具指令 |
| §8.3 L621:b | missing不等stale、ensure/donor资格及特定收据组合 | 现行 | excluded | 源缓存实现状态机和事故 | 一般缓存两层诊断保留 |
| §8.3 L622–624:a | 8.4 诊断信号与产生处的质量 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.4 L625:a | 信号看实际结果、生产者保证质量且一信号一义 | 现行 | duplicate | `signal.meaning`、`evidence.program-state` | 不承诺好材料永不误读 |
| §8.4 L626:a | 过载只认CPU idle+memory_pressure，不以loadavg推断 | 现行 | generalized | `signal.meaning`、`resource.static-dynamic` | 不导入固定平台指标唯一性；选适用观测 |
| §8.4 L626:b | 累计日志计数带boot/current/full/last窗口 | 现行 | duplicate | `signal.meaning` | 保留单位窗口，不移植源标签 |
| §8.4 L626:c | wrapper恒零不代表任务；143需对齐时间与原因 | 现行 | duplicate | `evidence.failure`、`job.lifecycle` | 不依特定退出数字通用归因 |
| §8.4 L626:d | 误导信号须修producer、缺器按需铸器 | 现行 | duplicate | `signal.meaning`、`structure.pressure` | 不用次数制造重复平台 |
| §8.4 L627–629:a | 8.5 codex 异常的实际日志硬门 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.5 L630:a | 失败/超时/无结果须读实际日志后归因或修复 | 现行 | duplicate | `evidence.failure` | 读取限当前授权；禁止opaque-log时不可越界 |
| §8.5 L631:a | 核实际prompt/输出/退出和时间，不能假定题等于派发 | 现行 | duplicate | `evidence.failure`、`transport.literal` | 源rollout个人路径不迁移 |
| §8.5 L631:b | 连续活动/低CPU/超时/空stdout各不足以判hung/难 | 现行 | duplicate | `evidence.failure` | 不把验证器失败当任务难而降目标 |
| §8.5 L632:a | 归因必须有真实存在来源，无法取得不能补猜 | 现行 | duplicate | `evidence.failure`、`evidence.measure` | 不强制源日志目录或固定报告格式 |
| §8.5 L632:b | 日志只留必要结果来源，不拷贝过程；尚无lint不冒机器门 | 现行 | duplicate | `artifact.no-diary`、`judge.strength` | 当前dispatch禁止opaque读取优先 |
| §8.5 L633–635:a | 8.6 宿主作业所有权与后台边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.6 L636:a | 未知或超过前台预算的长任务用宿主作业句柄 | 现行 | duplicate | `job.lifecycle` | 按实际能力，不移植Claude600s上限 |
| §8.6 L637:a | 一次生命周期包含真实任务，wait全部孩子并传真实退出 | 现行 | duplicate | `job.lifecycle` | 不能detach后pgrep当完成 |
| §8.6 L638:a | 内部并发合法须同一调用收拢全部孩子 | 现行 | duplicate | `job.lifecycle` | 同时保留并发例外与完成条件 |
| §8.6 L639:a | 源shell自审/命令名单/heredoc剥离及转录反思器 | 现行 | excluded | 源检测器和个人会话审计流程 | 不新增日志扫描平台 |
| §8.6 L639:b | 可搜shell文本不证明行为；成功不能证明生命周期合规 | 现行 | duplicate | `judge.strength`、`job.lifecycle` | 守护范围 |
| §8.6 L640–642:a | 8.7 原生同步等待与外部轮询边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.7 L643:a | 有wait/watch原语先用，不自造sleep循环 | 现行 | duplicate | `job.wait` | 不强制gh或make具体命令 |
| §8.7 L644:a | 无原语外部状态才有界轮询，预定判据且每轮读数 | 现行 | duplicate | `job.wait` | 时间戳/节奏条件完整保留 |
| §8.7 L645–647:a | 8.8 完成通知与唯一等待通道 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.8 L648:a | 有主动通知就唯一通道，通知后读产物合法 | 现行 | duplicate | `job.wait` | 实际宿主能力条件，不照搬TaskOutput禁令 |
| §8.8 L649:a | 源自审器测试数量、特定Bash任务路径与跳过反例 | 现行 | excluded | 源实现细节和个人转录审计 | 不扩新runtime |
| §8.8 L649:b | 工具未进CI且无人运行时无保证 | 现行 | duplicate | `judge.strength`、`judge.counterexamples` | 不把可调用工具当强制门 |
| §8.8 L650–652:a | 8.9 所有脚本的通用性与可复用工具的仓库居所 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.9 L653:a | 所有脚本不得依个人路径/checkout/隐式shell配置 | 现行 | duplicate | `tool.portability` | 临时脚本同样适用 |
| §8.9 L654:a | 根路径/外部服务/凭据/环境显式，启动校验，缺则失败 | 现行 | duplicate | `tool.portability` | 不回退作者机器配置 |
| §8.9 L654:b | 声明平台依赖安装方式，能力检测真实可用 | 现行 | duplicate | `tool.portability` | 检测能力不推断选测权威 |
| §8.9 L655:a | 不同cwd/空格路径/无个人启动环境真测，未测平台明说 | 现行 | duplicate | `tool.portability-check` | 不冒跨平台保证 |
| §8.9 L656:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.9 L657:a | 跨会话工具tracked本仓而非个人目录 | 现行 | generalized | `tool.maintainable-owner` | 允许已有版本化提供包，宿主显式登记，不强复制 |
| §8.9 L658:a | 一次性胶水可临时，跨会话/指南引用需维护所有者 | 现行 | duplicate | `tool.maintainable-owner`、`artifact.temporary` | 保留临时例外，不自动安装技能 |
| §8.9 L658:b | 胶水入仓不因位置欠重复单测 | 现行 | duplicate | `test.behavior`、`tool.glue` | 有承重逻辑仍需测试 |
| §8.9 L659–661:a | 8.10 语言服务器与语义读数 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.10 L662:a | 语义引用/调用/可达用编译器或语言服务器 | 现行 | duplicate | `tool.semantic` | 不固定C#/Lean提供方，不改全局安装 |
| §8.10 L662:b | 缺能力先实测恢复，不grep猜语义 | 现行 | duplicate | `tool.semantic`、`action.blocked` | 本次禁止技能/全局写优先 |
| §8.10 L663:a | 字面检索可粗筛，符号位置可能错需采集口径 | 现行 | duplicate | `tool.semantic` | 明确工具+符号+位置 |
| §8.10 L664:a | Lean内置/C#插件及.claude宿主例外 | 现行 | excluded | 源语言服务器与全局配置 | 不迁移 provider 安装平台 |
| §8.10 L664:b | 换机须测能力，配置存在不等可移植 | 现行 | duplicate | `tool.portability-check` | 范围限制 |
| §8.10 L665–667:a | 8.11 PR open/watch 的消息与退出契约 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.11 L668:a | 文本经文件完整传字节，固定显式远端HEAD SHA | 现行 | duplicate | `transport.literal`、`query.identity` | 去pr.sh/make固定实现 |
| §8.11 L668:b | create/token/auto-merge/watch 同步有界流程 | 现行 | excluded | 源PR工具生命周期协议 | caller 负责 Git，不导入认证行为 |
| §8.11 L668:c | 缺值报错不以本地HEAD补，查询核PR与check/run身份 | 现行 | duplicate | `query.identity` | 旧head只等不能判新结果 |
| §8.11 L668:d | 矛盾/GraphQL部分错/分页未完/超时不是成功 | 现行 | duplicate | `query.identity` | 保留所有错误种类的含义 |
| §8.11 L669:a | make折叠退出码与canonical结果outcome区分 | 现行 | generalized | `query.identity`、`evidence.measure` | 目标具体退出码遵其工具合同 |
| §8.11 L670–672:a | 8.12 CI 分类、持续集成与真实事件验证 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.12 L673:a | CI hotfix/feature/refactor分类及4/8PR固定配额 | 现行 | excluded | 源CI集成制度与数量 | 用户无任意配额；保留适用真实事件核验 |
| §8.12 L674:a | 不用固定稳定时长，不等tip；性能/超时/首run仍有效 | 现行 | generalized | `cost.valid-evidence`、`ci.evidence-scope` | fresh候选按用户要求，不导入strict禁令 |
| §8.12 L675:a | 集成与交付环境等效保护，真实版本对应 | 现行 | duplicate | `ci.evidence-scope`、`delta.local-ci` | 不宣称远端已部署 |
| §8.12 L675:b | 被测差异逐项说明，分支过滤不选workflow版本 | 现行 | duplicate | `ci.actual-events`、`ci.evidence-scope` | 实际执行物必须核对 |
| §8.12 L676:a | 原lane/安装PR/两Draft/不合镜像历史的固定流程 | 现行 | excluded | 源Git编排拓扑 | caller指定fresh integration/fastforward交付优先 |
| §8.12 L676:b | 候选修复回流交付、变行为复验、无实际diff不造dummy | 现行 | generalized | `candidate.refresh`、`ci.evidence-scope` | 不按虚构提交制造验证 |
| §8.12 L677:a | 验证结果绑定测试候选/base/workflow/run、覆盖/性能/未达 | 现行 | duplicate | `ci.actual-events`、`ci.evidence-scope`、`cost.profile` | 不强制tracking Draft |
| §8.12 L677:b | 证据入口不替安装后触发，结果不存流水 | 现行 | duplicate | `ci.evidence-scope`、`artifact.no-diary` | 守护限制 |
| §8.12 L678:a | 逐原PR镜像/一PR一安装/禁止批量同步 | 现行 | excluded | 源CI历史镜像流程 | 不是目标delivery合同 |
| §8.12 L678:b | 显式两端分类与混面警告不豁免普通检查 | 现行 | duplicate | `registry.explicit`、`policy.mixed-warning` | 不引SL名 |
| §8.12 L678:c | 源PR到被测候选/事件证据对应，汇总绿不替每项覆盖 | 现行 | generalized | `ci.actual-events`、`ci.evidence-scope` | 不强制mirror表单或4/8资格 |
| §8.12 L679:a | 实际漂移成本时刷新同一dev基线、保留净成果 | 现行 | duplicate | `candidate.refresh`、`candidate.isolation` | 最新用户fresh feature/integration约束优先 |
| §8.12 L679:b | 旧新Draft替代和独立安装等Git动作 | 现行 | excluded | 源双Draft交付流程 | caller所有权 |
| §8.12 L680:a | 新输入仅补受影响验证，旧证据仅原范围有效 | 现行 | duplicate | `candidate.refresh`、`cost.valid-evidence` | 不机械全组重计 |
| §8.12 L680:b | 新环境首真实run仍需核，不能借重建逃失败或降保护 | 现行 | duplicate | `ci.evidence-scope`、`recovery.validate` | 无固定等待门 |
| §8.12 L680:c | 年龄/tip移动本身不足以无条件重建，需实际读数 | 现行 | generalized | `candidate.refresh`、`cost.measure` | 初次fresh创建仍按用户要求 |
| §8.12 L681:a | 分析实际成功/失败/跳过、关键路径、缓存、重建与异常 | 现行 | duplicate | `ci.actual-events`、`cache.diagnose`、`cost.profile` | 不要增设永久性能账本 |
| §8.12 L681:b | 必要优化可比复验，达标停，无关改进不扩张 | 现行 | duplicate | `cost.measure`、`cost.valid-evidence` | 不靠SHA机械重计 |
| §8.12 L682:a | 适用事件/过滤两侧/路由/权限/artifact/cache行为真实覆盖 | 现行 | duplicate | `ci.actual-events` | 程序行为测试复用现有 |
| §8.12 L682:b | 可比条件测性能，不能本机外推CI或减检测提预算 | 现行 | duplicate | `cost.profile`、`cost.measure` | 不复制源性能硬门配置 |
| §8.12 L683:a | 稳定单位/版本/连续4或8/哪些PR可计不可计 | 现行 | excluded | 源集成数量与资格制度 | 不迁移配额 |
| §8.12 L683:b | 跑前预期负向需命中具名判词，漏检与意外失败仍修 | 现行 | duplicate | `evidence.predeclare`、`test.mutation` | 负例不是基础设施故障或成功计数 |
| §8.12 L684:a | 证据足够即停，无疑点不因SHA/时间/求放心全重验 | 现行 | duplicate | `cost.valid-evidence` | 不引4/8计数前提 |
| §8.12 L684:b | 最终并集/落地/未覆盖事件仍需各自验证 | 现行 | duplicate | `candidate.landing`、`ci.evidence-scope` | 旧证据不能越界 |
| §8.12 L685:a | 重验先指明行为变化、失败断言、身份、失效范围与停止条件 | 现行 | duplicate | `cost.valid-evidence`、`evidence.predeclare` | 无新ledger/template/tool |
| §8.12 L685:b | 部分失效只撤部分，有效证据不全清 | 现行 | duplicate | `cost.valid-evidence` | 固定资格重计制度排除 |
| §8.12 L686:a | 非预期失败保留判词诊断，重跑不抹；未明原因阻受影响验收 | 现行 | duplicate | `evidence.failure`、`cost.valid-evidence` | 不盲全量重验或把真缺陷伪称格式 |
| §8.12 L687:a | run事件、入口/reusable workflow修订、候选/base分列 | 现行 | duplicate | `ci.actual-events` | M/B不替workflow版本 |
| §8.12 L688:a | GitHub PR取源、分支过滤、merge冲突和缓存权限实现 | 现行 | excluded | 源CI平台具体配置 | 实际事件语义须核，不导入固定入口/权限 |
| §8.12 L688:b | workflow与脚本接口同交付核真实调用版本配对 | 现行 | duplicate | `ci.actual-events`、`candidate.refresh` | 保持实际caller/consumer |
| §8.12 L689:a | 安装后第一真实run立即核，未触发是缺口，最终落地再核 | 现行 | duplicate | `ci.evidence-scope`、`candidate.landing` | 不扩hotfix特权或延后免验 |
| §8.12 L690:a | 替代事件只证明实际版本权限，披露原生缺口继续可达工作 | 现行 | duplicate | `ci.evidence-scope`、`action.blocked` | 不能用计数或旧绿抵缺口 |
| §8.12 L690:b | 不得先交付未达标CI到默认分支来取得可测窗口 | 现行 | generalized | `candidate.landing`、`ci.evidence-scope` | 保持交付验收，不导入Draft制度 |
| §8.12 L691:a | 探针载荷临时，清理不删有效成果，必要复验 | 现行 | duplicate | `artifact.temporary`、`candidate.refresh` | 源安装PR/镜像分支流程排除 |
| §8.12 L691:b | 交付前逐文件核路径增删mode内容与基线差异 | 现行 | duplicate | `candidate.refresh` | 不只比标题数量patch-id |
| §8.12 L692:a | 原lane转Ready/PR dev、不带mirror历史 | 现行 | excluded | 源Git交付编排 | caller fastforward交付不同 |
| §8.12 L692:b | 最终候选并集验收不由integration旧绿替代，异树诚实 | 现行 | duplicate | `candidate.landing` | 不导入strict禁令 |
| §8.12 L693:a | 评审/auto-merge/Draft/分支清理的固定时点 | 现行 | excluded | 源Git生命周期制度 | 当前独立评审由caller负责 |
| §8.12 L693:b | MERGED且落地后适用run成功才交付，tracking CLOSED仅验证结束 | 现行 | duplicate | `candidate.landing` | 完成角色边界保留 |
| §8.12 L693:c | 纯政策文档不为4/8制度造PR，也不称已部署重构 | 现行 | generalized | `test.behavior`、`judge.strength` | 无新的配额门 |
| §8.12 L694:a | 本地不验CI权限/布局，旧caller新接口须真事件暴露 | 现行 | duplicate | `ci.actual-events` | 不同基线一绿一红不证因果或稳定 |
| §8.12 L695:a | 验收引实际候选/事件/性能证据，正文可改非硬门 | 现行 | duplicate | `ci.evidence-scope`、`judge.strength` | 不复制必填表单 |
| §8.12 L696–698:a | 8.13 workflow 测试禁令与早反馈边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.13 L699:a | 永久禁止所有断言workflow内容的测试 | 现行 | excluded | 与目标适当行为/结构验证原则不兼容 | 不导入blanket禁令 |
| §8.13 L699:b | 形状绿可在if:false时不执行，不能代真实行为 | 现行 | duplicate | `ci.actual-events`、`judge.counterexamples` | 保留反例界限 |
| §8.13 L700:a | 仅三种旧生产逻辑fixture豁免且removal-only | 现行 | excluded | 源workflow测试例外清单 | 目标消费者测试复用现有工具 |
| §8.13 L701:a | 旧workflow检测退役、原强制保证已丢失 | 现行 | duplicate | `judge.strength` | 不冒称删除检查无损；不迁移具体退役测试 |
| §8.13 L702:a | 形状扫描可绕过或不跑，只是早反馈 | 现行 | duplicate | `judge.counterexamples`、`judge.strength` | 不复制source scanner |
| §8.13 L703–705:a | 8.14 判官集成与真实触发 PR | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.14 L706:a | 规则/判官变更先integration真验证 | 现行 | duplicate | `candidate.isolation`、`ci.actual-events` | 用户fresh dev约束主导，无4/8附带 |
| §8.14 L707:a | 多层三段、两Draft/镜像/交付原lane固定流程 | 现行 | excluded | 源CI/Git编排 | 本次caller所有权 |
| §8.14 L707:b | 候选修复保留回流、变行为定向复验、达标停 | 现行 | duplicate | `candidate.refresh`、`cost.valid-evidence` | 不混入无关历史 |
| §8.14 L708:a | 新门必须实际应拦应放触发，层PR绿可能未触判据 | 现行 | duplicate | `test.selection`、`ci.actual-events` | 预定具名finding与实际版本，不强制专用PR数量 |
| §8.14 L708:b | 探针验完退休，负向不能混为正常交付 | 现行 | duplicate | `artifact.temporary`、`evidence.predeclare` | 不自动发外部PR |
| §8.14 L709:a | revert-of-revert须新lane与固定Git配方 | 现行 | excluded | 源Git恢复编排 | caller负责Git |
| §8.14 L709:b | 目标过滤/base不证workflow来源，一绿非持续稳定 | 现行 | duplicate | `ci.actual-events`、`evidence.scope` | 限制原样保留 |
| §8.14 L710:a | 层绿/触发/覆盖证据不可少，正文不构硬门 | 现行 | duplicate | `ci.evidence-scope`、`judge.strength` | 不引Ready流程或固定计数 |
| §8.14 L711–713:a | 8.15 dev 红时的内容 lane 改投 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.15 L714:a | dev红时四种研究文件面改投日名integration-theory | 现行 | excluded | 源研究lane改投政策 | 不移植目录名单或平台 |
| §8.15 L715:a | 按实际因果而非所改路径认定无关 | 现行 | duplicate | `recovery.causal` | 自己的失败仍修，不借路径伪无关 |
| §8.15 L716:a | 新分支/首次绿即整体合回/不能跨第二红/删除再建 | 现行 | excluded | 源改投Git生命周期 | caller合同不同 |
| §8.15 L717:a | 替代候选仍适用检查，局部改投不等修复共享dev | 现行 | duplicate | `recovery.validate`、`candidate.landing` | 无旁路权限 |
| §8.15 L717:b | 计划显式登记/完整范围，一个红不推全部同红 | 现行 | duplicate | `registry.explicit`、`delta.range`、`evidence.scope` | 不动态推断输入 |
| §8.15 L718:a | 可变字段no consumer不称硬投影 | 现行 | duplicate | `judge.strength` | 源no consumer不豁免实际证据 |
| §8.15 L719–721:a | 8.16 CI 与判官的显式白名单权威 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §8.16 L722:a | 分类/项目测试/编译输入/影响/缓存材料唯一显式白名单 | 现行 | duplicate | `registry.explicit` | 当前用户进一步逐文件，不使用glob成员推断 |
| §8.16 L723:a | 移除旧动态发现，不以扫描/反射/调用/IO换名重建 | 现行；§8.16 登记权威优先，目标逐文件白名单 | duplicate | `registry.explicit`、`owner.migration` | 优先于源早期 §4.5/4.9 目录闭包 |
| §8.16 L724:a | 漏登/冲突具体失败，修登记并实际行为验证 | 现行 | duplicate | `registry.explicit`、`test.behavior` | 不因语义不完备另造分析器或全套回退 |
| §8.16 L725:a | glob展开/哈希/编译器增量只消费，不产输入权威 | 现行；§8.16 登记权威优先，目标逐文件白名单 | generalized | `registry.consume` | 目标禁止glob取代逐文件登记，正常编译非选测权威 |
| §8.16 L725:b | 缓存是种子，命中不是检查通过 | 现行；§8.16 登记权威优先，目标逐文件白名单 | duplicate | `cache.incremental-seed`、`registry.consume` | 保留下游验证 |
| §8.16 L726:a | 完整no-resource可not-required，缺范围不可无工/全套 | 现行 | duplicate | `registry.consume`、`delta.range` | 不把空diff与未知混同 |
| §8.16 L727:a | 政策文字不声明白名单迁移实现或机器全面执法 | 现行 | duplicate | `judge.strength` | 目标通用实现仍未完成 |
| §8.16 L728–732:a | 9. 验证、变异、引用完整性与确定性；9.1 陈述回声、TDD 与胶水边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.1 L733:a | 先核题与可执行行为预期、失败边界，再实现转绿 | 现行 | duplicate | `test.behavior`、`goal.deliverable` | 用户例外：低影响文字不造测试 |
| §9.1 L734:a | 胶水不复制单测，校验输入、真实退出、幂等安全重跑 | 现行 | duplicate | `tool.glue` | 三义务完整保留 |
| §9.1 L734:b | 有解析／判断／状态即承重程序，不因脚本豁免 | 现行 | duplicate | `test.behavior`、`tool.glue` | 不强制迁到源tools目录 |
| §9.1 L735:a | 真实执行与软纪律分开，不报未跑绿 | 现行 | duplicate | `judge.strength`、`evidence.measure` | 不引Lean门 |
| §9.1 L736–738:a | 9.2 引用的存在、一致性与消费者 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.2 L739:a | 引用从目标侧解析存在身份一致，语法不是完整性 | 现行 | duplicate | `reference.resolve` | 按实际消费者验证 |
| §9.2 L740:a | 新增承重引用配消费者；未检则补或撤字段和承诺 | 现行 | duplicate | `reference.resolve` | 当前guidance不声称通用judge已实现 |
| §9.2 L741:a | 历史样本只格式唯一而未核GID是规范实现缺口 | 现行 | generalized | `reference.resolve`、`judge.strength` | 源样本不是目标新读数 |
| §9.2 L742:a | 可普查不等有lint，格式不得冒指向 | 现行 | duplicate | `judge.strength`、`reference.resolve` | 守护限制 |
| §9.2 L743–745:a | 9.3 变异证据与六元组 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.3 L746:a | 打断机制须编译无错且非零且具名失败 | 现行 | duplicate | `test.mutation` | 三条件AND完整保留 |
| §9.3 L746:b | 看增删测试身份，净数相同不保检测；放行侧也测 | 现行 | generalized | `test.mutation`、`test.selection` | 不是新增prose关键词测试 |
| §9.3 L747:a | 位置/具名红/编译错数/退出/还原/跑前期望六项 | 现行 | duplicate | `test.mutation` | 只在实际使用变异时，不强造变异平台 |
| §9.3 L747:b | 期望独立于实现，别处红不能当目标检测 | 现行 | duplicate | `test.mutation` | 防恒真 |
| §9.3 L748:a | 期望事前给；多红少红均归因，不要求机械相等 | 现行 | duplicate | `test.mutation`、`evidence.predeclare` | 完整差异限制 |
| §9.3 L749:a | 变异真实性与差异解释不因表格存在成机器保证 | 现行 | duplicate | `judge.strength` | 守护限制 |
| §9.3 L750–752:a | 9.4 验证条件与分支上下文解耦 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.4 L753–754:a | 条件检查在命中与未命中上下文验证，不能只测同分支 | 现行 | duplicate | `test.selection` | 不强制新Git分支平台 |
| §9.4 L755–757:a | 9.5 head/base 与远端状态独立性 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.5 L758:a | PR事件权限/merge-ref缓存隔离/push写入/发布入口 | 现行 | excluded | 源GitHub安全配置 | 不导入具体权限/平台规则 |
| §9.5 L758:b | 固定M/B所有下游用同身份，冲突无候选不能报成功 | 现行 | duplicate | `query.identity`、`delta.range` | 不要求目标当前实现GitHub |
| §9.5 L758:c | 候选workflow不保证base文本，标注评审非独立保证 | 现行 | duplicate | `judge.strength`、`policy.mixed-warning` | 不复建base判官 |
| §9.5 L759:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.5 L760:a | 最终候选语义检查与基线规划角色分工 | 现行 | duplicate | `delta.candidate-judge`、`delta.range` | push不能用HEAD^1替完整范围 |
| §9.5 L761:a | before/after固定核HEAD、覆盖多提交删改名两端 | 现行 | duplicate | `delta.range` | 不依服务路径截断或祖先关系 |
| §9.5 L761:b | 缺对象可按固定身份取得；失败不可无工或全套 | 现行 | duplicate | `delta.range`、`query.identity` | 获取身份不授权任意latest输入 |
| §9.5 L762:a | 新树用显式初始输入，不猜base；删除无候选不报current通过 | 现行 | duplicate | `delta.range` | 现役 CI slice 显式 initial inventory，非历史 DELTA；完整范围见 docs/ci.md |
| §9.5 L763:a | 登记决定资源缓存，命中仅种子；完整no-resource才省资源 | 现行 | duplicate | `registry.consume`、`cache.incremental-seed` | 源dirty/preflight格式非目标现有支持 |
| §9.5 L763:b | 本地增量完整显式范围，不猜push事件 | 现行 | duplicate | `delta.range` | 当前runner边界仍干净快照 |
| §9.5 L764:a | 只执行候选，其它修订按角色读固定数据，不读latest | 现行 | duplicate | `delta.candidate-judge` | OID固定也不自动有输入资格 |
| §9.5 L764:b | 合成fixture仓可用自身修订，源具体refs禁名单 | 现行 | excluded | 源测试／Git引用白名单细节 | 目标按明确输入角色，无通用Git禁令 |
| §9.5 L765:a | 查询漂移origin会使同候选红绿变化，先核输入角色 | 现行 | duplicate | `delta.candidate-judge`、`delta.local-ci` | 不用strict补错输入 |
| §9.5 L766:a | RemoteStateIndependencePolicy仅余C#早反馈及绕过列举 | 现行 | excluded | 源扫描器实际状态和语言列表 | 不复制其实现 |
| §9.5 L766:b | 未测途径不可冒无远端执行读取 | 现行 | duplicate | `judge.counterexamples`、`judge.strength` | 两维反例保留 |
| §9.5 L767:a | 删remote名字不等无法按OID/URL读取，局部样本不推CI | 现行 | duplicate | `judge.strength`、`evidence.scope` | 不实施权限／remote操作 |
| §9.5 L767:b | PR深度2/push深度1/按before取对象后离线等具体传输 | 现行 | excluded | 源Git checkout与网络流程 | 不移植固定深度/权限 |
| §9.5 L768:a | 守护先列绕过与检查可跳过，不能自行宣称完备 | 现行 | duplicate | `judge.counterexamples` | 保留两维不是穷尽保证 |
| §9.5 L769–771:a | 9.6 测试时间、确定性与接线边界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.6 L772:a | 功能只依输入/注入时间，挂钟只hang guard，性能另测 | 现行 | duplicate | `test.determinism` | 不以重试时长/机器负载判功能 |
| §9.6 L773:a | RS0030禁止符号精确名单与xUnit项目 | 现行 | excluded | 源C#确定性工具配置 | 不引语言配额或编译器禁表 |
| §9.6 L774:a | watchdog触发归基础设施未解而非功能成功 | 现行 | duplicate | `test.determinism` | TRX/SkipException/退出格式不移植 |
| §9.6 L775:a | 单TestEnvironmentBridge/TestBudgets/marker约定 | 现行 | excluded | 源测试时间实现与组织 | 通用显式时间输入已保留 |
| §9.6 L775:b | preflight绿不证所有时间来源齐备 | 现行 | duplicate | `judge.strength` | 软约定不升级 |
| §9.6 L776:a | AST/字面matcher结构盲区，不能持续补匹配器冒语义 | 现行 | duplicate | `reason.representation`、`tool.semantic` | 不引保守扩展数学制度 |
| §9.6 L777:a | 别名/生成器/跨语言和禁规则被跳过两维反例 | 现行 | duplicate | `judge.counterexamples` | 语义和接线各需核验 |
| §9.6 L778:a | 注入时钟与同步信号推进，真实等待不判结果 | 现行 | duplicate | `test.determinism`、`job.wait` | 不移植具体脚本名 |
| §9.6 L778:b | 诊断资源观察采集失败不改业务退出码 | 现行 | generalized | `signal.meaning` | 可选诊断失败单独报告、保留实际任务退出结果；合同必需输入缺失仍须明确失败，不引源观测脚本或字段名 |
| §9.6 L778:c | 同输入慢机器不改功能红绿，flaky隔离不替修复 | 现行 | duplicate | `test.determinism` | 性能实验单独 |
| §9.6 L779:a | 符号/接线/软时间约定强度分列 | 现行 | duplicate | `judge.strength` | 不冒已有全脚本门 |
| §9.6 L780–782:a | 9.7 依赖闭包增量与全量读取的分界 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §9.7 L783:a | 检测选本次依赖闭包含反向，漏登记补而不全量兜底 | 现行 | duplicate | `delta.selection`、`registry.explicit` | 保留输入完整/局部性前提，不用归纳作绝对保证 |
| §9.7 L784:a | 可全读建索引/一次迁移重建，不全量准入裁决 | 现行；§8.16 登记权威优先，目标逐文件白名单 | duplicate | `delta.indexing` | §8.16显式白名单优先 |
| §9.7 L785:a | 不能传空changes绕delta，唤醒核值不只前缀/改名 | 现行 | duplicate | `delta.indexing`、`delta.range` | 实际登记影响，不推语义边 |
| §9.7 L785:b | base/diff与自身派生状态错位可自锁，修输入不全量兜底 | 现行 | duplicate | `recovery.causal`、`delta.candidate-judge` | 不引statement_id形式制度 |
| §9.7 L786:a | 入口可查，闭包语义完整性不可由图自动保证 | 现行 | duplicate | `judge.strength`、`delta.selection` | 守护限制 |
| §9.7 L787–791:a | 10. 容量、缓存输入、账本与查询；10.1 静态容量的三型与派生契约 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.1 L792:a | 容量合同按语义域/单位/可求值/资源变动定义 | 现行 | duplicate | `resource.contract` | 不按timeout等名字套规则 |
| §10.1 L792:b | 锁租约/协议/安全限/测试格式等域外 | 现行 | generalized | `resource.contract`、`resource.static-dynamic` | 按实际合同区分，非普遍禁动态反馈 |
| §10.1 L793:a | capacity/relation/policy三型强制择一及owner/案号等字段 | 现行 | excluded | 源容量三型官僚结构 | 保留一般推导与政策值区分 |
| §10.1 L793:b | 政策取值不冒推导、缺数据不填裸数、例外有依据复核 | 现行 | duplicate | `resource.contract` | 不强制永久案号和源owner |
| §10.1 L794:a | 特定min/floor多资源公式与平台字段列表 | 现行 | excluded | 源容量算法实例 | 不把具体公式写通用运行时 |
| §10.1 L794:b | 同单位上限、request/limit、任务强限/峰值区别 | 现行 | duplicate | `resource.contract` | 不混CPU/内存或瞬时RSS代理 |
| §10.1 L794:c | 缺必要数据报不可推导，N<1不得强行max1 | 现行 | duplicate | `resource.contract` | 保留不足状态和边界 |
| §10.1 L794:d | 容量反事实固定需求保留/吞吐/系数/舍入/域等非容量输入 | 现行 | duplicate | `resource.static-dynamic`、`resource.contract` | 不反读容量伪造响应，未强制特定公式 |
| §10.1 L794:e | 每遮蔽上限须有适用依据，测有效输出不只metadata | 现行 | duplicate | `resource.contract`、`resource.static-dynamic` | 不复制三型标签 |
| §10.1 L795:a | 静态前馈禁止漂移负载作输入，观测可诊断/标定 | 现行 | duplicate | `resource.static-dynamic` | 限定静态合同，不普遍禁反馈 |
| §10.1 L795:b | 有状态/稳定性/失败边界的动态反馈例外 | 现行 | duplicate | `resource.static-dynamic` | 关键例外完整保留 |
| §10.1 L795:c | 核同输入稳定、上限变化及边界，固定事实须测 | 现行 | duplicate | `resource.static-dynamic`、`evidence.measure` | 不导入现测固定平台常量 |
| §10.1 L796:a | 瞬时RSS不是峰值、重复容量读取是双源、名字不决定域 | 现行 | duplicate | `resource.contract`、`owner.canonical` | 通用工程含义 |
| §10.1 L796:b | STRATALINT并发5/#1910/旧perf脚本删除历史读数 | 上下文／有界历史样本 | context | 源静态容量override样本 | 不移植固定worker配额 |
| §10.1 L796:c | 旧读数不冒当前读者/双真源，外推不冒实测 | 现行 | duplicate | `evidence.measure`、`evidence.anomaly` | 具体平台细节保留为来源背景 |
| §10.1 L797:a | Kubernetes/cgroup/Parnas等成熟来源 | 上下文／有界历史样本 | context | 技术背景 | 不成为目标依赖 |
| §10.1 L798:a | 当前无lint/test/resolver，未来四项硬投影不是现役 | 现行 | duplicate | `judge.strength`、`resource.static-dynamic` | 不建立新容量判官 |
| §10.1 L799–801:a | 10.2 Lean 构建成本的两个坐标:缓存与证明项 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.2 L802:a | 慢构建先核cache状态/恢复/实际重编，不耗时猜困难 | 现行 | duplicate | `cache.diagnose`、`evidence.failure` | 去Lean具体字段与分钟阈值 |
| §10.2 L802:b | 13s/6.5min等同机有界样本，旧冷编17m51推断无效 | 上下文／有界历史样本 | context | 源缓存历史比较读数 | 不作为目标成本读数 |
| §10.2 L802:c | 成本引用须同树同载荷且确有执行，没证据不归因 | 现行 | duplicate | `cost.profile`、`evidence.measure` | 不导入枚举子图名称 |
| §10.2 L802:d | 软裸lake禁令不冒机器强制，归因需当次缓存证据 | 现行 | duplicate | `judge.strength`、`cache.diagnose` | 不执行源make配置 |
| §10.2 L803:a | darwin/linux arm64共享快照及native重建 | 现行 | excluded | 源Lean平台缓存兼容合同 | 目标macOS实测不外推平台支持 |
| §10.2 L803:b | 加入新平台前实测，x64未测不在支持内 | 现行 | duplicate | `tool.portability-check`、`cache.incremental-seed` | 保留验证实际范围 |
| §10.2 L804:a | 缓存正常仍昂贵转查实际计算；本地成功不证CI资源能力 | 现行 | duplicate | `cache.diagnose`、`cost.profile` | 不引10分钟阈值 |
| §10.2 L805:a | 用time/Lean profiler量声明成本和峰值，改后重量 | 现行 | generalized | `cost.profile` | 工具路径和语言参数不移植 |
| §10.2 L806:a | 减少一次驻留／线性表二次成本／过宽import的三例 | 现行 | generalized | `cost.profile`、`cost.measure` | 保留数据表示与依赖成本，不移植tactic配方 |
| §10.2 L807:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.2 L808–813:a | 四种配置墙钟/checked/峰值历史表 | 上下文／有界历史样本 | context | 源性能结果数据 | 无目标当前读数或普遍阈值 |
| §10.2 L814:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.2 L815:a | 负结果无改进也报告，不能本地内存下降推出远端SIGTERM因果 | 现行 | duplicate | `cost.profile`、`recovery.causal` | 保留未采远端证据界限 |
| §10.2 L815:b | 排除timeout/cancel等需逐项实际读数 | 现行 | duplicate | `evidence.anomaly`、`recovery.causal` | 具体源CI时间/组名不移植 |
| §10.2 L816:a | 不抬预算/碰运气/削完整性或目标来换绿 | 现行 | duplicate | `cost.measure`、`test.behavior` | 不移植escape witness或冻结pin制度 |
| §10.2 L816:b | 仅tactic/私有表示/import改且statement_id不变/deposit顺序 | 现行 | excluded | 源Lean数学语义与冻结改动域 | 一般保持语义承诺由 reuse.actual-consumers |
| §10.2 L817:a | 先找主成本及资源环境差异、数据结构渐近代价 | 现行 | duplicate | `cost.profile` | 成熟背景不作保证 |
| §10.2 L818:a | 优化理由须可比前后时间/峰值读数，归因限实际范围 | 现行 | duplicate | `cost.profile` | 模块级与聚合读数不能混 |
| §10.2 L818:b | 不一刀切禁宽import，现无逐模块峰值机器门 | 现行 | duplicate | `policy.proportional`、`judge.strength` | 不制造新性能gate |
| §10.2 L819–821:a | 10.3 一事不再理与内容寻址记忆化 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.3 L822:a | 纯函数同输入记忆化避免重复，key需有效输入 | 现行 | generalized | `cache.memoization`、`cost.valid-evidence` | 源绝对禁commit键适配为经论证最小相关身份 |
| §10.3 L822:b | 旧保守重放例子inactive不复建 | INACTIVE 的现行说明；不执行旧机制 | excluded | 源停用重放机制 | 停用 |
| §10.3 L823:a | 纯函数性/key靠核验，命中耗时可测 | 现行 | duplicate | `cache.memoization`、`judge.strength` | 不称key哈希证明完整 |
| §10.3 L824–826:a | 10.4 cache key 与权威增量补编 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.4 L827:a | Lake trace/语义版本决定报告复用，不比当前源摘要 | 现行 | generalized | `cache.compatibility`、`cache.incremental-seed` | 仅显式有据合同；不能普遍排除必要程序/政策输入 |
| §10.4 L828:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.4 L829:a | key少输入会假命中，多无关项假失效，接受语义同属输入 | 现行 | duplicate | `cache.memoization`、`cache.incremental-seed` | 两方向完整保留 |
| §10.4 L830:a | 每项问相关字段而非全文，较宽种子由权威下游补编 | 现行 | duplicate | `cache.incremental-seed` | 不能由某次改值无变化就证明永远无关 |
| §10.4 L830:b | 近似起点允许非精确命中，前提已有权威增量补验 | 现行 | duplicate | `cache.incremental-seed` | 不另造启发式或缓存平台 |
| §10.4 L831:a | metadata改key而Built0历史样本及禁止多层掩盖 | 现行 | generalized | `cache.diagnose`、`cost.measure` | 样本具体数值不迁移；仅相关性案例 |
| §10.4 L832:a | 特定Lean配置哪些影响olean、哪些不影响的清单 | 现行 | excluded | 源编译器输入语义与缓存实例 | 不移植为通用key清单 |
| §10.4 L832:b | 仅metadata测过，不外推leanOptions/依赖rev重编或CI耗时 | 现行 | duplicate | `evidence.scope`、`cost.profile` | 保留范围限制 |
| §10.4 L832:c | 全文/目录/commit键须论证相关字段 | 现行 | generalized | `cache.incremental-seed`、`cache.memoization` | 不绝对禁止完整输入身份 |
| §10.4 L833:a | action key/增量构建/种子风险成熟锚 | 上下文／有界历史样本 | context | 方法背景 | 缓存不是第二权威 |
| §10.4 L834:a | 缓存改动须命中/key与下游重建两层读数 | 现行 | duplicate | `cache.diagnose` | 两项AND保留 |
| §10.4 L834:b | 全文字段相关性须说明；可查不等语义可lint | 现行 | duplicate | `cache.incremental-seed`、`judge.strength` | 不造prose镜像测试 |
| §10.4 L835:a | 永久禁程序/判官/脚本/配置哈希作复用条件 | 现行 | excluded | 与用户完整有效输入/接受语义原则冲突 | 显式兼容版本仅在有据合同中可用 |
| §10.4 L835:b | 语义不兼容同PR bump，保持兼容须证据，机器不推断 | 现行 | generalized | `cache.compatibility` | 不导入源report字段名/版本算术或漏bump假保证 |
| §10.4 L835:c | 逐模块trace绑定真实编译闭包，程序只等build不入trace | 现行 | excluded | 源Lean报告缓存具体合同 | 不能普遍排除判官输入 |
| §10.4 L835:d | 报告复用与程序build各自职责，命中仍跑选中增量 | 现行 | duplicate | `cache.build-separate`、`registry.consume` | 无scope默认全登记目标属于源实现，不移植回退 |
| §10.4 L835:e | 选中构建失败非零且清成功收据，缓存不能覆盖 | 现行 | duplicate | `cache.build-separate` | 关键失败边界原样保留 |
| §10.4 L835:f | 空目标不用重缓存，登记决定程序目标 | 现行 | duplicate | `registry.consume` | 完整有效no-work才省资源 |
| §10.4 L835:g | 源两组PR耗时与test_producer_program_bytes_never_gate_reuse | 上下文／有界历史样本 | context | 源缓存历史证据与专用测试身份 | 不重复源测试或把其当通用正确性 |
| §10.4 L836–838:a | 10.5 git 已入账事实与重放禁令 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.5 L839:a | Git已入账信息完全信任，禁止所有重建验证 | 现行 | excluded | 与Git只固定字节不证明语义的用户原则冲突 | 有效旧证据可复用但须完整输入与适用合同 |
| §10.5 L840:a | 冻结字段终局不比报告，OID只增量，纯判词记忆化 | 现行 | excluded | 源冻结账本完整信任制度 | 一般 memoization 另有完整输入条件 |
| §10.5 L840:b | 以后必要原料与结果同存，勿为错误历史问题重建旧工具 | 现行 | generalized | `cache.query`、`artifact.results` | 只留实际需要；不保留源完整历史副本 |
| §10.5 L841:a | 旧公理闭包归因撤销，不补存储 | 现行 | excluded | 源研究历史问题与修订 | 停用旧归因 |
| §10.5 L842:a | schema/changed-path可验结构不证明目的 | 现行 | duplicate | `judge.strength` | 不引append-only制度 |
| §10.5 L843–845:a | 10.6 重放验证与必要查询 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.6 L846:a | 结果进入下游是必要查询，固定字节重复核验不增信息 | 现行 | generalized | `cache.query` | 语义验证／环境变更不是无用重放 |
| §10.6 L846:b | 重复纯函数按实际收益记忆化，一次不造缓存 | 现行 | duplicate | `cache.query`、`cache.memoization` | 不机械删必要读取 |
| §10.6 L847:a | FrozenActiveEntry/1972片/9.6MB/2s 单查询历史例 | 上下文／有界历史样本 | context | 源查询样本 | 不作为当前性能基准 |
| §10.6 L847:b | 一次必要查询不建memo，不能删验证后补缓存包装 | 现行 | duplicate | `cache.query` | 保留实际用途判断 |
| §10.6 L848–850:a | 10.7 账本强度、许可集与升级门 open | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §10.7 L851:a | 写入/升级全冻结公理闭包在许可集，永不退化 | 现行 | excluded | 研究公理与冻结升级门 | 不引全量历史重判 |
| §10.7 L851–852:a | 新环境验证当前适用要求，不错误比较历史环境派生值 | 现行 | generalized | `cache.query`、`cache.memoization` | 当前显式影响范围，保留真实必要输入 |
| §10.7 L853:a | 许可集唯一且放宽走τ=0贵路，测试钉死集合 | 现行 | excluded | 数学公理白名单与元层权限 | 一般唯一源/政策演进已保留 |
| §10.7 L854:a | 字段按真源／带权威加工／纯投影分别治理 | 现行 | generalized | `projection.criteria`、`owner.canonical` | 不采用“任何重算必错”绝对结论 |
| §10.7 L854:b | 升级只比公理许可、变异加axiom、写入无旁路 | 现行 | excluded | 研究升级机器要求 | 不构建通用新判官 |
| §10.7 L855:a | 升级保证仍open，写入样本不证新环境有效 | 现行 | duplicate | `judge.strength`、`evidence.scope` | 源具体事件数不迁移 |
| §10.7 L856–860:a | 11. 古典不动点与成熟锚；11.1 求真 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.1 L861:a | 格言是同批方法的成熟锚，守护边界不变 | 上下文／有界历史样本 | context | 引语用途说明 | 不把引用变新权威 |
| §11.1 L862:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.1 L863:a | 结论据事实不据记忆权威审美推测，先验后报 | 现行 | duplicate | `evidence.measure`、`review.independent` | 伦理表达不进入默认政策 |
| §11.1 L864:a | 知道与不知道分列，不冒领 | 现行 | duplicate | `evidence.anomaly` | 源open类型不强制 |
| §11.1 L865:a | 不臆测武断固执自证，自信也交独立证据 | 现行 | duplicate | `review.independent`、`goal.revision` | 去道德人格评判 |
| §11.1 L866:a | 多视角核共享先验，不能伪多样性 | 现行 | duplicate | `review.disclosure` | 不强制异模型provider |
| §11.1 L867:a | 述而不作文献三态尽调引语 | 现行 | excluded | 伦理／学术权威引语与源§3.7流程 | 准确归属一般义务已保留 |
| §11.1 L868–870:a | 11.2 待人 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.2 L871:a | 下游可用、可复核、诚实、清理并报告遗留边界 | 现行 | duplicate | `artifact.results`、`artifact.no-diary`、`candidate.landing` | 排除恕道伦理角色与源账本 |
| §11.2 L871:b | 约束人和AI同样适用，不凭身份免验证 | 现行 | generalized | `action.autonomy`、`judge.register` | 用户可信会错而非零信任 |
| §11.2 L872:a | harness服务下级程序 | 现行 | generalized | `method.host-tools`、`structure.pressure` | 不引忠道伦理或τ递归 |
| §11.2 L873:a | 出错先查自身/工具而不甩锅 | 现行 | duplicate | `evidence.failure`、`recovery.causal` | 不以道德标签替因果 |
| §11.2 L874:a | 承诺须有效新鲜验证，失败留可复用教训 | 现行 | duplicate | `candidate.landing`、`artifact.results` | 新鲜按有效输入，不机械重跑 |
| §11.2 L875–877:a | 11.3 规矩与利器 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.3 L878:a | 先定义处理方法，单点聪明不免规则 | 现行 | duplicate | `judge.register`、`review.independent` | 伦理比喻不作证明 |
| §11.3 L878:b | 规矩=真值DAG/门/器且按τ付费无后门 | 现行 | excluded | 研究本体／固定工具与分PR旧制度 | 一般规则演进见 policy.evolution |
| §11.3 L879:a | 诚实优于赶进度，打地鼠反慢 | 现行 | duplicate | `evidence.measure`、`core.repair-producer` | 不引道德评价 |
| §11.3 L880:a | 先利器的工具使用引语 | 现行 | duplicate | `method.entry`、`method.host-tools` | 具体§8命令未导入 |
| §11.3 L881–883:a | 11.4 俭约与自然 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.4 L884:a | 工具bug优先修，修复同样验证 | 现行 | duplicate | `action.blocked`、`recovery.validate` | 去修器成本塔 |
| §11.4 L885:a | 控制适度、不预建、不镀金 | 现行 | duplicate | `policy.proportional`、`structure.pressure` | 非所有未知风险都禁止 |
| §11.4 L886:a | 唯一源、最小规范、程序与数据分居 | 现行 | duplicate | `owner.canonical`、`owner.program-data` | 非递归信任塔 |
| §11.4 L887:a | 消病因后删补偿，以实际成本判断价值 | 现行 | generalized | `projection.consumption`、`cost.measure` | 不把更小代码量当唯一正确性 |
| §11.4 L887:b | harness成熟必须净行数及仪式耗时都下降 | 现行 | excluded | 源普遍净行数减少验收 | 与目标实质迁移/必要内容增长冲突 |
| §11.4 L888:a | 勿只造补偿/验证机器而不解决真实病因 | 现行 | duplicate | `core.repair-producer`、`structure.pressure` | 净行数硬判据和引语本体排除 |
| §11.4 L889:a | 性质约束放实际可能受影响处，确定性看producer闭包 | 现行 | generalized | `delta.selection`、`test.selection` | 显式输入及局部性前提，不自动所有内容豁免 |
| §11.4 L890:a | 无为/自然/水伦理引语与算法定址 | 现行 | excluded | 伦理和源地址本体 | 自主与服务可用方法另映射 |
| §11.4 L890:b | 稳态自主、工具服务目标 | 现行 | duplicate | `action.autonomy`、`method.host-tools` | 不引无人值守保证 |
| §11.4 L891–893:a | 11.5 度身与既判力 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.5 L894:a | 量腹而食引用容量方法 | 现行 | duplicate | `resource.contract`、`resource.static-dynamic` | 不引三型/公式/固定worker数 |
| §11.5 L895:a | 一事不再理引用记忆化方法 | 现行 | duplicate | `cache.memoization`、`cache.query` | 保留完整有效输入限制 |
| §11.5 L896–898:a | 11.6 砺与生于忧患 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §11.6 L899:a | 失败留下可复用判据或回归，实际检验异常 | 现行 | duplicate | `artifact.results`、`evidence.measure`、`review.independent` | 排除磨难伦理和exit3特定含义 |
| §11.6 L900:a | 格言与SL同一不动点的伦理/机器等同 | 现行 | excluded | 伦理本体与源执法等同论 | 机器保证只看实际执行 |
| §11.6 L900:b | 可机器判与其余核验分工 | 现行 | duplicate | `judge.strength` | 不导入古训为强制政策 |
| §11.6 L901–905:a | 12. 导航与收束；12.1 操作导航 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §12.1 L906:a | 操作细节导航导言 | 上下文／有界历史样本 | context | 来源导航 | 非目标runtime输入 |
| §12.1 L907:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §12.1 L908:a | agents/CONTEXT固定路径及2K token必读 | 现行 | excluded | 源宿主导航/阅读配额 | 目标只读自身host_context |
| §12.1 L909:a | 完整spec单一且原位演进 | 现行 | generalized | `owner.canonical`、`policy.evolution` | 固定源spec路径排除 |
| §12.1 L910:a | 理论源路径及PZG/GICT卷 | 现行 | excluded | 源研究导航 | 不移植为必读 |
| §12.1 L911:a | 八官宪章固定角色路径 | 现行 | excluded | 源角色/技能导航 | 不新增skills或provider平台 |
| §12.1 L912–914:a | 12.2 发现、逻辑、账与美 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §12.2 L915:a | 本文件为不动标架，操作查源地图spec | 上下文／有界历史样本 | context | 来源收束与导航 | 源是迁移数据，不是本次指令 |
| §12.2 L916:a | 发现连网/逻辑真伪/账平/美幸存的本体收束 | 现行 | excluded | 研究与伦理总纲引语 | 有用求证方法已各映射，不另造同义政策 |
| §12.2 L917:a | 空行／引语分隔 | 上下文／有界历史样本 | context | 标题／排版上下文 | 已读；本范围无另立义务，正文逐项列于邻行 |
| §12.2 L918:a | 无人值守也不能丢诚实，每构建核实际结果 | 现行 | generalized | `evidence.measure`、`evidence.program-state` | 不引每构建全仓账平或无人自治保证 |
