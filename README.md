# chrono-harness

用 Rust 构建的、以显式登记和 DELTA 判官为核心的 harness，附独立的宿主指令生成器。

**已实现指令生成、独立 CI 生成器、本地/CI 共用 check 入口，以及 chrono-judge/v1 运输和 registration、filemap、routes、projects、cost、mixed、workflow 的有界合同。五份完整治理登记仍 proposed；现役 CI 继续使用明确版本化的 slice。**
完整中文合同、数据结构、协议与验收条件见 [SPEC.md](SPEC.md)。

[公开测试版 v0.1.0-beta.1](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.1) 提供 macOS arm64 / Linux x86_64 的原生二进制；宿主无需 Rust，按[发布与安装合同](docs/distribution.md)锁定采用。

公开的独立示例宿主：

| 仓库 | 用途 |
| --- | --- |
| [chrono-harness-examples-go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | Go 生产模块与独立测试模块，显式构建、测试和文件登记 |
| [chrono-harness-examples-ts](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | TypeScript，无 package.json 或 tsconfig，分别登记生产与测试类型检查 |
| [chrono-harness-examples-mix](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | Go、TypeScript、独立 Python 脚本，共享数据通过显式依赖选择测试 |

这三个仓库是测试版安装与本地/原生 CI 的实际验收宿主；覆盖范围和当前限制见 [示例索引](docs/examples.md)。

宿主约束仅放在 [.chrono-harness](.chrono-harness/config.json)。Rust 项目集中在 `crates/`，生产/测试配对为 `runner` / `runner-tests`、`judge-ci` / `judge-ci-tests`、`judge-registration` / `judge-registration-tests`、`judge-filemap` / `judge-filemap-tests`、`judge-routes` / `judge-routes-tests`、`judge-projects` / `judge-projects-tests`、`judge-cargo` / `judge-cargo-tests`、`judge-cost` / `judge-cost-tests`、`judge-mixed` / `judge-mixed-tests`、`judge-workflow` / `judge-workflow-tests`、`inputs` / `inputs-tests`、`distribution` / `distribution-tests`、`ci` / `ci-tests` 和 `instructions` / `instructions-tests`；各自独立 manifest、lockfile、target，无根 workspace。指令生成器保持独立；CI 判官与生成器复用 runner 的通用运输/argv 接口。

```text
crates/                    独立 Rust 生产项目及各自测试项目
  runner/                  判官运输与统一 check 入口
  runner-tests/
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

默认产生含完整中文核心的普通 `CLAUDE.md`，`AGENTS.md -> CLAUDE.md` 是字面相对链接。新宿主可用 `--locale en` 绑定英文；`--methodology M` 保留自定义 UTF-8 方法为 opaque file atom，未声明语言时为 und；`--host-context C` 独立指定上下文。

默认两种语言的完整新根均已实测落在 Codex 默认 32 KiB 项目指令上限内；字节数、有限余量及宿主定制边界见[实际消费者读数](docs/methodology-extraction.md#实际消费者边界)。

日常编辑宿主 catalog 与输出计划，再运行 `chrono-instructions generate --host-root H`。计划可引用共享原子，选择语言并生成任意登记 Markdown 或聚焦 skill。不选布局时按依赖先行的 DFS；可选具名布局显式组织标题和内容，必须完整覆盖非空闭包一次。默认双语共用三部分、12 主题的 `general` 布局，阅读层次不代表权威或执行顺序；聚焦 skill 仍平铺。缺失选中翻译、循环或无效引用均明确失败，无自动翻译或 fallback。可复制 schema 与组合配方见[生成合同](docs/instructions.md)。

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

现役 CI 使用独立的 `judge-ci` 执行显式登记的 DELTA 检查；runner 只承担外部判官运输与报告。独立 `chrono-ci` 从宿主配置生成 GitHub Actions，提供 init/generate/verify 和事件输入准备。

```sh
python3 .chrono-harness/ci/bootstrap.py .
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/github.json
# 候选须已提交且检出干净；本地和 CI 使用完全相同的入口
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID
```

复制二进制初始化新仓、操作扩展、schema、首次采用和事件合同见 [docs/ci.md](docs/ci.md)；可复制完整实例见 [examples/ci-host](examples/ci-host/README.md)。`verify` 检测工作流漂移，不先修复输出。未知路径、缺对象、脏输入、无效登记或失败命令均非零；文档等闭包外变更明确报告未选项目检查。

带 `--context .chrono-harness/state/context.json` 的 full check 调用登记的 v1 外部判官，绑定请求身份、候选二进制摘要、白名单环境与直接前驱 DAG。registration 验证五份结构、固定身份、工作树 dirt、保留输入及受影响引用，并提供唯一历史解释视图。FILEMAP 生成两端 impact v2，并按显式 stability 与 integration 登记选择测试；routes 合并显式执行计划并绑定实际工具；projects 核配对并执行、核对真实回执。现役 scoped CI 复用该规划和执行路径，保留其已注明的选择及输入边界。版本化输出见 [FILEMAP impact 合同](docs/filemap-impact.md) 和 [执行合同](docs/execution.md)。`.chrono-harness/config.json` 仍 proposed、缺完整输入闭包与绑定，完整宿主检查必须非零。完整 SPEC 的生产者、直接测试与缺口见 [覆盖矩阵](docs/spec-coverage.md)。完整 Cargo/SDK 输入闭包、裁决确定性、本地/CI 同判、完整宿主的分支与 integration 接线仍未认证。原生 GitHub 事件须以实际 run 的固定身份和结果验证；本地测试不能替代。

full 报告提供 §9 全部顶层字段与执行物列表，汇集实际 findings 和具名 outputs 并标来源。未配置或未取得的工具、有效输入、影响、测试、成本结果显式为 null，unresolved 给出原因；配置中的版本不冒充实测版本。scope 仍为 configured-judges，parity 为 unestablished，有界 registration/filemap pass 不表示完整治理成功。

Registered routes/projects execution, actual receipts, FILEMAP v2 and the finite historical decoder are documented in [docs/execution.md](docs/execution.md). ci.verify belongs to the real ci/ci-tests pair. The host remains proposed with incomplete Cargo/SDK closure; workflow has bounded certification; autonomous delivery remains future work. Mixed classification and warnings use the [explicit surface contract](docs/mixed.md). Declared costs and unknown warnings have their own [cost contract](docs/costs.md).

稳定性与规则语义变更的测试选择现已接入同一 routes/projects 执行链，见 [workflow 选择合同](docs/workflow-selection.md)。[workflow 判官](docs/workflow.md) 在 context v2 上核对分支新鲜度、实际 integration 证据及显式退休迁移。自动创建 worktree、提 PR、合并与核验落地仍待实现，当前由调用方负责。

显式外部输入可由独立 `chrono-inputs capture/pair` 生成两端快照和保留内容，判官流式核对，无需把大文件字节塞进 JSON。调用及格式见 [输入快照合同](docs/inputs.md)。这不自动发现或补齐 Cargo/SDK 依赖，也不自动启用完整宿主。

测试版统一从本库 [GitHub Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 分发。独立 `chrono-distribution` 按显式清单打包、合并原生平台产物、生成宿主安装登记与入口，验证清单和二进制摘要后安装；宿主无需保留产品源码包或编译 Rust。合同与命令见 [发布与安装](docs/distribution.md)。

通用 projects 登记不要求 manifest、lockfile 或根目录，actions 可使用任意非空登记名称。Cargo workspace/路径依赖检查属于独立可选 [chrono-judge-cargo](docs/cargo-projects.md)；宿主目录和语言不承担隐式登记或选测权威。
