# 发布与安装

`distribution` 负责发布和安装，`distribution-tests` 专门测试它；两者都是独立 Rust 项目。

带固定版本的平台二进制在 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 集中公开分发；[Go、TS、mix 示例](examples.md) 是实际消费宿主。

接入的仓库（下称“宿主”）只提交安装登记和生成入口，不提交二进制、SDK 或缓存。

## 发布者

本仓的构建配方是 `.chrono-harness/release/build.json`，只接受 `chrono-release-build/v5`。

构建和验证分成明确的独立单元；旧格式会在启动工具或创建输出前失败。

配方显式列独立构建 manifest、Rust 工具链、工具绑定、项目登记路径和 `verification_operations`。

操作 ID 从登记项目或独立 scripts 的 actions 取得原始 argv，由所选验证单元执行，再由 collector 调用唯一 `pack` 实现；项目必须逐项登记，测试命令沿用原登记，不再抄一份。

v5 的验证单元通过 `needs` 显式选择所需构建，可以为空；只导入所选资产、原始回执并观察对应暂存路径。

列表为空时，不读构建回执，也不向 Git 发空 pathspec 的跟踪查询。

collector 仍必须收齐全部构建和验证，核对各验证实际消费的精确依赖与资产；整个平台的耗时从最早启动的单元开始计算。

本地遵守登记的 `local_workers`、单元和平台时限。

独立 Python 发布配方单元测试不依赖产品二进制；Rust diagnostics、instructions 与 routes 专属测试登记为零发布二进制依赖，并明确启用 Rust 工具链。

instructions 测试直接调用库及 dispatch；routes 测试调用库并使用临时测试工具。

两者的 Cargo 路径依赖照常编译，发布验证不下载或暂存未消费的工具。

对应生产二进制仍由独立 build 单元构建并交 collector 打包。

`verification_consumers` 是 v5 的可选消费约定：以验证单元 ID 为键，每项登记非空 `need` 与显式 `release_assets` 列表。

空列表表示不消费发布二进制，Cargo 的库源码依赖不写成发布资产。

预检拒绝未知单元／资产、重复资产、缺少说明及与 `needs` 不符的前置；不推断消费关系。

单元与 collector 回执保留 `dependency_contracts`，`declared` 只表示登记一致。

有发布资产但未采用消费约定时，保留 `unverified` 并输出 `W_RELEASE_CONSUMPTION_UNVERIFIED`，不能因此认定资产多余。

库调用与实际二进制调用仍须分别用真实消费者验证。

配方显式列 `rust_components`（可为空），需要 Rust 的单元按声明安装组件；无需 Rust 的验证不安装工具链。

`consumer_staging` 声明 `release_plan`、`host_config` 和非空 `bindings`。

每项显式选择 release plan 的资产名与测试消费者目的路径；目的必须属于宿主唯一的 `tracked: false` artifact，不能重复、互相嵌套或与任何发布源路径重叠。

拒绝路径越界及路径组件中的 symlink。

验证／collector 在复制前通过字面 `git ls-files` 拒绝已跟踪的目的路径，这项读数属于运行证据。

其它结构预检失败不启动子进程。

构建后把发布可执行文件的原始字节和 mode 暂存到显式目的路径。

当前配方按[发布计划](../.chrono-harness/release/plan.json)逐项登记各发布工具的构建消费路径与宿主安装消费路径，并通过[构建配方](../.chrono-harness/release/build.json)显式选择各生产项目的专属测试项目，包括 instructions、FILEMAP、routes、cost 和 mixed。

它们包含 Git 绑定、声明文件核验、保留输入、七判官与 integration/delivery 的实际 CLI 消费者；库调用测试仍只验证各自的实际调用范围。

测试选择来自配方中的操作白名单，不按目录或发布资产推断。

产品消费者的合成宿主在 fixture 中显式采用本平台的解释器版本；历史快照不变，错误版本的反例仍执行。

本仓 macOS 的固定版本登记由独立登记的宿主 migration script 测试验证，不把该宿主实例当作 Linux 产品测试的环境。

当前单元回执 `chrono-release-unit/v1` 保留状态、原始过程与 `consumer_staging`：所选资产/源/目的绑定、暂存时源身份，以及暂存后、验证或打包前后的文件摘要、大小、mode、读取错误和匹配结论。

任一身份变化或缺失阻止成功发布；失败子进程后仍观察身份，保留原退出码。

复制失败也保留实际可读状态。

此处验证有界时点的字节身份；不认证两个观察之间无临时替换、输入及其依赖的完整性或每个库测试都调用 CLI。

配方在构建前拒绝未知、重复、歧义操作、无效 argv 与缺失工具；输出路径必须不存在，已有目录、文件或链接不会被覆盖。预检失败不启动子进程，也不伪造构建报告。

本地驱动的 `local.json` 使用 `chrono-release-local/v1`，保存实际单元结果与驱动进程；单元回执区分 `status: passed | failed`，其 `processes` 按实际顺序保留 argv/cwd、退出码及原始 stdout/stderr 文件引用、字节数和摘要。

验证项以 `phase: verification` 和原始 `operation` ID 标识；声明的 `verification_operations` 不是已执行清单。

源码和工具只记录实际观察到的身份，无法取得时如实记为未知。

普通子进程失败会先保留本次及先前结果，失败单元按原始退出码终结，独立单元按显式依赖继续；被信号终止的负退出码仍留在过程记录中，CLI 以 `128 + 信号号` 退出。版本不符、程序未启动等失败单独记录阶段与原因，不把成功子进程的零退出改成非零，也不制造未启动进程。失败时输出目录可只含报告；它不是成功包。报告写入失败会明确报错，不能把已有子进程失败变成成功。该机制不承诺硬终止、断电、并发写入或不可写存储下的完整恢复。

这里执行的是明确登记的操作，保存的是实测结果；它尚不能证明完整治理、构建输入完整、二进制来源完整或跨平台同判。消费者必须核对相应版本的状态、实际操作与源身份，不能以目录或报告存在判定发布成功。

这是本产品的宿主配方；其它语言仓库只安装发布的二进制。

`.github/workflows/chrono-release.yml` 投影为登记的 macOS、Ubuntu 独立单元，并在失败时仍尝试上传原始产物；上传构建 artifacts 不等于已发布。

这个发布工作流由 `chrono-ci` 从宿主显式[发布源](../.chrono-harness/ci/release.json)生成，合同与扩展边界见 [release CI](release-ci.md)。发布和实际宿主采用分别验收，旧发布不能证明当前候选已完成原生验证或发布。

```sh
chrono-distribution pack --root PRODUCT --plan PRODUCT/.chrono-harness/release/plan.json --output ABSENT_OUTPUT
chrono-distribution assemble --input MAC_PACKAGE --input LINUX_PACKAGE --output ABSENT_RELEASE \
  --base-url https://github.com/ChronoAIProject/chrono-harness/releases/download/VERSION --curl /usr/bin/curl
```

`pack` 拒绝脏源码，固定实际 Git commit/tree；只复制明确列出的可执行文件，不扫描 crates。

`assemble` 显式接收平台包及安装源，验证每个成员，拒绝源提交、树、版本或成员集不一致及重复平台。

最终 `release.json` 使用 `chrono-release/v1`，包含 `version/source_commit/source_tree/platforms`。

平台映射中每个具名工具有 `file/sha256/size`；文件名为 `NAME-OS-ARCH`，每个平台须提供 `chrono-distribution`。

组合目录另有同版本的 `install.sh`，内嵌清单与各平台安装器的摘要和长度，随全部资产一起发布。

该流程直接处理文件，不解压归档。

原生平台目前登记 `macos-aarch64`、`linux-x86_64`；实际产物以构建结果为准。其它平台需要显式增加构建与运行验证。构建证据、声明的源码身份及摘要不构成完整工具链/SDK 来源认证，也不保证不同平台结果相同。

## 一键安装

产品与宿主登记同步变更：Python 启动模板及其路径登记迁移为 Shell，安装规则仍归 Rust；组合发布命令新增显式安装源。保留原安装测试，增加真实 Shell/安装器和 HTTPS 命令边界验证；验证、下载及维护的总成本尚未测量。

[beta.29](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.29) 提供 `install.sh`、`release.json` 和两个平台共 32 个工具二进制。安装支持 macOS arm64 和 Linux x86_64；在宿主根运行以下固定版本命令：

```sh
curl -fsSL https://github.com/ChronoAIProject/chrono-harness/releases/download/v0.1.0-beta.29/install.sh | sh -s -- --host-root .
```

脚本默认安装该发布清单中的全部工具；重复的 `--tool NAME` 可明确选择子集。脚本识别 macOS/Linux 及 arm64/aarch64/x86_64，取得并校验对应安装器和原清单，再调用 Rust `adopt`、`install`。选择已有不同配置时拒绝覆盖，显式升级使用 `--replace`。下载、校验、采用或安装失败均非零；已启动的 Rust 安装器的退出码原样传递。

运行需要 POSIX `sh`、`uname`、`mktemp`、`cp`、`chmod`、`rm`、`wc`、`sha256sum` 或 `shasum -a 256`，HTTPS 源另需登记的 curl；无需 Python、Rust/Cargo、sudo 或修改全局配置。脚本来自所选固定版本，是首次信任入口；内嵌摘要固定字节，不单独认证发布者身份。临时下载在退出时清理，失败诊断留在原 stderr。

公开安装已有 macOS arm64 实际消费，Linux x86_64 的原生构建和登记验证不能代替公开安装验收。具体资产、摘要、构建结果及安装观察直接查所选发布、原 PR 和线上 CI，不在本文维护副本。

## 宿主采用

宿主安装和原生 CI 输入准备使用 Rust 工具及薄 Shell 入口，无需复制本仓 `.chrono-harness/ci/` 和 `.chrono-harness/release/` 下的 Python。这些脚本负责本仓源码构建和发布；真实 Python 业务及其输入探测是否需要 Python，由宿主自己的明确登记决定。

取得固定发布清单和对应平台的 `chrono-distribution` 后，明确选择要安装的工具：

```sh
chrono-distribution adopt --host-root HOST --manifest release.json \
  --base-url https://github.com/ChronoAIProject/chrono-harness/releases/download/VERSION \
  --curl /usr/bin/curl --tool chrono-harness --tool chrono-judge-ci \
  --tool chrono-ci --tool chrono-instructions
sh HOST/.chrono-harness/install.sh --host-root HOST
```

采用工具写出 `.chrono-harness/distribution.json` (`chrono-install/v1`) 及 `.chrono-harness/install.sh`。

登记含版本、HTTPS 源、清单摘要/长度、各平台注册 installer 摘要/长度、工具名称到 `.chrono-harness/bin/` 文件的显式映射。

生成的 Shell 绑定该声明原字节的摘要和长度，修改声明须重新 adopt 生成入口。

发布清单是采用时的受信任输入，调用者应从所选固定发布获得并核对；不使用会移动的 latest。

首次采用者也须核验执行的安装器身份。

本地和 CI 共用同一 `sh .chrono-harness/install.sh --host-root .`。生成入口验证安装声明和已锁定 installer，执行该 Rust installer；Rust 验证锁定清单、版本、平台和所选资产。缺失平台/成员、摘要/长度不符、目的冲突、未知 schema 均非零。安装不读取宿主源码、语言 manifest、目录布局或测试配置。语言 SDK 继续由宿主自己的显式登记负责。

离线分发在 assemble/adopt 时可以 `--source-dir ABSOLUTE_PACKAGE` 替代 HTTPS 参数，使用同一个校验/安装路径；本地绝对路径不适合提交给公共消费者。

HTTPS 传输使用明确登记的 curl，禁读 curlrc，只允许 HTTPS（含重定向），检查实际下载退出码。

Shell 只承担初始下载、字节校验和调用；常规安装语义由 Rust 维护。

旧 Python 启动模板已退役；外部既有宿主的脚本和 CI 命令须随采用新版安装器明确迁移，不自动删除其定制文件。

## 更新及故障边界

新版本通过重新 `adopt ... --replace` 显式修改锁定配置；不自动选择版本或改预期摘要。默认拒绝覆盖不同的采用配置/入口，重复相同采用允许。AI 自主更新登记并完成适用的 integration 和宿主检查。

安装先验证全部选中成员，再发布文件；已安装文件只有摘要、长度、执行权限合格才可复用。只替换登记的文件，其它 SDK/工具保留；本次不再选择的旧工具不会自动删除，退役由宿主明确处理。安装结果写 `.chrono-harness/state/distribution.json`，含实际发布身份、平台和逐工具摘要。

同一宿主的协作安装通过 `.chrono-harness/.distribution-lock` 互斥。普通文件替换失败尝试逆序恢复，恢复失败时报告备份目录。没有崩溃原子性或与不合作并发写者的隔离保证；进程被强杀后的锁/恢复目录须核实后恢复。源/目的 symlink、越界路径和非文件覆盖被拒绝。支持 Unix；不声称 Windows 支持。

专属测试覆盖安装、升级、损坏后修复、重复安装、保留宿主 SDK、坏清单/晚到坏资产不替换、普通替换失败后的恢复、平台/成员/安装器错配、目的冲突、路径与 symlink、配置定制保护、bootstrap 的真实退出结果及实际安装器在含空格路径/不同 cwd 下运行。发布与实际宿主 check 各保留自身成功/失败；安装成功不是治理通过。

发布配方由独立 Rust 项目 `host-release-native` 负责，唯一专属测试项目为 `host-release-native-tests`。完整发布沿用[发布入口](release-ci.md#v5-构建配方)，薄 shell 只委托 bootstrap 或恢复原生下载工具，解析与调度由 Rust 完成。

同一个未过滤 Rust 测试操作覆盖配方、真实 CLI、原件交接、真实打包和 cache 消费。打包使用测试 crate 自行编译的 distribution alias；只有 cache 回归需要发布资产 `chrono-cache`。具体依赖和原生消费边界由[发布说明](release-ci.md#验证单元实际消费什么)维护。

发布配方专属消费者测试通过实际 Python 入口验证显式操作顺序、含空格路径与不同 cwd、参数原样传递、未知／重复／歧义登记在构建前失败、失败阻止打包及保留原始非 UTF-8 输出。这些测试材料验证配方是否正确调用已有操作；真实 Rust 测试与平台行为另由原生发布作业验证。
