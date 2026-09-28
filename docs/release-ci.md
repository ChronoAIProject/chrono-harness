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

本仓的 [发布源](../.chrono-harness/ci/release.json) 明确列 macOS 与 Linux 两个 job，二者沿用同一已存在入口，本地与 CI 均执行：

```sh
python3 .chrono-harness/release/build.py . .chrono-harness/cache/release-platform
```

Rust、发布资产与验证操作属于这个宿主入口及其配方，生成器不解析它们。`ci.release.verify` 是宿主显式登记的操作，加入 ci-tests 的执行计划；发布源和输出都显式连接该测试及 distribution-tests。手写旧工作流仅在核对固定基线字节后由本次迁移替换，生成器没有自动接管入口。

专属测试使用真实 CLI 和字面 shell 消费者验证参数、原始失败、定制保留、无写入复用、漂移、错误登记与输出所有权；旧检查工作流测试继续覆盖其原版本语义。生成／测试成功不认证完整输入闭包、任意宿主构建或确定性同判。此扩展已进入公开 beta.10。本产品生成的发布工作流在[固定源码原生构建](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944)中执行，两个平台各通过 371 项登记测试。匿名下载的 macOS CI 二进制另验证了显式发布投影；这些读数不认证其它宿主的构建或 full 工作流执行。
