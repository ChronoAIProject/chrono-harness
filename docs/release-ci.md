# 生成发布工作流

`chrono-ci` 按配置中的 schema 选择生成规则。`chrono-github-ci/v1`、v2、v3 生成检查工作流，`chrono-github-release/v1` 生成原生发布构建工作流。生成器只处理明确登记的数据；项目、语言、平台和发布资产都须由宿主列出。它不执行构建或发布。

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

真实 CLI 和 shell 消费者测试覆盖字面参数、失败原件、定制保留、无写入复用、漂移、错误登记和输出归属；旧检查工作流测试保留各版本语义。原 v1 扩展已在 beta.10 发布。本仓生成工作流的[固定源码原生构建](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944)在两平台各通过 371 项登记测试，匿名下载的 macOS CI 二进制另核过发布投影。这些证据只覆盖实际消费者，不能证明其他宿主构建、full 工作流执行、完整输入或确定性同判。

当前本宿主采用 `chrono-github-release/v2` 和 `chrono-release-build/v5`：每平台的 build、verification 和 `always()` collector/package 均逐项登记。每个 job 和整个平台均受登记的 45 分钟时限约束。Rust 验证仍跑原始未过滤操作；测试清单与原生输入适配的验证归 Rust `ci-tests`，宿主 bootstrap、配方单元测试和真实打包集成测试继续各自登记。
