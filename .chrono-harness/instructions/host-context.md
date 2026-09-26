# chrono-harness 宿主上下文

本仓维护显式登记的 Rust harness 与独立宿主指令生成器。产品合同在 SPEC.md，当前行为与调用在 README.md、docs/instructions.md。

生产/专属测试配对：runner ↔ runner-tests，instructions ↔ instructions-tests。各自有 Cargo.toml、Cargo.lock、target，无根 workspace；instructions 不依赖 runner。构建/检查使用显式 manifest 与 --locked；仅测试项目的 build/check 带 --tests。操作、所有权、依赖和未知成本分别登记在 .chrono-harness/projects.json、FILEMAP.json；变更文件需同步精确登记。

通用判官、DELTA、调度与 CI gate 尚未实现。五份通用登记仍 proposed；chrono-harness check 返回 3，不产生验证通过报告。chrono-instructions 的专用 manifest 校验和生成已实现，两者不可混报。

产品默认位于 assets/instructions/catalog.json 与 default-manifest.json；宿主独立采用 catalog.json、manifest.json 和本上下文。根 CLAUDE.md 受管块直接呈现中文核心，AGENTS.md 是字面相对链接 CLAUDE.md；编辑 catalog 或其显式 file 源/输出计划，再运行登记的 generate。不要独立编辑投影，也不必重复读取已呈现的规则源。

当前专用 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。默认新宿主一条 init 从内嵌双语数据采用仅根输出，--locale en 可显式绑定初始英文；无运行时 checkout 或语言推断。已知旧登记自动保留 opaque 方法为 und file atom，升级不覆盖定制。本仓已明确采用产品逐条迁移的 98 个双语内容叶子与 16 个普通聚合；原有 17 个稳定入口及 core.general 保留，旧精选段落由单一内容叶子替代，已退休 monolith 不恢复。固定源及逐条处置在可选 docs/methodology-clause-map.md，许可在 docs/licenses/；这些不是必读政策或生成输入。旧 core.ownership 显式依赖 projection.criteria／projection.consumption，core.behavior 依赖 ci.actual-events；独立消费仍有原义务。默认新宿主双语根的实际字节预算及有限余量见可选 docs/methodology-extraction.md，不是运行时新增判官。英文 docs/generated/general-methods.en.md 和 skills/diagnose-recurring-failures/SKILL.md 由同一 catalog/manifest 生成；skill 仅选择 evidence/reuse/repair-producer 的 7 个内容叶子，不做全局安装。删除输出条目保留旧文件，需授权 AI 明确退休；marker 不代表仍受管理。

产品双资产是编译/测试输入，宿主 catalog/manifest/context 和已存在投影是运行输入，具体 FILEMAP 边保持显式。FILEMAP projection.sources 是 proposed 的多源元数据扩展，不代表通用判官已执行。源翻译的语义完整性需内容核验；生成和格式验证不认证 AI 遵守或翻译等价。

按任务目标自主实施、验证与修复，保持独立项目/专属测试和真实退出状态。分支约定与计划中的 integration 策略见 SPEC.md；Git 生命周期遵循当前任务授权，不从本文推断已启用机器门。不要改全局配置或参考仓库，不将过程转录写入本仓。
