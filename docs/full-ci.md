# 在 CI 中传递完整检查上下文

`chrono-github-full-ci/v1` 生成明确的 GitHub 手动工作流，把调用方提供的完整上下文交给与本地相同的 runner 命令：

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/config.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --context .chrono-harness/state/full/context.json
```

公开 beta.10 已包含此扩展，但它不会启用产品宿主仍为 proposed 的完整登记，也不替换当前按范围检查的 CI。

固定源码 beta.10 在 macOS arm64、Linux x86_64 运行过登记的 CI 和七程序消费者测试。匿名下载的 macOS CI 二进制也在真实 Git 样本中验证发布／完整工作流生成、漂移拒绝／重新生成，以及上下文和参数原样准备。这些分别是实际平台产品测试和本地公开二进制消费；生成的完整工作流尚未在 GitHub 实际执行，宿主全面启用也未完成，见[发布证据](distribution.md)。

`chrono-github-full-ci/v2` 使用 schema4 短入口。生成和准备的参数保留所选范围：`check`、`check --unit ID` 或 `check --collect`。登记的 CI action 只有一条 provider 路径，所以此配置必须精确匹配请求选择。

多个自动完整单元通过[CI 单元](ci-units.md)中的 `chrono-github-units/v2` 映射使用已有事件准备和真实 CI 汇总。本文手动工作流仍要求调用方提供精确上下文和准备好的清单证据，不自动收集。

## 登记配置并采用

提供如下 JSON，并把 bootstrap、平台替换成宿主明确登记的选择：

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full.yml",
  "name": "完整仓库检查",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["python3", ".chrono-harness/bootstrap.py", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "context_path": ".chrono-harness/state/full/context.json",
  "preparation_path": ".chrono-harness/state/full/preparation.json",
  "artifact_directory": ".chrono-harness/state/full/"
}
```

`chrono-ci init --host-root H --config INPUT` 将其采用到 `.chrono-harness/ci/full.json`，保留已有定制。`generate` 和 `verify` 使用同一源和渲染器，verify 不修复漂移。

每份当前完整工作流都带完整 provider 标记和精确源路径。共用输出预检直接核对完整标记，不解析旧工作流的准备命令来猜归属。其他源不能替换已有输出，即使原输出不分单元也一样；只有历史标记的旧输出不会自动接管。

未归属输出、符号链接、重叠路径、未知字段、未固定提交的 action、含表达式的设置都会失败。设置是字面单行字符串，允许的参数中 shell 特殊字符保留原字节。源、工作流、bootstrap、政策和实际消费者都须明确登记到 FILEMAP。

本次同步调整产品与规范，移除了旧归属推断和第二次工作流读取，也移除固定旧渲染字节的断言。当前定制、冲突、漂移、生成 Bash 和上下文准备行为仍有验证。运行及总验证节省未测。

## 单元和汇总各用独立配置

可选 `scope` 严格使用 `chrono_harness::units::Scope`：`{"kind":"unit","unit":"compile"}` 或 `{"kind":"collect","manifest":".chrono-harness/state/evidence/manifest.json"}`。省略时保留原不分单元 v1 的生成／准备行为，重新序列化也省略。`null`、未知种类／字段、无效单元 ID／清单路径，以及输入输出归属重叠都会失败。旧 provider 读取器拒绝新 `scope` 字段。

每份源只选择一次范围，手动触发仅接收调用方上下文，不提供范围菜单。以下两份是配置示例，不代表产品宿主已经采用。各自登记自己的工作流、名称、bootstrap 和产物路径；共享 runner 和检查政策是宿主明确选择。

`.chrono-harness/ci/full-compile.json`：

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full-compile.yml",
  "name": "完整检查的编译单元",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["/bin/sh", ".chrono-harness/provision-compile.sh", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "scope": {"kind": "unit", "unit": "compile"},
  "context_path": ".chrono-harness/state/full-compile/context.json",
  "preparation_path": ".chrono-harness/state/full-compile/preparation.json",
  "artifact_directory": ".chrono-harness/state/full-compile/"
}
```

`.chrono-harness/ci/full-collect.json`：

```json
{
  "schema": "chrono-github-full-ci/v1",
  "workflow_path": ".github/workflows/full-collect.yml",
  "name": "完整检查汇总",
  "runs_on": "ubuntu-24.04",
  "checkout_action": "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
  "upload_artifact_action": "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  "timeout_minutes": 30,
  "bootstrap": ["/bin/sh", ".chrono-harness/provision-collection.sh", "."],
  "runner": ".chrono-harness/bin/chrono-harness",
  "generator": ".chrono-harness/bin/chrono-ci",
  "check_config": ".chrono-harness/config.json",
  "scope": {"kind": "collect", "manifest": ".chrono-harness/state/full-collect/manifest.json"},
  "context_path": ".chrono-harness/state/full-collect/context.json",
  "preparation_path": ".chrono-harness/state/full-collect/preparation.json",
  "artifact_directory": ".chrono-harness/state/full-collect/"
}
```

逐项使用已有生成和核对入口：

```sh
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/full-compile.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/full-compile.json
.chrono-harness/bin/chrono-ci generate --host-root . --config .chrono-harness/ci/full-collect.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/full-collect.json
```

修改并重新生成其中一份，只更新它负责的工作流，保留另一工作流和两份宿主源。每份源、输出、bootstrap、清单／证据位置和消费者都须明确登记；生成不自动登记单元，也不推断依赖。汇总 bootstrap 或调用方负责准备清单及其原报告；provider 不编造清单、不获取报告，也不调度其他工作流。

渲染和准备都在 runner／config／base／candidate／context 后追加同一 `Scope::argv()`。编译例在上方本地命令中使用编译上下文路径，再加 `--unit compile`；汇总例使用汇总上下文，再加 `--collect .chrono-harness/state/full-collect/manifest.json`。本地和 CI 仍只有同一标准检查方法。

provider 验证传输语法和归属，完整检查程序决定单元／汇总能否进入。分范围完整检查要求匹配的 full-v3／v4 `execution_units` 和封存范围请求实现。准备成功或记录命令样本不证明完整分范围执行或通过准入；生成工作流的真实执行和宿主启用仍须另行取得证据。

完整配置须使用 v3 或 v4，并明确[绑定 Git](git-facts.md)。CI 不自动补工具、检查程序、完整输入、项目结构、语言或证据获取。bootstrap 参数须准备全部声明程序，以及上下文引用的保留输入、blob、集成证明和完成报告。缺证据仍由检查程序判失败。登记产物目录必须容纳所需输出，上传不会发现目录外证据。

## 上下文和执行过程

工作流只接收一个 `workflow_dispatch` 字符串输入 `context`，内容为完整 JSON。须提供 schema 2、明确的 `integration` 或 `delivery` 角色、完整基线／候选 OID，以及检查所需字段。分支开始／观察时间、fork、dev tip、operation、保留输入引用和集成证据都由调用方提供；不回退用当前时钟，不按分支名猜角色，不自动选证明文件。

checkout 选择给定候选。准备流程使用候选绑定的 Git 核对 HEAD、完整配置字节、provider 源字节，以及给定 `github.workflow_sha` 下生成工作流的 blob。只有精确探测确认缺对象后，才从 `origin` 获取缺失基线或工作流修订。探测／抓取失败保留原进程字节、摘要和退出。

这只核对给定 Git 身份，不独立认证 GitHub 来源、凭据、Git 配置或委托输入是否齐全。

准备流程原样写出解码后 context 字符串的 UTF-8 字节，包括空白，另写 `chrono-full-ci-inputs/v1` 报告。报告含原上下文字节／哈希、源身份、观察到的 Git 进程和标准参数；状态为 `prepared`，`governance: not-evaluated`，`parity: unestablished`。准备器只验证传输字段，其他上下文含义和准入由检查程序负责；所以虚假的分支年龄声明可能准备成功，却在检查时失败。

生成的 Bash 步骤把原准备 stdout／stderr 保存到产物目录下的 `prepare.stdout.json`、`prepare.stderr`，传递真实退出码。shell noclobber 在调用前拒绝已有输出流文件；这两个名称保留，不能与上下文／报告路径重叠。完整检查执行带 `--context` 及可选范围后缀的共用参数。上传用 `always()` 保留登记目录和失败证据；早期 bootstrap 失败可能只留下作业日志。

## 发布和重试

准备流程在写任何文件前先检查上下文、报告两个目的。已有字节完全相同可以复用。v1 中，若上下文字节及所有有实际含义的准备观察相同，仅 `launcher_pid`、`child_pid`、`ownership_fds` 不同，则复用原准备报告，并单独保留新尝试完整原报告。其他不同字节冲突，不覆盖。

v2 保留不可变原件，再通过已有写入器替换声明的当前文件。Git 获取会改变后续观察，因此 fetch 后重跑可能与首份准备报告冲突。此时使用新的明确运行位置／工作树，并保留旧产物，不能删旧证据假装首次调用没发生。

发布复用逐文件写入器，不是多文件崩溃事务，也不隔离并发写入者；IO 失败可能留下单个已发布输出。准备成功不是完整检查通过或同判证明。`check_config` 可使用与本地相同的严格一层原生选择器；准备保留入口路径、记录选中目标绑定，生成工作流的稳定入口参数不变。

真实消费者、生成工作流执行、宿主启用、输入齐全和本地／CI 比较是不同证据要求。手动工作流不负责创建 PR、合并、发布 required check 或核对落地。

## 输入后续工作的范围与来源纠正

后续输入实现保留原负责人处的采集／准备和 Cargo v6／directory v3 源码，沿用当前快照／组合、保留执行、精确 Git v2 选择及 worktree v3。登记的独立 runner 保护仍为 proposed，缺少当前绑定、参与和真实 CI 验收；它没有启用历史完整政策。完整 SPEC 目标仍未完成，本次源码只算 I1 交接。

PR163 的 `65d577f0c570451b674d271642d385b316c7085b` 有真实 distribution 消费者：bootstrap-core.json、bootstrap-detector.json、bootstrap.json 都选择 distribution-release.json，bootstrap.py／共享启动执行或验证安装器、回执和载荷。因此先前“没有已提交配置消费这条分支”的说法不符合这个固定来源。当前 distribution 产品已经负责安装，保留遗漏的宿主集成不要求另造安装器或切到 beta.21。

PR205 的 `native/macos/config.json`、`cargo-runner.json` 样本与 PR163 原 config 和 runner 政策**字节不同**，不能当作原样替换。不同 Git blob 身份、遗漏的保护／package／检查程序／发布源码，以及历史证据义务均保留在[输入来源保管](input-source-custody.md)。这里不把任何遗漏认定为冗余或已交付。完整真实 CI 启用、精确两端组合、当前安装消费者参与及更强同判仍未建立。

**产品与宿主政策曾同步修改：**新增观察器／Cargo 行为及其登记的获取／独立消费者声明，保留当前 Git／worktree 政策含义。固定政策到 `script:release-build` 的过时 FILEMAP 边已改指当前 `project:release-build`。所选生产者实际执行前，源码安装、真实 CI、存储和总维护成本均未知。
