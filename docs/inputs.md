# 明确收集并保留输入快照

`chrono-inputs` 只采集固定配置 `environment.inputs` 中的文件 ID，以及 `environment.inherit` 中的变量名。可选 `capture --unit ID` 通过 registration／input 共用的前置范围投影，为已登记完整单元采集输入，包括共享操作和检查程序自身的消费者。

它不发现依赖、不修改预期摘要、不增加测试，也不宣称输入齐全。它是独立 Rust 生产／测试配对，读取固定提交需要 Git。config v3 使用明确的[Git 事实绑定](git-facts.md)，v1／v2 保留原读取器。本仓 Cargo／SDK 输入登记仍不齐全，安装传输工具不会全面启用完整检查。

## 把输入绑定到实际消费者

config v3 可选择 `input_closure.bindings` 白名单。每项都有唯一 `id`、`consumer`、调用方定义的非空 `kind` 和非空 `inputs`：

```json
{
  "id": "registration-git-facts",
  "consumer": "judge:registration",
  "kind": "git-facts",
  "inputs": ["tool:facts-git", "input:facts-git-bytes", "environment:PATH"]
}
```

消费者须已登记为 `project`、`script`、`test` 或 `judge` 节点，输入须已登记为 `tool`、`input` 或 `environment`。FILEMAP 的 `project_edges` 还须有每个输入到该消费者的明确边。边的 `kind` 由 FILEMAP 独立验证，不必等于绑定里由调用方定义的 `kind`。

节点未知、类型无效或缺少输入到消费者的边，registration 报 `E_REFERENCE`。config v1／v2 拒绝新字段，保留历史格式。

`declared-complete` 表示可信 AI 对宿主所定义检查和输入范围作出的负责工程声明。程序仍核声明引用、身份、两端快照和实际操作；已知缺输入、必需依赖未解决、缺绑定／快照、漂移都要失败。声明不能抹掉问题或替代生产者结果，所选范围内真实工具链、SDK 和配置义务仍保留。

v3 将输入范围标为 `declared-complete` 时，必须提供 `bindings`，每个登记 `tool`、`input`、`environment` 节点至少出现在一条绑定中。它检查清单一致性，不能证明不存在未登记外部输入，所以 registration 仍报告 `completeness_proven: false`。

登记要求和选中义务都通过后，普通完整检查可以在这个值下通过，只表示该次变更满足现役登记要求。普通验收不要求普遍证明隐藏输入不存在，也不要求虚拟机。本仓声明仍为 proposed／incomplete。

当前 macOS 27 政策声明了标准输入选择、CI 事件／Git／传输环境、获取凭据、测试原件目的和已安装工具消费者，绑定覆盖全部配置环境与工具节点。公共 FILEMAP 点名实际消费者，包括使用 `CARGO_INCREMENTAL` 的 34 个 Cargo 构建／测试负责人；它们也绑定 `CARGO_TERM_COLOR`。执行器按这些原样声明操作，`GH_TOKEN` 仍是获取凭据，不进入检查业务输入。

两项文件输入绑定现已把既有 runner 和 diagnostics Cargo 保护消费者连到它们所选的显式 FILEMAP 输入。runner 在原有 18 项文件绑定旁补充 2,653 项引用；diagnostics 绑定其声明的 1,138 项文件。因此政策中的全部 2,671 个文件输入节点都已出现在绑定内。它们属于现有包／缓存、配置／缺失、可执行／运行时和清单声明，没有发现或新增输入、依赖边或执行。diagnostics action 保留登记的配置与政策选择器，为绑定的本地 Git 选择 macOS 27 政策和 `cargo/diagnostics-local.json`。

其他消费者、原快照、完整编译器／SDK 输入和真实 CI 采用仍不齐，政策保持 `status: proposed`、`enforcement: not-implemented`、`input_closure.status: incomplete`。验证沿用候选 bootstrap、标准变更检查和真实 PR／落地检查，总验证及存储成本未测。

可写 `CARGO_HOME` 现为当前工作树的 `.chrono-harness/cache/bootstrap-owner-tooling/cargo`，与登记源码 bootstrap 一致。package／配置输入位置及本地 diagnostics Cargo 政策一同移动，预期摘要不变。HOME、TMPDIR 继承现有环境，不写其他工作树目录；独立工作树因此不共享可变 Cargo 缓存／配置，明确绑定的 Rust 工具链和 SDK 仍是共享只读输入。

bootstrap 获取和实际保护／标准检查消费者须验证移动后的 Cargo home，必需字节缺失仍失败。本地和 CI 必须使用同一登记命令；环境间必然同判还要求完整相同有效输入和确定性求值。当前生产者仍为 `completeness_proven: false`，报告保持 `parity: unestablished`，没有独立比较或改写报告命令，也不增加普通完整检查门。

一对相同观察不能证明输入齐全或普遍确定性，不得手填更强结论。

## 声明各类输入是否已覆盖

config v3 可选择带版本的 `input_closure.coverage`：

```json
{
  "schema": "chrono-input-coverage/v1",
  "required": [{"consumer": "judge:registration", "domains": ["git-facts", "remote-response"]}],
  "rows": [
    {"consumer": "judge:registration", "domain": "git-facts", "state": "bound",
     "bindings": ["registration-git-facts"], "reason": "绑定 Git 字节和环境"},
    {"consumer": "judge:registration", "domain": "remote-response", "state": "not-applicable",
     "bindings": [], "reason": "该消费者只读取本地对象"}
  ]
}
```

`required` 点名非空可执行消费者范围及各自非空、唯一的领域。每个消费者／领域组合恰好对应一行，缺少、额外或重复都失败。领域名由宿主定义，例如编译器库、SDK、委托工具、配置、网络、时钟、随机性、并发；不从语言、目录或命令推导。

每行须有非空原因。`bound`、`absent` 要引用同一消费者的非空绑定，绑定本身也须有非空输入。一个绑定可服务多个领域，但同一行不能重复。`absent` 还要求全部引用节点都是明确声明缺失的文件，由已有保留／当前输入检查核实际缺失；因此原声明缺失的凭据文件出现，即使领域行没变也失败。

`not-applicable`、`unresolved` 不允许列绑定。前者只是 AI 对适用范围的声明，不是实际观察到缺失。采用 coverage 且标为 `declared-complete` 时，全部绑定至少被一行引用，不能有 unresolved。输入未齐的配置可保留 unresolved，但仍不能通过启用检查。

registration 成功响应以 `chrono-input-coverage-result/v1` 输出 `input_coverage`，保留原声明，范围为 `declared-domains`，且 `completeness_proven: false`。这验证清单一致和声明文件状态，不证明没有未披露读取或 OS／网络／工具链完整。

省略 coverage 保留原 v3 要求和报告格式，v1／v2 拒绝扩展。新消费者读取旧 v3 快照不会替它补 coverage，旧二进制拒绝未知字段。AI 采用时同步升级所选程序、加入自己明确选择的范围和行。

这是一张关系表，不是依赖发现器。registration 不扫描目录、manifest、语言文件、命令或工具链来补绑定。有效绑定只记录 AI 声明了什么，不能证明宿主没有漏输入。`input_closure.status`、`unresolved` 记录负责声明和剩余工作；Cargo／编译器／SDK 仍需各自登记和测试。

## 采集和配对命令

```sh
chrono-inputs capture --host-root /path/to/host \
  --config .chrono-harness/config.json --commit FULL_COMMIT_OID \
  --output .chrono-harness/state/input-candidate.json

chrono-inputs pair --host-root /path/to/host \
  --base-snapshot .chrono-harness/state/input-base.json \
  --candidate-snapshot .chrono-harness/state/input-candidate.json \
  --output .chrono-harness/state/inputs.json
```

capture 要求检出精确提交，磁盘配置字节与固定 blob 相同。它读该提交的原登记值，不是未提交检查或完整通过报告。每个观察到的普通文件通过有限缓冲复制到 `.chrono-harness/state/inputs/blobs/<sha256>`，同内容共用一个 blob，已有内容地址复用前核身份。

v1 缺失／非普通输入、无效路径会失败，不发布快照。v2 可记录明确观察到的缺失，非普通文件仍失败。pair 绝不从今天的文件重建旧字节。

config v1 快照为 `chrono-input-snapshot/v1`，包含 `commit`、`config_path`、标准配置值摘要 `config_digest`、`environment` 和 `files`。环境保留字符串、空字符串、缺失／null；配置覆盖仍在 config 中，不采未声明环境变量。文件 ID 映射到 `{blob, sha256, length}`。

这些是观察值。即使文件变化，capture 也可采到它而不改登记预期哈希，之后检查程序会拒绝不符。快照成功只表示采集完成。

pair 保留两份明确提供的快照。可选 `--base-root`、`--candidate-root` 选择来源仓库，默认都为 `--host-root`。它将引用 blob 复制到目的状态目录并核身份，不访问历史外部输入位置，也不猜新位置。快照和 blob 可一起作为 CI 产物传输。基线原字节必须仍可用，缺基线 blob 就失败，即使候选字节存在也不能替代。

快照／配对输出必须在 `.chrono-harness/state/`。重复相同输出不写入，不同内容碰到已有路径则报 `E_SNAPSHOT_EXISTS` 并保留旧证据；新观察须选新路径。blob 和输出引用拒绝符号链接路径段及越界。普通 IO 用临时文件和不覆盖发布，不保证断电恢复或源码并发变化。

context 的 `retained_inputs` 指向生成 pair。registration 接受原内嵌 `{bytes: [...]}`，或恰好一个 `{blob, sha256, length}` 引用，核对端点／配置绑定、文件 ID、原 blob 摘要／长度及当前候选磁盘身份。

完整单元采集和实时验证共用明确单元输入投影，无关保留项可省略，提供的仍须验证。普通完整检查和汇总保留全局输入义务；汇总核原 blob，并按对应投影比较各原单元证据，不读实时业务输入、不启动业务／版本工具。结构和引用检查仍全局进行。

未知文件 ID、冲突表示、缺失／损坏 blob、候选输入变化都失败。blob 流式读取，不进入检查 JSON；匹配的内嵌和 blob 产生相同有效输入结果。报告仍为 `completeness_proven: false`，哈希只绑定声明输入，不证明没有遗漏。

专属测试覆盖二进制／环境原样保留、不可变输出、去重、原输入消失后传输、坏地址、无效路径和真实七程序消费者。大文件与拒绝用例调用真实检查入口，不手造成功报告。

## 文件存在、缺失和平台选择

config v2 每个输入必须明确为 `{id, location, presence: "present", sha256}` 或 `{id, location, presence: "absent"}`。缺失没有 hash 字段，空 hash 仍表示未绑定，绝不是 absent。

v2 采集生成 `chrono-input-snapshot/v2`，普通文件仍用 blob，实际缺失为 `{absent: true}`，空文件也有独立 blob 和零长度。capture 记录真实状态，即使与声明冲突；registration 负责拒绝矛盾。v2 要求版本化快照、精确配置绑定及每个 ID 的明确状态，v1／无版本旧观察不能表达缺失。

入口是平台选择器时，生成 `chrono-input-snapshot/v3`。`config_path` 仍为调用方入口，`effective_config_path`、`selection` 绑定选中的直接政策。结构选择绑定 `chrono-registry-selection/v1` 含入口、目标、平台；选择器摘要仍在固定登记字节中。registration 按固定端点核绑定，并按真实路径保存 selector／目标。直接 v1／v2／v3 配置继续使用原快照格式。

新增或改名候选单元，可在 `--commit`、`--unit` 之外同时给 `--base BASE_OID --candidate CANDIDATE_OID`，commit 必须等于其中一端。它使用 registration 已有两端投影，包括基线尚无候选单元 ID 时的历史要求；仍只观察当前检出端，不拿当前字节代替历史观察。

## 离线组合已有原件

`chrono-inputs compose --host-root H --config P --base BASE_OID --candidate CANDIDATE_OID --manifest M --output O --receipt R` 离线组合明确地址的原配对。有限清单使用 `chrono-input-composition/v1`，`sources` 各含 `pair: {path, sha256}`、可选 `transport: {source_directory, directory}` 和原报告 `artifacts` 映射。不自动发现来源，最多 4096 项，JSON 仍限 64 MiB。复制和身份检查沿原描述符／传输要求流式进行。

组合核对每份快照精确端点、配置、选择器和环境绑定。兼容重叠观察可合并，存在状态、摘要、环境或配置矛盾则拒绝。两端必须覆盖全局必需的全部声明输入，包括已选业务测试之外的检查程序输入；缺覆盖时点名端点和输入，不用实时业务输入、版本探测或 SDK 安装补齐。

来源 `pair` 地址也可指另一端存在前采到的一份真实单端快照，由其 commit 决定所属端。两端仍须从明确来源取得完整覆盖。原快照／pair 字节保存在 `.chrono-harness/state/inputs/originals/`，独立 `chrono-input-composition-result/v1` 回执点名清单、组成项和输出。快照 v2／v3 形状不变，回执仍为 `completeness_proven: false`。

`chrono-inputs capture-governance --host-root H --config P --base OID --candidate OID --manifest P --output P` 使用同一原来源清单提供基线检查程序输入投影，再单独观察已检出候选，不把候选字节当历史基线。必需输入来自 registration 已有的空业务计划投影。

生成 pair、流式 blob 和 `OUTPUT.receipt.json` 保留源身份及原字节；缺基线观察／覆盖，或端点／配置／存在状态／blob 不符都会拒绝。真实 CI 发布在种子发布前，把 provider 明确的 `composition_sources` 交给此命令。原件由宿主登记 bootstrap 提供，日常检查命令不变。

pair 原样传递缺失状态，不打开历史路径。registration、routes、projects 共用验证，操作后再核候选。任一端为 config v2 时，有效结果用 `chrono-effective-inputs/v2`；文件事实含 `presence`，存在项另有摘要／长度，缺失项两者都没有。状态变化须匹配登记变化，只沿明确 `input:ID` 边传播，包括旧边；稳定缺失及无连接变化不选无关测试。

v2 位置拒绝符号链接路径段，包括悬空链接，以及 `..`。平台临时目录经链接暴露时使用标准绝对位置。父目录不存在可算缺失，但文件位置是目录、祖先不是目录、权限或其他 IO 错误都不是缺失。前后观察检测持续变化，不检测观察间改后恢复。

Cargo 保护保留有关联的已声明缺失，并在真实操作后复核。absent ID 不能提供 package manifest、配置、包字节或工具；配置查找清单和关系仍须明确，不推断生成。

两端可以都为 v2。workflow v3 可按精确端点版本选择宿主登记的 v1→v2 解码器，见[解码说明](execution.md)，其专属兼容测试必须执行通过。历史 config 和 v1 快照保留原义，未绑定摘要不能变成缺失。运行时不生成宿主转换算法，也不因此启用本仓 proposed 完整配置或补齐编译器／后端／链接器／SDK／构建脚本输入。

## 按登记观察和获取输入

`chrono-inputs observe --host-root H --manifest P --output P` 为明确的获取声明产生事实，复用当前保留进程模块；capture、pair、capture-governance、compose 要求不变。观察不自动登记发现的文件，也不改预期身份。

清单明确选择平台、环境、有界操作、字面或前次 stdout 提供的路径、文件及有界目录范围。登记操作解析唯一项目／脚本 action，沿用原参数；前置可执行／编译器选择、单操作限制和失败即停都明确声明。报告保留来源 manifest／projects 原件、可执行身份、启动记录和原输出；非零、超时、溢出、别名失败仍是失败，发布不覆盖旧输出。

观察器中断后若只留下进程原件、没有最终报告，重试在原进程目录下预留新尝试目录。新报告只引用新尝试，遗留原件保持完整供诊断／验收。已有已发布报告仍禁止复用输出路径。获取负责人保留失败尝试到调试和验收结束，再交宿主普通证据保留／清理流程。

目录观察可生成 `chrono-input-directory/v3` 原件，保存精确子项，包括根 `.`、空目录和只有别名的目录。Cargo 保护读取原拓扑，并在真实消费后重核。按标准目标观察文件会保留走过的别名身份，不放宽普通输入路径校验。配置／凭据存在性观察只留身份或缺失，不留凭据内容。

Python 辅助程序归宿主 bootstrap，明确选择 stdlib、platstdlib、扩展、运行时镜像和配置位置，不声称整个 Python 基目录都是输入。

登记的原生来源链为：`host.acquire-native-{macos,linux}` → 所选 bootstrap-acquire 配置 → 源码构建的 14 个安装参与工具 → `inputs.acquire-PLATFORM` → `inputs.provision-native` → 全部 34 个登记 fetch → `inputs.observe-PLATFORM`，包含全部 34 组 fetch／metadata。

原 macOS／Linux 编译器、SDK／完整 Developer、运行时／配置和失败别名范围仍保留。只有配置操作成功后才安装，获取失败不能推定二进制已经安装。构建前存活的协调所有者是另一个参与者，仍须实际源码／政策／工具链／安装身份和受保护关联工作树使用。bootstrap 在 stderr 转交原所有者报告路径；该保留报告才是受管理进程和租约的依据。

独立来源链为 `inputs.provision-isolated` → 已有 worktree `use inputs.produce-admit-isolated` → `host.bootstrap-isolated`、34 次 fetch、紧接的 `guard.check.runner`。保护配置点名当前直接政策 `.chrono-harness/git/macos-27.json`，不把根 Git 身份选择器重新解释成直接政策。

该消费者仍为 proposed，实际 Cargo、编译器、package／resolver、SDK、配置和保留输入绑定尚未解决。政策保留必需领域和预期包参数，不采用历史机器字节。有源码不等于保护机制已采用。执行前，负责人须绑定并核对观察器选中的 Cargo／Rustup home、rustc 与私有 bootstrap 及保护配置完全相同，仅继承 home 不证明一致。

获取参与仍保留当前根 worktree v3 政策。这些来源声明没有采用 Linux 关联／协调政策或完整 Linux 宿主配置，真实平台采用仍待完成。

新增真实 CI 工作流选择有限输入集成分支并传递两份登记配置。原事件、参数、输出、退出、run／attempt、工作流修订随获取原件及 14 个安装目的保存。工作流 job 键不等于远端数字 job ID，须从原运行解析真实 job、artifact 和生产 attempt。提交工作流不代表已经运行、已消费安装工具或已落地。

根 `chrono-git-configs/v3` 绑定两个当前原生可执行身份，明确共用固定数据路径；worktree v3 允许这两个身份，未知字节仍失败。格式与不可变数据范围见[Git 选择](git-facts.md#exact-host-executable-selection)。没有额外采用输入集合扩展、历史完整执行政策、beta.21 安装或新安装器。

旧原生 config／runner 样本及过时比较、FILEMAP 项已经移除，固定来源仍在 Git。当前获取测试直接读取现役观察声明；保留的遗漏和原件不同的纠正见[输入来源保管](input-source-custody.md)。全面启用、输入齐全、固定两端原件、真实 CI 验收和同判仍未完成。

### 本机 runner 的输入政策

macOS 27 直接政策通过按配置选择的 FILEMAP 表，把 runner v6 每个字面输入 ID 连到 `project:runner`；固定原生政策保持公共图。包文件和 crate 归档预期来自保留的 PR163，须匹配独立下载的包。所选私有 Rustup 1.29.1 来自官方版本归档及公开校验值，旧 1.29.2 观察仍是失败选择，不能变成预期。

普通保护使用当前工作树 bootstrap Cargo 缓存，以及明确共享只读 Rust 工具链、空包装器、sysroot、链接器和 SDK 环境。配置缺失记录点名本工作树 Cargo home。包参数包含当前 runner 直接依赖 `tracing`，它已在声明的传递包清单中，不改包文件或 resolver 输入摘要。

独立观察器和私有 bootstrap 提案仍使用另一 home，消费前须明确协调，不能认为已与普通保护一致。

本机 runner 的 `cargo-cache.cc.index` 显式采用当前稀疏索引原件：新增 `cc 1.11.0` 后共 231 条记录，原有 230 条版本及 JSON 记录的字节不变，锁定的 `cc 1.4.6` 记录和包校验值也不变。此前保护正确拒绝旧登记摘要与当前索引的差异，尚未启动 metadata 或编译。本次只更新这个完整文件的预期摘要，不增加索引等价规则或自动更新预期的路径；其他包、配置、编译器、SDK 和前后身份检查继续适用，真实保护消费和候选验收须另行核验。

当前 Xcode 获取识别 Xcode 26.5／17F42。只登记 Developer 时，`usr/bin/altool` 别名指向兄弟 SharedFrameworks，因越出范围被拒绝。因此选中的目录清单改覆盖外层 `Xcode.app/Contents`，directory v3 检查不变；`DEVELOPER_DIR` 仍指 Developer，`CHRONO_XCODE_CONTENTS` 选择外层清单。当前本机 runner 和 diagnostics 的构建政策从原清单选择 `.chrono-harness/inputs/local-xcode-build.json`：声明的链接工具、四个发行库、clang 21 资源及别名，以及发行／工具链 metadata。文件摘要和精确父目录子项名不变。移动 SDK 和图形应用内容不属于此构建子集；完整 macOS SDK 仍单独绑定。[Cargo 说明](cargo-projects.md)记录了依赖审计和政策边界。SDK、Developer、配置、运行时范围仍明确。

原拒绝获取和后续成功结果分别保留。生成 Rust、SDK、Xcode 清单是登记状态模块下的必需原输入，摘要在所选政策中；只有来源声明，不能提供或证明这些字节。

输入位置变化属于宿主政策和登记迁移，不启用完整检查。当前 Git 选择另行登记。保护执行、干净候选 bootstrap／标准变更、真实获取和安装后消费证据都须分别标明实际来源。历史端点原件缺失、完整输入／同判、其他受保护项目、批次评审和实际交付仍待完成。获取／构建／检查窗口及保留字节须分别测量，总成本和节省未知。

登记的 macOS 原生获取对已声明 Xcode 26.6 也使用外层根，保留 `xcodebuild -version`，发布独立 `guard-inputs/xcode-contents.json`。Contents 根直接登记绝对位置，观察要求拒绝 `..` 后缀，原拒绝尝试保留。34 组 fetch／metadata、14 安装目标、目录项上限、失败不安装都不变。真实完整保护须先明确绑定新原件／根，获取本身不启用它。

<a id="native-diagnostics-originals-before-guard-adoption"></a>

## 先取得真实 diagnostics 输入原件，再采用保护

登记的原生 diagnostics 生产入口为 `host.bootstrap-diagnostics-inputs`：

```sh
/usr/bin/python3 -B .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-diagnostics-inputs.json
```

它通过已有 Rust 1.95.0 bootstrap 登记，在普通原生 `/Users/runner/.cargo` 范围构建候选 `chrono-inputs`，允许为锁定观察器 manifest 获取输入，再委托 `inputs.observe-diagnostics`。后者使用登记 diagnostics fetch 及锁定／离线 metadata 参数。私有宿主相对 Cargo home 只保存该消费者的获取结果，不准备全部 34 项目。

fetch 可下载缺失锁定包，metadata 保持离线；不执行保护，也不改预期身份。观察明确选择原生 `/Users/runner` Rust 1.95.0 和已声明 Xcode 26.6，保留源码 commit／tree／status、Cargo／编译器身份、原生 target 输出、SDK／developer／工具选择、根／home 及工作树祖先 Cargo 配置事实。

另保留四份 directory v3 原件：Rust sysroot、SDK、外层 Xcode Contents、diagnostics 专属 Cargo registry。后端和构建脚本工具字节仍可在其中定位。根缺失、选错、别名不支持、操作失败或清单超限，都由已有观察器保留失败；观察版本不会成为新预期版本。

全部输出归 `.chrono-harness/state/diagnostics-native/`：bootstrap 报告及流、`observation.json` 及其 source／launch／stream 原件、`guard-inputs/{rust,sdk,xcode-contents,packages}.json`、私有 Cargo home。普通 diagnostics 单元已上传 `.chrono-harness/state/`，可用原传输保留完整依赖。各观察子进程仍限 120 秒／1 MiB，每个目录最多 200,000 项，原生 job 仍限 45 分钟。清单 IO、产物体积和总原生耗时未测。

普通原生集成点在 `ci/units.json` 的 `units.diagnostics.bootstrap`：共享启动／核心验证之后、`check --unit diagnostics` 之前。所选保护 bootstrap 先验证导入核心配置，在普通 Cargo home 为锁定 manifest 允许获取并构建候选保护程序，再委托 `host.bootstrap-diagnostics-inputs`，最后安装保护程序后检查。

两份 bootstrap 明确选普通 home 和 `CARGO_NET_OFFLINE=false`，避免继承 diagnostics 专属离线环境阻止前置工具构建。沿用已有 Cargo bootstrap action，不填充私有 diagnostics registry。登记生产者构建自己的观察器，在单元已有状态上传目录保留报告和流。委托声明不能证明真实运行。

本地 diagnostics 输入、环境和编译器边留在 macOS 27 配置范围；原生图保留公共保护／观察器依赖，不借本地输入。输出采用前须核精确候选／源码、原生产 run／attempt、官方 Rust 预期及 Xcode／配置范围。

受保护的 `build.diagnostics` 选择 `.chrono-harness/cargo/diagnostics-config.json`。其原生 Git 身份选择 `.chrono-harness/git/pinned-macos-diagnostics.json`，本机身份仍用 macOS 27。原生直接政策保留私有 Cargo home、必需离线环境、编译器／sysroot／链接器／SDK 和全部包／配置预期。

普通 `.chrono-harness/config.json` 保留 dev 原生环境，以及当前 action 所需保护／观察器工具名。其他按范围构建／测试使用自己的原 Cargo 获取范围和当前锁定 manifest。共享启动与 diagnostics 生产者都不提供普通 registry 内容；无需靠热 registry、具名 crate 补丁或全项目准备解决隔离问题。

**宿主政策与消费者曾同步修改：**私有环境现在只适用于已有保护／观察器消费者，两项 bootstrap 前置选择普通原生环境。Python 测试核所选启动 home、冷前置获取，以及离线冷缓存／错误 home／错误锁拒绝和原输出。来源登记和这些预期不代表实际本地／原生结果，兼容所有者部署、候选检查及真实保护／构建／测试验收仍为必需。

选择器为受保护构建增加一次小型登记读取，总 bootstrap／包／清单／CI／存储成本未测，限制和唯一现有获取路径不变。

原生直接政策和 diagnostics v6 合同已明确采用以下合格原件：干净源码 `6ca53a96f0f0aceca69149a162ac24696bb10bda`，run `40961603966`，attempt `1`，diagnostics job `123757592461`，artifact `12936817318`，ZIP SHA-256 为 `bba25e442cfd89804dfd68cf608883c99143a605ea3640a19adb6ba604103f66`。

所选原生政策绑定原 Cargo／rustc、Xcode 26.6 工具、diagnostics 私有 Cargo home、明确缺失的配置，以及原生 sysroot／SDK／developer 根。按配置选择的 FILEMAP 边保留 1,127 个包源码文件身份，并与合格 registry 清单核对。本机 macOS 27 政策及其输入边仍只适用于本机。

**原生构建输入政策曾有以下调整：**`toolchain.rust.inventory` 现在选择已提交的 `.chrono-harness/inputs/diagnostics-rust-build.json`，SHA-256 为 `d22ae2e29961289dd9835ac4bdd51d9a942e16209702dc76fe9ce46d63157901`。这份固定的 `chrono-input-directory/v3` 保留子集，恰好包含已接纳 J／N 原始 Rust 清单共有的 158 条文件记录、全部 22 条目录记录、原根和所有声明的别名。它只去掉 `lib/rustlib/components` 的文件内容记录；Cargo、rustc、后端、标准库内容和四份组件安装 manifest 都保留精确身份。

当前消费者直接选择固定 Cargo／rustc／sysroot，执行锁定且离线的 metadata／build，不管理组件。启动阶段另核原源码／报告／安装／执行物绑定。因此，构建和来源检查仍绑定这些实际内容，但不再确认安装器组件登记文件的内容。

旧政策绑定全部 159 条原始文件记录。新构建凭据不绑定 `lib/rustlib/components` 的内容、文本成员、文件类型或原始内容变化；未改动的 `lib/rustlib` 目录子项仍要求它的名称和存在。这是明确区分文件内容与目录结构的政策边界，不含“只改顺序就等价”的规则，也不证明该文件的成员或类型，更不证明输入完整或本地与 CI 同判。

所有保留的现用内容、物理父路径、目录结构、根、别名，以及子进程结束后仍存在的保留输入变化，继续由原 v3 消费者保护。配置、包装器、环境、包、SDK／developer／链接器、原启动绑定和上限沿用原合同。本机 macOS 27／默认六组件及普通在线前置构建各自保留原选择。

观察器仍把完整、未改动的 `guard-inputs/rust.json` 写为独立诊断原件，不把它选作本次构建清单。已接纳的 J 原始 SHA-256 为 `792aad114bf2ffec0fea2ebae5fd921cc03446cd4a1a524c933197b3c4e6757b`；N 为 `2452c5a1b1a08232f42c144765c69d1dee9bca7ac24ea9214374bb499662c29b`。

运行时 `sdk.json` 的预期仍为 `95337d9300b02f0e75bde2d6aec955d5a8f3a9fc1e8a951726c15c433f74221b`。完整 `xcode-contents.json` 原件的 SHA-256 为 `866c18f9c3469a4cf734e452748b2eff0c4f3bcbeccccbab515c6a29d3309d4d`，观察器继续独立保留它；构建选择下述固定子集。包文件采用仍以 `packages.json` 为准，其 SHA-256 为 `ba0f4178c06dede474b4d6c4adb39fe0dafaf01af15d3bcde2a5a1532cfcd2da`。

**可见宿主输入政策修改：**原生 `toolchain.developer.inventory` 选择已提交的 `diagnostics-xcode-build.json`。其记录来自上述已采用的 Xcode 26.6 原件，最新失败运行 `42134957152/1` 又保留了相同摘要。范围沿用本机清单的五个构建工具、四个发行库、完整 clang 21 资源子树及版本别名、Xcode／工具链 metadata；文件摘要、字面别名及所选物理父目录的精确子项名称均保留原行。无关应用、移动 SDK 等未选内容不再逐次复核，所选父目录中的对应子项名称仍须存在。完整 macOS SDK、原始观察、各次命令后复核及 600 秒期限不变。这是输入范围调整，不证明原生工具导入依赖完整、系统框架完整或同判；新候选仍须真实原生构建、测试和汇总验收。

这些固定预期来自已确认合格的保留数据，不从当前安装反推。观察器的获取过程和原始输出不变，普通构建明确使用 `--offline`，通用保护行为不变。

登记验证可能选中范围较广的现有消费者，本地／原生验证和存储成本未知。本机 macOS 27 检查不能验证所选原生 Xcode／SDK 绑定；共享 diagnostics 政策现在描述原生消费者，真实原生受保护构建、diagnostics 测试和标准汇总仍待完成。原件缺失、资格确认失败和内容变化继续判失败；生产者成功不代表受保护构建、单元／汇总、完整输入、同判或实际落地已通过验收。
