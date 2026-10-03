# 发布与安装

`distribution` 与 `distribution-tests` 是独立 Rust 项目。版本化原生二进制在 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 集中公开分发；[Go、TS、mix 示例](examples.md) 是实际消费宿主。宿主只保留安装登记与生成入口，二进制、SDK 和缓存不提交。

## 发布者

本库 `.chrono-harness/release/build.json` 现采用 `chrono-release-build/v4` 的显式独立单元；v2/v3 保留原有顺序配方。旧顺序配方显式列独立构建 manifest、Rust 工具链、工具绑定、项目登记路径和有序 `verification_operations`。操作 ID 仍从登记项目或独立 scripts 的 actions 取得原始 argv，在显式 ROOT 中逐项执行，再调用唯一 `pack` 实现；不扫描项目、不复制测试命令。v2 保持原有无暂存语义，并拒绝 `consumer_staging` 字段（包括 null）。

v3 另须显式列 `rust_components`（可为空）；本仓选择 rustfmt 以运行迁移消费者内登记的 format 操作。v3 必须声明 `consumer_staging.release_plan`、`host_config` 和非空 `bindings`。每项显式选择 release plan 的资产名与测试消费者目的路径；目的必须属于宿主唯一的 `tracked: false` artifact，不能重复、互相嵌套或与任何发布源路径重叠。拒绝路径越界及路径组件中的 symlink。源码身份读取后、构建前，实际 `git ls-files` 拒绝已跟踪的目的路径；这项读数属于运行证据，失败会产生失败报告。其它结构预检失败不启动子进程。

构建后把发布可执行文件的原始字节和 mode 暂存到显式目的路径。当前配方为 15 个发布工具逐项登记构建消费路径与宿主安装消费路径，并显式选择全部 15 个生产项目各自的专属测试项目，包括 instructions、FILEMAP、routes、cost 和 mixed。它们包含 Git 绑定、声明文件核验、保留输入、七判官与 integration/delivery 的实际 CLI 消费者；库调用测试仍只验证各自的实际调用范围。测试选择来自配方中的操作白名单，不按目录或发布资产推断。 产品消费者的合成宿主在 fixture 中显式采用本平台的解释器版本；历史快照不变，错误版本的反例仍执行。本仓 macOS 的固定版本登记由独立登记的宿主 migration script 测试验证，不把该宿主实例当作 Linux 产品测试的环境。

新配方发射 `chrono-native-build/v4`，保留 v3 的状态和原始过程证据，并增加 `consumer_staging`：资产/源/目的绑定、暂存时源身份，以及暂存后、每项验证前后、打包前后的文件摘要、大小、mode、读取错误和匹配结论。任一身份变化或缺失阻止成功发布；失败子进程后仍观察身份，保留原退出码，后置观察不能掩盖它。复制失败也保留实际可读状态。此处验证有界时点的字节身份；不认证两个观察之间无临时替换、完整输入闭包或每个库测试都调用 CLI。

配方在构建前拒绝未知、重复、歧义操作、无效 argv 与缺失工具；输出路径必须不存在，已有目录、文件或链接不会被覆盖。预检失败不启动子进程，也不伪造构建报告。

旧 v2 配方的 `build.json` 使用 `chrono-native-build/v3`，明确区分 `status: passed | failed`；beta.8 的成功报告仍为 v2。v3 的 `processes` 按实际顺序保留源码身份读取、工具链安装、版本读取、各 manifest 构建、登记验证和打包的 argv/cwd、退出码、原始 stdout/stderr 字节及摘要。验证项以 `phase: verification` 和原始 `operation` ID 标识；声明的 `verification_operations` 不是已执行清单。版本和源码身份只记录已成功取得的读数，未知源码为 null。

普通子进程失败会先保留本次及先前结果，再以原始退出码停止后续工作；被信号终止的负退出码仍留在过程记录中，CLI 以 `128 + 信号号` 退出。版本不符、程序未启动等失败单独记录阶段与原因，不把成功子进程的零退出改成非零，也不制造未启动进程。失败时输出目录可只含报告；它不是成功包。报告写入失败会明确报错，不能把已有子进程失败变成成功。该机制不承诺硬终止、断电、并发写入或不可写存储下的完整恢复。

这里仅消费显式操作登记并保存实测结果，不代表完整治理准入、构建输入闭包、二进制来源证明或跨平台同判。消费者必须核对相应版本的状态、实际操作与源身份，不能以目录或报告存在判定发布成功。

这是本产品的宿主配方；其它语言仓库只安装发布的二进制。`.github/workflows/chrono-release.yml` 投影为登记的 macOS、Ubuntu 独立单元，并在失败时仍尝试上传原始产物；上传构建 artifacts 不等于已发布。这个发布工作流由 `chrono-ci` 从宿主显式[发布源](../.chrono-harness/ci/release.json)生成，合同与扩展边界见 [release CI](release-ci.md)；公开 beta.10 包含原 v1 生成合同；本次 v2/v4 尚待调用方原生验证与发布。

[beta.11](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.11) 已从固定源码 `a4af1a0ca7f3a69f7b3298abc5b187a74578b3f9` 公开发布，包含两个平台各 15 个工具、合并清单以及各平台的原始构建报告和打包清单，共 35 个资产。其[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36566708631)使用 v3 配方／v4 报告，在每个平台通过十个专属测试项目的 391 项测试；核对 31 个实际进程、28 个暂存目的路径及 24 轮身份观察。已匿名下载核对合并清单及两个平台的 worktree／installer／CI／runner／scoped judge 字节。合并清单 SHA-256 为 `93c6d9b2a0da66a7bc555a4d475c6f300f6d705d0ee7c634c92c445c0e435330`。beta.11 新增显式元数据重建与原始 checkout 身份核对，并包含先前发布的 Git 绑定、CI 生成、context 传递、远端清理和发布证据合同，宿主须自行选择采用相应版本配置；测试与摘要核验不证明完整输入闭包或确定性同判。

已用匿名下载的 macOS arm64 `chrono-ci` 验证发布／full 工作流生成、漂移拒绝与恢复、原始 context 字节及规范 argv；用公开 `chrono-worktree` 清理实际交付分支并核对远端缺席。这些消费验证不代表生成的 full 工作流已在 GitHub 原生执行；具体范围见 [full CI](full-ci.md) 与 [worktree](worktree.md#remote-branch-retirement)。

```sh
chrono-distribution pack --root PRODUCT --plan PRODUCT/.chrono-harness/release/plan.json --output ABSENT_OUTPUT
chrono-distribution assemble --input MAC_PACKAGE --input LINUX_PACKAGE --output ABSENT_RELEASE
```

`pack` 拒绝脏源码，固定实际 Git commit/tree；只复制明确列出的可执行文件，不扫描 crates。`assemble` 显式接收平台包，验证每个成员，拒绝源提交、树、版本或成员集不一致及重复平台。最终 `release.json` 使用 `chrono-release/v1`，包含 `version/source_commit/source_tree/platforms`。平台映射中每个具名工具有 `file/sha256/size`；文件名为 `NAME-OS-ARCH`，每个平台须提供 `chrono-distribution`。没有 archive extraction。

原生平台目前登记 `macos-aarch64`、`linux-x86_64`；实际产物以构建结果为准。其它平台需要显式增加构建与运行验证。构建证据、声明的源码身份及摘要不构成完整工具链/SDK 来源认证，也不保证不同平台结果相同。

[beta.16](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.16) 已标记为公开 prerelease，固定源码为 `5428183235e8ccd56574ced6b303d9294e3a42ef`，包含 beta.15 之后的成对报告比较、显式输入绑定与覆盖登记、Cargo v5 工具链清单，以及 Git 声明文件核验。[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36655069168)在 15 个专属测试项目中通过 macOS arm64 的 588 项测试、Linux x86_64 的 583 项测试；差异是 5 项已有的 macOS 文件系统别名测试。每个平台原始报告保留 36 个成功进程、30 个暂存目的路径与 34 轮匹配的身份观察，15 个工具资产均与平台清单一致。共发布 35 个资产；合并清单 SHA-256 为 `fd9a3bda21db5ef1dff298e5a78463c346c69f89330477c08e4743480e6a19b7`。

匿名下载的清单和 macOS 安装器已按公开摘要核对，并在公开 Go 示例的本地 clone 中安装原有 6 个具名工具，实际执行 rates 与 harness 两个单元。声明的 Git 文件变化／缺席变化导致业务操作前失败，恢复后通过；原始历史配置未改写。启用 Git 记录时测得的报告超出旧 8 MiB 上限，消费配置显式采用 16 MiB，读数与命令见 [Git 输入核验](git-facts.md#public-binary-consumption)。这次本地消费没有更新示例的远端配置，也不代表生成的 provider-v3 已在原生 CI 启用。完整输入闭包、完整宿主启用与本地／CI 完整对等仍未完成。

[beta.17](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.17) 从固定源码 `47a18da29fb3aa92a147f0df8783dc49407175f5` 公开发布，新增 scoped／provider Git facts 的显式平台配置选择。[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36661720249)在 15 个专属测试项目中通过 macOS arm64 的 596 项测试、Linux x86_64 的 591 项测试；差异仍为 5 项 macOS 文件系统用例。每个平台核对 36 个成功进程、30 个暂存目的路径、34 轮身份观察及 15 个资产。共 35 个公开资产的摘要与大小均匹配原始产物；合并清单 SHA-256 为 `34e7286111e2d38e4254c40683310ff915292ff1be0a0ac2193ab9c0dfea5034`。

匿名安装的 macOS beta.17 已在 Go 接入候选上通过规范 DELTA 检查；global 配置变化及已登记缺席的 include 出现均在业务操作前失败。beta.17 原候选的原生 Linux 单元通过，当次汇总因报告读取上限失败，详见[原生接入边界](native-ci-adoption.md#native-git-policy-adoption)。发行产物通过验证不等于该宿主已完成接入、完整输入闭包或同判。

[beta.18](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.18) 已公开发布，固定源码 `b9665d1fa5b465d57ab84d5042712f30b297e882`，源码树与 PR #91 的 dev 落地相同。它分发 scoped v3 的可配置收集上限及紧凑报告；原始过程字节和候选／配置绑定保留，调大上限不能授权复用旧绑定报告。[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36666465494)在全部 15 个专属测试项目中通过 macOS arm64 的 601 项、Linux x86_64 的 596 项测试；差异为 5 项已有 macOS 文件系统用例。每个平台核对 36 个成功进程、30 个暂存目的路径、34 轮身份观察和 15 个工具资产。35 个公开资产的摘要与大小均匹配原始产物；合并清单 SHA-256 为 `5a59cb11fe857a31bbde87dde46828ecae072527b3cae3ed254b8fdc607ae74c`，6252 字节。匿名下载清单及两个平台的安装器也匹配。完整输入闭包、完整宿主治理及跨平台同判仍未认证。

[beta.19](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.19) 已公开发布，固定源码 `f8f28119096e9b50b2456024b473b252ea5f7cdd`、源码树 `0c51ad5c88c04a3e78be5f4edb1ed4b4f3c63e13`，与 PR #96 的 dev 落地相同。它分发显式生成目录的 Git 清单排除，覆盖 full／initial／scoped、registration、projects 和迁移消费者，并与 worktree 共用字面排除规则。[原生发布检查](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36674223982)在全部 15 个专属测试项目中通过 macOS arm64 的 606 项、Linux x86_64 的 601 项测试；差异仍为 5 项已有 macOS 文件系统用例。每个平台核对 36 个成功进程、30 个暂存目的路径、34 轮身份观察与 15 个工具资产。发布后核对全部 35 个资产的摘要与大小，并核对标签实际指向上述源提交；匿名下载的清单和两平台安装器也匹配原始产物。合并清单 SHA-256 为 `857caddfba0ebfe0ec5780c7a31217b30ef0c65bff1b2b6152c8587bf439c731`，6252 字节。完整输入闭包、完整宿主治理和确定性同判仍未认证。

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

beta.11 的匿名公开安装入口在已升级混合宿主的真实 clone 和含空格／Unicode 的子工作树中运行，核对所选工具的公开摘要。原始 checkout 消费拒绝 Git `core.filemode=false` 隐藏的执行位变化；重建消费保留暂存／未暂存、二进制、可执行文件、字面 symlink 与旧 metadata，并通过登记 cleanup 清理已保存工作。该次消费平台为 macOS arm64；不外推其它恢复状态或原生 full dispatch。
