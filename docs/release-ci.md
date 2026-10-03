# 显式发布工作流生成

`chrono-ci` 的 `init`、`generate`、`verify` 按配置中明确的 schema 选择生成合同。既有 `chrono-github-ci/v1`、v2、v3 仍生成检查工作流；`chrono-github-release/v1` 生成原生发布构建工作流。生成器不发现项目、语言、平台或待发布资产，不执行构建或发布。

发布源包含 `schema`、`workflow_path`、`name`、`push_branches`、固定提交的 `checkout_action`／`upload_artifact_action` 和非空 `jobs`。每个 job 明确登记 `id`、`runs_on`、`timeout_minutes`、`command`（完整 argv）、`artifact_name`、`artifact_directory`（以 `/` 结尾的相对目录）。job ID、产物名必须分别唯一；目录不是 glob，参数不作 shell 展开。拒绝未知／重复字段、无效路径、空命令和配置值内的 GitHub 表达式。空 `push_branches` 表示只允许手动触发。

```sh
# 从外部显式配置采用；保留已存在的宿主定制
chrono-ci init --host-root HOST --config RELEASE_SOURCE.json
# 在宿主源修改后重新生成，或只检查漂移
chrono-ci generate --host-root HOST --config .chrono-harness/ci/release.json
chrono-ci verify --host-root HOST --config .chrono-harness/ci/release.json
```

`init` 将首次采用源放在宿主 `.chrono-harness/ci/release.json`；后续 init 保留已采用源。generate／verify 的源必须在宿主 `.chrono-harness/`。输出带独立的 `github-release/v1` 所有权标记，不能覆盖未归属工作流、已有检查工作流或符号链接。重复生成无写入，verify 不修复漂移。共用的文件写入器逐文件替换，不提供多文件崩溃事务或并发写者保证；改名／删除登记不自动删除旧工作流，须显式退休。

生成的每个 job 使用登记 runner，检出 `workflow_dispatch` 的 `source` 或 push 的提交，然后以 Bash 执行原始 argv。`source` 是交给 checkout 的修订字符串；生成器不验证其为完整 commit OID。实际源码身份及验收由登记构建消费者核验。所有参数以字面方式引用；失败命令保留原退出码，后续 `always()` 上传步骤尝试保存原始产物，缺少产物仍是错误。各 job 独立运行，不因一个失败而取消其它 job。工作流只有 `contents: read` 权限，不创建 Release、不合并平台清单，也不上传发行资产。

本仓的 [发布源](../.chrono-harness/ci/release.json) 明确列 macOS 与 Linux 平台；v1 时各使用一个 job 和同一顺序入口，v2 当前采用路径见下文。完整本地入口继续为：

```sh
python3 .chrono-harness/release/build.py . .chrono-harness/cache/release-platform
```

Rust、发布资产与验证操作属于这个宿主入口及其配方，生成器不解析它们。`ci.release.verify` 是宿主显式登记的操作，加入 ci-tests 的执行计划；发布源和输出都显式连接该测试及 distribution-tests。手写旧工作流仅在核对固定基线字节后由本次迁移替换，生成器没有自动接管入口。

专属测试使用真实 CLI 和字面 shell 消费者验证参数、原始失败、定制保留、无写入复用、漂移、错误登记与输出所有权；旧检查工作流测试继续覆盖其原版本语义。生成／测试成功不认证完整输入闭包、任意宿主构建或确定性同判。原 v1 扩展已进入公开 beta.10。本产品生成的发布工作流在[固定源码原生构建](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944)中执行，两个平台各通过 371 项登记测试。匿名下载的 macOS CI 二进制另验证了显式发布投影；这些读数不认证其它宿主的构建或 full 工作流执行。

本宿主现采用 `chrono-github-release/v2` 与 `chrono-release-build/v4`。这是产品投影扩展和宿主调度政策的同次变更：原两项顺序 job 改为每个平台 15 个 build、15 个完整 verification 和一个 `always()` collector/package；每个 job 仍为 45 分钟，平台从首个 build 开始至最终验收也须不超过 45 分钟。未改变原始 15 项测试 argv，包括 workflow 项目的完整 59 个用例；每日检查的三个 workflow 组没有用于发布筛选。当前完整资产观察合同保留，未声称它是最小语义依赖集。原生并发、两平台时限、重试及公开发行仍须由调用方实际验证。

v1 的输出和无依赖行为保留。v2 增加固定提交的 `download_artifact_action`，每个 job 可明确列 `needs`、`downloads`（生产 job 与以 `/` 结尾的字面目的目录）、`always` 和 `dependency_metadata`（字面文件）。生成器核重复／未知依赖、循环及下载／输出／元数据路径重叠，不解析语言、命令或资产来补边。上传步骤直接导出 `artifact_id`，独立原始 attempt 步骤导出 `run_id`／`attempt`；artifact 名附当前 run/attempt，失败产物不覆盖旧尝试。下载只使用选定的原始 artifact ID，不按名称寻找最近产物。`toJson(needs)` 作为原生当前依赖结果及选定输出传给宿主，最终 job 另将同一原始 JSON 写至登记元数据路径。其它宿主可以登记任意字面非 Rust 命令；这些传输字段没有 Rust 或本仓语义。

宿主唯一调度入口仍是 `.chrono-harness/release/build.py`。`build.json` 显式拥有 unit ID、manifest/asset 或原始 operation、依赖边、handoff 目录、成本、原生 job 对应、局部并发数与时限。`projects.json` 是原始测试 argv 的唯一所有者，`plan.json` 是资产源路径的唯一所有者，`consumer_staging` 声明保留的全部目的路径；不从目录、名称或测试正文发现单元。固定入口为：

```sh
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT --unit ID
python3 .chrono-harness/release/build.py ROOT ABSENT_OUTPUT --collect
```

v4 完整本地入口要求 clean 固定 HEAD，以有界并发运行相同注册单元；每个单元使用独立临时 clone、固定 OID 和声明 handoff，输出中保留原始单元证据和本地驱动进程。仅删除本次创建的临时 roots，不改源检出。通过后的平台清单和 15 项资产位于 ABSENT_OUTPUT 根；原始 receipt、逐单元过程与完整驱动状态一并保留。v2/v3 完整入口仍执行原有顺序方法。已有输出（包括 symlink）在子进程前被拒绝；结构预检失败不制造成功报告。

build 单元只构建声明 manifest 一次，运输一个可执行文件及原始 receipt／过程流；可执行文件位于 mode 保持的 tar，而不进入 JSON 或 64 MiB portable-artifact 容器。摘要和复制分块流式处理，没有增加通用容量上限。verification 下载全部 15 项声明 producer，以源 commit/tree、平台、配方、工具链、归属 action、大小／摘要／mode 核对，每项完整执行原始 operation，并观察全部源和消费者的 before/after 身份。stdout/stderr 为原始文件，receipt 记录其字节数／摘要、原始 argv、cwd、退出码、工具／实际编译器身份及时间；每个 job 的实际环境分别记录，不伪造相同环境或完整输入证明。

collector 不安装工具链、不构建、不跑测试。它核 exact build/test 成员、当前依赖结果和选中原始 run/attempt；允许兼容的成功前置来自较早 producing attempt，但最新 failure/cancelled/skipped 不能用旧 pass 掩盖。更换选中 producer 或字节使不对应的测试 receipt 失效。collector 核原始过程退出／流、原始 argv、全部资产观察与生产 lineage 后，运行被验证向量中的 `chrono-distribution pack`，再核实际最终清单、15 项包字节／mode、源及平台。最终 artifact 含完整原始 receipt／过程流和明确选定 artifact-ID 引用；独立 job 的失败 artifact 仍保留。

专属 owner 验证包括任意非 Rust 字面命令、原生元数据、独立 sibling、隔离本地调度、真实 distribution pack、非 UTF-8 原始失败、缺失／损坏／重封装错误 receipt、路径／mode、变更字节、当前失败与 carried attempt。测试不认证实际原生平台时限、普遍环境同判或公开采用。最终 canonical committed check、独立评审、原生两平台执行及重试、发布和 main/examples 采用由调用方负责。
