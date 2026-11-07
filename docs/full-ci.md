# 在 CI 中传递完整检查上下文

`chrono-github-full-ci/v2` 生成显式完整上下文的 GitHub 手动工作流，调用宿主 schema4 登记的短入口：

```sh
.chrono-harness/bin/chrono-harness check
```

范围固定在 provider 源中；单元生成 `check --unit ID`，汇总生成无值 `check --collect`。同一 CI 输入 action 只选择一份 provider，必须精确匹配本次请求。自动 push／PR 的完整单元使用独立的 `chrono-github-units/v2`，见 [CI 单元](ci-units.md)。

**产品与规范同步收窄：**旧 `chrono-github-full-ci/v1` 已退役，生成、init 和准备均拒绝旧声明。删除长命令投影、旧准备步骤、无实际消费者的 provider `generator` 字段、重复上下文检查及按 PID／所有权描述符归一化的重试分支。仍有实际消费者的显式 runner API 保留；固定历史输入与来源义务也保留。总构建、CI、存储和迁移成本未测。

公开 beta.10 的旧投影在 macOS arm64、Linux x86_64 有登记产品测试及 macOS 公开二进制本地消费；这些历史观察不证明当前短入口已发布或生成的完整工作流已在 GitHub 执行。产品宿主全面启用、完整输入和同判仍未完成。

## 登记配置并采用

宿主登记 provider、输出、bootstrap、工具、政策及实际消费者。下面的 bootstrap 和平台须替换成宿主明确选择：

```json
{
  "schema": "chrono-github-full-ci/v2",
  "workflow_path": ".github/workflows/full.yml",
  "name": "完整仓库检查",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["python3", ".chrono-harness/bootstrap.py", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "check_config": ".chrono-harness/config.json",
  "context_path": ".chrono-harness/state/full/context.json",
  "preparation_path": ".chrono-harness/state/full/preparation.json",
  "artifact_directory": ".chrono-harness/state/full/"
}
```

`chrono-ci init --host-root H --config INPUT` 采用到 `.chrono-harness/ci/full.json`，保留当前宿主定制。`generate` 和 `verify` 使用同一源及渲染器，verify 不修漂移。每份投影的归属标记包含精确源路径；其他源、只有历史通用标记的输出和未归属输出均不能自动接管。

完整 config 使用 schema4，并登记 `canonical_check.profile`、现有本地生产者及 CI 输入 action。CI action 的工具为 `chrono-ci`，参数为：

```json
["check-inputs", "--config", ".chrono-harness/ci/full.json",
 "--event-env", "GITHUB_EVENT_NAME", "--payload-env", "GITHUB_EVENT_PATH",
 "--revision-env", "CHRONO_WORKFLOW_REVISION"]
```

工具、环境和输入的实际消费者边须完整登记。宿主继承 `CHRONO_CHECK_SOURCE` 及上述三个事件变量；生成步骤设置 `CHRONO_CHECK_SOURCE=ci`、固定 workflow 修订，其余事件变量由 GitHub 提供。短入口在同一登记命令中获取输入并运行七判官；不手工串接另一套检查配方。

未固定提交的 action、未知字段、表达式设置、符号链接、输入／输出归属重叠和非法路径都会失败。允许的设置是字面单行字符串，shell 特殊字符保留原字节。`check_config` 可使用严格一层原生选择器，稳定入口路径和选中配置绑定均保留。

## 单元与汇总

可选 `scope` 为 `{"kind":"unit","unit":"compile"}` 或 `{"kind":"collect","manifest":".chrono-harness/state/evidence/manifest.json"}`，省略则全局 DELTA。`null`、未知字段／种类、非法单元或清单路径均失败。

每份源各有明确工作流、名称、bootstrap、上下文／报告和上传根，生成及 init 只更新该源拥有的投影。清单须与其他输入／输出分离。修改其中一份不修改另一份；共享 runner 和政策是宿主明确选择。生成不登记单元、不推断依赖，也不创建或获取汇总清单。

分范围执行要求所选 full-v4 政策有匹配的 `execution_units`。汇总调用方或 bootstrap 必须供应清单及其原报告；provider 准备只验证传输，范围与治理准入仍由七判官决定。上传目录须涵盖实际输出及必要原件，上传不发现目录外证据。

## 上下文与执行

工作流只有一个 `workflow_dispatch` 字符串输入 `context`，其内容为完整 JSON。调用方提供 schema2、明确的 `integration`／`delivery`、完整 base／candidate OID，以及分支开始／观察时间、fork、dev tip、operation、保留输入引用和适用的集成证据。准备器不猜角色、时间或证明路径，不自动补工具、输入和证据。

checkout 使用给定 candidate。候选绑定的 Git 核对 HEAD、完整配置、provider 源字节及指定 `github.workflow_sha` 的投影 blob。只有精确确认缺对象后才从 `origin` 获取固定基线或 workflow OID；失败保留原进程字节、摘要和退出。此绑定不独立认证 GitHub 来源、凭据、Git 配置或输入齐全。

准备器原样保留 context 字符串的 UTF-8 字节，包括空白。`chrono-full-ci-inputs/v1` 报告含原上下文路径／SHA-256、固定源、Git 进程和短 `canonical_argv`；`prepared`、`governance: not-evaluated`、`parity: unestablished` 均不表示检查通过。陈旧时间等领域语义由实际判官裁决。

**报告合同与产品同步收窄：**删除没有运行消费者的 `context.input_bytes` 数字字节数组；上下文正文由已有不可变原件保存，消费者沿路径与摘要读取。实际原件、当前视图和输入接口保留。完整构建、CI、存储及维护成本未测。

生成器直接生成短检查步骤。输入 action 保留不可变上下文、准备报告和原始进程结果；当前 context／preparation 文件只是可替换视图，原件以路径与摘要消费。重试保存实际本次观察，不归一化进程 PID 或所有权载体，也不复用旧报告冒充新观察。Git 获取会改变后续观察，但不覆盖已有不可变原件。

上传使用 `always()` 保留登记根和失败证据，早期 bootstrap 失败可能只有作业日志。单文件写入不是多文件崩溃或并发事务。真实消费者测试、原生生成派发、公开发布、完整宿主启用和本地／CI 同判分别验收；此投影不创建 PR、合并、发布 required check 或核落地。

## 输入后续工作的范围与来源纠正

后续输入实现保留原负责人处的采集／准备和 Cargo v6／directory v3 源码，沿用当前快照／组合、保留执行、精确 Git v2 选择及 worktree v3。登记的独立 runner 保护仍为 proposed，缺少当前绑定、参与和真实 CI 验收；它没有启用历史完整政策。完整 SPEC 目标仍未完成，本次源码只算 I1 交接。

PR163 的 `65d577f0c570451b674d271642d385b316c7085b` 有真实 distribution 消费者：bootstrap-core.json、bootstrap-detector.json、bootstrap.json 都选择 distribution-release.json，bootstrap.py／共享启动执行或验证安装器、回执和载荷。因此先前“没有已提交配置消费这条分支”的说法不符合这个固定来源。当前 distribution 产品已经负责安装，保留遗漏的宿主集成不要求另造安装器或切到 beta.21。

PR205 的 `native/macos/config.json`、`cargo-runner.json` 样本与 PR163 原 config 和 runner 政策**字节不同**，不能当作原样替换。不同 Git blob 身份、遗漏的保护／package／检查程序／发布源码，以及历史证据义务均保留在[输入来源保管](input-source-custody.md)。这里不把任何遗漏认定为冗余或已交付。完整真实 CI 启用、精确两端组合、当前安装消费者参与及更强同判仍未建立。

**产品与宿主政策曾同步修改：**新增观察器／Cargo 行为及其登记的获取／独立消费者声明，保留当前 Git／worktree 政策含义。固定政策到 `script:release-build` 的过时 FILEMAP 边已改指当前 `project:release-build`。所选生产者实际执行前，源码安装、真实 CI、存储和总维护成本均未知。
