# chrono-harness

用 Rust 构建的、以显式登记和 DELTA 判官为核心的 harness，附独立的宿主指令生成器。

**已实现指令生成、独立 CI 生成器、本地/CI 共用 check 入口，以及 chrono-judge/v1 运输和 registration、filemap、routes、projects、cost、mixed、workflow 的有界合同。五份完整治理登记仍 proposed；现役 CI 继续使用明确版本化的 slice。**
完整中文合同、数据结构、协议与验收条件见 [SPEC.md](SPEC.md)。

[公开测试版 v0.1.0-beta.19](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.19) 提供 macOS arm64 / Linux x86_64 的原生二进制；宿主无需 Rust，按[发布与安装合同](docs/distribution.md)锁定采用。

公开的独立示例宿主：

| 仓库 | 用途 |
| --- | --- |
| [chrono-harness-examples-go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | Go 生产模块与独立测试模块，显式构建、测试和文件登记 |
| [chrono-harness-examples-ts](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | TypeScript，无 package.json 或 tsconfig，分别登记生产与测试类型检查 |
| [chrono-harness-examples-mix](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | Go、TypeScript、独立 Python 脚本，共享数据通过显式依赖选择测试 |
| [chrono-harness-examples-initial](https://github.com/ChronoAIProject/chrono-harness-examples-initial) | 无业务项目的文档宿主，显式登记首次库存检查与后续 DELTA，使用生成的 v2 CI |

这四个仓库是测试版安装与本地/原生 CI 的实际验收宿主；覆盖范围和当前限制见 [示例索引](docs/examples.md)。

宿主约束仅放在 [.chrono-harness](.chrono-harness/config.json)。Rust 项目集中在 `crates/`，生产/测试配对为 `runner` / `runner-tests`、`judge-ci` / `judge-ci-tests`、`judge-registration` / `judge-registration-tests`、`judge-filemap` / `judge-filemap-tests`、`judge-routes` / `judge-routes-tests`、`judge-projects` / `judge-projects-tests`、`judge-cargo` / `judge-cargo-tests`、`judge-cost` / `judge-cost-tests`、`judge-mixed` / `judge-mixed-tests`、`judge-workflow` / `judge-workflow-tests`、`inputs` / `inputs-tests`、`distribution` / `distribution-tests`、`ci` / `ci-tests` 、`worktree` / `worktree-tests` 和 `instructions` / `instructions-tests`；各自独立 manifest、lockfile、target，无根 workspace。指令生成器保持独立；CI 判官与生成器复用 runner 的通用运输/argv 接口。

```text
crates/                    独立 Rust 生产项目及各自测试项目
  runner/                  判官运输与统一 check 入口
  runner-tests/
  worktree/                  显式目标抓取、工作树创建与登记重建
  worktree-tests/
  distribution/            发布打包、版本锁定和宿主安装
  distribution-tests/
  inputs/                  显式输入采集与保留内容运输
  inputs-tests/
  judge-registration/      v1 登记结构、实际快照与受影响引用
  judge-registration-tests/
  judge-filemap/           有类型 union 图、种子、因果闭包与影响输出
  judge-filemap-tests/
  judge-routes/            登记方法、工具绑定和回执核对
  judge-routes-tests/
  judge-projects/          受影响配对与真实操作执行
  judge-projects-tests/
  judge-cargo/             显式采用的 Cargo 一致性判官
  judge-cargo-tests/
  judge-cost/              显式四维成本、旧新来源和未知值告警
  judge-cost-tests/
  judge-mixed/             显式规则变化分类、混合修改与旧新成本警告
  judge-mixed-tests/
  judge-workflow/          分支新鲜度、退休迁移与 integration 证据
  judge-workflow-tests/
  judge-ci/                scoped CI 登记、快照、DELTA 与操作执行
  judge-ci-tests/
  ci/                      CI 初始化、生成和事件准备
  ci-tests/
  instructions/            原子指南与 skill 生成
  instructions-tests/
assets/instructions/       产品默认规则和输出计划
docs/                      合同说明、来源和生成文档
examples/                  可复制的宿主示例
skills/                    本仓生成并采用的 skill
.chrono-harness/           本仓采用的约束、登记和工具绑定
.github/workflows/         从宿主配置生成的 CI
```

`crates/` 只组织目录；构建与选测由 `.chrono-harness/` 中的显式登记决定。
`assets/` 提供可分发的产品默认值，宿主采用后的独立数据归 `.chrono-harness/`。

```sh
# 在本项目 checkout 中安装；确保 Cargo 的 bin 目录在 PATH 中
cargo install --locked --path crates/instructions
# 安装后从任意工作目录初始化已有宿主，只需这一条命令
chrono-instructions init --host-root "/path/to/existing-host"
```

可执行文件内嵌[通用规则 catalog](assets/instructions/catalog.json) 与 root-only 默认 manifest，复制二进制后也无需 checkout。逐条迁移后的 98 个内容叶子具有中文/英文 variant；16 个普通聚合保留原有 17 个稳定入口及 core.general，依赖显式；宿主 `.chrono-harness/instructions/` 采用独立 catalog、manifest 与空上下文。

默认产生含 20 个叶子短流程的普通 `CLAUDE.md`，`AGENTS.md -> CLAUDE.md` 是字面相对链接。每项受治理操作只呈现一条当前登记路径，定制与演进通过登记及适用验证完成。新宿主可用 `--locale en` 绑定英文；`--methodology M` 保留自定义 UTF-8 方法为 opaque file atom，未声明语言时为 und；`--host-context C` 独立指定上下文。

默认两种语言的短流程新根字节数及宿主定制边界见[实际消费者读数](docs/methodology-extraction.md#实际消费者边界)；完整双语库仍可供显式组合。

日常编辑宿主 catalog 与输出计划，再运行 `chrono-instructions generate --host-root H`。计划可引用共享原子，选择语言并生成任意登记 Markdown 或聚焦 skill。不选布局时按依赖先行的 DFS；可选具名布局显式组织标题和内容，必须完整覆盖非空闭包一次。默认双语共用五节同级的 `workflow` 布局；完整 `core.general/general` 的三部分、12 主题仍供自定义输出，本仓英文指南继续选择它。阅读层次不代表权威或执行顺序；聚焦 skill 仍平铺。缺失选中翻译、循环或无效引用均明确失败，无自动翻译或 fallback。可复制 schema 与组合配方见[生成合同](docs/instructions.md)。

当前身份为 schema 2 / `atomic-rules/relative-alias/v3`。已知 read-both/v1 与 literal-core/relative-alias/v2 自动前向迁移，保留旧方法/上下文精确字节、路径与权限，不拆 prose 或改为默认。重复 init 省略选项保留采用数据，显式不同输入或 locale 拒绝；原子组合不能被 raw method 覆盖。

根块外原文属于宿主；两普通根只在渲染后整份字节相同才转换，有差异明确报错供授权 AI 保全并整合。额外输出为有身份标记的整文件投影，拒绝覆盖未声明所有权的现有文件。删除或改名输出条目保留旧文件；旧 skill 须明确退休，否则仍可能被发现。无变化时无写入，已有有效根链接不重建。

支持 Unix，行为验证限定当前 macOS。普通 IO 失败尝试回滚并报告未恢复路径/备份；无崩溃原子性或并发写者保证。退出码 0 完整、2 用法错误、1 生成/IO 失败。
[来源说明](docs/methodology-extraction.md) 与 [逐条处置表](docs/methodology-clause-map.md) 记录固定 918 行来源的实际义务、泛化、重复与排除。[派生文本许可](docs/licenses/methodology-attribution.md) 仅适用于相应指令资产；规则经过修改、泛化及翻译，不改变整仓许可。本仓通过同一工具生成根中文、[英文方法](docs/generated/general-methods.en.md) 与[重复故障诊断 skill](skills/diagnose-recurring-failures/SKILL.md)；默认新宿主只生成根指南。生成不证明 AI 遵守、翻译语义等价或判官已执行。

```sh
cargo build --locked --manifest-path crates/runner/Cargo.toml
cargo test --locked --manifest-path crates/runner-tests/Cargo.toml
./crates/runner/target/debug/chrono-harness --help
./crates/runner/target/debug/chrono-harness spec status
```

使用 `sha2` 的独立 Cargo 构建根在 dev 及其继承的 test profile 中，将该上游依赖设为 `opt-level = 3`，降低输入摘要核验的计算成本，同时保留 debug assertions。依赖库的 profile 不向调用方传递，因此这项配置由每个生产、测试项目的 manifest 显式拥有。

现役 CI 使用独立的 `judge-ci` 执行显式登记的 DELTA 检查；runner 只承担外部判官运输与报告。独立 `chrono-ci` 从宿主配置生成 GitHub Actions，提供 init/generate/verify 和事件输入准备。宿主可通过 `push_baselines` 显式登记分支前缀及远端基线；本仓的 integration 连续 push 均对照所观察的 dev 提交，使用相同的 check 指令。

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
# 候选须已提交且检出干净；本地和 CI 使用完全相同的入口
.chrono-harness/bin/chrono-harness check --unit ci
```

普通命令固定为 `check`、`check --unit ID` 和 `check --collect`。config schema4 在固定 `.chrono-harness/config.json` 登记 profile、输入生产者 action、工具、环境和协议边界。未设或 `local` 的 `CHRONO_CHECK_SOURCE` 自动从登记 remote/workflow target 取得 base 和 clean committed HEAD；生成的 CI 显式设 `ci`，命令内执行原事件准备与原生汇总。真实短 argv、解析后的配置／端点／scope 和原始生产者证据分别保留；采用短合同的宿主拒绝旧长拼写。full unscoped 从 worktree start/reconstruct 的目的地 origin receipt 生成 schema2 context；full 独立单元／汇总短入口已有源实现，实际原生 full 启用、相关二进制发布及完整 SPEC 验收仍未完成。旧配置的显式合同及未迁移示例保留。

公开工具支持[按显式单元生成条件 job、汇总原始报告及保留宿主自定义的迁移](docs/ci-units.md)。本仓在 `check.json` 登记完整测试计划及其单元归属，在 `units.json` 显式采用 `job_gating`：一个 Git DELTA detector、按登记单元生成的 job-level if 独立 job 和一个 `always()` aggregate，全部投影到 `.github/workflows/chrono-ci.yml`。只将 aggregate 的 **chrono / collection** 设为 required check；本次不修改远端保护规则。未选单元不分配 runner 或启动 SDK；各单元保留独立 runner/bootstrap/timeout/证据与 job 重跑。Git 给出完整 PR base/head 或 push before/after；integration 继续使用 detector 一次观察并固定的 dev 基线。完整 Git DELTA 不设路径数量门。日常入口仍为 `check`、`check --unit ID`、`check --collect`，汇总执行零业务操作并交现役判官准入。旧 provider 不自动改义，公开 Go/TS/mix 的发布迁移与本次原生 Actions 验证归调用方。[新复制示例](examples/ci-host-job-gating/README.md) 使用 installed tools 和同一短入口。

复制二进制初始化新仓、操作扩展、schema、首次采用和事件合同见 [docs/ci.md](docs/ci.md)；可复制完整实例见 [examples/ci-host](examples/ci-host/README.md)。`verify` 检测工作流漂移，不先修复输出。未知路径、缺对象、脏输入、无效登记或失败命令均非零；文档等闭包外变更明确报告未选项目检查。

带 `--context .chrono-harness/state/context.json` 的 full check 调用登记的 v1 外部判官，绑定请求身份、候选二进制摘要、白名单环境与直接前驱 DAG。registration 验证五份结构、固定身份、工作树 dirt、保留输入及受影响引用，并提供唯一历史解释视图。FILEMAP 生成两端 impact v2，并按显式 stability 与 integration 登记选择测试；routes 合并显式执行计划并绑定实际工具；projects 核配对并执行、核对真实回执。现役 scoped CI 复用该规划和执行路径，保留其已注明的选择及输入边界。版本化输出见 [FILEMAP impact 合同](docs/filemap-impact.md) 和 [执行合同](docs/execution.md)。`.chrono-harness/config.json` 仍 proposed、缺完整输入闭包与绑定，完整宿主检查必须非零。完整 SPEC 的生产者、直接测试与缺口见 [覆盖矩阵](docs/spec-coverage.md)。完整 Cargo/SDK 输入闭包、裁决确定性、本地/CI 同判、完整宿主的分支与 integration 接线仍未认证。原生 GitHub 事件须以实际 run 的固定身份和结果验证；本地测试不能替代。

配置 v3 可用 `facts_git` 显式绑定 Git 的工具、输入摘要、版本和环境，贯穿 full/initial 读取及下游判官；v1/v2 保持旧语义。调用方式与边界见 [Git 事实绑定](docs/git-facts.md)。CI provider v3 可通过 `facts_config` 显式选择同一绑定机制用于事件准备；仅明确缺失的对象触发 fetch，工具失败保留原始证据。scoped profile v2 也可通过 `policy.facts_config` 选择该绑定，两端快照及可选完整登记解释复用同一 Git，成功与失败均保留实际进程证据；选测和操作环境仍沿用 scoped 合同。这些扩展已包含在 beta.9；升级二进制不自动采用新版配置。本仓使用 scoped v3 的独立单元扩展，没有自动采用上述 Git 绑定；完整输入闭包尚未完成。

full 报告提供 §9 全部顶层字段与执行物列表，汇集实际 findings 和具名 outputs 并标来源。未配置或未取得的工具、有效输入、影响、测试、成本结果显式为 null，unresolved 给出原因；配置中的版本不冒充实测版本。scope 仍为 configured-judges，parity 默认是 unestablished；有界 registration/filemap pass 不表示完整治理成功。

普通 full check 在登记要求及全部选中义务通过后表示该 DELTA 满足现役登记合同；AI 对宿主治理／输入范围作负责的 declared-complete 工程声明，现役 registration 仍可报告 completeness_proven:false。已知缺输入、未解决的必需依赖、缺绑定／快照及漂移仍失败，实际工具链／SDK／配置义务保留；普通验收不要求普遍隐藏输入证明或 VM。本宿主仍 proposed/incomplete，不能宣称完整启用。

两次独立 full check 均取得完整有效输入证据后，可运行
`.chrono-harness/bin/chrono-harness parity --host-root . --report .chrono-harness/state/report.json --compared-report .chrono-harness/state/ci-report.json`。
该命令要求 completeness_proven:true 与完整观察，只比较已发布报告的裁决字段，成功时同步更新当前报告及其 retained `report_path`；更强前提缺失或裁决不同均保留 unresolved 并以非零退出。parity 不另加普通 full check 门；本地与 CI 必须同命令，普遍同判仍依赖完整相同输入与确定性求值，成对观察本身不证明这些条件。

Registered routes/projects execution, actual receipts, FILEMAP v2 and the finite historical decoder are documented in [docs/execution.md](docs/execution.md). ci.verify belongs to the real ci/ci-tests pair. The host remains proposed with incomplete Cargo/SDK closure; workflow has bounded certification; autonomous delivery remains future work. Mixed classification and warnings use the [explicit surface contract](docs/mixed.md). Declared costs and unknown warnings have their own [cost contract](docs/costs.md).

稳定性与规则语义变更的测试选择现已接入同一 routes/projects 执行链，见 [workflow 选择合同](docs/workflow-selection.md)。[workflow 判官](docs/workflow.md) 在 context v2 上核对分支新鲜度、实际 integration 证据及显式退休迁移。工作树创建与按显式计划重新应用旧 DELTA 由独立工具实现；AI 语义协调、提 PR、合并与核验落地仍由调用方负责。

显式外部输入可由独立 `chrono-inputs capture/pair` 生成两端快照和保留内容，判官流式核对，无需把大文件字节塞进 JSON。调用及格式见 [输入快照合同](docs/inputs.md)。这不自动发现或补齐 Cargo/SDK 依赖，也不自动启用完整宿主。

测试版统一从本库 [GitHub Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 分发。独立 `chrono-distribution` 按显式清单打包、合并原生平台产物、生成宿主安装登记与入口，验证清单和二进制摘要后安装；宿主无需保留产品源码包或编译 Rust。合同与命令见 [发布与安装](docs/distribution.md)。

`chrono-ci` 还可按显式 `chrono-github-release/v1` 源生成和核对发布构建工作流，复用同一 init/generate/verify 入口。平台、命令和产物目录均由宿主登记，本地与 CI 执行同一发布命令；该扩展已包含在 beta.10，见[发布 CI 合同](docs/release-ci.md)。

完整治理可显式采用 `chrono-github-full-ci/v1`：传入固定 context 原字节，生成 CI
调用与本地相同的 `check … --context …`。工具和引用证据由宿主 bootstrap 供应；准备
成功不等于判官通过。该扩展已包含在 beta.10，本仓完整宿主仍未启用，见[完整 CI](docs/full-ci.md)。

通用 projects 登记不要求 manifest、lockfile 或根目录，actions 可使用任意非空登记名称。Cargo workspace/路径依赖检查及已登记 registry/Git 输入、metadata 与真实操作的联合校验属于独立可选 [chrono-judge-cargo](docs/cargo-projects.md)；宿主目录和语言不承担隐式登记或选测权威。

新工作树使用独立的 `chrono-worktree start` 与宿主 `.chrono-harness/worktree.json`；它抓取已登记目标并保留实际 Git 回执。`reconstruct` 通过逐路径计划在新工作树暂存仍需保留的旧变化；冲突和旧工作均保留，完成后仍须重新提交检查。创建／重建合同及尚未实现的 PR／落地范围见 [worktree 文档](docs/worktree.md)。该二进制已随 beta.8 在 macOS arm64 / Linux x86_64 公开发布；宿主通过锁定的发布安装登记采用。

源码提供显式 opt-in 的自动清理：start/reconstruct 登记新生工作树并重试终态清理，调用方完成／落地且加入任务后使用固定 `chrono-worktree finish --path <工作树>`；独立入口为 `chrono-worktree maintain`，登记消费命令使用 `use --operation` 持有使用保护。主宿主逐项采用已登记输出，保留 state/bin，默认只清缓存；没有定时空闲清理。合同与失败／证据边界见 [自动生命周期清理](docs/worktree.md#opt-in-automatic-lifecycle-cleanup)。

`chrono-worktree recover`／`cleanup`／`cleanup-fetch` 消费宿主 state 下的显式维护计划：保留原失败报告，验证重建后的状态和锁，或核对保留引用与工件白名单后清理指定工作树。它们不产生治理或合并通过判词；完整格式及未实现的恢复范围见 [维护合同](docs/worktree.md#registered-recovery-and-cleanup)。这三个入口已包含在公开 beta.8 中，可由宿主的版本锁定安装入口取得。

公开测试版提供 `chrono-worktree cleanup-remote`，以显式远端地址、提交保留关系及精确 OID lease 清理远端工作分支，保留失败和缺失重试结果。该入口从 beta.10 起公开分发；见 [远端清理合同](docs/worktree.md#remote-branch-retirement)。

公开 beta.8 还提供 `recover-interrupted`：在创建工作树或取得清理锁前保留恢复身份，对缺失／截断结果按显式计划核对当前状态并解锁，原操作结果仍标未知。`cleanup-fetch-interrupted` 还按 fetch 前保留的独立身份清理中断留下的临时 ref，要求明确的当前 OID 与保留分支。这两个入口已包含在公开 beta.8 中；格式与剩余恢复范围见 [中断恢复](docs/worktree.md#interrupted-checkout-recovery)。

beta.11 新增显式 `inspect-rebind`／`rebind` 元数据重建，以及所有现役检查／worktree 消费者共用的原始 checkout 身份核对。真实公开混合宿主已验证：Git 配置隐藏的执行位变化仍被拒绝，损坏索引重建保留未提交工作、显式选择的索引和旧元数据。重建不推断丢失的历史索引，不证明原操作成功；源码新增 `resume-rebind`，按保留 intent 与原计划续跑中断／失败重建，保留原结果与备份；beta.15 开始包含此入口。见[元数据重建](docs/worktree.md#explicit-metadata-rebind)与[原始快照身份](docs/git-facts.md#literal-checkout-identity)。

候选源码增加显式 automatic_cleanup/v2 的内核租约与未 finish 缓存恢复，并为短 check/bootstrap 登记自动参与。本地真实 Cargo test／原生子进程回归验证包装器终止后的租约保留；已提交组合、协调者二进制部署、兼容生产者切换和支持平台验收仍归调用方，尚未宣称完整采用。
