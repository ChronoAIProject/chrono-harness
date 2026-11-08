# 生成发布工作流

`chrono-ci` 按配置中的 schema 选择生成规则。当前 scoped 检查来源为 `chrono-github-ci/v4`；旧 v1／v2／v3 已退役。`chrono-github-release/v1` 生成原生发布构建工作流。生成器只处理明确登记的数据；项目、语言、平台和发布资产都须由宿主列出。它不执行构建或发布。

## 登记与生成

发布源包含 `schema`、`workflow_path`、`name`、`push_branches`、固定提交的 `checkout_action`／`upload_artifact_action`，以及非空 `jobs`。每个 job 登记 `id`、`runs_on`、`timeout_minutes`、`command`（完整 argv）、`artifact_name` 和 `artifact_directory`（以 `/` 结尾的相对目录）。job ID 和产物名各自唯一。目录按字面处理，不是 glob；参数不作 shell 展开。未知／重复字段、无效路径、空命令和配置值中的 GitHub 表达式都会被拒绝。`push_branches` 为空时只能手动触发。

```sh
# 从外部显式配置采用；保留已存在的宿主定制
chrono-ci init --host-root HOST --config RELEASE_SOURCE.json
# 在宿主源修改后重新生成，或只检查漂移
chrono-ci generate --host-root HOST --config .chrono-harness/ci/release.json
chrono-ci verify --host-root HOST --config .chrono-harness/ci/release.json
```

首次 `init` 把源放到宿主 `.chrono-harness/ci/release.json`，后续 init 保留已采用源。generate／verify 的源必须位于宿主 `.chrono-harness/`。输出用独立 `github-release/v1` 标记声明归属，不能覆盖无归属工作流、检查工作流或符号链接。重复生成不写文件，verify 只报漂移、不修文件。写入器逐文件替换，不保证多文件崩溃事务或并发写入；登记改名／删除后，旧工作流须显式退役。

生成的 job 使用登记 runner，检出 `workflow_dispatch` 的 `source` 或 push 提交，再用 Bash 执行原 argv。`source` 是直接交给 checkout 的修订字符串，生成器不保证它是完整 commit OID；构建消费者负责核实际源码身份及验收。参数按字面引用，失败保留原退出码；后续 `always()` 上传尝试保留原产物，产物缺失仍报错。job 独立运行，一个失败不会取消其他 job。工作流仅有 `contents: read`，不创建 Release、合并平台清单或上传发行资产。

新登记应明确设置 `include_hidden_files: true`。v1／v2 都支持，本仓已采用；它让上传保留登记产物根中的隐藏路径，范围仍取 `artifact_directory`。省略或 false 保留原投影及序列化源身份。

## 本仓采用

[发布源](../.chrono-harness/ci/release.json)明确列出 macOS 和 Linux。早期 v1 每平台一个 job，执行同一顺序入口；当前采用 v2。完整本地入口仍为：

```sh
python3 .chrono-harness/release/build.py . .chrono-harness/cache/release-platform
```

Rust、发布资产和验证操作由该宿主入口及配方管理，生成器不解析它们。`ci.release.verify` 是明确登记的宿主操作，已加入 ci-tests 计划；发布源及输出也连接到 distribution-tests。迁移旧手写工作流时先核固定基线字节，生成器不自动接管其他文件。

真实 CLI 和 shell 使用方测试覆盖字面参数、失败原件、定制保留、无写入复用、漂移、错误登记和输出归属。历史原生构建与公开包结果归[原运行](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944)及[发布](distribution.md)，不在这里维护测试数量或安装记录。每次交付须核所选候选、平台和实际使用方；旧结果不能证明当前候选、其他宿主、full 工作流执行、完整输入或确定性同判。

当前本宿主采用 `chrono-github-release/v2` 和 `chrono-release-build/v5`：每平台的 build、verification 和 `always()` collector/package 均逐项登记。每个 job 和整个平台均受登记的 45 分钟时限约束。Rust 验证仍跑原始未过滤操作；测试清单与原生输入适配的验证归 Rust `ci-tests`，宿主 bootstrap、配方单元测试和真实打包集成测试继续各自登记。

每个验证只登记实际消费的构建子集。instructions、routes、diagnostics 不消费发布二进制，`needs` 为空且 `rust_toolchain=true`，可独立启动。原生输入准备的行为验证归 Rust `ci-tests`，随原始未过滤 `test.ci-tests` 执行，复用其既有构建和发布资产依赖，不再单独启动 Python 验证 job。原 IO、执行物解析、凭据省略、纳秒时间顺序和有界失败回执均在 Rust 配对测试中核对；完整种子和转发链仍由 workflow 测试验证。新增验证／传输成本未测，原生两平台验收和公开发行须核对应候选的实际结果。

宿主 Rust bootstrap 的 `test.host-bootstrap-native-tests` 也纳入两平台发布验证。它直接编译源码与 Rust 测试工具，不消费发布二进制，`needs` 为空且 `rust_toolchain=true`；原始未过滤操作保留登记的独立 `--target-dir`。这只新增发布验证 job，日常 scoped 检查仍归现有 `host-bootstrap` 单元，成本未测。

薄 shell 启动入口的 `test.host-bootstrap-launcher-tests` 同样纳入两平台发布验证。它使用独立登记的 shell 命令替身，验证协调者拒绝、保护描述符与参数转交，以及原退出码；不预编译 Rust 测试工具，也不消费发布二进制，`needs` 为空且 `rust_toolchain=false`。发布验证保留原未过滤操作；新增 job 的实际总成本未测。

## v2 的依赖与传输

v1 输出和无依赖行为保留。v2 增加固定提交的 `download_artifact_action`；每个 job 可明确列出 `needs`、`downloads`（生产 job 与以 `/` 结尾的字面目的目录）、`always`、`dependency_metadata`（字面文件）。生成器拒绝重复／未知依赖、循环和下载／输出／metadata 路径重叠，不按语言、命令或资产推断依赖。

上传直接导出 `artifact_id`，独立原始 attempt 步骤导出 `run_id`／`attempt`。产物名包含当前 run／attempt，失败产物不覆盖旧尝试；下载使用选中的原 artifact ID，不按名称找最近产物。`toJson(needs)` 将当前原生依赖结果及选中输出交给宿主，最终 job 将同一原 JSON 写到登记 metadata 路径。其他宿主可登记任意非 Rust 字面命令，这些传输字段没有 Rust 或本仓专用含义。

重复的 v2 下载配置使用 YAML 锚点／别名，且只复用生产 job 与目的目录完全相同的登记。每个消费者仍独立下载，原 artifact ID、条件、目录、`needs`、命令、时限、并发和证据采集不变；实际下载次数不减少。写工作流或首次采用源前，生成器按 500,000 字节保守检查 GitHub 的 500 KB 限额，超限则失败并保全已有文件。

## v5 构建配方

宿主唯一调度入口是 `.chrono-harness/release/build.py`。`build.json` 管理 unit ID、manifest／asset 或原始 operation、依赖、handoff 目录、成本、原生 job 对应、局部并发和时限。`projects.json` 独自管理原测试 argv，`plan.json` 独自管理资产源路径，`consumer_staging` 列全部保留目的路径；不扫描目录、名称或测试正文找单元。

```sh
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT --unit ID
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT --collect
```

只接受 `chrono-release-build/v5`，旧格式在启动工具或创建输出前拒绝。完整本地入口要求固定 HEAD 且检出干净，以有限并发跑同一批登记单元。每单元使用独立临时 clone、固定 OID 和声明 handoff；输出保留原单元证据及本地驱动进程。只删本次创建的临时根，不改源检出。通过后，平台清单和[计划](../.chrono-harness/release/plan.json)全部资产位于 ABSENT_OUTPUT，原 receipt、逐单元过程及完整驱动状态一同保留。已有输出（含 symlink）在子进程前拒绝，结构预检失败不制造成功报告。

build 单元只构建声明 manifest 一次，传输一个执行物及原 receipt／过程流。执行物放在保留 mode 的 tar 中，不放入 JSON 或 64 MiB portable-artifact 容器。摘要／复制分块流式处理，不增加通用容量上限。verification 的 `needs` 必须是明确构建子集，可为空；只下载并核选中资产、消费路径和原生产者身份。

每个 verification 必须声明布尔 `rust_toolchain`。true 安装并核登记 Rust 工具链；false 只绑定 Git 和实际操作工具，不探测／安装 Rust，也不覆盖继承的 Rust 环境变量。选择不能从语言、命令名或依赖数量推断。消费 Rust 二进制不要求 Rust 编译器。Rust 验证须匹配所选构建工具链；无 Rust 验证只记录实际工具和过程，不能冒领编译器观察。collector 逐单元按声明核证据。

本地调度只等待显式前置，并受 `local_workers` 限制；无依赖验证可与构建同时就绪。前置失败保留真实失败结果，collector 仍要求全部登记单元成功。stdout／stderr 保为原文件，receipt 记录字节数／摘要、原 argv、cwd、退出码、工具身份和时间，不虚构相同环境或完整输入。

## 汇总与失败证据

collector 不安装工具链、构建或测试。它核精确 build／test 成员、当前依赖结果和选中原 run／attempt。兼容且成功的前置可来自更早生产 attempt；当前 failure／cancelled／skipped 不能被旧 pass 掩盖。换生产者或字节后，不匹配的测试 receipt 失效。

核过过程原退出／流、原 argv、全部资产观察及生产来源链后，collector 才运行被验向量中的 `chrono-distribution pack`；随后核最终清单、计划中全部包字节／mode、源码及平台。最终 artifact 含完整原 receipt／过程流及明确 artifact-ID 引用，独立 job 的失败 artifact 仍保留。

v5 的 `failure_evidence` 可按 verification operation 登记失败目录。每项是以 `/` 结尾的字面相对目录，须归宿主唯一 untracked artifact；同操作目录不得重复或嵌套。未登记目录不采集。验证失败后，生产者把这些目录的普通文件及原字节摘要写入发布 artifact，如实记录目录缺失、非普通文件、容量省略和复制错误。采集与原生 fixture 共用 64 MiB 内容上限；辅助采集失败不覆盖原 stdout／stderr、退出码或业务失败。路径不能从测试名、语言或输出推断。传输已有证据不证明所有异常都用了标准错误链。

采用缓存的发布单元在失败时，通过配方已有的固定上传 action，单独保留 `.chrono-harness/state/` 诊断原件（含隐藏文件）。诊断 artifact 独立命名，不改变正式发布 artifact 的上传目录、归档根或 collector 下载合同；不再运行额外缓存传输报告或复制原件到正式发布目录。上传结果与业务结果分别核对，上传失败仍使步骤失败。

专属测试覆盖任意非 Rust 字面命令、原生 metadata、独立相邻单元、隔离本地调度、真实 distribution pack、非 UTF-8 失败、缺失／损坏／错误重封装 receipt、路径／mode、字节变化、当前失败和沿用 attempt。实际平台时限、普遍同判或公开采用仍须真实验证；已提交 canonical、两平台原生执行／必要重试、发布和 main/examples 采用各自验收。

## 验证单元实际消费什么

v5 的 `verification_consumers` 为验证登记 `need`（实际调用位置、需求、输出用途）和 `release_assets`。资产集合必须恰好等于 `needs` 对应构建资产，否则 `E_RELEASE_CONSUMPTION`；非空依赖缺说明时，原 receipt 和 stderr 报 `W_RELEASE_CONSUMPTION_UNVERIFIED`。本宿主[配方](../.chrono-harness/release/build.json)全部验证均采用，回归覆盖全集，并变异核验多余／缺失依赖。通用 v5 仍兼容未采用此合同的宿主。`declared` 只证明登记一致；真实用途和最小依赖须检查实际调用、共享 fixture，并在不提供其他发布二进制时执行原未过滤操作，源码搜索不能代替。

缓存传输的 bootstrap 安装路径和发布验证的 staging 路径用途不同。`chrono-cache` 发布资产只暂存到 `crates/cache/target/debug/chrono-cache`，原 cache-tests 在那里调用；`.chrono-harness/bin/chrono-cache` 归 bootstrap，供外层 plan／recover 使用，发布 staging 不再替换它。源资产、摘要、生产者及前后观察仍按 v5 核验。

Python 发布 integration 组保留真实 distribution pack，并加入真实 cache CLI 的 plan→recover→stage→consumer→plan 回归。该组明确增加 `build_cache`，本地 FILEMAP、shared_operations、两原生 job 的 needs／downloads 同步登记，未新增独立单元。回归用同字节真实 CLI 检出 inode 替换，核传输摘要、恢复原件及重新规划的执行物身份；它不模拟原生后端、不同编译配置或完整外围 workflow。失败时将逐进程原 stdout／stderr／argv／退出及配置保留到登记 release-test-failures；原生 collector 仍要求全部真实单元成功。新增构建、下载、回归成本应分别实测，其他登记成本仍为 unmeasured。

发布 runner 标签由[发布登记](../.chrono-harness/ci/release.json)维护。Linux 缓存探测通过[选择器](../.chrono-harness/config.json)读取[发行政策](../.chrono-harness/git/linux-release.json)，只绑定发布消费的 Git 事实和产物声明，不提供 Linux canonical/full 政策。版本和摘要以该登记及[工作树政策](../.chrono-harness/worktree.json)为准，不在操作说明复制本轮主机读数。未知 Git 字节继续在执行前拒绝，候选是否可用仍须实际验证。

编译器一致性比较保留 Rust 版本、代理程序、实际 cargo／rustc 字节与 incremental 声明；Git 自身的身份与原过程仍逐单元核验，源码 commit／tree 仍须一致。Git 不属于编译器指纹，避免同源码、同编译器因 runner 的 Homebrew Git 包装字节不同而被误拒。真实 fixture 回归同时验证不同 Git 包装可消费、同版本但不同编译器字节仍拒绝。`host.bootstrap-tests` 的断言输入释放实际调用 `chrono-harness retention`，因此两平台发布验证同步补登记 `build_runner` 与 `chrono-harness` 暂存依赖；没有跳过原测试。

Linux 发行政策也保留工作树所有者读取登记所需的完整结构，以及真实 custody fixture 使用的 Python 工具槽；Python 版本保持未知（`null`），fixture 自行绑定实际解释器。该登记不声称完成 Linux canonical/full 采用。

Linux 发行政策继承 schema4 必需的 `CHRONO_CHECK_SOURCE`；未设置时仍由现有入口选择 local。`host.bootstrap-tests` 失败时沿用已有有界采集，保留登记的 `bootstrap-custody-tests/` 原件；目录缺失或容量省略仍如实报告。断言同时显示真实退出码和 stdout／stderr 尾部及原路径，不把空 stderr 当作完整错误。

Linux 政策也声明公共依赖图实际引用的工具槽与环境变量，供真实源码 DELTA fixture 解析两端登记；未观察的 Rust 工具版本保持 `null`。这仅补齐引用关系，不代表 Linux canonical/full 输入已经完整绑定。失败诊断格式变化时，同步迁移原有 custody 断言，仍核验主错误、失败状态和原始输出。

发布驱动在子进程运行时分别实时转交原 stdout／stderr，不再等结束后才显示。结束后仍等待两条流读完，再保存原字节、摘要、退出码和回执；转交失败继续收取输出，不能覆盖子进程原失败。这样可以从原生日志定位超时前实际执行到的位置；被强杀前未完成的回执仍不构成成功证据。新增回归用阻塞输入握手验证输出在子进程结束前可见，仍核原失败退出和保留字节；转交的进程／线程和 IO 成本未测。
