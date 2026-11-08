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

当前 macOS 27 政策声明了标准输入选择、CI 事件／Git／传输环境、获取凭据、测试原件目的和已安装工具消费者，绑定覆盖全部配置环境与工具节点。公共 FILEMAP 点名实际消费者，包括使用 `CARGO_INCREMENTAL` 和 `CARGO_TERM_COLOR` 的 Cargo 构建／测试负责人。执行器按这些原样声明操作，`GH_TOKEN` 仍是获取凭据，不进入检查业务输入。

macOS 27 不再把某次源码 bootstrap 的 `chrono-ci`、`chrono-worktree`、`chrono-inputs` 安装摘要作为后续构建的固定预期。三个文件输入、对应绑定引用和配置专属 FILEMAP 边一同删除；工具名、版本及其实际消费者继续由现有工具登记和依赖边负责。本次安装的文件身份由实际 bootstrap／检查报告产生，按其消费者与保留期使用，不复制回跟踪政策。

macOS 27 不固定 curl、shasum、tar 的系统版本输出；它们使用与 macOS 26 政策相同的 `expected_version: null`，命令和真实下载／校验／解压用途继续登记。Verus 的所选版本、下载地址、摘要和证明结果合同归现役 bootstrap／项目操作。无实际使用方的 `CHRONO_XCODE_CONTENTS` 不进入环境。SDK、开发目录和链接器不覆盖为某一 Xcode 安装的绝对路径；沿已登记变量继承当前选择，未设置时由既有 Rust／Cargo 和 Xcode 工具选择。这些取舍不补齐 full 输入或执行物绑定。

普通构建与行为测试沿各自登记计划执行。

macOS 27 与其他直接配置一样使用共享判官登记，`runner.sha256` 保持未绑定；删除仅在七个旧安装摘要上有差异的独立判官清单。调用参数、顺序、版本和职责继续由共享登记负责。这是退役过期宿主观察，不能解释为省略实际执行物验证后的 full 通过：该采用仍为 proposed／not-implemented／incomplete，现有 full 绑定检查继续拒绝缺失身份，完整治理及真实 full 验收仍待完成。

其他消费者、原快照、完整编译器／SDK 输入和真实 CI 采用仍不齐，政策保持 `status: proposed`、`enforcement: not-implemented`、`input_closure.status: incomplete`。验证沿用候选 bootstrap、标准变更检查和真实 PR／落地检查，总验证及存储成本未测。

可写 `CARGO_HOME` 为当前工作树的 `.chrono-harness/cache/bootstrap-owner-tooling/cargo`，与登记源码 bootstrap 一致；每个工作树使用自己的 Cargo home 缓存／配置。HOME、TMPDIR 继承现有环境，不写其他工作树目录。bootstrap 绑定所选 Rust 工具的版本和组件摘要；SDKROOT／DEVELOPER_DIR 等沿现有环境继承；路径选择没有建立 SDK 内容绑定，祖先 Cargo 配置及其余输入是否完整仍待验证。

bootstrap 与标准检查继续使用该 Cargo home，所选必需输入缺失仍失败。本地和 CI 必须使用同一登记命令；环境间必然同判还要求完整相同有效输入和确定性求值。当前生产者仍为 `completeness_proven: false`，报告保持 `parity: unestablished`，没有独立比较或改写报告命令，也不增加普通完整检查门。

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

采集各端时，`--unit ID` 使用该端登记的单元名。单元改名后分别用旧名和新名采集，再用 `pair` 配对原快照。registration 按候选单元的计划验证两端输入，`pair`、`compose` 继续核对明确端点和原件。

## 离线组合已有原件

`chrono-inputs compose --host-root H --config P --base BASE_OID --candidate CANDIDATE_OID --manifest M --output O --receipt R` 离线组合明确地址的原配对。有限清单使用 `chrono-input-composition/v1`，`sources` 各含 `pair: {path, sha256}`、可选 `transport: {source_directory, directory}` 和原报告 `artifacts` 映射。不自动发现来源，最多 4096 项，JSON 仍限 64 MiB。复制和身份检查沿原描述符／传输要求流式进行。

组合核对每份快照精确端点、配置、选择器和环境绑定。兼容重叠观察可合并，存在状态、摘要、环境或配置矛盾则拒绝。两端必须覆盖全局必需的全部声明输入，包括已选业务测试之外的检查程序输入；缺覆盖时点名端点和输入，不用实时业务输入、版本探测或 SDK 安装补齐。

来源 `pair` 地址也可指另一端存在前采到的一份真实单端快照，由其 commit 决定所属端。两端仍须从明确来源取得完整覆盖。原快照／pair 字节保存在 `.chrono-harness/state/inputs/originals/`，独立 `chrono-input-composition-result/v1` 回执点名清单、组成项和输出。快照 v2／v3 形状不变，回执仍为 `completeness_proven: false`。

快照 v3 校验接受现役 Git v3 选择器生产的 `selection.registry_path`，要求它为有效相对路径；组合仍核对完整选择身份与固定端点登记，路径漂移继续拒绝。Rust 真实采集／配对／组合用例覆盖该字段及无效路径，真实宿主两端组合另作验收。完整输入／治理和同判仍未建立，验证总成本未测。

pair 原样传递缺失状态，不打开历史路径。registration、routes、projects 共用验证，操作后再核候选。任一端为 config v2 时，有效结果用 `chrono-effective-inputs/v2`；文件事实含 `presence`，存在项另有摘要／长度，缺失项两者都没有。状态变化须匹配登记变化，只沿明确 `input:ID` 边传播，包括旧边；稳定缺失及无连接变化不选无关测试。

v2 位置拒绝符号链接路径段，包括悬空链接，以及 `..`。平台临时目录经链接暴露时使用标准绝对位置。父目录不存在可算缺失，但文件位置是目录、祖先不是目录、权限或其他 IO 错误都不是缺失。前后观察检测持续变化，不检测观察间改后恢复。

两端可以都为 v2。workflow v3 可按精确端点版本选择宿主登记的 v1→v2 解码器，见[解码说明](execution.md)，其专属兼容测试必须执行通过。历史 config 和 v1 快照保留原义，未绑定摘要不能变成缺失。运行时不生成宿主转换算法，也不因此启用本仓 proposed 完整配置或补齐编译器／后端／链接器／SDK／构建脚本输入。

## 输入获取范围

通用 `observe` 命令及其目录盘点输出已退役。当前宿主与已审计的四个固定外部宿主未发现采用，找到的调用仅来自专属合成测试；未知外部使用仍未知。不另建替代观察入口。

当前输入工具保留 `capture`、`pair` 和 `compose`。它们按明确端点和登记保留实际文件、缺失状态、环境及来源原件，不自动发现依赖，也不补齐编译器／SDK／配置输入。完整输入闭包和同判仍未建立，成本收益未测。

本仓已退役只向旧输入分支上传主机盘点的原生初始获取链：当前检查和发布没有消费其全工具安装及编译器／SDK／Python／配置盘点输出。对应工作流、转发脚本、宿主观察声明、探测 helper、acquire／observe／metadata 操作和专属测试一同移除；不另建替代盘点入口，也不把这项退役列为待激活义务。

普通构建继续使用登记的锁定 Cargo 操作；core／所有者 bootstrap 仍实际调用的 fetch 保留。安装工具与原件由当前生产者按自己的实际使用方保管，不为历史盘点重建主机快照。普通输入快照与传输保持各自合同；其余采用仍须核对当前使用方，不能由测试或声明存在证明必要。

根选择器采用既有 `chrono-git-configs/v1` 平台映射，四份 scoped schema4 政策退役 Git SHA／版本白名单、重复输入和对应 FILEMAP 边。Git facts 与工作树均一次绑定实际 Git，保留真实版本进程和逐调用字节保护；不另造本机快照或激活层。显式预期及 full v3 合同继续严格核验。格式与固定基线数据边界见[Git 选择](git-facts.md#exact-host-executable-selection)和[工作树政策](worktree.md#git-身份与固定输入)。

旧样本、声明和比较归 Git 历史；仍有用途的缺失原件与独有工作由[输入保管者](input-source-custody.md)负责。退役不补齐历史观察，也不证明完整输入、固定两端原件、full CI 验收或同判。

### 工具链入口与本机路径

普通本机操作按有效 `PATH` 解析登记的 Cargo 一次，仍核对精确版本。macOS 27 保留
`RUSTUP_TOOLCHAIN=1.95.0`，继承实际 `PATH`、`RUSTC`、`RUSTFLAGS` 和 `RUSTUP_HOME`，
不把某次个人 checkout 的工具链目录写成后续候选的运行预期。
调用环境须能找到已有 Cargo，bootstrap 另须能找到 rustup；使用私有 Rust 安装时由调用方
在进程环境提供其 `PATH`／`RUSTUP_HOME`。缺工具时保留原失败，不在 Git 中补个人路径。

bootstrap 使用已有 profile、`rustup which --toolchain` 与官方组件摘要选择并重核
Cargo／rustc；编译器、wrapper 和实际路径观察沿该生产者报告消费。完整宿主工具表
不再重复登记没有直接 action 的 rustc／rustup，相关声明和 FILEMAP 边同步移除。
这不改变 bootstrap 的固定版本、组件绑定或失败拒绝，也不建立完整输入或同判。

### 退役 diagnostics 的专属输入链

diagnostics 的路径依赖使用方会自行编译源码，不消费独立受保护构建的产物。当前没有额外环境保护的必要验收终点；本仓使用普通锁定 Cargo 构建，专属观察、私有 Cargo home 准备、vendor 目录来源、主机快照、配置级输入和启动 profile 已退役。

当前安装身份由 bootstrap／检查原件提供；普通输入按显式摘要、端点快照及实际消费前的字节检查处理，diagnostics 格式、构建和未过滤测试仍执行。其余输入采用仍须分别核对实际必要性，现有引用不证明永久保留。输入闭包继续 incomplete，完整同判未建立，成本收益未知；历史原件按真实调试／验收义务保管。

<a id="native-diagnostics-originals-before-guard-adoption"></a>

原 diagnostics 原生保护准备链及其合格原件属于历史来源；当前入口已退役。历史配置、固定身份、观察和失败的完整对应保存在 Git 及原有限保留 artifact 中，不再复制为当前程序输入。
