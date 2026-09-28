# 发布与安装

`distribution` 与 `distribution-tests` 是独立 Rust 项目。版本化原生二进制在 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 集中公开分发；[Go、TS、mix 示例](examples.md) 是实际消费宿主。宿主只保留安装登记与生成入口，二进制、SDK 和缓存不提交。

## 发布者

本库 `.chrono-harness/release/build.json` 使用 `chrono-release-build/v3`，显式列独立构建 manifest、Rust 工具链、工具绑定、项目登记路径和有序 `verification_operations`。操作 ID 仍从登记项目或独立 scripts 的 actions 取得原始 argv，在显式 ROOT 中逐项执行，再调用唯一 `pack` 实现；不扫描项目、不复制测试命令。v2 保持原有无暂存语义，并拒绝 `consumer_staging` 字段（包括 null）。

v3 另须显式列 `rust_components`（可为空）；本仓选择 rustfmt 以运行迁移消费者内登记的 format 操作。v3 必须声明 `consumer_staging.release_plan`、`host_config` 和非空 `bindings`。每项显式选择 release plan 的资产名与测试消费者目的路径；目的必须属于宿主唯一的 `tracked: false` artifact，不能重复、互相嵌套或与任何发布源路径重叠。拒绝路径越界及路径组件中的 symlink。源码身份读取后、构建前，实际 `git ls-files` 拒绝已跟踪的目的路径；这项读数属于运行证据，失败会产生失败报告。其它结构预检失败不启动子进程。

构建后把发布可执行文件的原始字节和 mode 暂存到显式目的路径。当前登记 14 个工具各自的构建消费路径与宿主安装消费路径，并选择 distribution、worktree、runner、inputs、ci、judge-ci、registration、projects、workflow 和 Cargo adapter 的专属测试。它们包含 Git 绑定、保留输入、七判官与 integration/delivery 的实际 CLI 消费者。instructions 未作为该新增消费链的测试对象；仍是显式构建及打包成员。 产品消费者的合成宿主在 fixture 中显式采用本平台的解释器版本；历史快照不变，错误版本的反例仍执行。本仓 macOS 的固定版本登记由独立登记的宿主 migration script 测试验证，不把该宿主实例当作 Linux 产品测试的环境。

新配方发射 `chrono-native-build/v4`，保留 v3 的状态和原始过程证据，并增加 `consumer_staging`：资产/源/目的绑定、暂存时源身份，以及暂存后、每项验证前后、打包前后的文件摘要、大小、mode、读取错误和匹配结论。任一身份变化或缺失阻止成功发布；失败子进程后仍观察身份，保留原退出码，后置观察不能掩盖它。复制失败也保留实际可读状态。此处验证有界时点的字节身份；不认证两个观察之间无临时替换、完整输入闭包或每个库测试都调用 CLI。

配方在构建前拒绝未知、重复、歧义操作、无效 argv 与缺失工具；输出路径必须不存在，已有目录、文件或链接不会被覆盖。预检失败不启动子进程，也不伪造构建报告。

旧 v2 配方的 `build.json` 使用 `chrono-native-build/v3`，明确区分 `status: passed | failed`；beta.8 的成功报告仍为 v2。v3 的 `processes` 按实际顺序保留源码身份读取、工具链安装、版本读取、各 manifest 构建、登记验证和打包的 argv/cwd、退出码、原始 stdout/stderr 字节及摘要。验证项以 `phase: verification` 和原始 `operation` ID 标识；声明的 `verification_operations` 不是已执行清单。版本和源码身份只记录已成功取得的读数，未知源码为 null。

普通子进程失败会先保留本次及先前结果，再以原始退出码停止后续工作；被信号终止的负退出码仍留在过程记录中，CLI 以 `128 + 信号号` 退出。版本不符、程序未启动等失败单独记录阶段与原因，不把成功子进程的零退出改成非零，也不制造未启动进程。失败时输出目录可只含报告；它不是成功包。报告写入失败会明确报错，不能把已有子进程失败变成成功。该机制不承诺硬终止、断电、并发写入或不可写存储下的完整恢复。

这里仅消费显式操作登记并保存实测结果，不代表完整治理准入、构建输入闭包、二进制来源证明或跨平台同判。消费者必须核对相应版本的状态、实际操作与源身份，不能以目录或报告存在判定发布成功。

这是本产品的宿主配方；其它语言仓库只安装发布的二进制。`.github/workflows/chrono-release.yml` 在登记的 macOS、Ubuntu runner 上实际执行同一配方，并在失败时仍尝试上传原始产物；上传构建 artifacts 不等于已发布。这个发布工作流由 `chrono-ci` 从宿主显式[发布源](../.chrono-harness/ci/release.json)生成，合同与扩展边界见 [release CI](release-ci.md)；公开 beta.9 尚不包含该生成扩展。

[beta.9](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.9) 已从固定源码 `c817a9043d9b9596547a3916d6eed84752ded2e5` 公开发布，包含两个平台各 15 个工具、合并清单以及各平台的原始构建报告和打包清单，共 35 个资产。其[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36447808917)使用 v3 配方／v4 报告，在每个平台通过十个专属测试项目的 352 项测试；核对 31 个实际进程、28 个暂存目的路径及 24 轮身份观察。已匿名下载核对合并清单及两个平台的 worktree／installer 字节。beta.9 包含显式 Git 绑定及上述发布证据合同，宿主须自行选择采用相应版本配置；测试与摘要核验不证明完整输入闭包或确定性同判。

```sh
chrono-distribution pack --root PRODUCT --plan PRODUCT/.chrono-harness/release/plan.json --output ABSENT_OUTPUT
chrono-distribution assemble --input MAC_PACKAGE --input LINUX_PACKAGE --output ABSENT_RELEASE
```

`pack` 拒绝脏源码，固定实际 Git commit/tree；只复制明确列出的可执行文件，不扫描 crates。`assemble` 显式接收平台包，验证每个成员，拒绝源提交、树、版本或成员集不一致及重复平台。最终 `release.json` 使用 `chrono-release/v1`，包含 `version/source_commit/source_tree/platforms`。平台映射中每个具名工具有 `file/sha256/size`；文件名为 `NAME-OS-ARCH`，每个平台须提供 `chrono-distribution`。没有 archive extraction。

原生平台目前登记 `macos-aarch64`、`linux-x86_64`；实际产物以构建结果为准。其它平台需要显式增加构建与运行验证。构建证据、声明的源码身份及摘要不构成完整工具链/SDK 来源认证，也不保证不同平台结果相同。

## 宿主采用

取得固定发布清单和对应平台的 `chrono-distribution` 后，显式选择要安装的工具：

```sh
chrono-distribution adopt --host-root HOST --manifest release.json \
  --base-url https://github.com/ChronoAIProject/chrono-harness/releases/download/VERSION \
  --curl /usr/bin/curl --tool chrono-harness --tool chrono-judge-ci \
  --tool chrono-ci --tool chrono-instructions
python3 HOST/.chrono-harness/install.py HOST
```

采用工具写出 `.chrono-harness/distribution.json` (`chrono-install/v1`) 及 `.chrono-harness/install.py`。登记含版本、HTTPS 源、清单摘要/长度、各平台注册 installer 摘要/长度、工具名称到 `.chrono-harness/bin/` 文件的显式映射。发布清单是采用时的受信任输入，调用者应从所选固定发布获得并核对；没有浮动 latest。首次采用者也须核验执行的安装器身份。

本地和 CI 共用同一 `python3 .chrono-harness/install.py .`。生成入口验证已锁定 installer，执行该 Rust installer；Rust 验证锁定清单、版本、平台和所选资产。缺失平台/成员、摘要/长度不符、目的冲突、未知 schema 均非零。安装不读取宿主源码、语言 manifest、目录布局或测试配置。语言 SDK 继续由宿主自己的显式登记负责。

离线分发可以 `--source-dir ABSOLUTE_PACKAGE` 替代 HTTPS 参数，使用同一个校验/安装路径；本地绝对路径不适合提交给公共消费者。HTTPS transport 是明确登记的 curl，禁读 curlrc，只允许 HTTPS（含重定向），检查实际下载退出码。Python 3 仅承担初始 installer 下载与调用；常规安装语义由 Rust 维护。

## 更新及故障边界

新版本通过重新 `adopt ... --replace` 显式修改锁定配置；不自动选择版本或改预期摘要。默认拒绝覆盖不同的采用配置/入口，重复相同采用允许。AI 自主更新登记并完成适用的 integration 和宿主检查。

安装先验证全部选中成员，再发布文件；已安装文件只有摘要、长度、执行权限合格才可复用。只替换登记的文件，其它 SDK/工具保留；本次不再选择的旧工具不会自动删除，退休由宿主显式处理。安装结果写 `.chrono-harness/state/distribution.json`，含实际发布身份、平台和逐工具摘要。

同一宿主的协作安装由 `.chrono-harness/.distribution-lock` 排他。普通文件替换失败尝试逆序恢复，失败报告恢复目录。没有崩溃原子性或与不合作并发写者的隔离保证；进程被强杀后的锁/恢复目录须核实后恢复。源/目的 symlink、越界路径和非文件覆盖被拒绝。支持 Unix；不声称 Windows 支持。

专属测试覆盖安装、升级、损坏后修复、重复安装、保留宿主 SDK、坏清单/晚到坏资产不替换、普通替换失败回滚、平台/成员/安装器错配、目的冲突、路径与 symlink、配置定制保护、bootstrap 真退出及实际安装器在含空格路径/不同 cwd 下运行。发布与实际宿主 check 各保留自身成功/失败；安装成功不是治理通过。

发布配方专属消费者测试通过实际 Python 入口验证显式操作顺序、含空格路径与不同 cwd、参数原样传递、未知／重复／歧义登记在构建前失败、失败阻止打包及保留原始非 UTF-8 输出。这些夹具验证配方委托；真实 Rust 测试与平台行为另由原生发布作业验证。
