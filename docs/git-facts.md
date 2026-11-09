# 使用已登记的 Git 核对事实

公开 beta.11 包含此前 full v3／scoped v2／provider v3 的 Git 绑定，也包含下文按实际文件核对工作树的改动。

完整 config v3 要求 `facts_git: {"tool": "git", "input": "git-executable"}`，非空版本和 present 输入摘要仍须明确。短命令 config v4 可只声明 `facts_git: {"tool": "git"}`，并将工具的 `expected_version` 显式设为 null；读取器一次解析已登记程序，绑定实际路径、SHA-256 和版本进程，后续调用继续核相同字节。省略 input 不产生预期身份保证。若声明 input 或版本预期，仍严格核对，摘要不符在版本探测前拒绝，输入位置须解析到同一工具。工具名、PATH、环境及真实消费者继续显式登记。

## 先核对工具，再运行 Git

runner 获取 Git 事实前先读所选候选配置，绑定实际执行物；显式摘要在版本探测之前核对。随后用同一 Git 将配置原字节与固定候选 blob 比较。候选工具读取两端，包括历史格式数据；不执行基线二进制。

完整 registration、FILEMAP、routes、projects、cost、mixed 和 workflow 检查都使用请求绑定的读取器。初始清单和 `chrono-inputs capture` 也使用候选配置绑定。

进程环境只含明确继承和配置的值。未继承到的变量保持缺失，配置值覆盖继承。裸工具名须登记 PATH，包括明确为空的 PATH，环境中其他 Git 不能代替它提供事实。显式提供 `GIT_DIR`、`GIT_WORK_TREE`、`GIT_INDEX_FILE` 会因重定向工作树而被拒绝；原 no-replace-objects／no-optional-locks 参数保留。

Git `-C` 使用绑定的标准工作树路径。相对调用方的宿主根先与此绑定核对，避免进程改变工作目录后再作另一遍相对解析。

每次调用遵守配置的协议时限和输出上限；调用前后核配置字节、所选路径和可执行／输入摘要。完整／初始报告及完整检查输出的 `git_facts` 记录绑定、环境和实际进程，保留原 stdout／stderr、摘要、退出和截断／超时失败。

早期失败用 `E_GIT_FACTS`；已构造绑定时附紧凑 JSON 观察，不编造完成报告或未启动进程。消费者在调用 Git 前拒绝缺失／不符绑定。限制针对每次调用，累计报告存储不等于整次运行内存／磁盘配额。

绑定 Reader 的 `Reader::observation()` 输出 `chrono-git-facts/v2`。进程记录使用 `chrono-retained-process/v2`，或有界无损 v1 回退；反序列化 `ProcessResult` 前须用 `full::expand_process` 展开。历史 `chrono-git-facts/v1` 内嵌进程记录。

请求绑定读取器明确要求 facts v2，在任何 Git 调用前拒绝 v1、缺失或未知版本。绑定、环境、保护和选择器核对不变，未绑定的旧观察仍为 `null`。历史内嵌或编码进程数据可以解码，但 v1 请求兼容须按版本明确处理，不会静默转换。耦合生产者／消费者要用同 facts 格式，宿主部署和发布验收归调用方。

## 只复用已验证的不可变数据

绑定 Reader 只有在已有 `verify_oid` 确认完整提交身份和有效完整 tree OID 后，才复用同一完整不可变 OID、同一精确请求路径的成功 blob 字节。此前读取仍重新取得。复用不增加资格探测进程，调用方也不专为启用缓存调用 `verify_oid`。

每次复用仍核标准根，返回前后重核 selector、config、可执行和声明输入。记录指向原成功进程回执，不编造调用或重放观察。新 Reader 自行取得证据。可变 ref、HEAD、索引、工作树、未跟踪事实、旧未绑定读取、失败和缺对象永不缓存。登记快照和配置核验共用采集模块，但各自解析、端点和准入检查不变。

成功解析的文件树清单使用同一 Reader 内资格和保护，保留原进程字节。重复物理工作树检查可复用固定树，但重新读取当前索引和文件。畸形树、进程后输入漂移不能形成缓存。

绑定 Reader 与工作树工具共用登记批量传输及分帧验证，各自提供有界 Git 调用并保存原回执。已解析登记路径和已验证不可变快照的字面 blob 路径使用它，不推导依赖。快照导出保留字节、符号链接、只读文件和目的检查，消费每段原 blob 时重核绑定输入。

未验证端点和旧读取器保持逐项读取。批量元数据决定有界内容分组，小列表或帧超过批量预算时直接读取。成功缓存保留原来源坐标；畸形、失败或漂移获取不能写入缓存。

原有界 `rev-parse --verify OID^{commit}` 和树观察保留回执及返回要求。外形像十六进制、实际却指向另一对象的 ref 每次重读；身份无效、缺失、畸形或探测失败不能启用复用。事件提交可用性和 fetch 探测仍归原负责人，保留原 batch-check 语义。

## 快照和标准命令

config v3 保留 v2 外部输入存在／缺失及快照含义。直接政策的 `chrono-input-snapshot/v2` 和既有效输入格式不变。严格一层平台选择器使用 `chrono-input-snapshot/v3`，增加原入口路径、实际目标路径和 `chrono-registry-selection/v1` 结构绑定，分别保留选择器与目标字节。

capture 发布输入快照，不发布 Git 进程报告或完整检查结论。config v1／v2 拒绝 `facts_git`，包括 `null`，保留原 Git 行为。跨版本完整检查仍须 SPEC 要求的明确工作流迁移与兼容证据，能解析格式不代表宿主完成迁移。

完整本地／CI 的原命令不变：

```sh
chrono-harness check --config .chrono-harness/config.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --context .chrono-harness/state/context.json
```

这里的配置入口也是原生选择器的稳定路径。完整和初始加载器保留入口身份，使用选中的直接政策，拒绝直接把目标路径写入参数。固定快照按真实路径保留各端原选择器和政策字节；下文 v2 不可变数据绑定仍只打开候选可执行文件。

scoped `chrono-ci-check/v2` 通过 `policy.facts_config` 明确选择读取器，与可选完整 `registration_config` 独立。两端快照、原父提交、索引／未提交检查、登记解释和执行后检查共用它。`evidence.git_facts` 保留成功及失败进程，包括版本绑定失败。scoped v1 不变，拒绝新字段即使为 `null`；选测和操作环境保留自身含义，不因绑定而全面启用检查，见[按范围检查要求](ci.md#configuration-and-selection-contract)。

CI provider v3 的事件获取也通过 `facts_config` 明确选择，见[事件说明](ci.md#registered-event-git)。本产品仓库使用独立单元 scoped v3 CI、schema4 绑定 Git，完整登记仍为 proposed。

v3／v4 绑定不能证明完整检查、Git 配置及委托依赖齐全、确定同判，或不存在瞬时并发替换。所需 Git 配置、解释器／库、OS 等实际输入仍须宿主明确登记，读取器不自动发现依赖。

专属回归覆盖含空格／Unicode Git 路径、环境同名工具遮蔽、完整／初始消费者、七程序集成／交付、无效引用、请求／环境替换、错误摘要／版本、持续字节漂移、原二进制输出／退出、输出上限和超时。这些是有限实现检查，不是 Lean 精化证明或独立评审。

<a id="explicit-platform-configurations"></a>

## 明确登记各平台配置

按范围检查或 provider v3 的 `facts_config` 可指向独立、已登记选择器，替代直接 full v3／schema4 政策：

```json
{
  "schema": "chrono-git-configs/v1",
  "platforms": {
    "macos-aarch64": ".chrono-harness/git/macos.json",
    "linux-x86_64": ".chrono-harness/git/linux.json"
  }
}
```

键使用当前二进制的 Rust `OS-ARCH` 常量。宿主负责每条映射和政策，包括程序、可选预期、环境和保护文件。程序可用已声明 PATH 一次解析的裸名称；读取器不猜语言或布局、不生成声明；未登记平台不能回退。

选择器和目标须为 `.chrono-harness/` 下的字面相对路径，拒绝链接和嵌套选择器。所有映射都检查语法，本机只读所选政策；目标须采用 full v3 或 schema4 Git 绑定，不回退未绑定模式。完整 CLI 可接收绝对选择器或相对其他 cwd 的路径，但启动 Git 前保留宿主相对入口，用于共用的无符号链接检查；直接配置路径标准化不变。

读取器把两份原文件绑定到固定候选，在版本探测及每次 Git 调用前后重核字节。进程后变化保留原输出和退出。报告增加 `git_facts.selection`，含选择器路径／摘要、实际平台和所选政策路径；`git_facts.config_path`、`config_sha256` 仍指所选 full v3／schema4 政策。请求消费者拒绝替换或缺失选择观察。直接政策保持原报告格式和含义。

选择器、政策及实际 FILEMAP 依赖都要登记，选择配置不自动产生边。同一选择器现由完整 runner 根入口、初始配置、输入采集、按范围可选 registration 和完整 CI 配置消费，保持各已登记平台的命令与入口一致。

不同所选政策就是不同输入，不能证明跨平台同判、输入齐全、本仓真实完整工作流采用或全面启用。Go 的有限采用及原 CI 记录见 [PR #18](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/18)；当前接入方式与限制归[示例索引](examples.md)。

<a id="exact-host-executable-selection"></a>

## 按本机可执行文件的精确身份选择

`chrono-git-configs/v3` 扩展一层选择器，处理同 OS／架构但可执行字节不同的宿主。每个平台声明绝对 `program` 和非空 `configurations`，每项为 `{"sha256":"<登记摘要>","path":".chrono-harness/<直接政策>.json","registry_path":".chrono-harness/<登记数据政策>.json"}`。摘要和目标路径须有效且在平台内唯一，目标仍是直接 full v3 或 schema4 政策，拒绝嵌套或回退项。`registry_path` 必须是该平台注册目标；不同当前可执行身份可明确共用同一固定数据路径。

选择只读声明可执行文件的字节，不运行它。摘要精确匹配才选目标；所选政策的 Git 程序、present 输入位置和预期摘要须与选择逐字一致，之后才运行通常的版本探测。未知字节失败并报告观察路径／摘要，不能在验证中把观察值写成预期值。

后续保护仍绑定可执行、选择器、政策和固定候选。选择观察另存 `registry_path`，消费者拒绝替换。快照路径标识该端所选直接政策，保留的选择器字节定义选择规则。

请求绑定的不可变快照先查固定端点是否登记候选实际可执行摘要；若没有，只允许使用候选明确给出的 `registry_path`。该端必须登记此路径及自身匹配的工具／输入摘要。因此读基线不打开或要求基线可执行，也不要求旧选择器登记当前可执行。

路径缺失、绑定不一致或目标歧义时失败，不猜其他政策。快照身份由单个采集目标和原选择器重建，不读取今天的 Git。固定端点 v2 字节只作数据，不作执行路径。打开候选及后续每次获取仍核实际可执行、选择器、配置和输入；政策移动须在各端明确登记。

本仓不维护主机 Git 身份白名单。[选择器](../.chrono-harness/config.json)使用既有 v1 OS／架构映射：macOS 选择 macos-27，Linux 选择 linux-release；现役 schema4 政策只声明 Git 工具，版本预期显式为 null。当前消费者需要一次运行内使用同一实际 Git，已有观察和逐调用保护已满足该要求，没有验收终点要求把每台主机的预期 SHA／版本写入 Git。实际身份和原结果归 state／线上 artifact，按真实用途保留。

缓存产物登记和发布暂存仍读取明确原生政策的产物元数据，不由 Git 字节选择，也不因相似而删除其它政策。固定基线的 v2／v3 选择器及显式身份只作候选解码器的数据输入，不执行历史 Git。产品与宿主政策同改：scoped 接受实际观察，full v3 的显式预期合同不变；验证沿原源码 bootstrap、DELTA 和原生 CI，完整输入／同判及总成本仍未建立。

每次解析选择器只读一次候选程序，固定端点解释只用明确身份／路径和保留数据。总验证／存储成本未测。独立样本把固定政策作为数据并绑定自己的工具，不代表该平台允许执行政策里的 Git，也不代表真实 CI 验收。

## 保护明确登记的 Git 输入

config v3 可选择带版本的保护，不改旧快照：

```json
"facts_git": {
  "tool": "git",
  "input": "git-executable",
  "guard": {
    "schema": "chrono-git-inputs/v1",
    "inputs": ["repository-config", "global-config", "absent-worktree-config"]
  }
}
```

各 ID 须非空、唯一，且恰好引用一个 `environment.inputs` 文件。存在文件要有 SHA-256；缺失文件须明确声明 absent 且不带摘要。宿主自行选择 Git 获取期间必须绑定的配置、include、解释器、库或其他文件，并登记实际位置和 FILEMAP 消费者。工作树的 `.git` 可能只是指针文件，保护逻辑不猜管理目录或展开 include 路径。

读取器在首次版本探测前，以及后续每次 Git 进程前后核对这些文件，包括通过同一读取器进行的 provider fetch。启动前不符则不启动；执行后不符则整次调用失败，但保留原进程字节、退出和限制。目录、符号链接和 IO 错误不能当作缺失。普通文件／缺失观察与快照采集、registration 验证共用。

`git_facts.inputs` 以 `chrono-git-inputs-result/v1` 保留首次验证状态，后续请求消费者要求观察完全相同。省略 `guard` 保留原 v3 行为和报告格式，旧程序拒绝未知扩展。真实完整／初始消费者测试覆盖旧配置字节未变时采用，以及 Git 启动前拒绝。runner 另测引用缺失／重复、缺失状态、不支持类型、逐调用漂移和会修改输入的子进程，保留失败回执。

这只执行所选文件范围的要求，不枚举全部 Git 配置、委托程序、OS 或网络输入，不证明输入齐全，也不检测两次观察间改后又恢复的并发变化。因此报告仍为 `completeness_proven: false`、`input_closure_complete: false`，快照及标准命令要求不变。公开 beta.16 已分发此扩展。

<a id="public-binary-consumption"></a>

## 公开工具的使用边界

公开工具的安装和身份核验归[分发说明](distribution.md)，真实宿主及其固定发布选择归[示例索引](examples.md)。本地 Git 政策、已安装工具消费和远端 provider 采用须分别验证；本机专属配置不会自动成为远端示例的政策。旧发布的本地运行不能证明当前候选、输入齐全或同判，历史安装与容量实验归 Git／原发布材料。

Git 原始进程观察会增加报告传输量。上限按实际待消费原件的大小登记，超限时保留原失败；一次容量实验不能成为其他宿主的通用限额。单次 Git 进程限制、原件摘要核对和完整输入义务继续适用。

<a id="registered-artifact-inventory"></a>

## 在清单中排除明确登记的产物目录

快照获取时，Git 不输出明确登记产物目录里的未跟踪内容。到调用方再过滤，会让允许存在的 SDK／状态输出先占用输入采集预算。

完整、初始、按范围采集、registration 复核、项目执行保护和迁移解码，现在共用字面排除参数生成逻辑，工作树清单也复用它。每个排除项只来自调用方明确声明的根层、区分大小写目录前缀，不发现产物归属、宿主布局或依赖。

重复斜杠或点路径段等非标准拼写不形成排除，保留调用方原路径分类。末尾斜杠让替代目录的普通文件或符号链接仍可见。已跟踪字节和索引另行核对，被忽略的相邻路径和未声明路径仍可见。

清单命令将 `GIT_LITERAL_PATHSPECS`、`GIT_GLOB_PATHSPECS`、`GIT_NOGLOB_PATHSPECS`、`GIT_ICASE_PATHSPECS` 覆盖为 `0`，避免宿主设置改变明确 pathspec 含义。绑定读取器记录每进程实际环境和原输出，保留前后配置保护；其他 Git 调用保持登记环境。原始 `Reader::checkout`、`facts::checkout` 仍返回完整清单，排除由调用方明确选择。

迁移解码可创建已声明产物，不改变候选输入；前后比较仍在业务前拒绝新未声明文件和已跟踪输入修改。这不提供并发写入沙箱、不补齐完整输入，也不增大输出上限。

公开 [beta.19](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.19) 分发此变化，beta.18 仍保留原枚举。固定源码原生配方在 macOS arm64、Linux x86_64 验证相关完整／初始／按范围／迁移消费者，实际范围和原包见[发布证据](distribution.md)。

<a id="literal-checkout-identity"></a>

## 按实际文件核对工作树

配置过的 Git diff 可能隐藏真实差异：`core.filemode=false` 忽略可执行位，文本属性标准化换行，`core.symlinks=false` 接受普通文件模拟链接，clean 过滤器把不同物理字节转换成相同 blob。执行 diff 还可能运行过滤器。

工作树读取器现在直接比较实际文件和固定 Git 文件树，并另行比较每个索引 stage／mode／OID。相反的索引与工作文件修改不能互相抵消。该比较不运行工作树 diff、clean 或 textconv 过滤器。

共用 runner 实现读取已有明确树／索引清单，将普通文件字节加 Git blob 头，通过 RustCrypto SHA-1 或 SHA-256 流式求摘要；同时核普通文件／真实符号链接类型和 Git 所有者执行位。符号链接只对目标字节求摘要，不跟随目标。文件缺失或不符就是输入变化，目录别名不能使文件冒充另一物理父目录。

打开文件不跟随末尾链接，也不因替换成 FIFO 而阻塞。IO 或清单畸形会失败。这是 Git 对象身份检查，不是新的密码学信任保证，不检查依赖、登记政策或选择测试。

完整和初始消费者共用工作树读取器，按范围 CI 在操作前后比较已跟踪内容。工作树创建／清理对照 HEAD；重建和协调后的恢复对照明确选定暂存树。清单使用已有登记 Git 执行器，保留原操作报告和部分工作。rebind 有独立保全要求，不要求有意保留的未提交工作与新索引一致。

会生成 CRLF、过滤后文件或普通文件模拟链接的配置，须由宿主改成能产出精确登记快照的设置，检查器不静默标准化文件。Git 执行位之外的权限、时间戳、ACL、扩展属性和硬链接结构不属于 Git 树身份；不承诺并发写入或崩溃下原子快照。

支持条目仍是普通文件和真实符号链接，登记路径须 UTF-8。Gitlink 和特殊文件不能冒充普通源码。完整 Git 配置、委托程序、OS、编译器／SDK 输入和宿主全面启用仍有独立未完成事项。

历史公开二进制与混合宿主的验收观察见[原 PR](https://github.com/ChronoAIProject/chrono-harness/pull/66)及其发布记录；这些观察不证明当前候选通过、输入完整或确定同判。
