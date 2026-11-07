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

macOS 27 不再把某次源码 bootstrap 的 `chrono-ci`、`chrono-worktree`、`chrono-inputs` 安装摘要作为后续构建的固定预期。三个文件输入、对应绑定引用和配置专属 FILEMAP 边一同删除；工具名、版本及其实际消费者继续由现有工具登记和依赖边负责。本次安装的文件身份由实际 bootstrap／检查报告产生，按其消费者与保留期使用，不复制回跟踪政策。

`cargo-guard` 已无现役项目操作使用，其工具声明和公共依赖边已退役；runner 和 diagnostics 的专属保护输入也已退役。普通构建与行为测试沿各自登记计划执行。

macOS 27 与其他直接配置一样使用共享判官登记，`runner.sha256` 保持未绑定；删除仅在七个旧安装摘要上有差异的独立判官清单。调用参数、顺序、版本和职责继续由共享登记负责。这是退役过期宿主观察，不能解释为省略实际执行物验证后的 full 通过：该采用仍为 proposed／not-implemented／incomplete，现有 full 绑定检查继续拒绝缺失身份，完整治理及真实 full 验收仍待完成。

其他消费者、原快照、完整编译器／SDK 输入和真实 CI 采用仍不齐，政策保持 `status: proposed`、`enforcement: not-implemented`、`input_closure.status: incomplete`。验证沿用候选 bootstrap、标准变更检查和真实 PR／落地检查，总验证及存储成本未测。

可写 `CARGO_HOME` 现为当前工作树的 `.chrono-harness/cache/bootstrap-owner-tooling/cargo`，与登记源码 bootstrap 一致。HOME、TMPDIR 继承现有环境，不写其他工作树目录；独立工作树因此不共享可变 Cargo 缓存／配置，明确绑定的 Rust 工具链和 SDK 仍是共享只读输入。

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

新增或改名候选单元，可在 `--commit`、`--unit` 之外同时给 `--base BASE_OID --candidate CANDIDATE_OID`，commit 必须等于其中一端。它使用 registration 已有两端投影，包括基线尚无候选单元 ID 时的历史要求；仍只观察当前检出端，不拿当前字节代替历史观察。

## 离线组合已有原件

`chrono-inputs compose --host-root H --config P --base BASE_OID --candidate CANDIDATE_OID --manifest M --output O --receipt R` 离线组合明确地址的原配对。有限清单使用 `chrono-input-composition/v1`，`sources` 各含 `pair: {path, sha256}`、可选 `transport: {source_directory, directory}` 和原报告 `artifacts` 映射。不自动发现来源，最多 4096 项，JSON 仍限 64 MiB。复制和身份检查沿原描述符／传输要求流式进行。

组合核对每份快照精确端点、配置、选择器和环境绑定。兼容重叠观察可合并，存在状态、摘要、环境或配置矛盾则拒绝。两端必须覆盖全局必需的全部声明输入，包括已选业务测试之外的检查程序输入；缺覆盖时点名端点和输入，不用实时业务输入、版本探测或 SDK 安装补齐。

来源 `pair` 地址也可指另一端存在前采到的一份真实单端快照，由其 commit 决定所属端。两端仍须从明确来源取得完整覆盖。原快照／pair 字节保存在 `.chrono-harness/state/inputs/originals/`，独立 `chrono-input-composition-result/v1` 回执点名清单、组成项和输出。快照 v2／v3 形状不变，回执仍为 `completeness_proven: false`。

**产品与宿主政策同步修改：**快照 v3 校验接受现役 Git v3 选择器生产的 `selection.registry_path`，要求它为有效相对路径；组合仍核对完整选择身份与固定端点登记，路径漂移继续拒绝。Rust 真实采集／配对／组合用例覆盖该字段及无效路径，真实本机两端组合另作验收。本机 diagnostics 同时迁到现有 vendor 输入；产品格式修复与宿主采用分别保留证据，完整输入／治理和同判仍未建立，新增验证总成本未测。

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

本机 runner／diagnostics 的额外 guard 及专属 fetch／vendor 准备入口已退役；普通构建继续使用各自登记的锁定 Cargo 操作。通用输入观察及原生获取只服务各自明确消费者，不因本次退役自动移除。

获取参与仍保留当前根 worktree v3 政策。这些来源声明没有采用 Linux 关联／协调政策或完整 Linux 宿主配置，真实平台采用仍待完成。

新增真实 CI 工作流选择有限输入集成分支并传递两份登记配置。原事件、参数、输出、退出、run／attempt、工作流修订随获取原件及 14 个安装目的保存。工作流 job 键不等于远端数字 job ID，须从原运行解析真实 job、artifact 和生产 attempt。提交工作流不代表已经运行、已消费安装工具或已落地。

根 `chrono-git-configs/v3` 绑定两个当前原生可执行身份，明确共用固定数据路径；worktree v3 允许这两个身份，未知字节仍失败。格式与不可变数据范围见[Git 选择](git-facts.md#exact-host-executable-selection)。没有额外采用输入集合扩展、历史完整执行政策、beta.21 安装或新安装器。

旧原生 config／runner 样本及过时比较、FILEMAP 项已经移除，固定来源仍在 Git。当前获取测试直接读取现役观察声明；保留的遗漏和原件不同的纠正见[输入来源保管](input-source-custody.md)。全面启用、输入齐全、固定两端原件、真实 CI 验收和同判仍未完成。

### 退役 diagnostics 的专属输入链

diagnostics 的路径依赖使用方会自行编译源码，不消费独立受保护构建的产物。当前没有额外环境保护的必要验收终点；本仓恢复普通锁定 Cargo 构建，退役专属观察、私有 Cargo home 准备、vendor 目录来源、主机快照、配置级输入和启动 profile。实际修改与保留边界见 [Cargo 所有者](cargo-projects.md#本仓已退役的额外保护链)。

专属声明、主机输入及对应边已随保护链退役。当前安装身份由 bootstrap／检查原件提供；其余 native 和 acquisition 声明仍须分别核对实际必要性，现有引用不证明永久保留。

随该使用方退役，专为这条链引入的运行时来源解析、端点快照例外和 Cargo 动态摘要路径一并移除；其余跟踪宿主未采用该输入模式。专属合成测试不构成产品保留理由。普通输入继续按显式摘要、端点快照及实际消费前的字节检查处理；普通 diagnostics 格式、构建和未过滤测试仍执行。历史原件、真实失败及未完成工作的保管义务不变。输入闭包继续 incomplete，完整同判未建立，成本收益未知。

两项只登记自身、没有执行计划或工作流调用的本机 isolated／developer 观察入口也已退役，对应的个人绝对路径声明和 FILEMAP 边一并删除。当前原生获取与观察继续使用各自现役登记。

<a id="native-diagnostics-originals-before-guard-adoption"></a>

原 diagnostics 原生保护准备链及其合格原件属于历史来源；当前入口已退役。历史配置、固定身份、观察和失败的完整对应保存在 Git 及原有限保留 artifact 中，不再复制为当前程序输入。
