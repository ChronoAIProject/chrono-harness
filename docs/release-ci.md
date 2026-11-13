# 生成发布工作流

发布在完整开发路径中的位置见[开发、集成与发布流程](development.md#发布策略与当前自动化)。本文负责发布工作流的生成、构建、公开发行、安装和失败恢复合同；每个候选仍须核对实际原生结果。

`chrono-ci` 按配置中的 schema 选择生成规则。当前 scoped 检查来源为 `chrono-github-ci/v4`；旧 v1／v2／v3 已退役。`chrono-github-release/v2` 生成原生发布工作流，包含登记的持续交付触发；原 v1 只提供构建。生成器只处理明确登记的数据；项目、语言、平台和发布资产都须由宿主列出。它不执行构建或发布。

## 登记与生成

发布源包含 `schema`、`workflow_path`、`name`、`push_branches`、固定提交的 `checkout_action`／`upload_artifact_action`，以及非空 `jobs`。每个 job 登记 `id`、`runs_on`、`timeout_minutes`、`command`（完整 argv）、`artifact_name` 和 `artifact_directory`（以 `/` 结尾的相对目录）。job ID 和产物名各自唯一。目录按字面处理，不是 glob；参数不作 shell 展开。未知／重复字段、无效路径、空命令和配置值中的 GitHub 表达式都会被拒绝。未采用下文 `delivery` 时，`push_branches` 为空表示只能手动触发。

```sh
# 从外部显式配置采用；保留已存在的宿主定制
chrono-ci init --host-root HOST --config RELEASE_SOURCE.json
# 在宿主源修改后重新生成，或只检查漂移
chrono-ci generate --host-root HOST --config .chrono-harness/ci/release.json
chrono-ci verify --host-root HOST --config .chrono-harness/ci/release.json
```

首次 `init` 把源放到宿主 `.chrono-harness/ci/release.json`，后续 init 保留已采用源。generate／verify 的源必须位于宿主 `.chrono-harness/`。输出用独立 `github-release/v1` 标记声明归属，不能覆盖无归属工作流、检查工作流或符号链接。重复生成不写文件，verify 只报漂移、不修文件。写入器逐文件替换，不保证多文件崩溃事务或并发写入；登记改名／删除后，旧工作流须显式退役。

生成的 job 使用登记 runner，检出 `workflow_dispatch` 的 `source` 或 push 提交，再用 Bash 执行原 argv。`source` 是直接交给 checkout 的修订字符串，生成器不保证它是完整 commit OID；构建消费者负责核实际源码身份及验收。参数按字面引用，失败保留原退出码；后续 `always()` 上传尝试保留原产物，产物缺失仍报错。job 独立运行，一个失败不会取消其他 job。未采用 `delivery` 的工作流仅有 `contents: read`，只构建并保留 Actions artifacts。

本仓将 `push_branches` 置空，采用 `delivery` 订阅 dev 的 `chrono / collection` 完成事件；仅成功的 push 运行启动自动测试版。临时 integration 通过 `workflow_dispatch` 的 `source` 和默认 `channel=build` 验证构建，不公开版本。常规变更仍由登记的 PR CI 验收。

新登记应明确设置 `include_hidden_files: true`。v1／v2 都支持，本仓已采用；它让上传保留登记产物根中的隐藏路径，范围仍取 `artifact_directory`。省略或 false 保留原投影及序列化源身份。

## 本仓采用

**PR 分发与宿主发布登记同改：**发布计划新增 `chrono-pr`，两平台各增加 `build_pr`，collector 显式接收其资产及回执；原 `verify_pr_tests` 不增加发布资产依赖。两个生产构建使用现有按平台缓存入口，明确登记 PR 源码、manifest／lock 和 target，恢复后仍执行原构建、封存及保存。未增加平台、权限、测试门或发布控制层；新增构建、传输与存储成本未知。实际公开清单、安装和安装后的 PR 调用须核当前候选的原结果。

[发布源](../.chrono-harness/ci/release.json)明确列出 macOS 和 Linux。早期 v1 每平台一个 job，执行同一顺序入口；当前采用 v2。完整本地入口仍为：

```sh
/bin/bash .chrono-harness/release/native.sh . .chrono-harness/ci/bootstrap-release-frontend.json .chrono-harness/cache/release-platform
```

本仓原生 CI 中订阅编译缓存的发布命令明确传入 `CARGO_HOME=.chrono-harness/cache/bootstrap-owner-tooling/cargo`，与缓存探测及配置文件缺失声明使用同一目录。变量由现有 `/usr/bin/env` 转交给原调度入口，参数、单元及依赖不变；工作目录仍为各实际检出根。普通本地入口继续继承调用环境，不据此宣称与 CI 同判。具体缓存合同见[缓存归属](ci-cache.md#本仓的缓存归属)。

Rust、发布资产和验证操作由该宿主入口及配方管理，生成器不解析它们。`ci.release.verify` 是明确登记的宿主操作，已加入 ci-tests 计划；发布源及输出也连接到 distribution-tests。迁移旧手写工作流时先核固定基线字节，生成器不自动接管其他文件。

真实 CLI 和 shell 使用方测试覆盖字面参数、失败原件、定制保留、无写入复用、漂移、错误登记和输出归属。历史原生构建与公开包结果归[原运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944)及[发布](distribution.md)，不在这里维护测试数量或安装记录。每次交付须核所选候选、平台和实际使用方；旧结果不能证明当前候选、其他宿主、full 工作流执行、完整输入或确定性同判。

当前本宿主采用 `chrono-github-release/v2` 和 `chrono-release-build/v5`：每平台的 build、verification 和 `always()` collector/package 均逐项登记。每个 job 和整个平台均受登记的 45 分钟时限约束。Rust 验证仍跑原始未过滤操作；测试清单与原生输入适配的验证归 Rust `ci-tests`，宿主 bootstrap 与 Rust 发布测试继续各自登记，真实打包和 cache 消费回归由同一未过滤发布测试操作覆盖。

CI、workflow 和 worktree 专属测试项目由 Cargo 从生产 `main.rs` 编译 worktree 测试 CLI。
两平台的 `verify_ci_tests`、`verify_judge_workflow_tests`、`verify_worktree_tests` 不再依赖、下载或暂存
worktree 发布资产；配方的 `needs`、使用方声明与 CI 下载登记同步删除这项无使用方的依赖。
worktree 专属测试仍复制并运行实际 CLI，保留只读安装、所有者发布和中断断言；原测试、分组、线程及限制不变。
Linux 发布 43213816878 与 43223906654 的同一短检查租约用例中，外层原 30 秒期限先到期，随后维护清理缓存，具体阶段成本仍未知。该样本复用现有原生日志，分别为协调宿主和检查宿主保留最多两份 4 MiB 的 debug 日志，并在原缓存断言显示实际维护回执。原件仍由既有失败诊断所有者有限保留；没有新日志机制、执行入口、用例、线程或期限。阶段观察用于定位重复工作，不能单独证明租约错误或修复成功。

诊断输入也会影响保留行为：`CHRONO_WORKTREE_TEST_RECEIPTS` 会为成功夹具登记诊断使用方，不能用于同时断言成功夹具整体回收的原 adoption 单元。该单元沿默认失败保留路径取得日志；显式全量保留造成的回收断言失败须单独记录，不能解释成产品租约泄漏。
正式生产构建、最终打包和真实安装继续消费生产资产。测试根沿用原开发配置，完整发布耗时、传输和维护的净成本收益未测。

CI 专属测试同时统一复用已有的 `chrono-ci-test-cli`，由相同生产 `main.rs` 编译。两平台 `verify_ci_tests` 不再等待、下载或暂存 ci 发布资产；registration 也按下文采用测试 CLI。CI、projects 和 judge-ci 专属测试也分别编译唯一的 judge-ci 生产 `main.rs`，各自发布验证不再依赖、下载或暂存 judge-ci 资产。projects 验证仍消费 filemap、routes 和 projects 的真实发布 CLI。完整上下文、发布命令和生成宿主的原未过滤测试、断言及限制不变；正式 `build_ci`、`build_judge_ci`、其它使用方、最终打包与安装仍保留，完整成本收益未测。

workflow 专属测试也从唯一的 workflow 生产 `main.rs` 编译测试 CLI。两平台 `verify_judge_workflow_tests` 删除 workflow 发布资产的依赖、下载和消费声明，保留其余实际使用的 CLI。生产 `build_judge_workflow`、inputs 验证和最终打包仍消费生产资产；原未过滤测试、profile、断言及限制不变，完整发布成本收益未测。

cache 专属测试直接编译生产 CLI，恢复及封存样本绑定同一执行物的真实身份。两平台 `verify_cache_tests` 不再等待、下载或暂存 cache 发布资产，原未过滤操作保留；native 发布测试与最终打包仍消费生产 cache，完整发布成本收益未测。

runner 专属测试也直接编译生产 `main.rs`，通过 Cargo 提供的路径调用测试 CLI。两平台的 `verify_runner_tests` 不再依赖、下载或暂存 runner 发布资产，仍执行原未过滤操作。worktree 两平台的 `verify_worktree_tests` 也改用 Cargo 从同一生产 `main.rs` 编译的 runner 测试 CLI，删除 runner 发布资产的依赖、下载和消费声明，仍消费 CI 输入程序。发布前端和最终打包继续消费生产 runner；测试根保留原锁、开发配置、未过滤操作及断言，完整发布成本收益未测。

judge-ci 两平台的 `verify_judge_ci_tests` 由测试项目编译 runner 和 judge-ci 两个生产入口，使用 Cargo 提供的 CLI 路径验证真实调用；不再依赖、下载或暂存 runner 发布资产。发布前端及原未过滤验证操作保留，测试根沿用原锁、开发配置、断言、线程和时限；两平台完整发布与公开采用仍须单独验收，完整成本收益未测。

cost 和 mixed 两平台的 `verify_judge_cost_tests`／`verify_judge_mixed_tests` 也由各自测试项目编译 runner 唯一生产入口，共享 Host 使用 Cargo 提供的 CLI 路径；删除这两个验证对 runner 发布资产的前置、下载和消费声明。filemap、cost 和 mixed 发布资产仍供原真实调用消费；发布前端、原未过滤操作、锁、开发配置、断言、线程和时限保留。测试与生产编译配置不同，不声称字节或性能相同；两平台完整发布与安装仍须单独验收，完整成本收益未测。

CI、inputs、registration、filemap、projects、workflow 六项验证也改用各自测试项目从 runner 唯一生产入口编译的 CLI。两平台删除这六项验证的 runner 资产依赖、下载和暂存声明，其余发布资产仍供原调用消费。原 CLI 身份绑定、复制、探测、未过滤操作、断言、profile、线程和时限保留；生产 `build_runner`、发布前端、最终打包与安装仍按原登记执行。测试配置不证明发布字节或性能相同，完整原生发布与安装另行验收，完整成本收益未测。

distribution 专属测试由 Cargo 从生产 `main.rs` 编译测试 CLI，再经真实安装入口核对字节、权限和回执。两平台的 `verify_distribution_tests` 不再依赖、下载或暂存 distribution 发布资产，原未过滤操作与断言保留。生产 `build_distribution`、最终打包和实际安装仍消费发布资产；完整发布成本收益未测。

inputs 专属测试由 Cargo 从生产 `main.rs` 编译测试 CLI。两平台的 `verify_inputs_tests` 移除 inputs 发布资产的依赖、下载和消费声明，registration、mixed 与 cost 按下文采用测试 CLI，其余四个生产判官仍供真实调用消费，执行原未过滤操作。生产 `build_inputs`、其他真实使用方和最终打包保留；完整发布成本收益未测。

CI、registration、cost、mixed、filemap、projects、inputs、workflow 八项验证均由各自测试项目从 registration 唯一生产 `main.rs` 编译测试 CLI，保留原复制、探测、身份绑定及真实协议调用。两平台删除这些验证的 registration 发布资产依赖、下载和暂存声明，其余实际消费的资产保留。生产 `build_judge_registration`、最终打包和安装继续消费发布资产。原未过滤操作、断言、锁、开发配置、线程、分组和时限保留；测试构建不证明发布字节相同，两平台完整发布与安装另行验收，完整成本收益未测。

inputs、workflow 和 mixed 三项验证也各自从 mixed 唯一生产 `main.rs` 编译测试 CLI。两平台删除这三项验证的 mixed 发布资产依赖、下载和消费声明；生产 `build_judge_mixed`、最终打包及安装继续消费该资产。原未过滤操作、锁、开发配置、断言、线程、分组和时限保留；测试构建不证明发布字节或性能相同，完整原生发布与安装另行验收，完整成本收益未测。

cost、mixed、inputs 和 workflow 四项验证也各自从 cost 唯一生产 `main.rs` 编译测试 CLI。两平台删除这四项验证的 cost 发布资产依赖、下载和消费声明；生产 `build_judge_cost`、最终打包与安装继续消费该资产。原未过滤操作、锁、开发配置、断言、线程、分组和时限保留；测试构建不证明发布字节或性能相同，两平台完整发布与安装另行验收，完整成本收益未测。

每个验证只登记实际消费的构建子集。instructions、routes、diagnostics 不消费发布二进制，`needs` 为空且 `rust_toolchain=true`，可独立启动。原生输入准备的行为验证归 Rust `ci-tests`，随原始未过滤 `test.ci-tests` 执行，复用其既有构建和发布资产依赖，不再单独启动 Python 验证 job。原 IO、执行物解析、凭据省略、纳秒时间顺序和有界失败回执均在 Rust 配对测试中核对；完整种子和转发链仍由 workflow 测试验证。新增验证／传输成本未测，原生两平台验收和公开发行须核对应候选的实际结果。

宿主 Rust bootstrap 的 `test.host-bootstrap-native-tests` 也纳入两平台发布验证。它直接编译源码与 Rust 测试工具，不消费发布二进制，`needs` 为空且 `rust_toolchain=true`；原始未过滤操作保留登记的独立 `--target-dir`。这只新增发布验证 job，日常 scoped 检查仍归现有 `host-bootstrap` 单元，成本未测。

薄 shell 启动入口的 `test.host-bootstrap-launcher-tests` 同样纳入两平台发布验证。它使用独立登记的 shell 命令替身，验证协调者拒绝、保护描述符与参数转交，以及原退出码；不预编译 Rust 测试工具，也不消费发布二进制，`needs` 为空且 `rust_toolchain=false`。发布验证保留原未过滤操作；新增 job 的实际总成本未测。

## v2 的依赖与传输

v1 输出和无依赖行为保留。v2 增加固定提交的 `download_artifact_action`；每个 job 可明确列出 `needs`、`downloads`（生产 job 与以 `/` 结尾的字面目的目录）、`always`、`dependency_metadata`（字面文件）。生成器拒绝重复／未知依赖、循环和下载／输出／metadata 路径重叠，不按语言、命令或资产推断依赖。

上传直接导出 `artifact_id`，独立原始 attempt 步骤导出 `run_id`／`attempt`。产物名包含当前 run／attempt，失败产物不覆盖旧尝试；下载使用选中的原 artifact ID，不按名称找最近产物。`toJson(needs)` 将当前原生依赖结果及选中输出交给宿主，最终 job 将同一原 JSON 写到登记 metadata 路径。其他宿主可登记任意非 Rust 字面命令，这些传输字段没有 Rust 或本仓专用含义。

重复的 v2 下载配置使用 YAML 锚点／别名，且只复用生产 job 与目的目录完全相同的登记。每个消费者仍独立下载，原 artifact ID、条件、目录、`needs`、命令、时限、并发和证据采集不变；实际下载次数不减少。写工作流或首次采用源前，生成器按 500,000 字节保守检查 GitHub 的 500 KB 限额，超限则失败并保全已有文件。

## v5 构建配方

宿主唯一完整发布入口是 `.chrono-harness/release/native.sh`，它委托现有 Rust bootstrap 构建并安装 `chrono-host-release`，再直接运行该程序。配方解析、调度、状态与原件核验均由 `.chrono-harness/release/native/` 的 Rust 项目负责。`build.json` 管理 unit ID、manifest／asset 或原始 operation、依赖、handoff 目录、成本、原生 job 对应、局部并发和时限。`projects.json` 独自管理原测试 argv，`plan.json` 独自管理资产源路径，`consumer_staging` 列全部保留目的路径；不扫描目录、名称或测试正文找单元。

FILEMAP 将构建配方、资产计划和项目动作表登记为发布程序的运行输入，将发布 CI 源登记为工作流生成器的运行输入。各自的测试边另行保留；测试引用不能代替实际生产使用关系。

```sh
/bin/bash .chrono-harness/release/native.sh ROOT .chrono-harness/ci/bootstrap-release-frontend.json ABSENT_OUTPUT
```

原生发布每平台先由登记的 frontend job 执行同一薄入口，并以 `--export-frontend` 导出工具。所有业务单元与 collector 都显式依赖、下载该 job 的原 artifact ID，再通过 `.chrono-harness/release/consume.sh ROOT FRONTEND_DIRECTORY ABSENT_OUTPUT --unit ID` 或同一入口的 `--collect` 模式运行。薄 shell 只从 tar 恢复固定执行物及其权限；Rust 在业务执行前核对源码、当前依赖、工具路径与字节、原 bootstrap 报告、原操作和两条输出流。非 Rust 业务单元和 collector 无需编译器。

单元回执保留原 frontend 元数据；collector 核该单元实际使用的生产 attempt，并收取原元数据，不为每个单元重复复制工具或归档。frontend 失败或下载／原件不匹配时，不能执行原业务。新增构建、下载及元数据核验的总成本未测；本地回归、两平台原生运行、公开发行与宿主采用分别验收。

只接受 `chrono-release-build/v5`，旧格式在启动工具或创建输出前拒绝。完整本地入口要求固定 HEAD 且检出干净，以有限并发跑同一批登记单元。每单元使用独立临时 clone、固定 OID 和声明 handoff；输出保留原单元证据及本地驱动进程。只删本次创建的临时根，不改源检出。通过后，平台清单和[计划](../.chrono-harness/release/plan.json)全部资产位于 ABSENT_OUTPUT，原 receipt、逐单元过程及完整驱动状态一同保留。已有输出（含 symlink）在子进程前拒绝，结构预检失败不制造成功报告。

build 单元只构建声明 manifest 一次，传输一个执行物及原 receipt／过程流。执行物放在保留 mode 的 tar 中，不放入 JSON 或 64 MiB portable-artifact 容器。摘要／复制分块流式处理，不增加通用容量上限。verification 的 `needs` 必须是明确构建子集，可为空；只下载并核选中资产、消费路径和原生产者身份。

每个 verification 必须声明布尔 `rust_toolchain`。true 安装并核登记 Rust 工具链；false 只绑定 Git 和实际操作工具，不探测／安装 Rust，也不覆盖继承的 Rust 环境变量。选择不能从语言、命令名或依赖数量推断。消费 Rust 二进制不要求 Rust 编译器。Rust 验证须匹配所选构建工具链；无 Rust 验证只记录实际工具和过程，不能冒领编译器观察。collector 逐单元按声明核证据。

本地调度只等待显式前置，并受 `local_workers` 限制；无依赖验证可与构建同时就绪。前置失败保留真实失败结果，collector 仍要求全部登记单元成功。stdout／stderr 分流实时转交并保为原文件，每条流以 64 MiB 为上限；超限记录为失败并保留已收取原字节。receipt 记录字节数／摘要、原 argv、cwd、退出码、工具身份和时间，不虚构相同环境或完整输入。

## 汇总与失败证据

collector 不安装工具链、构建或测试。它核精确 build／test 成员、当前依赖结果和选中原 run／attempt。兼容且成功的前置可来自更早生产 attempt；当前 failure／cancelled／skipped 不能被旧 pass 掩盖。换生产者或字节后，不匹配的测试 receipt 失效。

核过过程原退出／流、原 argv、全部资产观察及生产来源链后，collector 才运行被验向量中的 `chrono-distribution pack`；随后核最终清单、计划中全部包字节／mode、源码及平台。最终 artifact 含完整原 receipt／过程流及明确 artifact-ID 引用，独立 job 的失败 artifact 仍保留。

v5 的 `failure_evidence` 可按 verification operation 登记失败目录。每项是以 `/` 结尾的字面相对目录，须归宿主唯一 untracked artifact；同操作目录不得重复或嵌套。未登记目录不采集。验证失败后，生产者把这些目录的普通文件及原字节摘要写入发布 artifact，如实记录目录缺失、非普通文件、容量省略和复制错误。采集与原生 fixture 共用 64 MiB 内容上限；辅助采集失败不覆盖原 stdout／stderr、退出码或业务失败。路径不能从测试名、语言或输出推断。传输已有证据不证明所有异常都用了标准错误链。

采用缓存的发布单元在失败时，通过配方已有的固定上传 action，单独保留 `.chrono-harness/state/` 诊断原件（含隐藏文件）。诊断 artifact 独立命名，不改变正式发布 artifact 的上传目录、归档根或 collector 下载合同；不再运行额外缓存传输报告或复制原件到正式发布目录。上传结果与业务结果分别核对，上传失败仍使步骤失败。

专属测试覆盖任意非 Rust 字面命令、原生 metadata、独立相邻单元、隔离本地调度、真实 distribution pack、非 UTF-8 失败、缺失／损坏／错误重封装 receipt、路径／mode、字节变化、当前失败和沿用 attempt。实际平台时限、普遍同判或公开采用仍须真实验证；已提交 canonical、两平台原生执行／必要重试、发布和 main/examples 采用各自验收。

## 验证单元实际消费什么

registration 的 Git facts 用例通过已有输入快照的 `{blob, sha256, length}` 引用保留完整测试执行物和其他声明输入，不再为两端快照把执行物逐字节展开成 JSON 数组。独立原字节、端点配置、摘要、长度、缺失状态和漂移检查仍保留；有界数据的内联／blob 等价性及坏原件拒绝由原 registration 用例覆盖。共享 Git 模型、原用例、断言、默认线程、profile、时限和发布依赖不变，没有新增依赖或传输格式。

该调整针对发布运行 43213816878 的 Linux registration 验证在四项 Git facts 用例超过 60 秒后收到 runner shutdown、退出 143 的观察；同症状也见于 43205633182。原运行没有给出系统关闭的具体原因，仍保持 unknown，不能由后续通过反推根因。JSON 展开是已确认的输入准备成本，完整时间／内存收益及两平台原生发布仍须实测；本地通过不能代替公开发行与安装验收。

v5 的 `verification_consumers` 为验证登记 `need`（实际调用位置、需求、输出用途）和 `release_assets`。资产集合必须恰好等于 `needs` 对应构建资产，否则 `E_RELEASE_CONSUMPTION`；非空依赖缺说明时，原 receipt 和 stderr 报 `W_RELEASE_CONSUMPTION_UNVERIFIED`。本宿主[配方](../.chrono-harness/release/build.json)全部验证均采用，回归覆盖全集，并变异核验多余／缺失依赖。通用 v5 仍兼容未采用此合同的宿主。`declared` 只证明登记一致；真实用途和最小依赖须检查实际调用、共享 fixture，并在不提供其他发布二进制时执行原未过滤操作，源码搜索不能代替。

缓存传输的 bootstrap 安装路径和发布验证的 staging 路径用途不同。`chrono-cache` 发布资产只暂存到 `crates/cache/target/debug/chrono-cache`，原 cache-tests 在那里调用；`.chrono-harness/bin/chrono-cache` 归 bootstrap，供外层 plan／recover 使用，发布 staging 不再替换它。源资产、摘要、生产者及前后观察仍按 v5 核验。

Rust `test.host-release-native-tests` 运行全部发布行为测试。真实 distribution pack 使用测试 crate 自行编译的 distribution alias；真实 cache 回归消费暂存到 `crates/cache/target/debug/chrono-cache` 的选中发布资产，并执行 plan→recover→stage→consumer→plan。因此发布验证只依赖 `build_cache`，不依赖发布版 distribution。本地 FILEMAP、shared_operations 和两平台 needs／downloads 同步登记；原两组 Python 验证由这一未过滤 Rust 操作替代。回归核传输摘要、恢复原件及重新规划的执行物身份，不能代替不同编译配置或完整外围 workflow 的验收。失败时将逐进程原 stdout／stderr／argv／退出及配置保留到登记 release-test-failures，collector 仍要求全部真实单元成功。

发布行为测试在创建临时宿主的 CLI／launcher 子进程时，统一移除外层发布的十个控制变量及四个 GitHub 工作流变量，再显式设置该样本的运行、依赖等输入。否则真实 preview 的源码 SHA 或工作流上下文会污染临时仓库，触发正确的身份拒绝。普通 CI 中的 Rust 命令替身回归模拟 build／preview／stable 父环境，核对清除继承值、保留显式输入与其他环境；它不调用 Git 或编译器，也不代替两平台原始未过滤测试与公开发布验收。

frontend 专属用例的 Git 和探针在命令边界复用只依赖标准库的 `context.rs`，严格核只读源码查询和探针参数；固定源码样本由替身返回，不初始化仓库。需要对象、索引和源码变更的 execution／staging 用例继续使用 runner 的共享 Git 模型。执行用输入硬链接已完成编译且不被测试改写的程序，内容变异样本独立解包，避免测试父进程把可执行 inode 的写描述符带入并发子进程。launcher 边界由 Rust 替身核原薄脚本摘要、唯一归档成员及字面参数，在独立进程中恢复执行权限并转交所有权描述符后 exec；测试不执行 shell、系统 tar 或 mv。真实薄脚本、系统归档和发布程序执行须由两平台生产入口验收。启动前保存意图，启动失败保存原错误和系统错误码，不能伪造子进程退出或输出。原断言、默认线程、未过滤操作和期限不变；该准备方式排除一类写入者竞争，不声称已定位旧 `ETXTBSY` 的具体描述符持有者。launcher 替身继续复用 `release_tool.rs`。此前 Linux 原件中的 Git 模型执行物为 46,686,040 字节，标准库替身为 4,658,392 字节，两个 frontend 样本耗尽原 30 秒时限；替换复用现有只读接口，去掉该样本不消费的对象／索引准备，不修改开发配置。字节数来自运行 43193884495 的失败原件，最终耗时和两平台发布效果仍须实际验收。

发布 runner 标签由[发布登记](../.chrono-harness/ci/release.json)维护。Linux 缓存探测通过[选择器](../.chrono-harness/config.json)读取[发行政策](../.chrono-harness/git/linux-release.json)，只绑定发布消费的 Git 事实和产物声明，不提供 Linux canonical/full 政策。Git 按所选政策一次解析并固定本轮实际身份；显式配置版本或摘要预期时，不符仍在执行前拒绝。运行读数归 state／线上原件，候选是否可用仍须实际验证。

**发布程序与判定政策同时修改：**编译器一致性比较保留 Rust 版本、实际 cargo／rustc 字节与 incremental 声明。命令执行物的摘要、长度和权限都等于本单元 rustup 时，将它作为 rustup 代理，跨单元按已核原 `rustup which --toolchain` 输出的实际编译器比较；直接编译器得到同一身份，自定义包装程序继续比较原字节。Git、rustup 和每次调用的工具身份及原过程仍逐回执核验，源码 commit／tree 仍须一致；它们不因此取得完整输入或跨环境同判保证。

运行 43205633182 的 Linux 构建／验证使用相同 cargo／rustc 版本及字节，但 runner 的两种 rustup 代理分别为 21,139,856 和 21,347,088 字节，旧跨单元比较在业务验证前拒绝。原失败、原代理观察和实际编译器观察继续保留。Rust 自含回执回归先检出旧拒绝，再验证代理变更、直接编译器可消费，自定义包装或实际编译器变化仍拒绝；既有 Git 包装差异与原件完整性回归保留。没有新增依赖、执行入口或预算，新增验证及完整原生发布成本未测，两平台和公开安装仍须实际验收。

旧 Python `host.bootstrap-tests` 及其专属发布验证单元随启动栈退役，对应构建依赖、下载和暂存消费声明同步移除；现役 Rust bootstrap 与薄 shell 测试仍独立执行。

Linux 发行政策保留工作树所有者读取登记所需的完整结构。发布编排和登记验证操作使用 Rust 与薄 shell，不再在发布配方或宿主政策中保留无对应操作的 Python 工具槽。多语言接口测试样本使用各自的工具声明；该登记不声称完成 Linux canonical/full 采用。

Linux 发行政策继承 schema4 必需的 `CHRONO_CHECK_SOURCE`；未设置时仍由现有入口选择 local。旧 Python custody 测试的发布采集声明随生产者退役；已保留失败原件仍按原使用方的诊断和验收期限保护，不因删除源码或声明而删除。

Linux 政策也声明公共依赖图实际引用的工具槽与环境变量，供真实源码 DELTA fixture 解析两端登记；未观察的 Rust 工具版本保持 `null`。这仅补齐引用关系，不代表 Linux canonical/full 输入已经完整绑定。失败诊断格式变化时，同步迁移原有 custody 断言，仍核验主错误、失败状态和原始输出。

发布驱动在子进程运行时分别实时转交原 stdout／stderr，不再等结束后才显示。结束后仍等待两条流读完，再保存原字节、摘要、退出码和回执；转交失败继续收取输出，不能覆盖子进程原失败。这样可以从原生日志定位超时前实际执行到的位置；被强杀前未完成的回执仍不构成成功证据。新增回归用阻塞输入握手验证输出在子进程结束前可见，仍核原失败退出和保留字节；转交的进程／线程和 IO 成本未测。

## 持续交付与恢复

### 正式版复用最终测试版

正式发布通过同一 `release.deliver` 入口晋升，显式提供 `preview_run` 和 `preview_attempt`。它们点名已经完成且成功的最终测试版发布运行；缺少、过期或损坏的原件阻止晋升，不自动回退为重建。正式版仍使用同一完整源码、原 dev CI、计划版本和固定测试版标签作为工作流 ref。

`delivery.promotion_build_jobs` 逐项登记正式版不再重复执行的作业。本宿主选择原两平台的 76 个构建、验证和 collector 作业；测试版与 build 通道保留全部原操作、时限、下载和产物。正式版重新执行两平台 frontend、现有发布者和两平台安装，共 5 个作业。publisher 可在正式通道消费明确跳过的构建依赖，但仍核当前 Linux frontend 成功；跳过不是测试通过，也不生成替代构建回执。

发布者通过固定 attempt API 核来源运行的仓库、工作流 ID／路径、事件和成功终态，再用原工作流 head 与候选的工作流 blob 核执行合同。原 publisher artifact 名来自当前登记和显式 run／attempt；按精确名称查询必须只有一个未过期工件；下载前后均核同一不可变 ID 及完整元数据，名称在期间被重建也须失败。平台 artifact ID 和实际生产 attempt 来自该原发布回执的 `dependencies`，仍逐项核原包的源码、测试版身份、collector、原始输出、工具字节及原平台跨度；不会把沿用的 attempt 1 改成完成运行的 attempt 2。

原测试版的公开状态、完整资产、标签源码和原 dev CI 由发布者既有的只读判法复核；平台包成员须与该公开测试版相同。既有 distribution `assemble` 接受成对的 `--from-version`／`--version`，核输入的精确版本与原字节，在新的原子输出中生成正式清单和安装入口。输入清单、二进制与回执保持原样。正式版回执另记 `chrono-release-promotion/v1`，引用原 run、工作流观察、工件元数据和原回执；导入原件归入本次 publication artifact，沿用其有限保留政策。正式公开和两平台 HTTPS 安装继续执行原不可覆盖、原进程与安装字节合同。

**程序与宿主政策同时变化：**新增显式晋升输入及正式作业选择，保留测试版全部构建和验证、正式版安装及公开身份合同。Rust 回归在外部命令边界 mock，核成功晋升、跨 attempt 选择、失败来源、工作流漂移、过期／错工件、下载失败、原件损坏、公开资产冲突和原 CI 不合格。它不能替代原生发布与四下游采用。新增下载、重复归档、维护和整体时间成本尚未测；与 v0.1.2 对照时分别报告实际执行作业、原生产耗时、传输量和运行跨度，不把并行耗时之和当 CPU 或计费时间。

| 问题与负责处 | 生产者、输入和使用方 | 替代方案与验收 |
| --- | --- | --- |
| 测试版与正式版重复产生相同工具；原发布者负责晋升 | 成功原发布运行及其不可变工件，供正式发布维护者与四个下游使用 | 单凭源码 SHA 不够；保留原件身份并核新公开版与真实安装 |
| 组包器要求同版本，无法产生正式清单；distribution 负责新的输出 | 原平台包与显式输入／输出版本，供现有 publisher 使用 | 不改写原包；行为回归核字节不变与失败时无输出 |
| 原工作流仍会重复构建；chrono-ci 只投影明确登记 | 宿主 build job 列表与 channel，供 GitHub 执行 | 不按名称猜角色；测试版保留全部作业，正式运行核实际选择及原件 |

本批同时修改产品程序和宿主政策：`chrono-ci` 投影触发、固定检出和作业权限；Rust 发布程序负责版本、原件和 GitHub 状态；宿主的 [delivery.json](../.chrono-harness/release/delivery.json) 选择仓库、落地工作流 ID／路径／分支、平台包目录、frontend 和实际工具。新增一个发布作业，复用两平台 collector 与 Linux frontend；测试版与 build 通道的构建及测试保持原命令和时限。新增 IO、作业和存储的总成本未知。

v2 可选 `delivery` 明确登记 `workflow`、`branch` 和 `publisher`。它与直接 push 触发互斥。所有作业检出原完成事件的 `head_sha`；只有发布作业取得 `contents: write` 和 `actions: read`，并接收当前 token。其余作业维持只读。手动输入 `channel` 可为 build、preview 或 stable；公开渠道还须提供固定 `source`、`verified_run` 和 `verified_attempt`。stable 另需匹配源码计划的 `version`，以及最终测试版的 `preview_run`／`preview_attempt`，依上文晋升合同消费已验收原件。

**发布程序与宿主政策同时修改：**宿主 `delivery.json` 的 `publication.branch` 选择测试版原 CI 分支 `dev`，必需字段 `publication.stable_branch` 选择正式版分支 `main`，两者须不同。正式版在任何 Release 写入前，经现有 GitHub 命令边界读取实际分支引用，核 `ref`、commit 类型及完整源码 OID，并在发布结果保留原 `release_branch`；缺失、不符或 API 失败均停止。测试版继续只核 dev 原 CI。晋升与正式版安装合同归[发布迭代](release-iteration.md#推进正式版分支)。验证增加 Rust 命令 mock 的成功、拒绝与测试版隔离回归，复用现有发布测试和 canonical 检查；每次正式发布增加一个只读 API 请求，完整验证与维护成本未测。

GitHub 的执行工作流由派发的 `--ref` 决定，`source` 只决定各作业检出的源码。派发前核两个固定修订的发布源、needs／downloads 与构建配方是否匹配；手动正式发布统一用已核验的最终测试版标签作为 `--ref`，标签须解析到完整 source，新 run 的工作流 head 也须一致。不要移动标签。自动 `workflow_run` 使用默认分支的工作流，列表 head 与发行源码可能不同；仅看到 head 不同不能判失败，但实际执行图与候选合同不符时，不能宣称该运行验收了候选。

GitHub 原生 frontend 在导出前读取 `GITHUB_REPOSITORY`、`GITHUB_WORKFLOW_REF` 和 `GITHUB_WORKFLOW_SHA`，核路径等于候选发布源的 `workflow_path`，再用现有全量 Git 检出比较两个固定提交的工作流 blob。提交不同但该文件字节相同可继续；文件不同、绑定缺失或 Git 读取失败则停止导出，业务单元无法取得 frontend。原进程、退出和两条输出流由现有发布执行器记录在 frontend artifact 的 `workflow.json` 及 `processes/`；通过时，`frontend.json.workflow` 保留工作流提交、候选提交、路径和共同 blob。GitHub 外的本地导出将该字段记为 null，不声称验证了远端执行图。此检查复用现有权限和 `fetch-depth: 0`，每平台最多两个固定 blob 读取；新增时间、IO 及整体成本尚未测量。

**发布程序与宿主输入登记同时修改：**frontend 新增读取现役发布源的运行输入边，现有命令 mock 回归覆盖相同源码、无关前移、不同 blob、缺绑定和原 Git 失败；临时宿主同时清除继承的 GitHub 工作流变量，只使用样本显式输入。新候选仍须 canonical、两平台原生导出与全部发布验证，以及真实下游采用；本地 mock 通过不代替这些结果。

发现派发输入不符合本意时，先保留固定修订差异及原运行，终止尚未公开的错误运行，等待最终状态并核发布作业及公开状态，再从匹配 ref 派发。取消后的 collector 可能因缺少 artifact_id 报错；保留真实错误，不把它归成已执行产品测试的回归。这个恢复使用原发布入口和现有权限，不降低必需验证，也不与原运行并行争用同一正式版本。

preview 版本由计划版本核心、原 CI run／attempt 和完整源码提交组成，例如 `v0.1.0-dev.1234.2.g<40位SHA>`。相同原 CI 身份重试得到同一版本；不同源码或 CI attempt 得到不同版本。单元、验证、collector 与 publisher 核同一身份。普通 build 使用计划版本，走完平台原件核验及组合打包后保留 Actions artifacts，不调用公开 Release API。

发布者核对当前依赖选择的平台原报告、完整原件摘要和全部资产；公开前另核原 CI 的工作流 ID、路径、push 事件、dev 分支、完整源码、attempt 和真实成功状态。它复用受验的 `chrono-distribution assemble`，输出固定版本的 release.json、install.sh 及平台二进制；随后创建 draft，上传缺少的资产，核 GitHub 返回的 size／sha256 后公开，并重新读取 Release、资产与实际标签源码。

恢复只继续同一版本、源码和资产身份：已有相同 draft 可以续传，已公开且完全一致的版本可核验复用；冲突、公开版缺资产、原 CI 不合格或上传失败均停止，不使用 clobber，也不把失败 draft 当公开交付。过程原流、请求正文与结果保留在发布作业 artifact，沿用已有有限产物保留与清理所有者。

重跑前先收取原运行的最终状态、失败步骤、原日志和可取得的回执。缓存准备、原件传输、业务执行和证据上传分别核对；业务回执通过不能覆盖失败的 job，下载失败也不能说成已经运行的测试失败。确认是可恢复的外部故障后，沿固定源码的显式 `needs` 选择失败作业及受影响的后续作业，复用有效的成功原件。原生部分重跑可能给沿用结果分配新 job ID 和当前 `run_attempt`；实际生产身份仍由选中的 artifact ID、原回执和原生产 attempt 决定，不能按新 job ID 拼产物名。列举 artifact 时读取完整分页。

部分重跑不会重置已选原件的生产时间。collector 从选中单元原回执的最早 `started_ns` 计整个平台跨度，仍受构建配方的 `platform_seconds` 约束；本宿主当前为 2700 秒。恢复前核对剩余窗口和受影响依赖。只重跑 collector 无法挽救已经过窗的输入；全部原生产者已过窗时，从该平台 frontend 重跑其完整依赖闭包，保留独立且有效的另一平台结果。原失败和时间戳继续保留，不改写时间、不提高预算，也不将未通过的包交给发布者。产品或登记修复则产生新源码候选，重新走测试版和下游验收。

控制面状态与日志可能暂时不一致：作业日志已结束时，run 仍可能为 `in_progress`，取消接口也可能返回 409；409 不证明已经取消。观察 API 的 502 或等待通道中断也不证明生产失败。保留原 run／attempt／会话及已取得的响应，接续同一运行的原生只读等待；取得真实终态后才选择恢复，不重复派发正式发布。等待和原件消费边界沿用[发布迭代流程](release-iteration.md)。

正式公开入口为 `release.deliver`，需要已下载且验证的 frontend、登记平台包、当前 needs 元数据和原 CI 身份。日常由生成的发布工作流提供这些输入；不会用本地测试结果替代原生平台、公开资产或真实安装验收。

本批也同时修改安装执行程序与宿主政策：发布作业之后，Linux／macOS 分别执行登记的 `release.install-linux`／`release.install-macos`，消费当前成功依赖选中的原发布产物与对应原生 frontend。构建模式从本次组合包采用并安装；公开通道执行本次发布绑定的 `install.sh`，从该固定版本的 HTTPS 地址下载。安装到新的临时宿主，核对安装器实际产生的源码、版本、平台、完整工具成员及安装字节和执行权限。外部命令失败、发布原件不符、旧源码或错误安装回执都不能通过。原命令输出、退出和安装回执进入各平台的有限保留 artifact；安装二进制留在 runner 临时缓存，不另上传副本。新增两个原生作业及其下载、校验、安装成本尚未测量。

发布进程回执由实际执行者记录逻辑工具名及解析后的 argv。安装端先核完整原流、原退出与引擎结果，再复用发布者的 API 判法：公开通道中，仅当前仓库／版本的 release 或 tag 的精确 GET 查询可接受退出 1、字符串状态 `404` 和 `Not Found`。首次发布时尚无 release／tag 属于正常观察；上传失败、其他接口／版本、403／5xx、信号、引擎失败及改写的原件继续拒绝。真实 GitHub 响应作为独立 Rust 测试输入登记；构建模式仍要求全部原进程成功。这只修复安装前的误判，两平台公开安装仍须分别取得真实回执。

验收自动触发、实际公开发布和两平台安装时，须分别核对对应固定源码的原生运行、公开资产和安装回执。
