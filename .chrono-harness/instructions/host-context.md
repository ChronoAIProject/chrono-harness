# chrono-harness 宿主上下文

本仓维护显式登记的 Rust harness 与独立宿主指令生成器。产品合同在 SPEC.md，当前行为与调用在 README.md、docs/instructions.md、docs/ci.md。

生产/专属测试配对：runner ↔ runner-tests，judge-registration ↔ judge-registration-tests，judge-filemap ↔ judge-filemap-tests，judge-ci ↔ judge-ci-tests，ci ↔ ci-tests，instructions ↔ instructions-tests。十二个项目分别位于 crates/<项目 ID>/，各自有 Cargo.toml、Cargo.lock、target，无根 workspace；instructions 不依赖 runner。crates/ 仅作目录分组，宿主约束仍在 .chrono-harness/。构建/检查使用显式 manifest 与 --locked；仅测试项目的 build/check 带 --tests。操作、所有权、依赖和未知成本分别登记在 .chrono-harness/projects.json、FILEMAP.json；变更文件需同步精确登记。

五份完整治理登记仍 proposed；独立 chrono-ci-check/v1 已显式消费 FILEMAP/projects 的 scoped CI 子集。runner 的 full v1 采集原始 Git 事实、运输外部判官/协议/结果；registration 拥有 full 登记结构、实际 checkout/config/context 核对、dirt 与受影响引用政策。filemap 复用 registration 的严格只读 loader/node 接口，拥有完整声明图、union 来源、种子/字段变化、有限因果闭包和版本化 outputs.impact（docs/filemap-impact.md）。judge-ci 通过显式 adapter 复用 filemap 的 union/closure，拥有 scoped CI 快照、DELTA、登记与操作执行，ci 拥有工作流生成和事件准备。先用 .chrono-harness/ci/bootstrap.py 构建登记工具，再对干净固定候选使用同一 chrono-harness check --config .chrono-harness/ci/check.json --base FULL_OID --candidate FULL_OID。full --context 已执行 candidate-only 登记判官，清空白名单环境、预启动摘要、JCS ID、四状态与 DAG/evidence 校验已实现；其余五判官及完整交付未实现；旧测试/owner/cost 仅为 impact 事实，尚无完整执行/退休/成本裁决，外部 input 声明测试不证明保留，registration 的非空外部输入 unresolved 保持，不能与现役 slice 混报。ci-verify 缺 full script path/配对与 chrono-ci 工具声明；full registration 会明确报错，routes/projects 迁移仍待完成。逐节及验收行边界在 docs/spec-coverage.md。成本未知、输入闭包不完整、同命令不自证同判。chrono-instructions 的专用 manifest 校验和生成独立保持。

产品默认位于 assets/instructions/catalog.json 与 default-manifest.json；宿主独立采用 catalog.json、manifest.json 和本上下文。根 CLAUDE.md 受管块直接呈现中文核心，AGENTS.md 是字面相对链接 CLAUDE.md；编辑 catalog 或其显式 file 源/输出计划，再运行登记的 generate。不要独立编辑投影，也不必重复读取已呈现的规则源。

当前专用 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。默认新宿主一条 init 从内嵌双语数据采用仅根输出，--locale en 可显式绑定初始英文；无运行时 checkout 或语言推断。已知旧登记自动保留 opaque 方法为 und file atom，升级不覆盖定制。本仓已明确采用产品逐条迁移的 98 个双语内容叶子与 16 个普通聚合；原有 17 个稳定入口及 core.general 保留，旧精选段落由单一内容叶子替代，已退休 monolith 不恢复。固定源及逐条处置在可选 docs/methodology-clause-map.md，许可在 docs/licenses/；这些不是必读政策或生成输入。旧 core.ownership 显式依赖 projection.criteria／projection.consumption，core.behavior 依赖 ci.actual-events；独立消费仍有原义务。默认新宿主双语根的实际字节预算及有限余量见可选 docs/methodology-extraction.md，不是运行时新增判官。catalog 的可选 layouts 与 output.layout 只组织阅读，不改 requires、权威或执行顺序；未选布局保持旧 v3 平铺字节。产品与本仓采用同一 general 双语布局（三部分、12 主题、98 正文各一次），宿主数据仍独立拥有。英文 docs/generated/general-methods.en.md 和 skills/diagnose-recurring-failures/SKILL.md 由同一 catalog/manifest 生成；skill 仅选择 evidence/reuse/repair-producer 的 7 个内容叶子，不做全局安装。删除输出条目保留旧文件，需授权 AI 明确退休；marker 不代表仍受管理。

产品双资产是编译/测试输入，宿主 catalog/manifest/context 和已存在投影是运行输入，具体 FILEMAP 边保持显式。FILEMAP projection.sources 是 proposed 的多源元数据扩展，不代表通用判官已执行。源翻译的语义完整性需内容核验；生成和格式验证不认证 AI 遵守或翻译等价。

按任务目标自主实施、验证与修复，保持独立项目/专属测试和真实退出状态。分支约定与计划中的 integration 策略见 SPEC.md；Git 生命周期遵循当前任务授权，不从本文推断已启用机器门。不要改全局配置或参考仓库，不将过程转录写入本仓。
