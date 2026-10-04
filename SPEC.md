# chrono-harness 规格 v0.1

状态：**DRAFT / SPEC-FIRST**。本文是待实现合同，不是已运行的验收报告。
当前实现含 v1 外部判官运输、独立 registration/filemap/routes/projects/cost/mixed/workflow 的有界合同、scoped CI 与宿主指令生成器；§14、§16 及 docs/spec-coverage.md 给出实际边界。
本文中的“必须”“错误”“判官”描述目标行为，除明确标注已实现者外均未执行。

## 1. 目标、权限与边界

Harness 的核心是降低耦合。显式依赖和唯一归属让局部改动可用自身的输入、接口与测试理解；共享定义只归一个自然所有者，独立单元分别构建、检查和重试。

Harness 的本质：每项受治理操作只提供一条当前已登记的规范路径，并用可执行判官检查其结果。
定制在执行前通过登记选定这条路径；日常指令只呈现当前路径，不并列提供替代配方。
AI 先把路径、归属、依赖、成本、操作及判定方式登记为白名单，再通过该方式处理任务。
改进方法先更新登记并完成适用验证，仍保留一个正式入口；AI 自由实现产品逻辑并自主更新判官。
默认指令按目标与入口、登记与隔离、实施、检查与修复、演进与交付的短流程排列。
缺少登记、重复入口、执行结果与登记不符必须有明确错误，不能靠“智能推断”补全。
系统信任 AI 非恶意，处理的是遗漏、漂移、错误和演进成本；AI 自主登记、修复和推进。
AI 拥有构建、测试、更新约束和分支重建所需权限，没有人工审批、签字或确认步骤。

可执行的项目准入约束必须实现为登记判官，不能隐藏在 runner、CI YAML、hook 或根指南中。
根指南直接呈现一般工作方法；它不是可执行判官，也不因严格措辞变成已运行的机器门。
runner 的固定职责仅为命令/协议解析、输入运输、启动指定程序、限时限量与结果聚合。
Git 差异是输入事实；是否允许脏目录、怎样选测试、分支是否过期，均由登记判官裁决。
插件、脚本和内置发行的判官遵守同一协议；“随产品发行”不等于“自动生效”。
宿主可替换判官，不能把某个项目名、固定目录规模或测试名称写死在通用运行时。

约束的可观察范围是提交快照、登记数据、harness 启动的操作及其报告。
不声称监控 AI 所有 shell 命令，不禁止调试命令，不用 OS 沙箱证明“只有一种方法”。
受治理操作的合格证据只能由其唯一登记入口产生；绕路、缺报告或不匹配报告会报错。
未受治理的编辑行为不需要虚构执行证据；新的治理范围由 AI 显式登记。

## 2. 仓库与自举边界

```text
SPEC.md                         产品合同
README.md                       使用、目录导航和实现状态
crates/                         独立 Cargo 项目的目录分组
  runner/                       运输、协议与统一 check 入口
  runner-tests/                 runner 专属测试
  inputs/                       显式输入快照与保留内容运输
  inputs-tests/                 输入生产者专属测试
  worktree/                     显式远端目标上的新工作树生产者
  worktree-tests/                真实 Git 创建与失败保留测试
  judge-registration/           v1 登记结构、快照与受影响引用判官
  judge-registration-tests/     registration 专属测试
  judge-filemap/                v1 完整声明图与 impact，复用注册数据
  judge-filemap-tests/          FILEMAP 专属测试
  judge-routes/                 唯一操作、工具绑定和回执核对
  judge-routes-tests/           routes 专属测试
  judge-projects/               项目配对、隔离和实际执行
  judge-projects-tests/         projects 专属测试
  judge-cost/                   显式四维成本与未知值告警
  judge-cost-tests/             cost 专属测试
  judge-mixed/                  显式规则分类与混合修改警告
  judge-mixed-tests/            mixed 专属测试
  judge-workflow/               新鲜度、演进与 integration 证据
  judge-workflow-tests/          workflow 专属测试
  judge-ci/                     CI 登记、快照、DELTA 与操作执行
  judge-ci-tests/                CI 判官专属测试
  ci/                           工作流生成与事件输入准备
  ci-tests/                     CI 生成器专属测试
  instructions/                 原子指令组合、CLI 与文件发布
  instructions-tests/           指令生成器专属测试
assets/instructions/            产品双语 catalog 与默认输出计划，编译输入
docs/                           合同说明、来源、许可与生成文档
examples/ci-host/               可复制的宿主采用实例
skills/                         本仓生成并采用的 skill
AGENTS.md -> CLAUDE.md           根指南的相对链接与受管正文
.chrono-harness/                本仓采用的独立宿主数据
  instructions/                 宿主 catalog、上下文与输出计划
  projects.json / FILEMAP.json   项目、操作、文件、依赖与成本登记
  config.json / judges.json     proposed 的完整治理配置与判官白名单
  workflow.json                 proposed 的分支、迁移与混合修改策略
  ci/                           现役 CI 配置及自举绑定
  bin/                          已安装指定二进制（不跟踪）
  state/                        输入、报告与证据（不跟踪）
.github/workflows/              CI 配置的生成投影
```

`crates/` 仅组织目录，不承担项目发现或选测。每个子项目有自己的 Cargo.toml、Cargo.lock 和 target。
没有根 Cargo workspace；不需要一起编译的项目不放进同一个编译单元。
生产 crate 同时导出库与极薄二进制，因为二者共享同一 CLI 行为；不另拆无价值项目。
测试 crate 通过显式 path dev-dependency 引用生产库，有独立 lockfile 和 target 目录。
产品 crate 不定义单元测试或 doctest；所有当前行为测试归唯一配对测试项目。
以后新增独立生产项目必须同时登记一个专属测试项目；不能以共享巨型测试项目替代。

宿主的所有生效 harness 约束、脚本配置、安装绑定和指令 canonical 材料都在宿主 `.chrono-harness/` 内。
根 CLAUDE.md 的受管块直接呈现所选方法，默认是短流程；AGENTS.md 是指向 CLAUDE.md 的字面相对符号链接；
块外原有文字属于宿主；方法在 canonical 源编辑后生成，不独立维护副本。
宿主可以使用其他仓库构建的二进制，但须在此显式绑定路径、版本与摘要。
自举时 `crates/runner/src/*` 仍然是 **product**，不能因产品是判官引擎便改称宿主 policy。
其稳定性需求由 workflow 的 `stability` 路径登记表达，不通过重新归类源码表达。
产品构建不会自动启用宿主规则、安装 hook、修改全局配置或访问参考仓库。

### 2.1 形式化真源与实现对应

chrono-harness 所需的通用形式化定义与证明统一归入 trureturing，按数学主题组织并优先复用已有结果；不建立独立 chrono-harness-trureturing 库，不为本产品复制证明或新增仅包装已有定理的数学声明。
chrono-harness 保留 Rust 实现、登记格式、实际操作合同、回归用例，以及 SPEC 子句到形式模型的对应说明。两库独立构建；宿主使用 harness 二进制不因此需要 Lean、trureturing checkout 或特定宿主语言／目录结构。

形式化 lane 使用 trureturing 的独立 worktree 与 PR 交付，其数学准入、编译及验证遵循该库现役规范。对应说明须固定所引用修订和声明，列明对象、参数、假设、结论与实际消费者；已找到、已编译、已证明模型命题、已检验实现对应是不同状态。未完成部分如实保留，不以引用或 CI 绿色冒领全部形式化。

形式模型与反例可以推动 SPEC 演进：发现缺失前提、错误推论或实现不符时，分别修订适用的模型、产品合同或实现，并验证受影响义务。不得为匹配当前实现而静默缩小用户目标或改写旧结论的适用范围。Lean 核验只证明其形式陈述；Rust 实现符合模型仍需单独的对应证据，完整依赖登记、局部性与确定性等现实前提也不能由图闭包或一次通过自动取得。

## 3. 登记格式与字段合同

Config accepts explicit `schema_version: 1 | 2 | 3 | 4`; projects/judges use version 1, and current FILEMAP/workflow use version 2. Config v2/v3/v4 require a presence declaration for every external input; v1 retains its original digest-only meaning. Every registry retains `status: "proposed" | "active"`. The candidate reader supports fixed historical v1 data only through its explicit interpretation contract.
当前五份均为 proposed；`config.enforcement` 为 `not-implemented`。
启用前由 AI 对宿主定义的治理／输入范围作负责的 declared-complete 工程声明，补齐该范围的实际输入、依赖、绑定、两端快照、二进制 SHA-256 和全部必需判官，再改为 active/enabled。
判官核验声明引用、身份、快照和观察到的操作；普通 full check 不要求证明所有隐藏输入不存在，也不要求 VM 或 OS 隔离。
已知缺输入、未解决的必需依赖、缺绑定／快照及漂移仍须失败，不能以声明或手写成功清除。
null 摘要只允许 draft；不能被解释为“任意二进制都已通过验证”。
类型表按各登记的现役 schema 描述；registration 严格验证结构。明确支持的历史 scoped 遗留项先由候选解释器转换并保留原始事实（见 §14），不作为第二套当前登记。
对象未列出的字段、重复 JSON key、重复 ID、悬空引用由 registration 判官报错。
路径是仓库根相对 UTF-8 POSIX 路径，不含 `..` 或隐式 glob；environment.inputs.location 可显式指定外部绝对路径。
所有文件逐个登记；目录条目只用于明确生成物，不允许替代源文件登记。

| config 字段 | 类型与约束 |
| --- | --- |
| runner | `{path, version, sha256}`；path 为指定入口，sha256 为 64 位小写 hex 或 draft null |
| registries | judges/projects/filemap/workflow 四个唯一 JSON 路径 |
| canonical_check | v1–v3 `{operation, argv: string[]}`；v4 adds `profile` and `inputs.local/ci` actions `{operation,tool,argv}`；唯一 `validate.delta` 路由 |
| tools | `{id, program, resolution, version_argv, expected_version}[]`；版本为字符串，draft 可为 null |
| facts_git | config v3/v4 必填 `{tool, input, guard?}`，前两项分别引用唯一 tools 与 present 文件输入；可选 guard 见下；v1/v2 不接纳此字段，包括 null |
| environment | `{inherit: string[], values: object, inputs: object[]}`；v4 可加 `credential_environment: string[]`，声明仅用于输入取得、不转发给 Git／判官的 inherit 凭据；v1 输入 `{id, location, sha256}`，v2/v3/v4 输入为 `{id, location, presence: "present", sha256}` 或 `{id, location, presence: "absent"}`；变量及输入文件白名单 |
| input_closure | `{status: "incomplete"\|"declared-complete", unresolved: string[], bindings?: {id, consumer, kind, inputs: string[]}[], coverage?: object}`；`bindings` 仅为 config v3 的显式白名单，工程声明不是完备性证明 |
| protocol | `{id, timeout_seconds, stdout_limit_bytes, encoding}`；正整数限制、UTF-8 |
| semantic_fields | `{path, pointers: string[], on}[]`；语义字段分类，见 §8 |
| artifacts | `{path, owner, kind, tracked: false}[]`；明确生成目录白名单 |
| enforcement | `not-implemented` 或 `enabled`；proposed 不产生通过报告 |

tools 的 `resolution` v1 仅有 `PATH-once`：只解析登记的 program 一次，报告绝对路径、
文件摘要和版本输出；不搜索其他同类工具作为替代，不自动纠正版本不匹配。
Config v3 在读取 Git 对象前，从显式 candidate 配置绑定 `facts_git`；该配置的原始字节随后必须与固定 candidate 中的 blob 相同。tool 的 program 按已声明 PATH 或显式路径解析，input.location 必须指向同一执行物，present 摘要先于任何启动核验，expected_version 必须为非空字符串并匹配 version_argv 的实际 UTF-8 stdout（只去末尾空白）。相同 candidate 绑定读取 base 与 candidate，不启动 base 工具。每次进程使用清空后重建的登记环境及 protocol 的时间／输出上限；显式 GIT_DIR、GIT_WORK_TREE、GIT_INDEX_FILE 因重定向 checkout 而拒绝。进程前后核配置、绑定路径与执行物摘要，保留原始输出字节、摘要、退出和边界失败。完整请求及消费者报告通过 `git_facts` 运输观察；下游核对绑定和环境，缺失或不符不得回退 ambient Git。执行前的失败以 `E_GIT_FACTS` 携观察返回；尚未启动的进程不伪造回执。完整调用、initial inventory 和输入 capture 共用该读取器；initial 仍不裁决 DELTA。Config v1/v2、旧 library facts API、scoped CI 与旧版事件准备保持各自旧合同。详见 [Git 事实绑定](docs/git-facts.md)。此绑定不证明 Git 配置、解释器、动态库、OS、委托工具及其它实际输入已完整登记，也不证明无并发瞬时变化或本地/CI 同判。

独立 `facts_config`、完整 `--config`、initial `Profile.host_config`、输入 capture、
scoped 的可选 `registration_config` 及 full-CI `check_config` 均可显式指向
`chrono-git-configs/v1`。其唯一数据字段 `platforms` 逐项登记执行二进制的 Rust
`OS-ARCH` 到完整 v3 Git 配置路径的映射。无匹配平台即拒绝，不探测可用工具、不推断
语言或目录；各平台文件仍由宿主明确登记。选择器及目标均为 `.chrono-harness/` 下的
字面相对路径，拒绝链接、自引用、嵌套选择器和旧版目标。

固定端点读取器独立按各自 commit 解析 entry 与 selected target，并在 registry snapshot
中按真实路径保留 selector、target 和五份 status-bearing registry 的原字节和值；不把
target JSON 别名到 selector，也不把 candidate target 套到 base。请求、registration
view（选择器模式版本化为 `chrono-registration-view/v2`）、历史 decode（选择器输入
版本化为 `chrono-historical-decode/v3`）、FILEMAP provenance、mixed documents 与 workflow checks 同时保留
原始 entry identity、effective target identity 和结构化 selector binding，缺失、替换或
漂移均拒绝。Git facts 仍按原合同绑定 candidate，历史 endpoint 只读固定对象，不执行
历史工具。选中输入快照使用版本化 `chrono-input-snapshot/v3` 绑定 entry、effective
target 与 selector；direct v1/v2/v3 和旧快照语义保持不变。

本地与 CI 使用同一 canonical entry argv，routes 仍登记稳定 entry；直接 target argv
替换被拒绝。不同平台选中不同政策仍是不同有效输入，不能据此声明 parity；完整 host
activation、portable initial judge bindings、complete effective inputs、native full
dispatch、Rust formal refinement 和完整 SPEC 仍未完成。旧直接配置保留原义。

Config v3 可在 facts_git 中采用 `guard: {schema: "chrono-git-inputs/v1", inputs: string[]}`。
非空 inputs 逐项引用唯一 environment.inputs 文件 ID，拒绝重复、未知引用和未绑定摘要；
present 必须匹配 SHA-256，absent 必须实际缺失，目录、符号链接、IO 错误不作缺失。
读取器在第一次版本探测前及每次 Git 调用前后检查这些宿主指定输入；失败不得继续读取
对象或 fetch，已运行的子进程保留原始输出与退出。下游请求须绑定相同的初始核验结果。
Git 配置、include、解释器或库是否完整仍由宿主显式登记；不扫描或添加输入，不自动解析
worktree 元数据位置。省略 guard 保持旧 v3 语义与报告形状，历史配置不补写。
具名 inputs 观察使用 chrono-git-inputs-result/v1，保留实际文件身份／缺失与
completeness_proven=false；前后观察不证明瞬时并发隔离、完整输入或确定性同判。

当前宿主显式登记 cargo、python3 与 chrono-ci 的版本合同；尚未具备完整 Cargo/SDK 输入闭包。换工具链由 AI 显式更新登记，声明版本不代替实际版本观察。可选 `chrono-cargo-inputs/v5` 进一步登记 sysroot、backend、linker、SDK 与 build-script 输入，但仍只证明登记的有限边界。
`argv` 是原始参数数组；只对注册的整参数占位符 `{base}`、`{candidate}` 做替换。
没有 shell 插值、模板语言或依据当前目录猜测 manifest 的行为。
只继承 environment.inherit 逐个列出的变量，缺值记录为 absent，values 显式覆盖。
inputs 为逐文件登记的有效输入，ID 唯一。v1 及 v2 present 的摘要 null 仍仅表示未绑定草稿；不得解释为缺失。v2 absent 禁止携带摘要；缺失不同于空文件。v2 快照保留实际 `{absent:true}` 或文件内容身份，判官核对登记与两端快照，并在操作前后核候选路径。目录、符号链接、权限及 IO 错误不作缺失；路径祖先也不得是符号链接。历史缺失只从保留快照读取，不重读今天的路径。报告绑定实际值，不自动读取 dotenv。

Config v2 可用于两端均采用该合同的宿主，也可经 workflow v3 显式版本选择的宿主 decoder 从 v1 迁移。v1→v2 仍须 §6 的实际转换及兼容测试；支持读取 v2 本身不证明已有宿主升级成功。文件状态变化通过 input 节点的显式边传播；没有边不推断消费者。捕获成功不是治理成功，缺失检查不是完整输入闭包或并发隔离证明。
外部输入须保留两端快照，变化通过登记的存在状态或摘要进入提交 DELTA；磁盘值不符或快照缺失报输入不足，不隐式扩展 DELTA。
PATH 用于一次解析具名工具，HOME/CARGO_HOME/RUSTUP_HOME 用于已声明的工具链运行；
完整闭包包括实际 cargo/rustc、编译器后端、链接器、SDK、目标库和所读配置文件，
包括 HOME/CARGO_HOME 下配置、rust-toolchain、间接依赖、fixture、外部数据及环境变量。
会影响裁决的随机种子、时钟值、网络响应和超时/资源条件也须显式固定并登记为有效输入。
文件输入用 FILEMAP 或 environment.inputs 登记；后者以 input:<id> 显式连接消费者，不推断边。
当前 input_closure 为 incomplete，inputs 为空；补齐宿主声明范围内的工具、输入、关系与证据后才可声明 declared-complete，unresolved 必须为空。
declared-complete 是可信 AI 对该范围承担责任的工程声明；真实工具链、SDK、配置及其它已知适用输入义务仍保留。判官不自动发现隐含依赖，声明通过不证明普遍输入完备。
Config v3 可额外登记 `input_closure.bindings`：每项必须有唯一 `id`、可执行消费者
`project:<id>`、`script:<id>`、`test:<id>` 或 `judge:<id>`、非空 `kind` 以及非空
`inputs`。`inputs` 只能逐项引用已登记的 `tool:<id>`、`input:<id>` 或
`environment:<name>` 节点；未知节点、错误种类或悬空引用由 registration 报
`E_REFERENCE`。v1/v2 拒绝该字段。bindings 只记录 AI 明确选择的关系，registration
不扫描目录、语言、命令、manifest 或工具链来补齐依赖，也不把登记成功解释为现实中
没有遗漏输入；声明范围的登记就绪由 `status`、`unresolved` 和各消费者的实际证据共同决定。
Config v3 可显式采用版本化 `input_closure.coverage`，合同为
`{schema: "chrono-input-coverage/v1", required: [{consumer, domains: string[]}], rows: [{consumer, domain, state, bindings: string[], reason}]}`。
`required` 非空，逐个列出可执行消费者及非空的宿主自定义输入域；rows 必须恰好覆盖
这些消费者／域组合一次。缺行、多行、未知域、重复域、错误消费者、未知 binding 和
空 reason 均报错。`bound` 和 `absent` 要求非空 bindings，每个 binding 同属该消费者
且有非空 inputs；一个 binding 可以关联多个域。`absent` 只接受显式缺失文件输入，
实际存在状态仍由两端快照及操作前后输入核验裁决。`not-applicable` 与 `unresolved`
不携带 bindings，前者是 AI 的适用性声明，后者明确缺证。采用 coverage 且声明
`declared-complete` 时，每个 binding 至少被一行覆盖且不能有 unresolved 域。
域名由宿主登记，例如 compiler-library、SDK、network、clock 或 randomness；判官
不猜项目语言、工具链或读取行为来补域。省略 coverage 保持既有 v3 含义；v1/v2
拒绝此扩展，旧二进制拒绝未知扩展。候选判官读取未采用扩展的历史快照，无需重写历史。
通过后产生 `chrono-input-coverage-result/v1`，保留完整声明与 `scope: declared-domains`、
`completeness_proven: false`；它证明登记的对应关系和已核验输入状态，不证明没有未登记域
或任意程序的读取闭包。完整有效输入、并发一致性和本地／CI 同判仍需实际消费者证据。
未知或已知缺失项返回 E_INPUT_UNDECLARED / E_EVIDENCE_UNRESOLVED；登记有效不证明真实输入完整。

| projects 字段 | 类型与约束 |
| --- | --- |
| owners | 唯一 owner ID 数组，包括无编译项目的 repository owner |
| projects | `{id, kind, actions, test_project?, tests_for?, manifest?, lockfile?, root?}[]`；三个 legacy 路径均可省略 |
| kind | `production` 或 `test`；production 必有 test_project，test 必有 tests_for |
| actions | 非空的显式具名方法对象；方法名不限定语言或活动；每项 `{operation, tool, argv}`。测试的 `execute` 是其执行入口合同 |
| scripts | `{id, path, test_script, actions}[]` 或 `{id, path, tests_for, actions}[]` |

每个 production 与一个 test 双向一一对应，test 不再要求递归配一个 test。
脚本以同样的一对一关系单独登记，不为它创建虚假的编译项目。
operation 在全部 actions 与 canonical_check 中唯一；同名不同参数也属于重复方法。
宿主可以没有 manifest、lockfile、统一 source root 或编译输出目录。可选 legacy manifest/lockfile 只是显式归属的 opaque 文件引用；root 不授予内容或输出推断权。通用 projects 判官检查一对一配对、显式测试边、实际文件归属与声明产物目录的隔离，不解析语言文件，也不根据路径猜 `target/`。产物登记变更与删除按两端显式 owner 唤醒对应 project/script，再沿 FILEMAP 边选择测试。
构建、测试依赖只由 FILEMAP 提供。语言工具可作为可选一致性判官，例如 [chrono-judge-cargo](docs/cargo-projects.md)；适配器消费显式语言政策与既有边，不能补登记或选测试。
可选 Cargo guard 的配置合同必须显式登记每个查找位置的存在／缺失、命令行配置与递归 include；文件内容沿既有输入 ID 和 FILEMAP 边连接消费者。缺失也是有效输入，不能以未发现文件替代缺失声明。登记表按 Cargo 的固定版本查找规则核完整性，不因此取得自动补登记权。配置闭包不代表编译器、SDK、构建脚本或外部服务输入闭包。当前有界实现及 v1→v2 迁移见同文档。
可选 guard v4 进一步显式绑定 Cargo 与 `RUSTC` 的输入 ID、文件身份、路径和版本，
要求宿主登记唯一编译器选择环境，拒绝另一套 Cargo 配置／wrapper 选择来源。
这只绑定被传给 Cargo 的执行入口，不证明其委托程序、编译器库、链接器或 SDK 闭包。
v2/v3 保持原解释；v4 的支持命令与边界见同文档，不由通用 projects 判官推断语言工具链。
可选 guard v5 保留 v4 字段并增加显式 `toolchain`：sysroot 使用
`chrono-input-directory/v1` 文件清单且只能由精确 `RUSTFLAGS=--sysroot=...` 选择；
backend 与 build-script 输入逐项连接 FILEMAP；linker 以独立工具、输入 ID 和环境变量
绑定；每个 SDK 以目录清单和精确环境根绑定。目录文件、选择路径和外部输入在 Cargo
前后重核，递归配置不得提供第二套 linker/sysroot 来源。v5 报告使用
`chrono-cargo-run/v3`，仍保持 `input_closure_complete: false`；它不推断未登记的
编译器库、委托进程、SDK 元数据、构建脚本读取、OS 或网络输入。v2/v3/v4 的解释保持不变。

v3 可显式选择逐项祖先清单，或断言宿主根的所有严格祖先都没有 Cargo 配置；后一种只核验具名范围的缺失，发现文件即失败，不推断登记或依赖。Cargo home 可显式相对同一宿主根解释，避免将 checkout 深度或个人绝对路径写死进可搬移政策。根、Cargo home、命令行与 include 输入仍逐项登记；v2 合同保留原义。适用迁移与观测边界见同文档，不能由配置可搬移推断整个宿主或完整输入已可移植。
脚本、fixture、环境文件、生成器输入都必须逐个登记依赖，不能按目录名猜测试集。
生成器自举 init/generate 通过已登记 cargo 工具的 `run --locked --manifest-path ... -- ...` 调用，
argv 显式指定本仓宿主根 `.`，init 不传材料路径以保留已有定制；首次采用内嵌精选核心与空上下文。
不新增工具发现或路径模板。任意外部宿主直接用 §16 的 CLI。

| FILEMAP 字段 | 类型与约束 |
| --- | --- |
| cost_models | 成本 ID → `{cpu_ms, wall_ms, peak_rss_bytes, io_bytes, basis}` |
| files | `{path, owner, surface, cost, edges: Edge[], symlink?, projection?}[]`；每路径恰好一次 |
| symlink | 可选字面相对目标字符串；登记实际链接身份，不能按解析后相等放宽字面目标 |
| projection | 可选 `{sources: string[], producer, scope}`；sources 为完整显式已登记源路径（manifest/catalog/file 源），producer 为已登记 project 节点；scope 为 `managed-block`（块外归宿主）或 `whole-file`。这是 proposed 形状扩展，不自动执行 |
| surface | `product / test / documentation / membership / instruction-policy / judge-policy / judge-implementation` |
| Edge | `{kind, to}`；from 隐含为 `file:<path>` |
| project_edges | `{from, kind, to}[]`；显式项目/脚本/外部输入关系 |
| execution_plans | FILEMAP v2: `test:ID` -> `{operations: string[], timeout_seconds, output_limit_bytes}`; ordered nonempty unique IDs, test execute required; shared precedence and bounds must agree before any launch |
| execution_scheduling | FILEMAP v2 optional: `{max_running: positive integer, resources: string[], claims: {operation: {resources: string[], outputs: artifact-path[]}}}`; every participating operation explicitly claimed, including empty lists; exclusive resources and overlapping output namespaces; absent preserves serial plan identity |
| test_costs | `{test, cost}[]`；每个可执行测试项目或脚本有成本引用 |

symlink/projection 是明确的文件表示及所有权说明，不自动增加编译或测试执行边。
本仓 AGENTS 的 symlink 恰为 CLAUDE.md，CLAUDE 的受管块与登记的英文 Markdown/skill 由 instructions 从宿主 manifest/catalog 产生；
框架资产的编译边、资产运输测试边和源/目标读取的 runtime-input 边另行显式登记。
这些 FILEMAP 扩展仍属 proposed；专用生成器 manifest 的实际链接/源校验不等于通用 FILEMAP 判官已实现。

图节点 ID 使用 `file:<path>`、`input:<id>`、`tool:<id>`、`environment:<name>`、`project:<id>`、`script:<id>`、`test:<id>`、`judge:<id>`。工具和环境记录必须显式连接消费者；不推断执行依赖。
input 节点引用对应状态的 environment.inputs；其边在 project_edges 显式列出，变化也成为种子。
环境变量及其他非文件输入以显式快照文件登记；两端值无法保留/比较时属于输入不足。
test 节点指向登记项目的 execute 操作或脚本的测试操作，不是猜出的测试函数。
每个 owner、cost、节点及操作引用必须存在；同一文件不能由两个项目隐式共享。
成本数字为非负估计，未知为 null，basis 必须解释来源；禁止把 null 当作 0。
所有项目都可通过登记演进，无固定项目数量上限；拆分项目时同时迁移归属与测试边。

| judges 字段 | 类型与约束 |
| --- | --- |
| migration_validator | candidate 中负责迁移证据的判官 ID；当前草案为 workflow |
| judges | `{id, executable, version, sha256, argv, selector, after, modes}[]` |
| selector | v1 使用 `every-delta`；仍然只检查该次 DELTA，不扫描裁决全部历史 |
| after | 必须存在的判官 ID 数组；有向无环，稳定 ID 顺序打破并列 |
| modes | v1 仅 `evaluate`；candidate 判官读取两端事实，见 §6 |

Default binaries use `.chrono-harness/bin/chrono-judge-<id>`. All seven judges have bounded implementations; full host activation remains incomplete. See [workflow evidence](docs/workflow.md).
每个 ID 实现时才创建并登记 `judge-<id>` 与专属 `judge-<id>-tests`，独立 manifest/lockfile/target。
七对判官项目分别编译测试，不由一个总判官二进制耦合；当前存在 runner、judge-registration、judge-filemap、judge-routes、judge-projects、judge-cost、judge-mixed、judge-workflow、judge-ci、ci、instructions 及各自测试，不创建空判官项目。
第三方程序可改成自己的 executable/argv，但宿主调用声明仍位于 `.chrono-harness/`。

| workflow 字段 | 类型与约束 |
| --- | --- |
| target_branch / feature_prefix / integration_prefix | 分支名与前缀，当前 dev / feature/ / integration/ |
| staleness | `{max_behind_commits, max_age_hours, combine: "any-exceeded"}` |
| stability | `{id, paths: string[], tests: string[], reason}[]`；精确路径 |
| semantic_changes_require_integration | bool；当前 true |
| mixed_change | `{level: "warning", code, requires_acknowledgement: false}` |
| integration | `{tests, bind, evidence}`；精确绑定字段与状态目录路径 |
| retirements | `{kind, id, replacement: string|null, reason}[]`；显式退出/迁移事实 |
| historical_profiles | workflow v2 retains the named FILEMAP v1 profile `{id,filemap_version,profile_path,script,test,mappings,legacy_records,ambiguities}`. Workflow v3 also accepts `{id,from_versions,to_versions,script,test,mappings,legacy_records,ambiguities}`; both version maps explicitly name all five registries (`config,filemap,projects,judges,workflow`). Selection requires exact before/after version agreement and one matching profile. |
| historical_profiles.method_replacements | Optional explicit `{from,to,reason}` rows in workflow v2/v3. Each method has exact `{owner,operation,tool,argv}`. Require one original legacy action and one current method, equal operation IDs and explicit changed-owner mapping. Duplicate, unchanged, stale or unused declarations fail; omission preserves the standalone strict old-method contract. Original definitions and operation obligations remain; semantic equivalence is not inferred. Source extension after beta.13; see docs/workflow.md. |
| migrations | `{from_version, to_version, script, test, mappings, reason}[]`；mappings 为旧/新记录 ID 对 |

本节每份文件有共同 schema_version/status，表格列出其余字段；不另设未登记的规则文件。
版本选择合同只登记需要执行哪个 candidate decoder，不推断宿主结构、语言或迁移算法。
v3 的版本选择会给 decoder 传原始登记及字节、candidate 登记和所选 profile；原始 config
不得被历史视图改写，旧输入快照仍按旧版本及其原始摘要核验。配置 v1→v2 可用此合同
验证宿主登记的转换及兼容测试，v1 的 `sha256:null` 仍是未绑定而非缺失。
实际转换收据与同一专属测试的执行结果均须通过 workflow 验证，版本声明本身不认证兼容。
指令生成专用 manifest 使用 §16 的独立合同，没有 active/proposed 状态，不参与判官启用。
登记的语法/引用约束也由 registration 判官实施；runner 只负责 JSON 运输与协议解析。
配置不可解析、进程无法启动等基础故障是运行时错误，不是一个未声明的业务判官。

## 4. 唯一检查指令、快照与 CI 对等

实现并安装指定二进制后，本地与 CI 必须从宿主根目录运行同一条指令：

```sh
./.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/config.json \
  --base "$CHRONO_BASE" \
  --candidate "$CHRONO_CANDIDATE" \
  --context .chrono-harness/state/context.json
```

两个变量必须是已存在的完整、不可变 commit OID（按仓库对象格式为 40 或 64 位 hex）。
不得直接传 `HEAD`、`dev`、短 SHA、工作目录别名或 CI 默认 merge ref。
`base` 是此轮已获取的 dev tip；`candidate` 是拟向 dev 交付的精确提交。
DELTA 比较二者的 tree，不自动改用 merge-base，base 不必是 candidate 的祖先。
分支过期和合并关系由 workflow 判官处理；不允许 CI 悄悄改成全量检查。
本地/CI 必须运输相同 OID、context 与版本化配置；其报告绑定这些内容的摘要。
同一指令保证调用和输入解释语义一致；相同裁决还要求完整有效输入、政策、执行物、工具链、
配置和环境相同，并且判官/测试确定性求值。仅 Cargo 版本或环境摘要存在不足以证明这些前提。
前提未建立时报告 parity:unestablished，不能承诺裁决相同；这不另加普通 full check 准入门。其已登记合同所需输入／证据不足仍报错。
普通 full check 在登记要求及全部选中义务通过后表示该 DELTA 满足现役登记合同；现役 registration 的 effective_inputs 仍可为 completeness_proven:false。

完成两次独立的 full check 后，可显式运行
`chrono-harness parity --host-root H --report P --compared-report Q` 生成成对对等证据。
该命令只消费两个已发布报告，不替代 canonical check，也不重新执行业务判官；它要求两份报告
均为 `configured-judges` 完成报告、所有 verdict-bearing 字段及判官响应均已观察、
`effective_inputs.completeness_proven = true` 且没有未解决的关键输入。它比较状态、快照身份、
环境、工具、有效输入、DELTA、影响、测试、findings 及判官结果；绝对执行路径按已观察二进制摘要
归一化，路径敏感性必须由有效输入显式登记。比较失败会保留当前报告并以非零错误退出，不能把
相同的未知或不完整报告标成 `parity:established`。这是独立的成对观察证据；即使比较成功，也不单凭一对报告证明普遍确定性。
其更强前提缺失时 parity 保持 unestablished，不阻止已满足现役登记合同的普通 full check；不得手填 completeness_proven:true 或清除已知未解决项。
宿主尚未登记的 compiler、SDK、网络、凭据或其他已知适用外部输入仍须补齐。

v1 明确选择干净、不可变 commit 快照这一工程边界；不另设 dirty-worktree 验证模式。
工作树必须检出 candidate，索引和已跟踪文件必须等于 candidate，子模块不得隐式展开。
未跟踪文件须登记并提交；只有 config.artifacts 中生成目录可忽略。
仅写进 `.gitignore` 不够：判官检查所有未跟踪路径，包括被 Git 忽略的路径。
生成目录里若出现跟踪文件，按普通 DELTA 文件要求登记；忽略不覆盖 Git 事实。
采集未跟踪路径时可按显式生成目录做字面、区分大小写的排除，避免先输出全部产物路径，再由调用方过滤。
排除不覆盖目录同名的文件／链接、相邻未登记路径或独立的已跟踪文件／索引核对；不规范的目录写法不得归一化成更宽排除。
迁移解码器前后也只比较受治理输入，允许写登记生成目录，新增未登记文件或修改候选源码仍须报错。
暂存、未暂存、未跟踪的非生成物变化或 candidate 不匹配返回 `E_SNAPSHOT_DIRTY`，不静默丢弃。
这些行为是 registration 判官的默认策略，runner 只传入 Git 状态事实。
仓库路径边界、特殊文件类型不支持时必须返回 `E_INPUT_UNSUPPORTED`，不能漏判。

`--config` 的磁盘内容必须等于 candidate 中同路径 blob；base 配置单独从 base tree 读取。
base.root 是从固定 base tree 导出的只读文件快照；不执行其中程序。candidate.root 是实际检出的候选工作树根，不是导出的干净副本。判官将其视为只读源，构建及证据仅使用登记生成目录。只读是输入消费合同，不声称 OS 沙箱隔离；registration 对比观察与固定对象。
context 为 UTF-8 JSON：`schema_version, base, candidate, dev_tip, branch_ref, fork_point,`
`branch_started_at, observed_at, operation, integration_evidence`，时间均为 UTC RFC3339。
workflow 使用 context schema 2，另须显式 `run_kind: integration | delivery`；integration 前缀的交付也必须使用 delivery。registration 保留 v1 供现有有界消费者读取。
operation 固定为 validate.delta；integration_evidence 为证据摘要或 null。
context 的 base/candidate 必须与 CLI 相同，dev_tip 必须等于 base；CI 不接受悄悄漂移。
Git ancestry 来自完整对象图；浅克隆缺对象返回 `E_HISTORY_MISSING`，AI 补取后重跑。
工作开始记录 branch_started_at，observed_at 是本轮获取 dev 的时间，不用提交时间猜分支年龄。
同一轮本地/CI 使用相同 context；dev 再推进就建立新一轮输入，不能冒用旧报告。

完整报告写入 `.chrono-harness/state/report.json` 与本轮唯一的 `run-<identity>.json`，格式、原始证据与摘要保持完整。短命令 stdout 只呈现原状态、退出码、有限的具名诊断与唯一原报告路径；超出行数或文本长度的诊断明确标示省略，完整内容从原报告读取。显式旧合同 `--config` 调用仍输出与存储报告相同的完整 JSON。报告的 report_path 指向后者；runner 将实际前序判官进程及 request_digest 传入 observations，供 workflow 核对执行身份与完成证据。
CI 只负责准备工具、不可变输入并调用指令、保存报告和原样传播退出码。
不得另写 CI 专属判官、skip 参数或本地宽松模式；现役生成 workflow 的 scoped 合同见 docs/ci.md；不设置虚假的绿色 workflow。

`chrono-github-full-ci/v1` 是显式完整 context 的独立 dispatch 投影。调用方提供
context 原始 UTF-8 JSON 字符串与所有引用证据；生成器不推断角色、时间、宿主布局或
证据获取方式。候选 config v3 的 Git 绑定核对 HEAD、配置、provider 源和指定 workflow
修订中的投影字节，只在精确缺对象读数后补取固定 OID。准备阶段保留 context 原字节，
另存 Git 观察、源身份和同一完整 check argv；其 `prepared` 不承担治理裁决，
parity 保持 `unestablished`。实际判官仍裁决 context 其余语义与缺失证据。
输出冲突不得覆盖旧证据，原始进程失败须保留并传播。单文件发布不认证并发或跨文件
事务；此投影不自动启用完整宿主、PR/合并流程或 required check。字段、重试与采用
边界见 [完整 CI 传输合同](docs/full-ci.md)。

固定快照的干净检查须同时比较 candidate 树、索引 stage 和实际文件。现役源码按 Git 对象头与原始字节计算 SHA-1／SHA-256 blob 身份，逐项核常规文件类型及 Git 的 owner-executable 位、符号链接字面目标；目录祖先不得沿链接读取。它不通过工作区 diff、textconv、clean filter 或换行转换决定文件身份；配置隐藏的字节、类型与执行位差异仍须拒绝。索引的新增／删除／mode／OID／未合并 stage 独立比较，不能由相反工作区变化抵消；既有 index flag 和 untracked/artifact 检查继续适用。核验只消费固定树／索引清单，不登记依赖、不选测试，不因缓存或 Git status 干净而跳过读取。完整／initial／scoped 检查及 worktree 创建、重建、恢复、清理复用同一文件比较实现，各自通过已有 Git 绑定读取树和索引。Gitlink、特殊文件及非 UTF-8 登记路径尚不支持；时间戳、ACL、xattr 不是 Git 树身份的一部分，且不承诺并发原子快照。完整 Git 配置、委托程序、工具链／SDK 输入闭包仍须另行补齐。详见 [原始快照身份](docs/git-facts.md#literal-checkout-identity)。


### 4.1 显式单元与独立条件 job

短入口仅在原报告发布后投影，不另执行判官或业务。失败生产者与版本探针保留原始回执，解码／校验失败引用原 acquisition；完成报告发布前的执行错误另留原错误，不能称为已完成报告。保留失败时如实返回原错误与保留失败，不伪造路径；早期配置／选择错误仍为原 E_CHECK，不宣称普遍输出上界。

无论仓库规模，每个能独立验收、独立重跑的单元均生成独立 job；当前目标是一份 parent workflow，先 detector，再 job-level if 单元，最后 `always()` aggregate（needs detector 与全部单元），仅 aggregate 为 required check。旧登记在显式迁移前保留各独立 workflow 合同。边界由宿主显式登记，必要前置与完整测试义务仍须保留。各单元具有独立检查名、checkout、启动命令、边界、报告及重跑。单元及完整测试计划的对应在宿主 `.chrono-harness/` 显式登记；不得按目录、语言或自动发现依赖分组。计划须恰有一个所属单元，漏登、重登和未知计划先报错。跨单元共用操作须逐项声明由哪些单元各自重复；本版不隐式传输工件或推断单元间依赖。

现役 config schema4 支持固定 root invocation `check`、`check --unit ID`、无值 `check --collect`；bare check 保留全局 DELTA。固定配置／原生 selector、bound Git、工具／环境／协议约束及 source-local/CI action 在执行前解析；`CHRONO_CHECK_SOURCE` 必须显式继承，无值／local 和 ci 各有唯一登记生产者，无混合覆盖、猜范围或手工 prepare。runner 分别保留真实 argv/cwd 与 PreparedCheck 的配置、端点、scope、context 原字节／语义摘要及原始生产证据，执行现役判官；canonical consumers 拒绝伪造展开 argv 和 selector 不一致。worktree v2 的 start/reconstruct 自动发布目的地 birth association，full unscoped 的 schema2 context 保留原 fork／birth，观察当前 HEAD／target／时间，引用实际历史输入，缺证据失败。local collection 由 CI owner 根据显式报告路径及独立执行物 pin 产生登记 manifest，无业务调度；provider v4/full provider v2 生成同一短 check step。旧合同显式拼写仅留给旧登记。full 独立短单元／汇总已接入显式 execution_units 与原始证据；当前实现不宣称 native 完成或完整 SPEC 验收。

现役源码扩展 `chrono-ci-check/v3` 的 `policy.units` 与 `shared_operations`。判官先完成两端 DELTA、移除／替代义务及全局所选操作图验证，再按 `--unit ID` 过滤执行；不能借单元过滤隐藏全局操作环。单元报告分别列本单元所选、全局所选、交给其它单元、所需单元及真正不需要的义务。某单元成功只承担自身结果，其失败不取消不相关 workflow。本地及该 workflow 均追加完全相同的 `--unit ID` 到登记 check 指令；v1/v2 原调用行为保留。

同一入口的 `--collect MANIFEST` 只重建当前义务并核所需原始单元报告，不能重跑业务测试。它检查完整报告集合、固定端点／配置／执行物 pin、原始判官输出、计划身份及候选操作／顺序／边界、工具观察、逐操作原始回执和成功状态；缺漏、重复、陈旧或失败报告不得全局通过。执行物 pin 由明确调用输入承担，不宣称据此证明构建产地、外部输入完整或跨环境同判。

`chrono-github-units/v1` 从显式 provider 生成各独立 workflow 与汇总 workflow。provider 只准备固定输入、收集和运输对应 candidate/event/attempt 的原始报告并运行上述指令；最终裁决仍归判官。push／pull_request 自动汇总核实际 workflow 源和 attempt 未漂移；手动单元组合使用显式 manifest，本版汇总 workflow 不提供未实现的手动触发。启动工具按各单元显式配置，不默认构建所有项目。完整字段、共享／重试合同、初始输入及当前采用边界见 [并发 CI 合同](docs/ci-units.md)。此扩展不自动启用完整七判官治理；公开 beta.12 起提供该扩展，beta.13 继续提供；产品仓库按登记将完整测试计划分配给显式单元，当前通过 `job_gating` 投影为一份 parent 内的独立条件 job；启动核心及共享操作也由宿主登记。

`job_gating` 是 `chrono-github-units/v1` scoped／v2 full 的显式可选采用；缺字段保持旧语义，不追溯重解释历史证据。检测复用 judge-ci collection inventory；full 调度复用 FILEMAP/routes 的纯声明选测及两端 assignment，SDK 输入快照仍归实际 full 准入。检测不得调用业务 check/probe、选全兜底或推断目录／语言／import；允许按宿主显式 source inputs 启动产品本身。Git 完整树／登记 blob 在不物化全部宿主源时可读，PR 固定 event base/head、默认 push 完整 before/after，既有 integration `push_baselines` 保留并由 detector 一次固定。浅 blobless depth2 是可用起点，缺端点明确补取；禁止静默 HEAD^／merge-base 和 workflow paths／paths-ignore。完整 Git DELTA 不设路径数量门，超过平台 API 路径上限仍完整处理。aggregate 的 Rust provider 必须拒绝 detection 失败及任一 required 单元非 success（含 cancelled、missing、unexpected skip）；仅 nonrequired 单元允许 skip。汇总仍经固定 `check --collect` 核原始报告及最终判官，不以 GitHub 状态代替准入。单元重跑保留同一 parent 的原 detector／成功前置 attempt，逐项核最新实际 job 与相应 original artifact attempt；不轮询其它 workflow 或等待自己完成。旧投影只按明确登记和精确源字节退休，init/generate/migrate 保留自定义并拒覆盖 user files。详细字段、原生环境登记、full context／external input 与历史 decoder 的未完成边界见 [CI units](docs/ci-units.md)。

full-v3/v4 的 `execution_units` 由同一 assignment/select/required 与七判官 DAG 裁决。schema4 日常入口仍为 `check`、`check --unit ID`、`check --collect`；裸 check 保留全局 DELTA，单元贡献不等于准入。汇总消费 `chrono-full-collection/v1` 的原始报告、请求、七判官绑定与回执，不执行业务或业务版本命令。`chrono-github-full-ci/v2` 保留显式 exact-context dispatch；`chrono-github-units/v2` 在现有自动 push/PR provider 中显式登记 collection/各 unit 的 full context 路径与上传映射。bootstrap/caller 必须供应同一精确 full context；producer 核它与实际事件端点一致，原生 gather 核 workflow/attempt、下载原始上传目录，再交同一短命令裁决。缺失或不匹配输入失败。本仓实际 full 启用、公开新二进制和远端原生 full 验收仍未完成。

local 独立 scopes 仅在当前端点、birth／role 与输入引用一致且新观察仍满足 workflow 登记的 age 判法时保留同一 context 及原 `observed_at`；超过期限自动保留旧原件并发布新的不可变观察，旧贡献不能满足新 context，当前分支仍由 workflow 拒绝 stale。裸 check 产生新观察。unit 的 live 文件输入由 registration/input 单一实现按显式 unit plans、操作所有者、FILEMAP 依赖及 closure bindings 选取，含共享操作及治理输入；结构和引用仍全局检查，缺失／漂移／畸形／未知输入失败。capture 复用该投影；collection 核原始 blobs 与逐单元有效输入投影，不启动业务或业务版本工具。这些源码行为不证明远端 full 启用、release 或完整 SPEC 验收。

宿主自定义与更新是交付合同：单元、启动参数、平台、超时及扩展调用由宿主显式配置；安装新二进制不重置这些源。`init` 保留已有宿主源，常规修改仍从该源生成和核验。显式迁移以旧、新 provider 为输入，先核旧投影及所有目标冲突，再更新受管输出、退休不再使用的旧输出；保留新源和无关宿主文件，不推断测试归属或替宿主改登记。公开 beta.13 的 `chrono-ci migrate` 支持 scoped v1／units 到 units，以及同源地址的显式旧配置快照；初始 inventory、full 和 release 合同不在该迁移入口中。完整 CLI 与失败边界见上述合同；投影迁移通过不等于 CI、完整治理或跨文件事务通过。

### 4.2 CI 缓存与增量编译（目标合同）

CI 必须实际恢复和保存可重建缓存，覆盖启动工具、当前判官及所选生产／测试项目的编译工作。检查与发布 CI 均适用；检测、单元及汇总 job 按各自登记的消费者取缓存，不默认构建或缓存全仓项目。仅有名为 cache 的本地目录或 artifact 上传不算跨运行缓存已启用。

缓存政策与 provider 源由宿主在 `.chrono-harness/` 显式登记，沿现有 generate/verify 入口生成与核验 CI 投影；不得独立手改生成 YAML。逐项登记缓存 ID、所有者、生产／消费操作、准确路径、兼容输入、键的构造、允许的恢复前缀、恢复／保存时机和处置方式。路径引用已有登记工件或显式外部工具缓存，不按目录、语言或文件扫描发现缓存。执行与选测仍只消费 FILEMAP 的声明。

| 缓存内容 | 必须保留与复用的范围 |
| --- | --- |
| 依赖取得缓存 | 已登记包管理器的下载、索引及固定依赖内容；恢复后仍核锁定版本与内容身份 |
| 判官与启动执行物 | 当前 runner、判官及辅助工具的构建输出和已登记安装产物；每个 job 只取自身需要的执行物 |
| 各独立项目的编译产物 | 生产项目与专属测试项目各自的 target／等价输出，包括依赖编译产物、build-script 输出、指纹及编译器增量数据；保持原有输出隔离 |

Rust 的登记编译操作必须启用增量编译，并将各项目实际使用的 `target` 中增量数据一并纳入持久缓存。debug/test 与 release 的 profile、`CARGO_INCREMENTAL`／profile 配置及真实生效值均须登记；不能只缓存包下载、最终二进制，或在 CI 关闭 incremental 后宣称缓存了增量编译。其它工具链按其显式登记的增量／编译缓存合同采用，不由通用 runner 推断实现。

缓存兼容域必须绑定缓存格式、生产／消费项目及操作、OS／架构／target、实际工具链与编译器身份、profile／features／flags，以及已登记的链接器／SDK／build-script／配置／环境输入。锁文件、manifest 与显式直接依赖的构建输入也进入相应键。提交 OID 可区分保存代际，但不能作为唯一兼容判断或唯一恢复条件，使每次源码变化都丢失可用的编译缓存。键必须由实际声明输入确定，不能用不相关目录变化替代依赖。

依赖取得、可复用编译中间产物与可直接执行的二进制分别明确恢复条件。编译缓存可以从同一兼容域的旧源码键恢复，以登记构建命令校验指纹并增量重建；跨工具链、target、profile 或其它不兼容域不得宽前缀恢复。直接复用判官二进制还须绑定候选源码及其显式传递依赖、锁文件和全部登记构建输入，核实际版本与摘要符合当前执行物绑定。缺少身份、来源绑定或完整适用输入时，将它作为构建候选或重建，不直接执行为候选判官；缓存中的历史判官不得参与当前裁决。

恢复必须在相应 bootstrap／构建前，保存必须在编译生产者退出、写入加入且停止后。缓存恢复／保存本身也是登记消费者；采用自动清理的宿主须在整个消费与保存期间持有相应使用保护，在最后租约释放、缓存被回收前完成保存，不能与清理并发读写。精确二进制命中并经当前绑定核验后可复用；恢复旧编译缓存时仍运行同一登记构建入口，只由构建工具决定可复用部分。禁止为缓存增加第二套构建配方或因命中跳过所选判官／测试。各独立 job 保留隔离的输出路径；跨 job／重跑／跨运行共享通过登记缓存传输，不让并发生产者写同一份活目录。保存已完成的可重建编译产物可以独立于后续测试失败，但必须保留真实构建／测试状态，不发布失败或未完成构建为有效执行物。

缓存未命中、不可用或损坏须按 §7 的标准日志记录，并沿同一入口执行冷构建／重建；恢复失败保留原始异常和处理结果。候选仍不满足当前执行物绑定或构建失败时，检查非零，不能用旧缓存或命中状态填成功。缓存不得运输治理通过状态、旧判官响应、测试结果、运行 context、生命周期锁／租约或必要证据；本轮报告和原始单元证据仍由实际生产者生成并按其 artifact 合同保存。

报告必须记录缓存 ID、请求键、实际恢复键、精确／兼容命中或 miss／error、核验与重建结果、保存结果及所消费执行物身份；未知测量仍为 unknown。普通日志时间和命中状态是操作观察，不决定裁决。冷缓存、精确命中及兼容旧源码缓存必须执行相同选中义务、保持相同调用合同；同判仍受 §4 的完整有效输入及确定性前提约束，缓存命中不建立 parity。

## 5. DELTA 与依赖图算法

在宿主定义的治理／输入范围内，DELTA 选择充分的前提是：两状态实际语义输入及其直接/间接依赖均完整登记，且判官/测试
谓词具有局部性：仅由已登记的语义输入决定，未受变更输入及其闭包影响的谓词保持不变；外部输入也属于状态。
图上闭包仅是相对于给定登记图的机械事实，真实依赖完整性与局部性是工程义务，不能由合法登记推出。
该范围内未解决的必需依赖、已知缺失的 fixture、环境、外部输入或非局部影响须报输入/证据不足，AI 显式补登记或调整检查单元；
不推断补边，不声称能自动发现所有遗漏，也不声称已检查每个历史对象或证明未来 Rust 实现。
AI 的 declared-complete 工程声明及登记核验支持普通合同验收；普遍隐藏输入不存在的证明不是该验收前提，也不能从一次通过推出。

DELTA 是 base tree 到 candidate tree 的集合差，包含路径、blob OID 和 mode。
按字节稳定排序路径，用 Git `--no-renames` 的含 NUL 输出或等价库接口读取事实。
增加 A：base 不存在、candidate 存在；修改 M：内容或 mode 不同；删除 D：仅 base 存在。
重命名规范化为 D(old)+A(new)，不依赖 Git 相似度阈值；展示可附 rename 提示，不改变裁决。
同内容改名仍须检查旧归属、旧消费者、旧测试和新路径登记。
二进制文件也按 blob 比较；没有文本 diff 不等于没有 DELTA。

第 1 步：读取 B/C 两份登记，解析变更文件及变更的记录 ID、字段和边。
第 2 步：建立 `G = G_base ∪ G_candidate`，保留每条边来源 base/candidate/both。
第 3 步：变更文件、已登记有效输入的两端变化为种子；登记变化另把变更记录目标加入种子；有类型边的增加、删除或改接必须把两端节点加入种子。
例如只删除 `runner → runner-tests` 测试边，也把 runner 与旧测试作为影响种子。
第 4 步：沿已登记的有类型正向边计算影响闭包，去重但保留来源和因果路径。
第 5 步：选出候选测试、判官、成本项；验证删除/迁移事实后执行候选中仍存在的测试。
第 6 步：每个 finding 必须关联一个 DELTA 路径或登记变更，以及明确的依赖因果链。
没有因果链的旧缺陷不应成为本次错误；可作为未裁决 context 列出。

可达节点集合不能代替有类型边、逐种子来源或执行要求。即使端点图与并集图的可达节点集合相同，也不能据此改用 candidate-only 图选测。例如 base 仅有 `project:p → test:t` 的 test-execution 边、candidate 删除该边、两端节点都已成为种子时，两图都到达这两个节点；只有保留旧边的并集仍选择旧测试。测试删除、替换或退休仍按第 6 节结算，不因节点已到达而省略。

| 边 kind | 方向与含义 | 不允许的推断 |
| --- | --- | --- |
| compile | 输入文件→项目；依赖项目→消费者项目 | 不解析 Rust import 来补关系 |
| build-input | manifest/lock/生成器/外部输入→项目 | 不把所有根目录文件当公共构建输入 |
| runtime-input | 已明确选择的资产/宿主 canonical 材料及为保留原文、校验标记而读取的已有输出字节→消费项目 | 不将运行数据伪装成编译输入，也不由运行时扫描补登记 |
| test-execution | 受影响项目/文件/脚本/输入→测试 ID | 不按文件名、测试名、Cargo 元数据猜测试 |
| judge-trigger | 文件/项目→判官 ID | 不凭扩展名选判官 |

compile/build-input/runtime-input 可沿显式项目 compile 边继续传播，但只有 test-execution 边能选测试。
workflow 的显式 stability 路径/测试及条件 integration.tests 同样是白名单关系；现役 FILEMAP impact v2 将适用要求降为有来源的 test-execution 边，再交同一闭包与执行链，见 [选择合同](docs/workflow-selection.md)。此步骤不认证分支、迁移或 integration 证据。
配对关系表达所有权，不隐式产生测试边；缺配对执行边由 projects 判官报错。
every-delta 判官始终被选择，judge-trigger 提供额外影响解释；二者取并集去重。
依赖闭包需要读取未变文件和历史登记，这是读取上下文，不是重判历史文件。
若某个声明的测试单元会验证完整项目，允许执行它；测试由 DELTA 选出且故障须说明因果。
与该 DELTA 无因果关系的既存测试失败标为 `E_EVIDENCE_UNRESOLVED`，不能捏造通过或旧违规。

删除文件从 base FILEMAP 取得 owner/依赖/成本；不能因为 candidate 不再登记而丢失影响。
新增文件必须在 candidate 注册；修改文件必须在两端有可解释的注册或显式补登记记录。
完整登记表可读作上下文；初始完整性检查只要求本轮新增、变更、受影响引用闭合。
未改动、无影响的历史缺登记留给后续明确 migration DELTA；不假造全库已经合规。
schema 不可解析影响本轮输入时是错误；不能以“历史问题”忽略无法构图。

### 5.1 形式对应与保留义务

以下对应固定引用 trureturing 修订 `5cd5558013f6e02612a617b2ba5b03db7a6de0f6` 的 [TwoStateLocalityIncrementalPreservation.lean](https://github.com/the-omega-institute/trureturing/blob/5cd5558013f6e02612a617b2ba5b03db7a6de0f6/D5/S3/ConceptDynamics/Governance/TwoStateLocalityIncrementalPreservation.lean)。模型只承载下述条件结论，不提升 Rust 实现、宿主登记或实际执行的验证状态。

| SPEC 对象／义务 | 形式对象与结论 | 仍须履行的实现义务 |
| --- | --- | --- |
| 未受影响对象的旧结论可保留 | `TwoStateLocalityIncrementalPreservation.two_state_locality_yields_incremental_preservation`：若属性具两状态局部性，声明依赖包含两端实际读取，对象自身及这些依赖值均未变化，则两端属性等价 | 状态须包括政策、执行物、工具链、配置、环境及实际外部输入；缺失值须与空值区分。真实读取的包含关系和局部性仍是前提，FILEMAP 格式正确、哈希存在或图闭合均不证明它们 |

`State` 对应固定端点，`Artifact` 对应受检查对象及其输入，`bytes` 对应包含缺失态的输入值；`reads` 是两端实际读取，`dep` 是宿主声明的依赖上界，`property` 是要保留的判定属性。使用 `Option String` 的精确应用已编译，可区分不存在与空字节。实际读取集合、局部性和依赖包含关系仍由宿主履行，不能把 `Local` 假设当作自动取得的结论。不另造仅包装既有结论的定理。

已合并的 trureturing [PR #10868](https://github.com/the-omega-institute/trureturing/pull/10868) 在修订 `168e85c372e38d926a25892a5f752c3f9ad8a173` 的 [TYPED_GOVERNANCE_ADMISSION.md §Theorem 6](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/docs/develop/theory/TYPED_GOVERNANCE_ADMISSION.md#L190) 交付种子边更新的节点集合推论，并登记对应的 [atom](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/Meta/Digestion/atoms/sha256/e041845285be68b9a306b63e427dfde5f58b09e7b8c977c1099c760f214c432e) 与[吸收映射](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/Meta/Digestion/backfill/typed-governance-admission/absorbed-closed/e041845285be68b9a306b63e427dfde5f58b09e7b8c977c1099c760f214c432e.yaml)。它复用修订 `64b78db1ad18a8a90b42e13a412dfc281d5c0922` 中已冻结的 [ConsequenceClosure.lean](https://github.com/the-omega-institute/trureturing/blob/64b78db1ad18a8a90b42e13a412dfc281d5c0922/D5/S3/ConceptDynamics/DagCompletion/ConsequenceClosure.lean) 的 `ConsequenceClosure.consequenceClosure_least`、`ConsequenceClosure.consequenceClosure_successorClosed`、`ConsequenceClosure.subset_consequenceClosure`；该 PR 未新增命名 Lean 定理或冻结条目。

Theorem 6 的派生结论是：给定任意顶点类型、两端有向关系及同一个种子集合，若每条新增或删除的关系边的终点都在种子中，则并集图、base 图、candidate 图从这些种子可达的节点集合相等。它允许环和无限顶点类型，路径仍是有限路径且允许零长度。证明遇到某端缺失的边时，从其已登记为种子的终点重新出发，因此不保留原来的起始种子。

与 FILEMAP 的对应是：顶点取显式节点 ID，关系取“存在某种 kind 的登记边”，种子取本轮变更种子的节点集合。第 5 节要求变更边两端进入种子，强于该推论仅要求变更关系边终点的前提；从 Rust 的有类型登记变化到该前提的实现对应仍须核验。此结果不授权删除旧边，也不授权把逐种子解释、路径、边类型、测试选择或成本替换为节点集合。精确有类型边投影应用及下述删边反例的既有编译证据不构成 Rust 算法的形式验证。

第 5 节的删边例子也给出实现对应的边界：把有类型边投影为“存在某个 kind 的边”，或只保留可达节点，会丢掉测试选择需要的信息。`project:p` 和 `test:t` 同为种子、唯一测试执行边被删除时，两图的可达节点均为这两个节点；candidate 图没有执行边，并集图保留旧执行边。因此任何只证明节点集合相等的结果都不足以替代旧边、逐种子解释、路径、测试选择、成本或退休义务。

当前对应限于上述参数映射、冻结声明与派生推论、精确应用及反例的既有编译证据与现有行为回归。相关 Rust 消费者是 `crates/judge-filemap/src/{lib,graph}.rs`；`crates/judge-filemap-tests/tests/impact.rs` 的 `file_record_field_and_edge_add_delete_retarget_changes_seed_both_ends`、`base_only_test_edge_preserves_removed_requirement_owner_and_cost` 和 `pairing_and_test_record_seed_without_execution_edge_do_not_select` 分别约束变更种子、旧测试要求与仅到达测试节点不执行。没有机械证明 Rust 相对模型的精化，也没有证明真实输入登记完整或任意判官确定性。

## 6. 登记变化、移除与演进

登记自身也是代码数据，其 A/M/D 进入 DELTA；不使用 candidate-only 图或“最后配置赢”。
FILEMAP 新增文件行、调整 owner、登记实际 build 输入/测试边是 membership 变化。
引用目标删除后仍存在的消费者报 `E_DANGLING_EDGE`；AI 修改消费者或明确登记迁移。
删除测试项目必须同步迁移生产项目的专属测试及执行边，不能留下一个无测试项目。
合法同时删除生产项目与其测试时，workflow.retirements 明确列出二者及原因。
旧测试计入 `retired_tests` 和成本影响；不存在的候选测试不被误标为已执行。
旧测试若无合法 retirement 或 replacement，返回 `E_REQUIRED_TEST_REMOVED`。
替代测试必须在 candidate 真正执行，旧成本依然可见；同一方法不永久冻结项目结构。

删除/替换判官要登记 `kind: "judge"` 的 retirement、replacement 或 null 和原因。
只执行 candidate 指定的当前判官，以 evaluate 模式按新政策判本轮 DELTA。
candidate 的 migration_validator 消费 registration/filemap/cost 的结果，核对 base 登记、
旧边、成本、retirement/replacement、迁移映射与 integration 证据；旧谓词不永久生效。
base 只提供历史事实，不启动旧判官，也不要求取得、重建或用适配器包裹失效旧执行物。
当前验证器可自主替换；其自身替换同样保留两端证据并经过新候选的 integration 验证。
不自动下载猜测的插件、不把未知删除当“关闭成功”；有意规则调整是正常自主工程动作。

schema 版本变更要提供登记迁移：旧/新版本、记录映射、删除原因和配对兼容测试。
workflow.migrations 中 script/test 必须引用已登记脚本，mappings 元素为 `{from, to}`，
from/to 可为 null 表示新增/删除，但不能同时为 null；删除原因放同条 reason。
仅实际 schema 转换需要迁移脚本及配对测试；同 schema 替换判官无需创建适配器。
candidate registration 按登记脚本处理旧 schema，保留原始记录、映射和摘要；证据不足报错，不落回默认配置。
新判官 ID、项目、语言和适配器都经白名单扩展；runner 无需知道当前项目全集。

## 7. 判官、脚本与插件协议

每个判官是显式 executable + argv，cwd 是 candidate 根目录，进程环境来自已登记工具。
启动直接传 argv，不经过 shell；需要 shell 的脚本必须显式指定解释器、脚本及其依赖。
stdin 恰好一个 UTF-8 JSON 文档，读到 EOF；stdout 恰好一个 JSON 文档，日志仅走 stderr。
脚本可用 Python、shell、Node 等；插件后端必须由登记适配进程转换为同一协议。
不要求 Rust dylib ABI、常驻 daemon 或自动插件发现；超时和输出上限来自 config.protocol。

请求的规范结构如下。尖括号为类型占位符；本例假设 base 已登记 old.rs 且无消费者，不是当前仓库快照：

```json
{
  "protocol": "chrono-judge/v1",
  "request_id": "<sha256 of canonical request without request_id>",
  "judge_id": "filemap",
  "mode": "evaluate",
  "base": {"commit": "<full oid>", "tree": "<tree oid>", "root": "<snapshot path>"},
  "candidate": {"commit": "<full oid>", "tree": "<tree oid>", "root": "<observed checkout root>"},
  "config_path": ".chrono-harness/config.json",
  "checkout": {"head": "<observed HEAD oid>", "tracked": [], "untracked": [], "index_flags": []},
  "runner": {"path": "<actual runner executable>", "sha256": "<sha256>", "version": "<version>"},
  "delta": [{"kind": "D", "path": "old.rs", "old_blob": "<oid>", "new_blob": null,
             "old_mode": "100644", "new_mode": null}],
  "registries": {"base": "<snapshot .chrono-harness path>",
                 "candidate": "<snapshot .chrono-harness path>", "digest": "<sha256>"},
  "context": {"path": "<context path>", "sha256": "<sha256>"},
  "impact": {"schema": "chrono-filemap-impact/v1", "seeds": ["file:old.rs"], "edges": [], "tests": [], "retired_tests": []},
  "prior_results": [{"protocol": "chrono-judge/v1", "request_id": "<registration request_id>",
    "judge_id": "registration", "status": "pass", "findings": [], "evidence": [], "outputs": {}}]
}
```

上述新增事实字段不裁决 admissibility：config_path 是两端共同的显式根相对配置路径；checkout.tracked 是相对 candidate 的索引/工作树变更路径并集，untracked 包括 ignore 路径；index_flags 是 Git ls-files -v 的完整 {path, tag} 记录，registration 拒绝 assume-unchanged/skip-worktree 隐藏条件，runner 不改写索引。runner 绑定实际当前执行物。registration 从固定 tree 判断支持的 entry 类型，重读对象、磁盘配置、context 和 checkout，忽略两次观察之间新增的已声明 artifact；其它变化报错。request_id 覆盖这些字段；不把 exported clean copy 当实际 checkout。base 导出保留在 state，普通文件无写权限，不执行 base 工具。
registries.digest 精确定义为 JCS `{"base": {"<path>": <parsed JSON>, ...}, "candidate": {"<path>": <parsed JSON>, ...}}` 的 SHA-256，每端包含 config_path 与该端 config 指定的四份 registry；context.sha256 为 context JCS 摘要，evidence.sha256 为原始文件字节摘要。所有 JSON 先拒绝重复成员。
普通 full DELTA CLI 要求两个真实 commit。根提交只通过第 12 节的显式初始化配置进入单候选清单协议；不构造伪造空 base，不产生 DELTA 治理成功。登记版本迁移按第 6 节的候选转换与实际兼容测试合同执行。

impact 的版本为 `schema: "chrono-filemap-impact/v1"`，以下示例列核心字段；完整附加事实见 [FILEMAP impact 合同](docs/filemap-impact.md)。impact.edges 为平铺的 `{from, kind, to, origin}`；origin 是 base/candidate/both。seeds 是确定性去重的节点 ID 列表，附加 seed_causes 保留每个原因的 ID、节点、实际路径／记录引用和原因。tests 是候选仍存在的选中测试节点 ID 列表，retired_tests 是候选已不存在的选中必需测试节点 ID 列表，两者确定性去重，只表达选择／删除事实，不代表执行或退休批准。required_tests 和完整两端记录、节点定义及因果来源作为附加字段保留。retired_tests 成员资格不得豁免 `E_REQUIRED_TEST_REMOVED` 或第 6 节的迁移／退休证据义务。
prior_results 仅含 after 列出的前置判官完整响应；不得从运行目录猜测旧输出复用。
registration 先消费原始快照与 context；filemap 据登记生成 impact，其他判官显式消费结果。尚无前驱产物时 impact 为 null，不填造已算出的空闭包；runner 仅将直接前驱 outputs.impact 同名传入，冲突报错，其余具名产物保留在 prior_results.outputs。
routes 先验证执行入口，projects 再执行测试；judges.after 中明确登记这个依赖顺序。
共享上下文的推导属于登记判官逻辑；runner 只传递具名输出，不在内部另写项目政策。
所有摘要使用 SHA-256；JSON 摘要使用 RFC 8785 JCS，文件 blob 按原始字节计算。
普通请求的 registries.digest 覆盖两端五份 JSON 的路径与内容，两个端点都必须存在。初始化请求没有 base 或 delta 字段，使用单份候选登记映射的独立摘要。

```json
{
  "protocol": "chrono-judge/v1",
  "request_id": "<same request_id>",
  "judge_id": "filemap",
  "status": "pass",
  "findings": [],
  "evidence": [],
  "outputs": {"impact": {"schema": "chrono-filemap-impact/v1", "seeds": ["file:old.rs"], "edges": [], "tests": [], "retired_tests": []}}
}
```

status 是 pass/warn/fail/error；每个 finding 含 `code, level, message, delta_refs, causes`。
delta_refs 是本请求 DELTA 路径或登记记录 JSON pointer；causes 是注册边路径数组。
evidence 条目为 `{path, sha256, kind}`，路径须位于 state；不能用不存在的输出作证明。
outputs 是登记的具名结果，filemap 的 impact 按上述结构；无结果时为 `{}`。
请求/响应 ID、协议、状态、必需字段或实际退出码不一致时返回 `E_PROTOCOL`。

| 子进程/总检查退出码 | 含义 |
| --- | --- |
| 0 | pass 或仅 warn；报告区分它们，warn 不要求确认 |
| 1 | 至少一个 fail：已完成判定，存在约束错误 |
| 2 | error：输入/协议/基础设施失败或必要证据不足 |
| 3 | 早期 spec-only CLI 的未实现退出码；现役 CI slice 不使用，完整配置拒绝返回 2 |

子进程输出 fail 却 exit 0、空 stdout、额外日志、截断 JSON、未知状态都属于协议错误。
本轮选中的当前进程崩溃、超时、不可执行、摘要不匹配、必需测试未执行都不能降级为通过。
总结果优先级 error > fail > warn > pass；基础错误阻止依赖任务，独立任务可以继续收集证据。
被阻止的任务列为 blocked，不能省略；完整通过必须有全部已选判官/测试结果。
测试操作本身不必输出判官 JSON；projects 判官调用登记操作并包装退出码、工具和结果摘要。
程序 stdout/stderr 是证据内容，不能靠查找字符串 PASS 代替进程状态。

### 7.1 标准日志体系（目标合同）

所有产品程序、判官、宿主脚本和插件适配器必须通过统一标准日志体系输出诊断、进度、告警及异常，禁止在业务代码中散落自由格式的 print／println／eprintln 充当日志。Rust 使用 `tracing` 事件／span 与统一初始化和格式化入口；其它语言使用其标准日志接口接入同一记录合同。日志字段、级别语义和格式的唯一实现归现有自然所有者，消费者复用，不各自维护拼接配方。

默认机器日志为 UTF-8 JSON Lines，每行恰好一个 `chrono-log/v1` 记录。日志格式／级别／sink 的选择及影响运行的环境变量由宿主显式登记；本地可选人类可读格式，仍由同一记录渲染。默认级别为 info，trace/debug 用于诊断，info 记录正常阶段与已处理观察，warn 记录降级／可恢复问题，error 记录当前操作未完成的失败。日志级别不是 pass/warn/fail/error 的裁决状态，stderr 非空不代表失败。

| 记录字段 | 合同 |
| --- | --- |
| `schema, timestamp, level` | `chrono-log/v1`、实际 UTC RFC3339 时间、`trace/debug/info/warn/error` |
| `producer, event, message` | 实际生产者、稳定事件 ID、供人阅读的说明；消费者不解析 message 决定行为 |
| `context` | 当前可知的 run/request/judge/operation、固定 base/candidate、阶段及输入／路径身份；未知者显式缺省，不伪造 ID |
| `fields` | 事件的具名结构化数据，包含必要的原退出、边界及缓存观察 |
| `error` | 有异常时附 §7.2 的完整错误链或已发布原始错误记录的身份引用；无异常可省略 |

日志默认只走 stderr；判官 stdout 保持恰好一个协议 JSON 文档。CLI 帮助／版本、登记数据输出、完整机器报告及短命令结果投影走各自已有输出合同；它们由标准输出入口统一渲染，诊断不能混入其中。子进程原 stdout/stderr 按字节保留，不能改写成 JSON 日志后丢掉原件；适配器通过日志记录其身份、状态与证据引用，显式 participation 的原样转发合同继续成立。

上下文随调用、异步任务和子进程边界传递，并绑定实际 request／operation，不能靠相邻行或字符串拼接猜归属。普通库层只附加上下文并返回错误；负责处理或结束该操作的边界发布一次对应事件，外层继续引用同一原始错误，避免每层重复倾倒整份输出。完整日志或原始错误写到登记 state 并由现役生产者发布身份；摘要必须给出真实路径／摘要。超时、输出超限或控制台省略时记录截断／缺失事实与可用原字节，不能把省略后的内容称作完整证据。sink 初始化、写入或证据发布失败保留其原异常，并与原业务异常一起按 §7.2 传播；尚无 logger 时只由统一应急入口向 stderr 输出结构化失败，不能静默丢日志。

### 7.2 异常传播、处理与一般规则（目标合同）

错误必须作为带类型和原始因果关系的值逐层向调用边界传播。每层可增加当前操作、输入、路径及稳定错误码，但必须保留原错误为 `source`；禁止把原错误仅格式化成字符串、新建无 source 的“失败”、置空 cause 或用通用码覆盖原生错误。Rust 使用 `std::error::Error::source` 的错误链，Python 使用保留 traceback 的重抛或 `raise ... from ...`；其它语言使用等价的 cause／inner exception。没有底层异常的直接领域拒绝可用 `source: null`，不得制造异常。

跨进程与报告边界使用独立、版本化的 `chrono-error/v1` 记录保留等价结构。每一层包含 `type, code, message, context, source`；根因保留实际类型、原始消息及已取得的原生错误码，已取得的栈／traceback 也保留。子进程失败附原 argv、退出／signal、stdout/stderr 字节或不可变证据引用，以及 timeout／overflow 等实际观察；尚未启动时不伪造进程结果。记录及引用由生产者产生，消费者核身份并展开完整链；引用不可用须明确报证据不足。现有 finding 的 `causes` 仍表示登记图边，不兼作异常链。新记录与其传输字段须通过显式 schema／协议演进接入，不能静默塞入当前严格 v1 请求／响应。

捕获、重试、降级、恢复、转换为领域结果或忽略预期缺失也是处理事件，必须附原始异常链和实际处理动作／结果，即便最终成功。成功只描述已完成的恢复或业务操作，不删除先前失败。重试逐次绑定尝试及原异常；次数、时限、可重试条件由当前登记规则决定，不靠循环直到偶然成功。受控的预期缺失可作为结构化领域状态返回，但若由异常触发，处理记录仍保留原异常。无异常发生的正常缺失不凭空补异常。

处理过程中又发生异常时，原业务异常与新异常均保留，标明哪个是 source、哪个是清理／日志／发布等相关失败，不能在 finally／析构／日志写入失败时覆盖原根因。顶层协议错误、CLI 错误码和短摘要必须指向同一完整错误记录；只有字符串摘要、包装器退出或“已处理”标记不足以证明异常链完整。失败、超时和无结果先读取已有实际错误、输入、原输出及退出，再决定修复；未知结果保持未知。

对任意路径、文本、OS／工具错误、外部响应及其它开放或无限输入集合，按类型、结构、语义不变量和能力边界定义通用处理规则，不逐个样本、消息片段、文件名或测试 case 打补丁。只有已登记的有限协议值／错误码可以枚举；未知成员进入统一且保留原始输入／异常的规则，不把未知当通过。新特殊分支必须具有独立语义和明确适用边界，不能只覆盖曾出现的一个失败文本。检查使用等价类、成功／失败／边界预期及必要反例；通过有限样本不宣称无限集合已穷举。

## 8. 默认判官、规则表面与警告

当前 judges.json 登记以下判官；registration、filemap、routes、projects、cost、mixed、workflow 已实现有界合同，宿主绑定仍 proposed，不存在默认隐形 gate。

| ID | 本轮职责与典型错误 |
| --- | --- |
| registration | schema/引用/白名单/快照/输入闭合；E_UNREGISTERED、E_SNAPSHOT_DIRTY |
| filemap | 两端图、类型边、归属、影响；E_DANGLING_EDGE |
| projects | 一一配对、项目隔离、所选构建/测试执行；E_TEST_PAIR、E_TEST_FAILED |
| routes | 唯一操作与 argv/证据一致；E_ROUTE_MISSING、E_ROUTE_AMBIGUOUS |
| cost | 显式成本与旧/新影响报告；未知值 W_COST_UNKNOWN |
| mixed | 规则与产品同一 DELTA 的可见成本警告；W_MIXED_JUDGE_PRODUCT |
| workflow | dev/integration、新鲜度、稳定性及登记迁移证据；E_BRANCH_STALE、E_INTEGRATION_REQUIRED |

FILEMAP.surface 显式声明整个文件表面，不通过文件内容、目录名或语言模型猜语义。
membership 文件中的部分字段通过 config.semantic_fields 声明为语义表面：
projects 的 actions 新增/修改/删除涉及治理方法；FILEMAP 已有行的 surface 改动/删除涉及分类。
pointer 的 `*` 仅匹配一个对象 key 或数组元素，不是文件路径通配符；行用 path/id 定位比较，
不能因 JSON 数组换序误判规则变化。schema_version 改动按登记 schema 迁移处理。
对同一文件取 base/candidate 表面分类并集，不能通过本轮重分类遗失旧语义影响。
config/judges/workflow 全文件为 judge-policy；独立判官脚本可登记为 judge-implementation。

代码加对应 FILEMAP 普通成员行或真实依赖边，是通常的代码+登记协同修改，不触发混合警告。
已有判定字段、动作、规则配置、判官实现或绑定二进制摘要变化属于规则语义变化。
此类变化与 product/test 文件修改同一 DELTA 时，mixed 输出 warning 及两组路径、旧/新成本。
warning 不升级成 blanket block、不索要批准、不要求人为确认或另起提交才能继续。
integration 是这类规则变更本身的稳定性要求，并非混合提交惩罚。
AI 先尝试在现有规则下解决任务；确需改判官时自主变更并验证，报告为什么和影响多大。
源码自举的 engine 修改只按显式 stability 登记要求 integration，本身不被当作宿主 policy。
现役 mixed 输出和语义选择合同见 [docs/mixed.md](docs/mixed.md)；integration 认证由独立 workflow 消费实际结果与完成报告，合同见 [docs/workflow.md](docs/workflow.md)。

## 9. 成本与执行报告

报告必须包含 `schema_version, status, base, candidate, candidate_tree, context_digest,`
`registry_digest, executables, tools, environment, effective_inputs, parity, delta, impact, judges, tests, findings, costs`。
executables 为 `{path, sha256, version}` 列表；tools 另记录实际解析路径和工具版本。
effective_inputs 记录两端完整有效输入的 ID、实际值摘要及来源证据；摘要存在本身不证明相等或完整。
parity 为 `{status: "established"|"unestablished", compared_report: string|null}`；普通 full check 保持 unestablished。
现役独立比较器仅在满足 §4 的更强输入／观察要求并与具名报告匹配时写 established；它记录成对证据，不证明普遍确定性，也不参与普通检查准入。性能测量差异分别报告。
judges/tests 记录每项的 selected/executed/blocked/retired 状态、退出码和证据摘要。
这里 selected 是执行计划状态，完整结果不能停留在 selected 就返回 pass。

costs 报告每个变更文件、影响项目、选择测试及旧删除节点的成本引用和来源。
四维向量分别为 CPU 时间、墙钟时间、峰值内存、IO 字节；不折算成虚构统一分数或罚税。
展示 declared_before、declared_after、affected_tests、retired_tests 和 unknown 列表。
不同资源的峰值不能直接相加；并行 wall 时间也不能当串行总和。
可按显式串行计划估算总 CPU/IO，任何输入未知则总估计注明不完整，不填 0。
实际测量放在 measured 下，附工具、样本与时间；不能把构建耗时冒充测试耗时。
当前所有成本 unknown 是诚实的初始登记；cost 判官报告 warning，不能捏造基准数据。现役有界成本输出见 [docs/costs.md](docs/costs.md)。

首增量的 `scope: "configured-judges"` 草稿也必须提供上述全部顶层字段。尚无生产者结果的
tools/effective_inputs/impact/tests/costs 写 `null`，扩展字段 `unresolved` 按报告 JSON pointer
给出原因；不能用空列表、零成本或配置声明冒充已观察结果。`sources` 按目标 JSON pointer
列出原始响应在同一报告中的 JSON pointer。验证通过的响应若提供同名 outputs，原值转入对应字段；
多份值相同可共用，冲突或生产者给 null 则保持 unresolved，原响应完整保留。这仅运输结果，
不由 runner 判断专门政策、输入闭包或具名产物的语义完整性。
findings 汇集所有验证通过的响应，逐条绑定来源；部分响应缺失时列明不完整原因，无有效响应时写 null。
executables 仍是 `{path, sha256, version}` 列表：runner 使用自身执行物和编译版本，判官使用
实际 process 观察的路径/摘要；未观察的判官版本为 null 并给原因。每个 judge 的 binding 仅是
配置元数据，executable_index 才指向已有进程观察，不把 blocked 或未获进程观察的绑定填成执行事实。
草稿 status/退出码只聚合配置判官与运输状态，parity 保持 unestablished；即使 scoped pass，
这些未知项仍不是完整治理成功。完整报告、专门生产者及完整输入证据的义务不因此减免。

## 10. dev、integration 与过期分支

现役 `chrono-worktree start` 按宿主 `.chrono-harness/worktree.json`、五份登记及实际远端抓取结果创建新工作树，记录固定 base/tree、分支、开始时间和真实 Git 回执。它只生产操作结果，不产生新鲜度、integration 或治理通过判词；后续仍调用相同的登记 check 入口。创建失败保留工作和身份供恢复，不重置已有分支或删除已有目录。实际合同与当前恢复、PR、合并边界见 [worktree 合同](docs/worktree.md)。

源码增加 opt-in 的 `automatic_cleanup`：宿主在 `.chrono-harness/` 登记版本化政策、准确工件路径／处置、存活协调锚、固定保留分支与可选删分支；未采用的旧配置保持显式维护语义。成功 start/reconstruct 自动登记实际 birth，并在新建前排空已登记终态项；finish 是调用方加入任务并确认完成／落地后的固定交接，运行时观察当前身份、固定保留 OID、产生不可覆盖终态回执并立即调用同一 drain，独立命令为 maintain。登记消费操作通过 use 复用现有进程 owner 与小型持久使用保护，和 finish／删除同步；任意未登记后台写入仍归调用方加入。缓存处置复用现役工件 owner，不推断语言或目录；未解决证据、脏源码及未保留提交保留 checkout，只清准确授权的可重建输出。整树清理继续委托原 Cleanup 保留合同。原失败、部分效果、缺失／中断及漂移均据实保留，未建立空闲定时清理、掉电事务或通用证据归档平台。主宿主政策保留 state/bin，默认 cache-only；真实宿主 terminal 事件与 Git/GitHub 落地归调用方。完整格式与限界见 [自动生命周期清理](docs/worktree.md#opt-in-automatic-lifecycle-cleanup)。

自动生命周期重试以 `chrono-worktree-retained-input/v1` 引用保留的原始证据，绑定协调宿主、相对 state 路径、摘要、长度及 receipt／原始 bytes 格式。不可变回执直接引用，部分或未封口结果的原字节独立保留；沿合同明确列出的引用核验原件，缺失、改写、绑定错误或符号链接拒绝处置。不得逐层嵌入历史正文导致重试报告递归膨胀，不把保留未知结果改判为成功，不改写既有原件。显式维护的旧报告合同保留；具体字段及消费规则见 worktree 合同。

现役 `chrono-worktree reconstruct` 接收固定旧 base/candidate 和完整逐路径 carry/retire 计划，复用 fresh 创建后重新应用所选 DELTA；成功仅表示已得到暂存树，冲突保留实际失败与锁，旧工作不删除。AI 仍须检查变化是否必要、修复冲突及登记、提交候选，再运行现役统一检查。工具不自动判断语义必要性，也不复制旧检查或 integration 成功。

现役维护入口复用同一宿主工作树配置与登记。`recover` 绑定原失败报告、实际工作树与锁、AI 显式给出的 HEAD 和已解决 index tree；验证后释放原锁，原失败不改写。`cleanup` 按显式路径／分支／HEAD、保留分支及逐项工件处置清理已保存工作。源码现先核所有选中工件为两端登记的非跟踪真实目录，且不含候选树已跟踪路径；在自有锁内单独删除这些工件，再由有时限的 Git 移除 checkout。逐项报告已缺失、尝试未核实或已核实缺失；文件系统删除不继承 Git 子进程时限，调用方须在宿主作业生命周期中运行。工件阶段失败保留锁及源码，协调后用既有恢复入口和显式重试继续；不承诺并发或掉电事务，也不保证任意规模的源码移除均不超时。最后核对实际移除并用期望 OID 删除可选分支；部分失败如实保留，明确允许缺失工作树的计划可继续重试。不从目录、语言、年龄推断回收对象，也不替代相同的 check 入口。`cleanup-fetch` 只清理原失败 start／reconstruct 回执绑定的临时 fetch ref，要求固定本地保留分支包含期望 commit，并以期望 OID 删除、核对缺失及部分效果。源码现已在 checkout／锁操作及 fetch 前分别保留不可覆盖的 intent；`recover-interrupted` 核对当前工作树并解锁，`cleanup-fetch-interrupted` 按固定保留分支清理该 intent 绑定的临时 ref，均保留原结果的缺失或原始字节并标原操作结果未知。调用 AI 须先确认原进程已停止；终态回执仍用普通维护入口，intent 不是操作成功证明。这些入口不重建损坏元数据；显式重建见下款，PR／合并编排仍未完成。格式与边界见 [维护合同](docs/worktree.md#registered-recovery-and-cleanup)。

公开 beta.11 提供 `inspect-rebind`／`rebind`：旧收据、intent 或 linked-worktree 元数据缺失／损坏时，由 AI 显式登记现有目标目录、已有工作分支及 HEAD、要采用的 index tree、原元数据成员、全新备份和 donor 路径。观察只为该范围生成文件内容／mode／字面链接身份，不发现依赖或推断丢失的原索引。执行前绑定观察和计划、保留 intent，将原 gitfile 与残存元数据保存在登记 state 中，再用同一 Git 创建 locked/no-checkout donor、应用显式 index tree 并 repair 指针；成功须核可见工作、备份、分支及索引，随后释放锁。原操作结果始终未知；重建成功不代表检查或治理通过。普通失败保留实际输出、阶段、备份和部分效果；已附着且仍持原锁的失败可经既有 `recover` 在 AI 解决工作后释放，源码新增 `resume-rebind`：以原 intent、原计划和明确观察的缺失／部分／失败结果，复用保留与附着步骤完成续跑；原结果与工作保持，冲突报错，完成不追认原成功，beta.15 开始包含此入口。备份 rename 要求同文件系统，不保证并发或掉电事务。这两个入口已通过 beta.11 的原生发布测试与公开混合宿主消费验证；完整格式与边界见 [元数据重建](docs/worktree.md#explicit-metadata-rebind)。

所有交付最终进入 dev；feature 与 integration 均从当时最新 dev 创建。

现役源码新增 `cleanup-remote`，按显式计划与登记 remote 核对唯一 push URL、工作分支期望 OID、固定本地及远端目标提交，复用 ancestor／same-tree 保留检查，以精确 OID lease 删除并核对远端缺失。真实退出、部分效果与显式缺失重试分别报告；不清理本地工作，不推断 PR 或治理成功。目标引用、配置与服务器观察不构成原子事务；目标在删除期间变化可导致删除后的失败。该入口已包含在公开 beta.10，合同见 [远端分支退休](docs/worktree.md#remote-branch-retirement)。

workflow 当前草案阈值：落后 dev **超过 3 个提交**，或本分支年龄 **超过 24 小时**，
任一成立即 E_BRANCH_STALE；等于阈值仍允许。二者在 workflow.json 可配置。
behind 是 fork_point 到 context.dev_tip 的提交数；年龄为 observed_at − branch_started_at。
fork_point 必须是 branch/candidate 与 dev 的共同祖先；负年龄或不完整记录是输入错误。
分支名匹配受登记前缀，dev 上的合并结果以其已登记来源分支 context 判定。

过期后 AI 自主保留旧分支为恢复点，将尚未保存工作先落在旧分支或外部补丁，
获取 dev，创建新名字 `feature/<task>-r2` 或 `integration/<task>-r2`，从最新 dev 开始。
逐项重新应用仍有必要的变化，解决冲突和需求漂移；不能把旧分支 merge 进新分支假装新鲜。
重新登记输入、构建、测试并生成新报告；旧报告与旧 integration 证据失效。
旧分支仅作可回退材料，完成前不删除工作；该流程不要求人介入。

判官语义/绑定改动或 stability 路径变动：先创建 fresh integration，组合拟交付完整变化。
integration 上执行同一 check 指令与注册稳定性测试；成功报告作为 integration 证据。
workflow 判官在显式 integration run_kind 上标记 `integration_run`，执行所需测试后生成证据，
不会递归要求本轮先提供自身成功证据；feature/dev 交付才消费已完成的 integration 证据。
纯普通变化在 feature 验证后可进入 dev；AI 可显式新增 stability 登记扩大测试需求。

integration.json 为 `{schema_version, base, candidate_tree, registry_digest, executables,`
`tools, environment, effective_inputs, required_tests, results_digest, branch_ref, fork_point, branch_started_at, observed_at, status}`。
绑定 base、候选 tree、两端登记摘要、执行二进制/完整工具链、有效输入与环境、所需测试及结果摘要。
证据放 state，不纳入 tree，从而不发生“报告包含自身提交摘要”的循环。
进入 dev 前最终 candidate tree 必须完全等于测试 tree，base 必须仍是最新获取的 dev。
允许 commit OID 因合并元数据不同而变，只要 tree/base/其他绑定字段相同；报告写明映射。
若合并冲突处理改变 tree 或 dev 推进，重新建立 integration 候选并测试，不能复用旧绿灯。
缺失、失败、过期或绑定不匹配证据分别为 E_INTEGRATION_REQUIRED / E_INTEGRATION_MISMATCH。
证据额外保留 producer 身份和 proof 中的实际 context、plan、results、inputs 与判官进程。交付必须同时取得发布该证据摘要的成功完成 runner 报告；未完成进程留下的证据不能使用。输入比较只规范化宿主路径与允许变化的候选 commit 身份，保留原始证据；交付仍执行选中测试。
报告来自可信但可能出错的 AI/runner；摘要用于识别误用，不设计对抗 AI 的授权系统。

## 11. 独立脚本和插件扩展示例

以下是未来新增脚本的登记形状，不是当前 files/scripts 里已经存在的文件：

```json
{
  "id": "format-report",
  "path": ".chrono-harness/scripts/format-report.py",
  "test_script": "format-report-tests",
  "actions": {"execute": {"operation": "script.format-report",
    "tool": "python3", "argv": [".chrono-harness/scripts/format-report.py"]}}
}
```

同时登记 python3 工具、测试脚本、两者 FILEMAP 行、compile/build-input/test-execution 边及成本。
测试脚本以 tests_for 指回 format-report；协议适配脚本登记为 judge-implementation。
任意插件后端也由这样的具名适配程序接收 JSON，输出统一 findings；backend 的版本、
配置、数据文件、环境输入都作为明确依赖登记，不能在启动时扫描目录扩充能力。
对于无法可靠列举输入的插件，返回 E_INPUT_UNDECLARED，AI 补登记或选可描述的适配方法。

## 12. 初始根提交与自取自举

空仓库没有 base，允许一次明确标记的根提交存储 spec/scaffold/草案登记。
初始根提交不是一次通过的 DELTA，也不生成“所有历史已合规”的报告。
随后修改都有真实 base；初次启用仍按 §6 由候选判官检查 activation DELTA 和迁移证据，并通过 integration。
base.status 为 proposed 时记录 previous_enforcement:none，不能回填之前的成功；根提交之外不得伪造空 base。
草案与 active 的迁移都不执行旧判官；启用必须补齐宿主声明的治理／输入范围并满足 registration 要求，input_closure 为 declared-complete 且无 unresolved。此工程声明不要求普遍隐藏输入证明；现役判官仍可报告 completeness_proven:false。

宿主可显式登记 `chrono-initial-check/v1` 初始化配置，包含完整登记的 `host_config`、具名 `judges` 及进程上限，使用同一 `chrono-harness check --config P --candidate OID --initial` 入口。本地和 CI 在初始化时仍须使用相同配置和指令。普通完整配置不自动降级到该模式；初始化配置拒绝 `--base` 与 `--context`，读取真实 commit header 排除非根提交，包括浅历史及 Git replace 覆盖造成的伪根。

初始化绑定采用 `every-initial` / `inventory`，通过 `chrono-initial-judge/v1` 只传候选端点、配置与登记摘要、实际 checkout／runner／环境观察和显式前驱结果。没有 base、DELTA 或猜测的判官，复用已有进程调用、摘要、严格 JSON 与退出码校验。默认 registration 初始化判官要求草案状态，复用登记引用检查并检查整个初始文件清单；它不执行项目测试、不证明输入闭包，也不启用治理。扩展判官和脚本通过该配置的显式 DAG 登记。

`chrono-initial-report/v1` 的 scope 为 `initial-inventory`，成功状态为 `complete`，base 和 delta 为 null，governance 恒为 `not-evaluated`；实际判官响应和进程回执另行保留。初始化检查成功不会修改草案登记或补写历史合规。配置、请求、报告及已验边界见 [执行合同](docs/execution.md#initial-inventory)。`chrono-github-ci/v2` 可在宿主 CI 源中显式声明 `initial_inventory: {path, profile}`，由同一 init/generate 生成初始配置与 workflow，verify 核对二者；初始事件与本地使用所声明的同一 check 配置，普通 DELTA 仍使用 check_config。v1 保持原义且不接受该扩展；v2 必须具备完整初始声明，不推断判官、语言、路径或摘要。来源与输出均由宿主显式登记，采用保留原有定制；生成不认证登记语义，不启用治理。具体所有权与失败边界见 [CI 合同](docs/ci.md#owned-github-projection)。完整自举来源、工具链闭包、原生 v2 初始事件采用与启用仍是独立义务。

产品自举按登记方式从指定产品提交构建，将结果放宿主 `.chrono-harness/bin/`，
写入具体版本、产品源提交、二进制摘要和工具链证据后，再调用唯一 check 指令。
源提交记录在 state 构建证据中，由绑定摘要连接；源码与宿主 policy 始终是两个表面。
更新已生效二进制绑定是规则变化，须 integration；AI 可自行完成，无人审批。
构建失败、缺组件或 draft 配置不得生成成功报告；从本机取包也不得默默解析 latest。

### 12.1 中央测试版分发

产品发布统一维护在 chrono-harness Releases，宿主不跟踪产品源码包或二进制。独立 distribution/专属测试对负责显式发布清单、平台成员、版本与 SHA-256/长度锁定、采用及安装；宿主保留 `.chrono-harness/distribution.json` 与生成的 `install.py`。本地/CI 都运行该入口，再调用同一 check。平台选择只使用登记的 OS/架构，不读取宿主语言、项目目录或依赖清单。

原生打包固定干净源提交/树，发布组合拒绝不同源、不同成员或重复平台。每份下载先验身份；全部所选成员验证后才替换，普通替换失败尝试回滚并报告恢复位置。显式更新可由 AI 自主执行；摘要不证明来源、完整工具链闭包或跨平台同判。完整格式、实现范围和故障边界见 docs/distribution.md。

## 13. 可执行验收矩阵（目标行为）

下表保持完整目标验收；逐行当前实现、直接测试与缺口在 docs/spec-coverage.md，不以一个增量的 cargo test 通过宣称整表完成。
以下充分选测结论均以 §5 的实际输入完整性与局部性为前提；图闭合本身不提供这些保证。

| 场景 | 预期裁决或可观察结果 |
| --- | --- |
| 已登记源码 M，有显式测试边 | 仅闭包所选项目/测试执行，记录 DELTA 因果 |
| 新文件 A 未登记，包括未跟踪/被 ignore 文件 | E_UNREGISTERED 或 E_SNAPSHOT_DIRTY；非零 |
| 新源码与普通 FILEMAP 行同改 | 正常执行测试，无 W_MIXED_JUDGE_PRODUCT |
| D 文件且 candidate 删除其 FILEMAP 行 | base 边仍保留；选择旧消费者测试和旧成本 |
| 同内容 rename old→new | D+A；检查两端登记，不依赖相似度 |
| 只改测试依赖边或删除边 | 变更记录成为种子，保留登记图的旧/新影响并集 |
| 删除生产项目与专属测试且有合法 retirement | retired 可见，剩余引用闭合；不假称旧测试已执行 |
| 删除仍需测试、缺 replacement | E_REQUIRED_TEST_REMOVED |
| 两个项目共用一个专属测试或缺配对 | E_TEST_PAIR |
| 修改 manifest 且发现依赖未登记 | E_DANGLING_EDGE / E_INPUT_UNDECLARED；不自动补边 |
| 声明范围内 fixture/间接/环境/外部输入已知缺登记或必需依赖未解决 | 输入/证据不足；不因图闭合假称完整，不推断边 |
| 只改 README，未登记测试执行边 | 不启动所有项目测试；登记判官仍只判该 DELTA |
| 增加脚本和独立测试完整登记 | 只执行显式测试边选择的操作；无自动发现 |
| 同 operation 注册两种入口 | E_ROUTE_AMBIGUOUS |
| 绕开受治理入口、缺少绑定报告 | E_ROUTE_MISSING / E_EVIDENCE_UNRESOLVED |
| 规则语义与产品同改，已有有效 integration | W_MIXED_JUDGE_PRODUCT；仅此警告时 exit 0 |
| 产品自举 engine 源码变更、无 policy 修改 | stability 触发 integration，不自动归为混合规则修改 |
| policy 或 stability 变更缺 integration | E_INTEGRATION_REQUIRED；AI 自主建立 integration |
| integration 已通过但最终 tree 或 base 改变 | E_INTEGRATION_MISMATCH；重测 |
| feature/integration 落后 4 提交或年龄 25 小时 | E_BRANCH_STALE；从最新 dev 重建 |
| 恰落后 3 提交、年龄 24 小时 | 不因阈值本身拒绝，其他要求仍适用 |
| 本地与 CI 同一 check 指令 | 同调用/输入解释语义，不单凭命令保证裁决相等 |
| 完整有效输入/政策/执行物/工具链/环境相同且确定性求值 | 相同选择和裁决；不要求非裁决性能测量值相同 |
| 仅 OID/context/Cargo 相同，其他输入或确定性未证实 | parity:unestablished；普通检查只按现役登记合同裁决，所需输入/证据不足仍报错 |
| 暂存/未暂存/未跟踪非生成物变化 | E_SNAPSHOT_DIRTY，不偷偷只判 committed 子集 |
| 浅历史缺 base 或 fork_point | E_HISTORY_MISSING；补取后重跑 |
| 判官退出成功但 stdout 为空/非法 JSON | E_PROTOCOL，exit 2 |
| 插件崩溃/超时/摘要不匹配 | error，exit 2；不得 fail-open |
| 全部成本未知 | W_COST_UNKNOWN，unknown 明示，不编造测量 |
| 替换/退休判官，旧二进制缺失、崩溃或不支持新 schema | 当前 migration_validator 核对保留的旧事实与迁移证据；candidate evaluate + integration，无旧执行前提 |
| 无 DELTA | 报告 delta=[]，不裁决历史；输入/分支条件仍检查 |
| 首个根提交 | bootstrap/no prior base，无绿色 DELTA 报告 |
| 当前 CLI：check | **已实现**独立 chrono-ci-check/v1、显式 Git 绑定的 v2 与带 --context 的 chrono-judge/v1；完整宿主仍因 proposed/缺绑定/未实现义务非零，具体范围见 §14 |
| 当前 CLI：未知命令/信息命令多余参数 | **已实现** E_USAGE，exit 2 |
| 当前 CLI：spec status | **已实现** draft/not-implemented/proposed，exit 0 |
| 当前 CLI：help/version | **已实现** 信息输出，exit 0；help 明示完整治理未实现 |
| 程序输出诊断／进度／告警或处理异常 | §7.1 标准日志记录与实际上下文；机器 stdout、结果输出及子进程原始字节合同保持完整 |
| 根因经多层包装并跨进程／报告传输 | 可从顶层记录恢复每层 source、原生信息及实际进程证据；只剩摘要字符串不满足 §7.2 |
| 异常已捕获，重试／降级／恢复最终成功 | 原异常与每次处理动作／结果仍可读取，最终状态由实际完成结果产生 |
| 处理／清理／日志发布期间再次失败 | 原业务异常与新相关异常同时保留；引用发布失败不伪造可用报告 |
| 开放集合出现未见过的合法输入或失败变体 | 按类型／结构／不变量及明确边界处理，未知异常保留并传播；不依赖 case 名或错误文本补丁 |
| CI 冷缓存与后续精确命中 | 观察真实恢复／保存与当前执行物核验；所选判官及测试在两轮均实际执行 |
| 源码变化，兼容旧编译缓存可用 | 同一登记构建入口增量重建；复用指纹／依赖／增量数据，候选判官源码及绑定更新 |
| 工具链／target／profile／features 或适用构建输入不兼容 | 拒绝不兼容恢复／直接执行物，按登记冷构建或重建；不能用宽前缀冒充精确命中 |
| CI 缓存未命中／损坏／服务失败 | 保留原异常及处理记录，沿同一入口重建；构建或当前执行物绑定仍失败时非零 |
| 独立 job 并发／重跑，或编译成功后测试失败 | 活输出隔离、生产者加入后保存；可保留完成的编译缓存，原测试失败及本轮证据不被缓存状态覆盖 |

## 14. Implementation boundaries

§4.2、§7.1 与 §7.2 是新增目标合同。当前实现已有 stdout 协议隔离、原始进程字节／退出和部分失败证据保留，但尚未统一结构化日志与跨层异常链，也未在现役生成的检查／发布 CI 中采用持久的依赖、判官及增量编译缓存。现有 target、bin、名为 cache 的工件目录及原始 artifact 传输不满足这些新增合同。日志／异常记录、缓存登记与投影须由后续实现及显式迁移接入；本次 SPEC 更新不改变当前 schema、执行入口或声称这些验收已通过。具体待实现项同步见 docs/spec-coverage.md。

The native release recipe v2 emits `chrono-native-build/v3` evidence with explicit passed/failed status and ordered actual process records for source observation, installation, versions, builds, registered verification and packaging. Ordinary child failures retain original bytes, hashes and exits before the CLI returns failure; unstarted processes and unknown source identity are not invented. Preflight rejects an existing output or invalid registration without starting work. Native workflow artifact upload is attempted on success and failure; report or directory presence alone is not release success. Hard termination, storage failure recovery, complete build provenance and input closure are not certified by this evidence. The recipe contract remains host-owned under `.chrono-harness/release/`; see [docs/distribution.md](docs/distribution.md).

Release recipe v3 explicitly declares required Rust components and selects release-plan assets and consumer destinations admitted by one untracked host artifact declaration. Git-tracked paths, symlinks and source/destination overlaps are rejected. Exact release bytes are staged at these declared consumer paths before registered verification. The v4 native report preserves original v3 process evidence and records source/destination identity observations after staging, before and after each verification operation and packaging. A changed or missing identity prevents success; a failed child keeps its actual exit even when the post-check also finds a change. These bounded observations do not certify absence of transient replacement, complete build input closure, all tests invoking CLI binaries, or full native host governance. V2 keeps its previous behavior and rejects the new field.

`chrono-github-release/v1` explicitly declares a GitHub release-build projection under host `.chrono-harness/`, selected by the existing chrono-ci init/generate/verify commands. Each job declares its runner, complete command argv, timeout and literal artifact directory/name; no host language, project layout or release recipe is inferred. The generator shares output preflight and writing with check projections but uses a distinct ownership marker. Verification does not repair drift; unknown declarations and ownership collisions fail. Failed build commands retain their exit and artifact upload is attempted with `always()`. Local and CI consumers use the same declared command. Source checkout is a requested revision, not a generator-certified immutable OID; the registered build consumer owns source identity and release acceptance. Generation neither publishes a release nor certifies input closure or parity. Existing check schemas retain their semantics; the host adoption and limits are specified in [release CI](docs/release-ci.md).

Runner owns factual Git/input transport, actual entry argv/cwd, bounded process observations, candidate executable binding, protocol and named-output aggregation. Registration owns strict current schemas, affected references, fixed snapshots, readiness, supplied retained input validation and finite candidate historical decoding. Filemap owns explicit typed DELTA union, records, causal closure and required test facts. Routes owns canonical method validation, ordered plans, single tool binding and receipt comparison. Projects owns affected reciprocal pairs, explicit ownership/output isolation, actual execution, blocked dependents and required replacement execution. Optional chrono-judge-cargo owns explicitly adopted Cargo workspace/path-dependency consistency and guarded retained-package/metadata validation; the generic core has no host-language or directory semantics. These are separate production/test projects with independent manifests, lockfiles and targets.

The current FILEMAP v2 execution and workflow v2/v3 historical-profile contracts are specified in [docs/execution.md](docs/execution.md). Config supports v1 and explicit-presence v2; projects/judges remain v1. Workflow v3 selects an explicitly registered host decoder by exact endpoint schema versions, including config v1→v2 compatibility with unchanged historical input semantics. It does not supply an automatic host config writer. Strict old v1 readers reject the new fields. The shipped host historical decoder remains chrono-ci-check/v1 plus FILEMAP v1, using original fixed bytes and explicitly supplied bindings. The candidate script/tool, mappings, output digests and every old definition are retained. ci.verify moves unchanged into ci.actions.execute and ci-tests executes it. Its old pseudo-script/owner/test is retired with an explicit replacement; all six old incoming verification triggers survive, while ci-tests-only triggers now also execute verification. No exact selection or cost equivalence is claimed.

Scoped judge-ci reuses routes planning and projects execution/receipt logic through an explicit adapter. Current host sequences live only in FILEMAP; historical v1 bindings remain decoder input. Scoped legacy selection/environment limitations remain labelled until full native CI replaces that consumer. ci still owns workflow generation and event preparation, and instructions remains independent. Bootstrap installs both new binaries and uses registered locked build actions with Rust 1.95.0. All costs remain unknown.

Scoped profile `chrono-ci-check/v2` requires an explicit `policy.facts_config` path under `.chrono-harness/`, selecting the candidate full-v3 Git binding before object acquisition. That reader reads both endpoints, the original commit parents, index/dirty facts, optional full-registration interpretation and post-execution facts. Candidate profile and facts-config bytes must match their fixed blobs. Failed binding after construction and later failures retain structured original process observations in `evidence.git_facts`. Optional `registration_config` independently activates its existing registry/canonical-invocation semantics; Git binding alone does not require full registry adoption. V1 rejects the new field including null. Selection, initial rules, operation environments, protocol and canonical check argv retain their scoped contracts; base enforcement names the actual endpoint schema. The product host has not adopted v2. This does not certify Git/delegated input completeness, full governance or parity.

Context may name retained_inputs containing original external-file bytes or content-addressed blob references and environment snapshots at both fixed endpoints. The independent chrono-inputs capture/pair producer and streaming validation are specified in [docs/inputs.md](docs/inputs.md). Registration checks digests, current candidate bytes, absent/empty variables and candidate observations. Hashes and declarations do not prove undisclosed input completeness. The positive project fixture exercises declared methods without manifest/lock/root, custom action names and arbitrary file locations; optional Cargo consumers preserve workspace/path-dependency checks and run locked/offline metadata before explicitly registered Cargo operations. The adapter validates retained registry/Git inputs, exact resolution and ordered configuration arguments through the same full/scoped action path; see docs/cargo-projects.md. Neither certifies compiler/SDK or undisclosed input closure. Actual repository ci-tests Cargo execution is covered through the scoped migration consumer. This host remains proposed/incomplete and full governance is nonzero.

CI provider v3 explicitly names `facts_config`, selecting a full config v3 candidate Git binding for every event preparation read and fetch. Candidate policy bytes and checkout identity are checked before network observation. A bounded batch probe accepts only the exact requested commit or an exact missing-object response; only the latter permits fetch. Malformed output, wrong object type and execution failure fail without fetch, retaining original process evidence. Successful fetch is followed by another probe and commit identity check. Context carries `git_facts`; preparation errors retain observations without inventing a completed context. Provider v1/v2 keep their prior interpretations and reject the new field, including null. V3 may explicitly include the existing initial-inventory projection; it never infers it. This does not adopt scoped v2 in the host, certify network/credential/Git configuration closure or establish generated-v3 native host adoption. See [docs/ci.md](docs/ci.md).

CI event preparation supports explicit push_baselines prefixes and named baseline refs on origin: matching integration creation and repair pushes compare the complete candidate with the fixed observed dev tip, while unconfigured pushes retain before/after semantics (docs/ci.md). This preparation does not certify workflow obligations. Workflow now certifies bounded branch, integration, retirement and finite migration contracts using actual observations and completed reports; see [docs/workflow.md](docs/workflow.md). Complete effective Cargo/SDK and external-input closure, initial adoption/bootstrap provenance beyond the explicit root inventory profile, full native CI/parity, AI reconciliation and complete stale reconstruction orchestration, PR/merge/landing and the final full-SPEC audit remain unfinished. Single-worker local behavior checks do not establish independent review, native CI or landing. Existing instruction generation and its documented platform/crash/concurrency boundaries remain in force. Every original numbered requirement and acceptance row remains the goal.

## 15. 实际参考经验

只读参考 trureturing 提交 `fbac8071cc862c8a43a247db2d8101dc61f655ea`：

- [Meta/FILEMAP.toml](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/FILEMAP.toml)：已有 resources/materials/require 登记；本设计保留显式影响，进一步逐文件化、区分边类型。
- [Meta/engineering-projects.json](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/engineering-projects.json)：明确 production、专属测试、references/build_inputs/execution_inputs；本设计避免从源码推断测试。
- [harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/.github/scripts/harness-gate.sh) 与 [local-harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/local-harness-gate.sh)：实际入口传入 base、候选和报告；本设计进一步固定同一 argv，去除本地/CI 入口差异。
- [judge-content-address.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/workflow/judge-content-address.sh)：判官地址包含 source/runtime/architecture/sdk；本设计同样绑定执行物与工具证据，避免复用错报告。

这些是经验来源而非兼容承诺，不复制已有复杂体系，不读取其运行时配置，不把它当依赖。
上述参考材料不因引用而被本产品修改；第 2.1 节的通用数学成果通过 trureturing 独立 lane 与 PR 交付。本产品没有宿主全局 hook、权限门禁或人工流程。

## 16. 宿主指令生成（已实现）

`chrono-instructions init --host-root H` 从内嵌产品 catalog/default manifest 采用独立宿主数据，默认只生成中文根正文与 AGENTS 字面相对链接。通用方法由 98 个双语内容叶子和 16 个普通空文本 aggregate 组成，原有 17 个 core 稳定入口及 core.general 全部保留；旧主题入口只组合原职责，新增便携方法由 core.general 显式选择。聚焦 skill 只选择修复产生处及其证据/复用前提（7 个内容叶子）。无需运行时 checkout、自动语言推断、网络翻译、包解析或新平台。

`core.ownership` 显式组合投影条件与消费义务，`core.behavior` 显式组合实际 CI 事件；单独选择旧入口仍提供原职责。默认新宿主的双语短流程根及本仓完整英文指南的读数见 [迁移说明](docs/methodology-extraction.md#实际消费者边界)；这是内容消费验证，不是通用运行时预算门。

当前 schema 2 / atomic-rules/relative-alias/v3 的 output plan 显式声明输出身份、路径、格式、locale、根引用及必要元数据。不选布局时确定性 DFS 依赖先行、共享 atom 每输出仅一次。可选 catalog.layouts / output.layout 保留旧字段语义；具名双语标题与显式内容放置组织阅读，不赋予权威或执行顺序。选择时校验标题语言、深度及非空闭包恰好一次覆盖，全部预检后才写入；引用/循环/重复身份与选中闭包缺翻译报具体错误。产品默认和本仓根 manifest 显式列出 20 个现有内容叶子，并选用 workflow 的五节同级短流程。完整 general 三部分/12 主题布局和 core.general 保留供自定义 Markdown／skills；本仓 general-en 仍选择全部 98 个叶子，聚焦 skill 保持原平铺。source variant 是 inline 或宿主 .chrono-harness 下的 file，原 UTF-8 字节保留。程序不认证翻译语义等价或组合的语义完整性。

根受管块保留宿主块外原文、sole donor 和预期整份比较语义。Markdown/skill 为带身份 envelope 的整文件投影，未拥有/畸形现有文件预写入拒绝。skill frontmatter 从首字节开始，元数据显式验证。当前 manifest 是唯一管理计划，删条目/改名保留旧输出，由授权 AI 显式退休；不建立历史 ledger 或扫描删除器。

已知完整旧 read-both/v1 和 literal-core/relative-alias/v2 自动迁为 und file atom，保留旧方法/上下文的精确字节、路径、普通权限与根块外原文，不自动拆分、猜语言或采用新默认。init --locale 只绑定新根；省略选项保留采用数据，显式冲突报错。全计划预检、路径/链接/实际别名检查、父先子后建目录及多输出回滚复用现有 publisher；manifest 最后发布。当前 macOS 普通 IO 为已测边界，无跨平台、崩溃/并发保证。

完整 schema、组合配方、CLI、迁移、所有权与恢复合同由 [docs/instructions.md](docs/instructions.md) 单一维护。
[来源说明](docs/methodology-extraction.md) 和 [逐条处置表](docs/methodology-clause-map.md) 记录固定来源 918 行、12 章、102 节的义务与例外处置；它们及 [来源许可](docs/licenses/methodology-attribution.md) 是可选来源资料，不是运行时输入或政策权威。派生内容经过修改、泛化和翻译；机械覆盖不证明语义完整，独立内容审计不能由生成替代。自举由同一工具生成中文根、英文 Markdown 与聚焦 skill；产品资产/宿主采用数据/生成投影均显式登记，不是第二份手工政策。
五份完整治理登记仍 proposed、input_closure 仍 incomplete；指令专用 manifest 校验与内容图解析不代表治理判官或 AI 遵守。另行采用的 CI slice 及其实际 DELTA 范围见 §14 和 docs/ci.md。

`automatic_cleanup/v2` 以稳定 admission flock 与附件代际 SH/EX 租约，在后续普通生命周期与已采用 check/bootstrap 中恢复未调用 finish 的会话中断，包含正常返回且没有遗留 use token 的情况。EX 取得后，准确的两端登记可重建缓存以专属不可覆盖 intent、原缺失／失败输入和新真实回执重试；不创建持久自有 Git lock，不设 terminal，不删除源码、分支、未保留提交、bin/state 或证据。旧／缺失／漂移身份保留，整树 finish 保留真实交接合同。描述符由原进程 engine 的子副本与登记 Python pass_fds 明确传递，不推断通用继承。本地真实 Cargo test／原生子进程回归验证独立父子进程的租约转交，完整采用仍需已提交组合与支持平台验收；不从父进程退出、PID 缺失、年龄或空闲推断子孙终止。源与宿主政策同时修改，兼容协调者部署、旧生产者排除、已提交组合、支持平台验收、真实宿主切换和 Git/PR/发布仍归调用方。
现役本地执行扩展由 projects 唯一调度 routes 已合并的 DAG。显式有限 cap 只限操作数，不推断 CPU／语言，不限工具内部线程；前置未终结等待，失败只阻断后继，冲突 ready 项可让位于独立项。回执保留原字节与失败，结果按规范计划顺序输出，线程与已知嵌套 runner 进程由原 runner 引擎收束。FILEMAP 声明缺失／未知／歧义在执行前失败；两端 cap、claim、冲突消费者与输出登记变更进入 DELTA，collection 仅重建核原登记和证据。Unix 匿名继承描述符承载已知 launch tree，超时后有限清理不延长执行额度；不承诺跨调用锁或关闭继承描述符的未登记外部进程树。本宿主 cap 2 与独立输出／workflow inventory 共用 claim 属于宿主政策；未从 owner suites 声称完整 canonical／native 成功、交付或完整 SPEC 验收。

`execution_scheduling.priority` 可显式登记启动优先顺序，只含不重复的已登记操作 ID；空／未知 ID 或类型错误在业务执行前拒绝。就绪项先按该列表、再按未列项的规范计划顺序选择；依赖、失败传播、cap 和资源／输出排斥仍同时生效。优先级不新增选择义务、不推断耗时，也不阻止不冲突的就绪项前进。完整列表进入计划身份及留存核对；缺省或空列表保留旧序列化身份。列表重排、修改和删除按两端登记影响所有计划消费者，报告顺序仍为规范计划顺序。

Unix runner 的中断恢复由同一 process engine 的单个已加入 observer 承载：exec 前固定 child 与 launcher 的活身份，macOS 用 kernel exit event，Linux 用 pidfd；只有两者的实际终结与已登记子树完成才能协调丢失 destructor 的 slot。PID 消失、已发信号或外层 exit 0 都不是完成回执；代际固定阻止旧事件／上下文消费复用 slot。清理仍限原一秒；中断 owner 的存活子进程终止后仍保留清理失败，未知完成不成功。进程 exit 0 后的 receipt／清理失败在 CI 摘要投影为 failed、exit null，原 receipt 保留真实 exit 0、诊断与绑定；collection 保留原失败而不重执行。宿主 cap、计划、选择义务及本地／原生验收仍由原登记与调用方负责。

observer 以与执行时限相同的系统单调时钟记录实际内核退出观察。监控恢复时已过期限，仅当该观察严格早于原期限且立即取得真实子进程退出状态，才保留原退出结果；观察缺失、过晚、时钟失败或仍未退出均不授予额外等待。取消、输出上限与收束失败仍独立生效。时间戳是退出的观察上界，不是内核记录的精确退出时间；observer 自身延迟时不推断更早退出。共享布局身份独立钉版，嵌套执行须配套运行时，不兼容继承描述符直接拒绝。

exec 前身份登记失败保留 observer 的实际 kernel errno；child 经已有私有 context descriptor 以有界、核返回值的写入发布固定 stage／reason／errno 记录，pre-exec 所有错误路径不分配，只返回 raw OS error；诊断文字由 parent 在 spawn 失败后构造。诊断缺失或部分写入不替换实际 errno，无 OS errno 的 owner 失败使用 EINVAL。已有 send loop 在原一秒 handoff 内重试 EAGAIN、EINTR、ENOBUFS，耗尽后保留最后 send errno；parent 区分 acknowledgement poll timeout 与 short read。启动失败仍不制造已执行 process／exit／完成回执，不延长原一秒 handoff／清理额度或注册执行时限。Linux pidfd syscall 显式采用已有 `libc::pid_t` ABI 类型；编译修复不证明 Linux 运行行为。

已登记 v2 birth 的封口中断由原 worktree owner 在正常入口取得实际排除、核原附件与政策后恢复；保留原缺失／部分结果及独立恢复回执，重复中断重用固定回执，不据恢复制造原成功或 finish。新消费在已发布缓存尝试后开启新代际、保留旧 intent/result；后续回收可绑定当前 HEAD。bootstrap 消费原 managed_command_failed 分类，原非零构建与后续生命周期拒绝及报告引用分别保留。
