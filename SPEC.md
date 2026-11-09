# chrono-harness 规格 v0.1

状态：**DRAFT / SPEC-FIRST**。本文是待实现合同，不是已运行的验收报告。
当前已有 v1 外部判官传输、七个独立判官（registration／filemap／routes／projects／cost／mixed／workflow）、限定范围 CI 和宿主指令生成器。它们只实现了部分要求，实际范围见 §14、§16 和 docs/spec-coverage.md。
本文中的“必须”“错误”“判官”描述目标行为，除明确标注已实现者外均未执行。

为便于阅读，本文沿用几个代码术语：

| 术语 | 本文含义 |
| --- | --- |
| 宿主 | 接入并配置本工具的仓库 |
| 判官 | 按登记规则检查输入和结果的程序 |
| DELTA | 固定基线到候选提交的变化，含登记关系变化 |
| 影响闭包 | 从变化出发，沿明确登记的依赖到达的全部受影响项 |
| 输入完整 | 适用输入及其直接、间接依赖均已登记、绑定并验证 |
| full／scoped／initial | 完整治理检查／限定范围检查／无基线的首次清单检查 |
| 同判（parity） | 有效输入、政策、工具和环境相同，且求值确定时，得到相同裁决 |
| fixture | 用于测试的临时宿主、工具或输入材料 |

字段名、命令、错误码及用于定位实现的名称保留原样。源码存在、测试通过、原生运行、实际落地是不同结果，各自需要证据。

## 1. 目标、权限与边界

**当前规范优先，无需历史兼容。** 产品、宿主登记和现役消费者以当前规范为合同；
版本、格式、接口或执行路径演进不承担向后兼容义务。同步迁移现役消费者与登记，
移除仅为历史兼容保留的分支、适配器、双轨入口和测试，不为旧版本另造兼容平台。
已有章节对旧格式、旧接口或兼容测试的描述只记录相应实现范围，不再形成历史支持义务。
固定基线的 DELTA 事实、仍有用途的历史源码和原始失败证据按当前用途保留；
读取这些数据不要求重启旧程序，也不授予旧合同当前运行时权威。

Harness 要减少模块之间不必要的牵连。依赖明确、归属唯一后，理解局部改动只需看它自身的输入、接口和测试。共享定义由一个所有者维护，独立单元分别构建、检查和重试。

Harness 的本质：每项受治理操作只提供一条当前已登记的规范路径，并用可执行判官检查其结果。定制在执行前通过登记选定这条路径；日常指令只呈现当前路径，不并列提供替代配方。AI 先把路径、归属、依赖、成本、操作及判定方式登记为白名单，再通过该方式处理任务。改进方法先更新登记并完成适用验证，仍保留一个正式入口；AI 自由实现产品逻辑并自主更新判官。默认指令按需求→审计→重构→开发组织；验证实际消费后再核 SPEC。

全局考虑是先核需求、现有能力、所有权、显式依赖影响及验证／CI／存储成本，再收窄实现，不是每次全仓审计或全测。现有能力足够就复用；审计证实归属或耦合问题才先在负责该内容的位置重构，开发只补真实消费者尚未覆盖的需要。缺少登记、重复入口、执行结果与登记不符必须有明确错误，不能靠“智能推断”补全。系统信任 AI 非恶意，处理的是遗漏、漂移、错误和演进成本；AI 自主登记、修复和推进。AI 在现有授权与权限内构建、测试、更新约束和重建分支，不新增或申请额外权限，也不增加人工审批、签字或确认步骤。

局部依赖失败保留真实失败及恢复条件，只阻断其显式依赖闭包，独立已授权工作继续；必需验收不旁路、不伪报通过。无必要真实消费者却依赖特殊权限的功能应删除，不增加替代远端激活机制；普通必要流程沿用已有权限范围。性能以最小充分的消费者、IO、进程和按 DELTA 影响比例的验证为依据，成本未知保持未知，不抬限额或删除必要行为测试。

功能必须有真实消费者。新增功能、抽象、扩展点及结构优化先明确现存问题、具体调用者、实际入口、所需输入、产出被谁使用以及可观察验收，再实施最小充分改动；不能假想未来场景、消费者或潜在扩展需求来建立功能。消费者可在同仓，也可在明确的外部仓库或真实用户操作中；同一交付新增的消费者必须解决已存在的问题并实际接入，不能为证明新功能有用途而制造空调用。

消费者、生产者及其依赖由宿主在 `.chrono-harness/` 显式登记，记录“现存需求／问题 → 生产功能 → 具体调用 → 输出用途”的因果关系及相应证据地址。仓库外消费者须有真实身份／仓库地址、固定版本或提交、调用位置与入口、所消费的接口／产物和实际接入证据；不因位于仓库外而豁免，也不能把发布包、README 使用示例、可选参数或测试 fixture 本身当作实际消费。专属测试验证功能，不为该功能凭空提供用途；服务现有测试流程的工具则以那个真实流程为消费者。证据不可取得或未接入时据实保持未验证，不伪造消费者或实现臆想功能。

受影响消费者的缺失登记、悬空引用、身份／调用不符及缺必需证据由登记判官沿同一 check 检查，需求忠实性与使用价值由 AI 对具体事实负责，不宣称图可证明用途。内部调用沿现有 FILEMAP／项目／操作的唯一登记源，外部消费沿显式版本化扩展登记，不另造自动发现或重复账本。仅为测试或将来消费构造的功能不得作为独立交付；必需中间实现与真实调用一起验收。既有未受影响功能不因本次新增合同全仓判红；确认无实际用途的代码由 AI 核对真实消费者后按第 6 节退役，未知使用情况不冒充无人使用。

### 1.1 必要性与“为什么”链

对每个受跟踪或受执行的对象，AI 必须连续追问“谁消费它、消费是为了解决什么、如果移除会使哪个当前结果失败、该结果又由谁实际使用”，直到链条落到当前不可替代的终点。终点只能是已接入的用户行为、生产／发布／外部接口义务，或当前 SPEC 由真实需求绑定的可观察正确性、完整性、安全性和恢复结果；已有明确需求、具体使用方和可观察验收的在研功能也有当前目标，接入和验收状态须如实保留。不能以本条自身、对象自身存在、登记引用或“检查需要检查它”自证。没有需求绑定的“以后可能用”“方便审计”“保持一致”“已有历史”“CI 里有信息”或“以后再接入”都不是终点。

必要性链的每一跳须明确需求／问题、生产者、消费者、输入、输出用途、替代方案和验收证据，复用现有登记、自然所有者说明及实际调用证据；FILEMAP 的依赖边、项目配对、测试、缓存和报告引用只证明关系，不自动证明终点必要。共享依赖、通用抽象和宿主 policy 沿所有实际消费者分别展开，任一分支没有当前终点就不能因另一分支存在而保留。专属测试可以验证真实行为，但测试本身不能成为没有产品或外部消费者的唯一理由。

必要性链不要求穷举不可见世界；到达一个有固定身份、可观察验收的已接入终点或明确在研目标即可停止。消费者身份、版本、入口或证据不可取得时状态为 `unknown`，不能填“合理”或转为 active。链条缺终点时由 AI 保留未解决结论；对象不得实现、登记为 active、加入构建／CI／缓存／报告保留或成为新增依赖。已有对象先补证或迁移；`unknown` 不构成退役证据，保留未完成工作和真实失败事实，不为“可能将来需要”继续扩张。

退役前核对原始需求、当前 SPEC、引入 PR 和在研／接入工作。本宿主未调用、尚未公开分发、仅有专属测试或接入证据缺失，单独都不能证明功能没有需求。有证据确认无当前需求（含需求已取消或过期），或替代方案覆盖原接口与行为合同、现役使用方已迁移并验收，再按第 6 节退役。不能先删改 SPEC 目标，再以改后的 SPEC 为删除作证；确需调整需求时须说明独立依据。保留有需求的目标，按证据简化其中的重复实现；在研身份本身也不证明每个实现细节必要。

必要性由 AI 对当前事实作判断；登记判官继续检查其现有合同中的引用、调用、输入和原件，不把字段齐全或关系闭合解释为价值已证明。必要性审查同样受本节约束：不能仅为执行本节另建对象数据库、逐跳 JSON、递归价值判定器或独立准入门。只有当前消费仍需要的说明才保留在负责该内容的位置；线上 CI 已提供的运行事实直接查原运行，历史审计和决策归 Git／PR，不复制成长期维护的快照。

需求、消费者或终点发生变化时重新核链；不能用一次过去的采用、一次 CI 绿色或一个仍被引用的旧报告永久证明必要。AI 可在授权内补证、迁移、简化或删除，不新增人工审批门；无法取得证据时只阻断依赖该链的工作，独立工作继续并保留 unknown。

可执行的项目准入约束必须实现为登记判官，不能隐藏在 runner、CI YAML、hook 或根指南中。
根指南直接呈现一般工作方法；它不是可执行判官，也不因严格措辞变成已运行的机器门。
runner 的固定职责仅为命令/协议解析、输入传输、启动指定程序、限时限量与结果聚合。
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
  diagnostics/                  标准日志和显式错误链，独立于 runner
  diagnostics-tests/            诊断库专属测试
  runner/                       运输、协议与统一 check 入口
  runner-tests/                 runner 专属测试
  identifiers/                  runner 等使用方共用的身份判断库
  identifiers-tests/            身份判断库专属测试
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
examples/ci-host-job-gating/    当前可复制的宿主采用实例
skills/                         本仓生成并采用的 skill
AGENTS.md -> CLAUDE.md           根指南的相对链接与受管正文
.chrono-harness/                本仓采用的独立宿主数据
  instructions/                 宿主 catalog、上下文与输出计划
  projects.json / FILEMAP.json   项目、操作、文件、依赖与成本登记
  config.json / judges.json     proposed 的完整治理配置与判官白名单
  workflow.json                 proposed 的分支、迁移与混合修改策略
  ci/                           现役 CI 配置及自举绑定
  bin/                          已安装指定二进制（不跟踪）
  state/                        当前状态、输入与有界留存的报告／证据（不跟踪，见 §10.1）
.github/workflows/              CI 配置的生成投影
```

`crates/` 仅组织目录，不承担项目发现或选测。每个子项目有自己的 Cargo.toml、Cargo.lock 和 target。
没有根 Cargo workspace；不需要一起编译的项目不放进同一个编译单元。
生产 crate 同时导出库与极薄二进制，因为二者共享同一 CLI 行为；不另拆无价值项目。
测试 crate 通过显式 path dev-dependency 引用生产库，有独立 lockfile 和 target 目录。
产品 crate 不定义单元测试或 doctest；所有当前行为测试归唯一配对测试项目。
以后新增独立生产项目必须同时登记一个专属测试项目；不能以共享巨型测试项目替代。

宿主的所有生效 harness 约束、脚本配置、安装绑定和指令 canonical 材料都在宿主 `.chrono-harness/` 内。根 CLAUDE.md 的受管块直接呈现所选方法，默认是短流程；AGENTS.md 是指向 CLAUDE.md 的字面相对符号链接；块外原有文字属于宿主；方法在 canonical 源编辑后生成，不独立维护副本。宿主可以使用其他仓库构建的二进制，但须在此显式绑定路径、版本与摘要。

自举时 `crates/runner/src/*` 仍然是 **product**，不能因产品是判官引擎便改称宿主 policy。其稳定性需求由 workflow 的 `stability` 路径登记表达，不通过重新归类源码表达。产品构建不会自动启用宿主规则、安装 hook、修改全局配置或访问参考仓库。

### 自身实现语言与分批重构

harness 自身的产品逻辑和工程控制逻辑使用 Rust；首次取得或构建原生入口时，可以使用薄 sh。shell 只负责固定参数、平台选择、下载校验、临时目录和启动原生工具；JSON 解释、依赖规划、状态维护、回执判断、重试及生命周期逻辑由 Rust 的实际所有者维护。宿主政策和本仓构建、发布配方继续归宿主登记，不写死在通用产品中。

普通宿主采用 harness 不应因此额外安装 Python。真实 Python 业务、外部 Python 接口和解释器自身配置探测，可以按实际使用方保留；不能仅为保留已迁移的 harness 实现而继续要求解释器。实现迁移时，同步迁移现役使用方、专属行为测试、工具绑定和输入登记，退役失去使用方的桥接、兼容路径及测试。

该重构按可独立验收的范围分批提交 PR 至 dev。每批保留原成功、失败、边界和证据消费要求，核对实际落地与适用后验，再开展依赖后续批次。shell 安装器、artifact 下载复用、原生 CI 适配和 Rust 测试库存保持当前职责；本仓启动与共享产物消费统一登记到 Rust 前端，仍须分别验证真实原生生产和消费。发布编排、通用测试样本的语言迁移及无使用方的桥接／探测清理，均按实际使用方和交付证据验收；bootstrap 的 Python 实现、伴生测试和导入专属断言随旧入口退役，现役 Rust 行为测试继续独立验收。运行、构建及迁移总成本未测，不预先宣称性能收益；阶段完成不表示全部迁移完成。

### 2.1 形式化真源与实现对应

chrono-harness 所需的通用形式化定义与证明统一归入 trureturing，按数学主题组织并优先复用已有结果；不建立独立 chrono-harness-trureturing 库，不为本产品复制证明或新增仅包装已有定理的数学声明。
chrono-harness 保留 Rust 实现、登记格式、实际操作合同、回归用例，以及 SPEC 子句到形式模型的对应说明。两库独立构建；宿主使用 harness 二进制不因此需要 Lean、trureturing checkout 或特定宿主语言／目录结构。

形式化工作 使用 trureturing 的独立 worktree 与 PR 交付，其数学准入、编译及验证遵循该库现役规范。对应说明须固定所引用修订和声明，列明对象、参数、假设、结论与实际消费者；已找到、已编译、已证明模型命题、已检验实现对应是不同状态。未完成部分如实保留，不以引用或 CI 绿色冒领全部形式化。

形式模型与反例可以推动 SPEC 演进：发现缺失前提、错误推论或实现不符时，分别修订适用的模型、产品合同或实现，并验证受影响义务。不得为匹配当前实现而静默缩小用户目标或改写旧结论的适用范围。Lean 核验只证明其形式陈述；Rust 实现符合模型仍需单独的对应证据，完整依赖登记、局部性与确定性等现实前提也不能由图闭包或一次通过自动取得。

### 2.1.1 Verus 规范性权威与实际调用

所有 SPEC、仓内决策和状态逻辑必须使用同一份 Verus 源码定义并完成证明；只允许明确列出的外部接口假设。新增规格、判断、状态维护及其独立边界预期必须直接使用 Verus，并在源码所有者的执行计划登记整 crate 证明，不能先增加未证明逻辑再记作迁移缺口。受影响证明与实际消费者的行为测试须由同次 DELTA 检查共同验收，汇总必须消费这些所有者的原结果；不要求每个消费 job 重复同一证明。源码、证明根、工具和参数的变化须通过显式依赖选中相应证明；消费者仍直接使用同一生产源码并保留自身测试。既有缺口和 Python 相关迁移按批推进；未完成迁移不等于已证明或免除最终义务。

`chrono-harness check` 及其本地／CI 结果使用方是当前真实消费者：通过、拒绝或未知，必须能追溯到已证明的规范谓词和真实生产调用。去掉证明后，普通测试仍能运行，但无法提供本节要求的保证。这一要求覆盖字段约束、可机器检查的登记关系、DELTA／影响选择、路由与项目准入、错误分类、CI 汇总、缓存／报告／工作树状态、生成器、bootstrap 及相应测试预期。

外部 IO、进程、Git、CI provider、JSON 编解码和工具链接口可以由适配器承载；接口假设须逐项登记。适配器必须把实际输入交给已证明的函数并保留其结果；适配器自身的准入、错误分类或状态判断仍属待证明逻辑，不能以实现语言或“外部脚本”为由豁免。OS、Git、Rust／Cargo、Verus／Z3 的接口假设不授予仓内函数 `external_body` 例外。

沿用宿主现役工具、项目操作与 FILEMAP 登记，明确固定 Verus 版本／commit、编译器和宏版本、证明根、参数、输入、外部接口与实际使用方。每个受治理的 program／specification 文件须有非空证明根、唯一登记的 Verus invocation 和真实生产调用。默认使用 `--no-cheating`、`--no-external-by-default` 和整 crate 检查；源码根、候选摘要、编译结果、原证明报告及真实调用必须互相绑定。不得另建重复权威来源。

FILEMAP 的文件可显式声明 `verification`：`role` 为 `program`、`specification` 或 `data`，并列出证明根的相对路径及登记操作。program／specification 必须至少有一个规范化根，根路径不能重复，且每个操作必须出现在现役 execution plan；data 不得列证明根。该字段由 runner 的 Verus 同源谓词解析和判定，登记层只转发结果。缺少声明的存量文件保持未知，不取得证明资格；新增或修改的受治理逻辑必须在同一 DELTA 登记声明并由真实生产调用消费。

全仓覆盖检查（coverage gate）的目标失败码为 `E_VERUS_COVERAGE`：证明缺失、未执行或与实际生产调用不符时，保持 `proposed`／`incomplete`，不能退回未证明判断后宣称已通过本节验收。只改语言声明、只编译证明根、普通测试或一次 CI 通过，都不能建立完整对应。当前已有局部证明和结果合同，全仓覆盖门仍未实现；普通 scoped 检查只证明现役登记的实际范围，不冒充全仓证明。当前实施范围见本文末尾及 [Verus 实施范围](docs/verus.md)。

### 2.2 实现与测试使用同一种语言

用什么语言实现，就用什么语言测试。生产单元的行为测试、断言、测试驱动及承载测试逻辑的子进程 helper 必须使用该单元的实现语言：Rust 用 Rust 测，Go 用 Go 测，TypeScript 用 TypeScript 测，Python 用 Python 测。Shell 脚本（包括 sh、bash）允许用 Python 测试。

统一的是 `chrono-harness check` 治理入口；底层执行器按登记调用各单元的测试工具。本仓 Rust 测试由 `cargo test` 执行，Python 测试由 Python 执行，不通过 Cargo 或 Rust 薄包装转发。被测程序的构建独立登记；测试中的外部依赖接口由 mock 提供，不为这些 mock 安装外部依赖的编译工具链。

不能把另一种语言的测试逻辑嵌入源码字符串、here-document 或解释器参数，也不能通过同语言薄包装调用另一种语言的测试来满足本条。Rust 的进程、文件描述符、资源上限等行为应由 Rust 测试及 Rust 子进程 helper 验证，不因编写方便改用 Python 或 Shell 承担测试逻辑。

测试涉及具体外部可执行文件时，一律在命令执行边界使用 mock，核对参数、输入输出、退出状态及必要的失败／超时行为，跨语言集成和真实消费者测试也不得例外执行外部依赖。测试驱动与断言仍遵守被测生产单元的语言要求。真实构建、生成、检查和交付仍由登记的生产入口执行；mock 通过不证明外部程序的实际行为。用于验证脚本／插件协议的外部 fixture 应独立成文件，显式登记所有者、输入、依赖、成本和消费操作；它只能提供所需接口行为，不能藏入替代同语言测试的驱动或断言。各生产单元仍各自对应一个专属测试项目，独立脚本及其测试可以单独登记。

混合语言宿主按生产单元分别配对。实现语言、测试语言及 Shell／Python 配对由宿主显式登记，配对判官核对这些声明与登记的测试入口，不按目录、扩展名、命令名或代码扫描推断语言及依赖。未登记语言、配对不符或观察到的实际执行不符须报错；声明核验不证明不存在隐藏的跨语言测试逻辑。此合同不要求宿主采用 Rust、特定目录结构或特定测试框架。

projects schema v2 要求每个 project／script 显式登记非空 `language`；projects 判官只核 DELTA 影响的专属配对，要求语言 ID 相同，唯一额外准入为生产 `shell`、测试 `python`。这两个保留 ID 由宿主显式选择，sh／bash 可登记为 `shell`；其它 ID 是不作推断、不归一化的字面值。schema v1 保留旧合同，不因升级二进制自动取得语言约束。

配对检查验证登记，不从文件正文证明实现、测试驱动与 helper 的实际语言，也不证明不存在薄包装。实际操作身份仍由登记路由与原始执行回执核对。本仓采用 v2 声明；scoped CI 与 projects 判官共用配对和语言规则，scoped v1 登记保留旧解释。显式测试清单由 Rust `chrono-ci test-inventory` 读取宿主 `chrono-workflow-inventory/v1` 配置，核对登记的 Cargo/libtest 未过滤、ignored 与分组列表：未过滤清单及各分组须非空，各组互不相交，合计精确覆盖未过滤清单，ignored 清单须为空；不据此推导依赖或选测；行为验证归 `ci-tests`。宿主 bootstrap 的 Rust 生产／专属测试项目及薄 shell 入口分别登记，由原生启动 CI 单元执行；Python 启动栈及其实现专属测试已从现役登记退役。示例的 v2 采用及 helper／外部 fixture 的完整语言登记仍未完成，不宣称本条全面启用。

### 2.3 仓库源数据、在线事实与主机绑定边界

`.chrono-harness/` 中的 JSON 只是格式，不因是 JSON 或位于宿主目录就获得入库理由。每个受跟踪文件必须明确属于以下一种表面，并在 FILEMAP／对应登记中有自然所有者、真实消费者、用途、有限存储预算和 §1.1 的必要性链终点：

* **可移植合同**：产品源码、锁文件、`rust-toolchain` 或等价工具链选择、CI 配方、镜像／OS-架构的预期身份、命令、输入 schema 和确实被判官或构建消费者读取的最小政策。它描述意图和准入边界，不冒充某次运行的事实。
* **确定性投影**：由已跟踪源通过登记生成器产生、能够在 CI 重新生成并由实际消费者校验的文件，例如工作流投影。投影必须有源、生成器、校验入口和有界大小；生成成功本身不证明投影被采用。
* **运行时事实／证据**：CI run/job/attempt 标识、时间、runner 主机名、checkout 或工具的绝对路径、`PATH`／`HOME`／`CARGO_HOME` 内容、已安装包或 SDK 清单、`cargo metadata` 输出、原始日志、报告、二进制、缓存和临时文件。它们必须写入被忽略的 `state` 或 CI artifact，并由生产者按 §10.1 清理，禁止提交 Git。

运行时事实无论文件大小、是否失败、是否“历史”或是否被另一个报告引用，都不能变成 Git policy source。CI 在线观察是本次运行事实的权威；仓库只能记录供 CI 复现的可移植预期，不能把一次在线结果手工复制成永久主机快照。报告需要跨 job 运输时保留为有界 artifact，并记录 owner、消费者、保留期限／容量和释放条件；期限届满后只留仍有消费者需要的摘要或处置回执。

可移植合同不得含个人 checkout、临时目录或某一 runner 的绝对路径。外部绝对路径只可作为未跟踪宿主绑定或运行时输入，必须由本轮实际观察提供；跟踪政策应使用仓库相对路径、稳定镜像／工具链身份或具名 `input` 绑定。AI 审计时将这类路径迁出跟踪政策，不以改写成另一条本机路径消除问题；已有路径、输入和引用检查继续执行其明确合同。

“固定主机／构建环境”只有在存在真实消费者且能改善可观察结果时才允许：登记最小的 OS／架构、镜像 digest、工具链版本、锁文件和命令即可；完整安装清单、SDK 目录、个人环境变量和 Cargo home 内容属于运行时输入，不能整体入库。每个额外绑定须说明消费者、失败时的裁决、owner、更新方式和退役条件。没有真实消费者、只供解释或只复制 CI 信息的 host policy，由 AI 按本节与 §1.1 审计，在同一迁移中删除其引用、FILEMAP 边和专属检查；未知用途先保留并报告，不能凭目录名或年龄批量删除。

跟踪的源政策须有有限的记录／字节预算和增长原因；超过预算时先消除重复事实、拆出运行时证据或迁移到更小的可移植合同。压缩不能替代删除无消费者的事实，也不能以“大文件”本身推断必要源政策可删。任何无法证明来源、消费者、预算和生命周期的跟踪 JSON 均不得转为 active；现存历史文件在迁移完成前只能保持 proposed/incomplete，并列入 §14 的未完成边界。

审计或迁移按以下顺序处理既有 host JSON：确认实际生产者和消费者；保留最小可移植合同；把本机／CI 观察移到 `state` 或在线 artifact；更新 FILEMAP、projects、routes 和测试边；再通过现役入口验证并删除已无消费者的文件。只删大文件而留下悬空引用不构成清理，也不能把 CI 绿色状态当作仓库政策的替代来源。

## 3. 登记格式与字段合同

Config 接受明确的 `schema_version: 1 | 2 | 3 | 4`；projects 接受 1／2，judges 使用 1，当前 FILEMAP／workflow 使用 2。Config v2／v3／v4 要求每个外部输入声明存在或缺失，v1 保留仅摘要的原含义。每份登记都有 `status: "proposed" | "active"`。候选读取器只按明确的解释约定读取固定历史 v1 数据。

当前五份均为 proposed；`config.enforcement` 为 `not-implemented`。启用前由 AI 对宿主定义的治理／输入范围作负责的 declared-complete 工程声明，补齐该范围的实际输入、依赖、绑定、两端快照、二进制 SHA-256 和全部必需判官，再改为 active/enabled。判官核验声明引用、身份、快照和观察到的操作；普通 full check 不要求证明所有隐藏输入不存在，也不要求 VM 或 OS 隔离。

已知缺输入、未解决的必需依赖、缺绑定／快照及漂移仍须失败，不能以声明或手写成功清除。null 摘要只允许 draft；不能被解释为“任意二进制都已通过验证”。类型表按各登记的现役 schema 描述；registration 严格验证结构。明确支持的历史 scoped 遗留项先由候选解释器转换并保留原始事实（见 §14），不作为第二套当前登记。对象未列出的字段、重复 JSON key、重复 ID、悬空引用由 registration 判官报错。

路径是仓库根相对 UTF-8 POSIX 路径，不含 `..` 或隐式 glob；environment.inputs.location 可显式指定外部绝对路径。所有文件逐个登记；目录条目只用于明确生成物，不允许替代源文件登记。

| config 字段 | 类型与约束 |
| --- | --- |
| runner | `{path, version, sha256}`；path 为指定入口，sha256 为 64 位小写 hex 或 draft null |
| registries | judges/projects/filemap/workflow 四个唯一 JSON 路径 |
| canonical_check | v1–v3 `{operation, argv: string[]}`；v4 增加 `profile`、可选独立 `initial_profile` 和 `inputs.local/ci` actions `{operation,tool,argv}`；唯一 `validate.delta` 路由 |
| tools | `{id, program, resolution, version_argv, expected_version}[]`；版本为字符串，draft 可为 null |
| facts_git | config v3/v4 必填 `{tool, input, guard?}`，前两项分别引用唯一 tools 与 present 文件输入；可选 guard 见下；v1/v2 不接纳此字段，包括 null |
| environment | `{inherit: string[], values: object, inputs: object[]}`；v4 可加 `credential_environment: string[]`，声明仅用于输入取得、不转发给 Git／判官的 inherit 凭据；v1 输入 `{id, location, sha256}`，v2/v3/v4 输入为 `{id, location, presence: "present", sha256}`、`{id, location, presence: "absent"}` 或下述运行时来源；变量及输入文件白名单 |
| input_closure | `{status: "incomplete"\|"declared-complete", unresolved: string[], bindings?: {id, consumer, kind, inputs: string[]}[], coverage?: object}`；`bindings` 仅为 config v3 的显式白名单，工程声明不是完备性证明 |
| protocol | `{id, timeout_seconds, stdout_limit_bytes, encoding}`；正整数限制、UTF-8 |
| semantic_fields | `{path, pointers: string[], on}[]`；语义字段分类，见 §8 |
| artifacts | `{path, owner, kind, tracked: false}[]`；明确生成目录白名单 |
| enforcement | `not-implemented` 或 `enabled`；proposed 不产生通过报告 |

tools 的 `resolution` v1 仅有 `PATH-once`：只解析登记的 program 一次，报告绝对路径、文件摘要和版本输出；不搜索其他同类工具作为替代，不自动纠正版本不匹配。Config v3 在读取 Git 对象前，从显式 candidate 配置绑定 `facts_git`；该配置的原始字节随后必须与固定 candidate 中的 blob 相同。

tool 的 program 按已声明 PATH 或显式路径解析，input.location 必须指向同一执行物，present 摘要先于任何启动核验，expected_version 必须为非空字符串并匹配 version_argv 的实际 UTF-8 stdout（只去末尾空白）。相同 candidate 绑定读取 base 与 candidate，不启动 base 工具。

每次进程使用清空后重建的登记环境及 protocol 的时间／输出上限；显式 GIT_DIR、GIT_WORK_TREE、GIT_INDEX_FILE 因重定向 checkout 而拒绝。进程前后核配置、绑定路径与执行物摘要，保留原始输出字节、摘要、退出和边界失败。完整请求及消费者报告通过 `git_facts` 传递观察结果；下游核对绑定和环境，缺失或不符不得回退 环境中的未绑定 Git。执行前的失败以 `E_GIT_FACTS` 携观察返回；尚未启动的进程不伪造回执。

完整调用、initial inventory 和输入 capture 共用该读取器；initial 仍不裁决 DELTA。Config v1/v2、旧 library facts API、scoped CI 与旧版事件准备保持各自旧合同。详见 [Git 事实绑定](docs/git-facts.md)。此绑定不证明 Git 配置、解释器、动态库、OS、委托工具及其它实际输入已完整登记，也不证明无并发瞬时变化或本地/CI 同判。

独立 `facts_config`、完整 `--config`、initial `Profile.host_config`、输入 capture、
scoped 的可选 `registration_config` 及 full-CI `check_config` 均可显式指向
`chrono-git-configs/v1`。其唯一数据字段 `platforms` 逐项登记执行二进制的 Rust
`OS-ARCH` 到完整 v3 Git 配置路径的映射。无匹配平台即拒绝，不探测可用工具、不推断
语言或目录；各平台文件仍由宿主明确登记。选择器及目标均为 `.chrono-harness/` 下的
字面相对路径，拒绝链接、自引用、嵌套选择器和旧版目标。

固定端点读取器独立按各自 commit 解析 entry 与 selected target，并在 registry snapshot中按真实路径保留 selector、target 和五份 status-bearing registry 的原字节和值；不把target JSON 别名到 selector，也不把 candidate target 套到 base。

请求、registrationview（选择器模式版本化为 `chrono-registration-view/v2`）、历史 decode（选择器输入版本化为 `chrono-historical-decode/v3`）、FILEMAP provenance、mixed documents 与 workflow checks 同时保留原始 entry identity、effective target identity 和结构化 selector binding，缺失、替换或漂移均拒绝。

Git facts 仍按原合同绑定 candidate，历史 endpoint 只读固定对象，不执行历史工具。选中输入快照使用版本化 `chrono-input-snapshot/v3` 绑定 entry、effectivetarget 与 selector；direct v1/v2/v3 和旧快照语义保持不变。

本地与 CI 使用同一 唯一登记入口 argv，routes 仍登记稳定 entry；直接目标 argv
替换被拒绝。不同平台选中不同政策，实际生效输入就不同，不能据此声称同判。完整宿主启用、可搬运的 initial 判官绑定、完整有效输入、原生 full 派发、Rust 实现的形式化对应及完整 SPEC 仍未完成。旧直接配置保留原义。

Config v3 可在 facts_git 中采用 `guard: {schema: "chrono-git-inputs/v1", inputs: string[]}`。非空 inputs 逐项引用唯一 environment.inputs 文件 ID，拒绝重复、未知引用和未绑定摘要；present 必须匹配 SHA-256，absent 必须实际缺失，目录、符号链接、IO 错误不作缺失。

读取器在第一次版本探测前及每次 Git 调用前后检查这些宿主指定输入；失败不得继续读取对象或 fetch，已运行的子进程保留原始输出与退出。下游请求须绑定相同的初始核验结果。Git 配置、include、解释器或库是否完整仍由宿主显式登记；不扫描或添加输入，不自动解析worktree 元数据位置。省略 guard 保持旧 v3 语义与报告形状，历史配置不补写。

具名 inputs 观察使用 chrono-git-inputs-result/v1，保留实际文件身份／缺失与completeness_proven=false；前后观察不证明瞬时并发隔离、完整输入或确定性同判。

当前宿主显式登记 Cargo、harness 工具和各自实际使用的工具版本合同；尚未具备 Cargo／SDK 输入及其依赖的完整性。换工具链由 AI 显式更新登记，声明版本不代替实际版本观察。`argv` 是原始参数数组；只对注册的整参数占位符 `{base}`、`{candidate}` 做替换。

没有 shell 插值、模板语言或依据当前目录猜测 manifest 的行为。只继承 environment.inherit 逐个列出的变量，缺值记录为 absent，values 显式覆盖。inputs 为逐文件登记的有效输入，ID 唯一。v1 及 v2 present 的摘要 null 仍仅表示未绑定草稿；不得解释为缺失。v2 absent 禁止携带摘要；缺失不同于空文件。v2 快照保留实际 `{absent:true}` 或文件内容身份，判官核对登记与两端快照，并在操作前后核候选路径。

目录、符号链接、权限及 IO 错误不作缺失；路径祖先也不得是符号链接。历史缺失只从保留快照读取，不重读今天的路径。报告绑定实际值，不自动读取 dotenv。

Config v2 可用于两端均采用该合同的宿主，也可经 workflow v3 显式版本选择的宿主 decoder 从 v1 迁移。v1→v2 仍须 §6 的实际转换及兼容测试；支持读取 v2 本身不证明已有宿主升级成功。文件状态变化通过 input 节点的显式边传播；没有边不推断消费者。捕获成功不是治理成功，缺失检查不是输入及其依赖的完整性或并发隔离证明。外部输入须保留两端快照，变化通过登记的存在状态或摘要进入提交 DELTA；磁盘值不符或快照缺失报输入不足，不隐式扩展 DELTA。

PATH 用于一次解析具名工具，HOME/CARGO_HOME/RUSTUP_HOME 用于已声明的工具链运行；完整输入范围包括实际 cargo/rustc、编译器后端、链接器、SDK、目标库和所读配置文件，包括 HOME/CARGO_HOME 下配置、rust-toolchain、间接依赖、fixture、外部数据及环境变量。会影响裁决的随机种子、时钟值、网络响应和超时/资源条件也须显式固定并登记为有效输入。

文件输入用 FILEMAP 或 environment.inputs 登记；后者以 input:<id> 显式连接消费者，不推断边。宿主实际采用状态与缺口归 docs/spec-coverage.md 及现役输入登记；不能从已有输入数量推出完整闭包。补齐宿主声明范围内的工具、输入、关系与证据后才可声明 declared-complete，unresolved 必须为空。declared-complete 是可信 AI 对该范围承担责任的工程声明；真实工具链、SDK、配置及其它已知适用输入义务仍保留。

判官不自动发现隐含依赖，声明通过不证明普遍输入完备。Config v3 可额外登记 `input_closure.bindings`：每项必须有唯一 `id`、可执行消费者`project:<id>`、`script:<id>`、`test:<id>` 或 `judge:<id>`、非空 `kind` 以及非空`inputs`。

`inputs` 只能逐项引用已登记的 `tool:<id>`、`input:<id>` 或`environment:<name>` 节点；未知节点、错误种类或悬空引用由 registration 报`E_REFERENCE`。v1/v2 拒绝该字段。

bindings 只记录 AI 明确选择的关系，registration不扫描目录、语言、命令、manifest 或工具链来补齐依赖，也不把登记成功解释为现实中没有遗漏输入；声明范围的登记就绪由 `status`、`unresolved` 和各消费者的实际证据共同决定。

Config v3 可显式采用版本化 `input_closure.coverage`，合同为`{schema: "chrono-input-coverage/v1", required: [{consumer, domains: string[]}], rows: [{consumer, domain, state, bindings: string[], reason}]}`。`required` 非空，逐个列出可执行消费者及非空的宿主自定义输入域；rows 必须恰好覆盖这些消费者／域组合一次。

缺行、多行、未知域、重复域、错误消费者、未知 binding 和空 reason 均报错。`bound` 和 `absent` 要求非空 bindings，每个 binding 同属该消费者且有非空 inputs；一个 binding 可以关联多个域。`absent` 只接受显式缺失文件输入，实际存在状态仍由两端快照及操作前后输入核验裁决。`not-applicable` 与 `unresolved`不携带 bindings，前者是 AI 的适用性声明，后者明确缺证。

采用 coverage 且声明`declared-complete` 时，每个 binding 至少被一行覆盖且不能有 unresolved 域。域名由宿主登记，例如 compiler-library、SDK、network、clock 或 randomness；判官不猜项目语言、工具链或读取行为来补域。省略 coverage 保持既有 v3 含义；v1/v2拒绝此扩展，旧二进制拒绝未知扩展。候选判官读取未采用扩展的历史快照，无需重写历史。

通过后产生 `chrono-input-coverage-result/v1`，保留完整声明与 `scope: declared-domains`、`completeness_proven: false`；它证明登记的对应关系和已核验输入状态，不证明没有未登记域或任意程序的读取闭包。完整有效输入、并发一致性和本地／CI 同判仍需实际消费者证据。未知或已知缺失项返回 E_INPUT_UNDECLARED / E_EVIDENCE_UNRESOLVED；登记有效不证明真实输入完整。

| projects 字段 | 类型与约束 |
| --- | --- |
| owners | 唯一 owner ID 数组，包括无编译项目的 repository owner |
| projects | `{id, kind, actions, test_project?, tests_for?, manifest?, lockfile?, root?}[]`；三个 legacy 路径均可省略 |
| kind | `production` 或 `test`；production 必有 test_project，test 必有 tests_for |
| actions | 非空的显式具名方法对象；方法名不限定语言或活动；每项 `{operation, tool, argv}`。测试的 `execute` 是其执行入口合同 |
| scripts | `{id, path, test_script, actions}[]` 或 `{id, path, tests_for, actions}[]` |

每个 production 与一个 test 双向一一对应，test 不再要求递归配一个 test。脚本以同样的一对一关系单独登记，不为它创建虚假的编译项目。operation 在全部 actions 与 canonical_check 中唯一；同名不同参数也属于重复方法。宿主可以没有 manifest、lockfile、统一 source root 或编译输出目录。可选 legacy manifest/lockfile 只是显式归属的 opaque 文件引用；root 不授予内容或输出推断权。

通用 projects 判官检查一对一配对、显式测试边、实际文件归属与声明产物目录的隔离，不解析语言文件，也不根据路径猜 `target/`。产物登记变更与删除按两端显式 owner 唤醒对应 project/script，再沿 FILEMAP 边选择测试。构建、测试依赖只由 FILEMAP 提供。宿主采用的语言工具须消费显式政策与既有边，不能补登记或选测试。

脚本、fixture、环境文件和生成器输入都必须逐个登记依赖，不能按目录名猜测试集。

生成器自举 init/generate 通过已登记 cargo 工具的 `run --locked --manifest-path ... -- ...` 调用，argv 显式指定本仓宿主根 `.`，init 不传材料路径以保留已有定制；首次采用内嵌精选核心与空上下文。不新增工具发现或路径模板。任意外部宿主直接用 §16 的 CLI。

| FILEMAP 字段 | 类型与约束 |
| --- | --- |
| cost_models | 成本 ID → `{cpu_ms, wall_ms, peak_rss_bytes, io_bytes, basis}` |
| files | `{path, owner, surface, cost, edges: Edge[], symlink?, projection?}[]`；每路径恰好一次 |
| symlink | 可选字面相对目标字符串；登记实际链接身份，不能按解析后相等放宽字面目标 |
| projection | 可选 `{sources: string[], producer, scope}`；sources 为完整显式已登记源路径（manifest/catalog/file 源），producer 为已登记 project 节点；scope 为 `managed-block`（块外归宿主）或 `whole-file`。这是 proposed 形状扩展，不自动执行 |
| surface | `product / test / documentation / membership / instruction-policy / judge-policy / judge-implementation` |
| Edge | `{kind, to}`；from 隐含为 `file:<path>` |
| project_edges | `{from, kind, to}[]`；显式项目/脚本/外部输入关系 |
| execution_plans | FILEMAP v2：`test:ID` → `{operations: string[], timeout_seconds, output_limit_bytes, operation_bounds?}`。可按已列操作 ID 另给完整正值 `{timeout_seconds, output_limit_bytes}`，输出不得超过 64 MiB；缺项沿用默认值，映射缺失／空时保留历史序列化。操作 ID 有序、非空、唯一，必须含测试 execute。启动前，共享操作顺序和实际上限必须一致 |
| execution_scheduling | FILEMAP v2 可选：`{max_running: positive integer, resources: string[], claims: {operation: {resources: string[], outputs: artifact-path[]}}}`。每个参与操作都须声明 claim，空列表也要写明；资源独占，输出命名空间不得重叠。省略保留串行计划身份 |
| test_costs | `{test, cost}[]`；每个可执行测试项目或脚本有成本引用 |

symlink/projection 是明确的文件表示及所有权说明，不自动增加编译或测试执行边。
本仓 AGENTS 的 symlink 恰为 CLAUDE.md，CLAUDE 的受管块与登记的英文 Markdown/skill 由 instructions 从宿主 manifest/catalog 产生；
框架资产的编译边、资产传输测试边和源/目标读取的 runtime-input 边另行显式登记。
这些 FILEMAP 扩展仍属 proposed；专用生成器 manifest 的实际链接/源校验不等于通用 FILEMAP 判官已实现。

图节点 ID 使用 `file:<path>`、`input:<id>`、`tool:<id>`、`environment:<name>`、`project:<id>`、`script:<id>`、`test:<id>`、`judge:<id>`。工具和环境记录必须显式连接消费者；不推断执行依赖。input 节点引用对应状态的 environment.inputs；其边在 project_edges 显式列出，变化也成为种子。环境变量及其他非文件输入以显式快照文件登记；两端值无法保留/比较时属于输入不足。

test 节点指向登记项目的 execute 操作或脚本的测试操作，不是猜出的测试函数。每个 owner、cost、节点及操作引用必须存在；同一文件不能由两个项目隐式共享。成本数字为非负估计，未知为 null，basis 必须解释来源；禁止把 null 当作 0。所有项目都可通过登记演进，无固定项目数量上限；拆分项目时同时迁移归属与测试边。

| judges 字段 | 类型与约束 |
| --- | --- |
| migration_validator | candidate 中负责迁移证据的判官 ID；当前草案为 workflow |
| judges | `{id, executable, version, sha256, argv, selector, after, modes}[]` |
| selector | v1 使用 `every-delta`；仍然只检查该次 DELTA，不扫描裁决全部历史 |
| after | 必须存在的判官 ID 数组；有向无环，稳定 ID 顺序打破并列 |
| modes | v1 仅 `evaluate`；candidate 判官读取两端事实，见 §6 |

默认二进制路径为 `.chrono-harness/bin/chrono-judge-<id>`。七个判官均有范围有限的实现，完整宿主启用仍未完成，见 [workflow 证据](docs/workflow.md)。每个 ID 实现时才创建并登记 `judge-<id>` 与专属 `judge-<id>-tests`，独立 manifest/lockfile/target。

七对判官项目分别编译测试，不由一个总判官二进制耦合；当前存在 runner、judge-registration、judge-filemap、judge-routes、judge-projects、judge-cost、judge-mixed、judge-workflow、judge-ci、ci、instructions 及各自测试，不创建空判官项目。第三方程序可改成自己的 executable/argv，但宿主调用声明仍位于 `.chrono-harness/`。

| workflow 字段 | 类型与约束 |
| --- | --- |
| target_branch / feature_prefix / integration_prefix | 分支名与前缀，当前 dev / feature/ / integration/ |
| staleness | `{max_behind_commits, max_age_hours, combine: "any-exceeded"}` |
| stability | `{id, paths: string[], tests: string[], reason}[]`；精确路径 |
| semantic_changes_require_integration | bool；当前 true |
| mixed_change | `{level: "warning", code, requires_acknowledgement: false}` |
| integration | `{tests, bind, evidence}`；精确绑定字段与状态目录路径 |
| retirements | `{kind, id, replacement: string|null, reason}[]`；显式退出/迁移事实 |
| historical_profiles | workflow v2 保留具名 FILEMAP v1 profile `{id,filemap_version,profile_path,script,test,mappings,legacy_records,ambiguities}`；v3 也接受 `{id,from_versions,to_versions,script,test,mappings,legacy_records,ambiguities}`。两份版本映射均须明确列全五登记（`config,filemap,projects,judges,workflow`），两端版本精确相符且恰有一个匹配 profile 才能选中 |
| historical_profiles.method_replacements | workflow v2／v3 可选的明确 `{from,to,reason}` 行。每个方法有精确 `{owner,operation,tool,argv}`，须对应一个原 legacy action 和一个当前方法，operation ID 相同，并明确映射所有者变化。重复、未变、过期或未用声明均失败；省略则保留原独立严格旧方法合同。原定义和操作义务保留，不推断语义等价。这是 beta.13 后的源码扩展，见 docs/workflow.md |
| migrations | `{from_version, to_version, script, test, mappings, reason}[]`；mappings 为旧/新记录 ID 对 |

本节每份文件有共同 schema_version/status，表格列出其余字段；不另设未登记的规则文件。版本选择合同只登记需要执行哪个 candidate decoder，不推断宿主结构、语言或迁移算法。v3 的版本选择会给 decoder 传原始登记及字节、candidate 登记和所选 profile；原始 config不得被历史视图改写，旧输入快照仍按旧版本及其原始摘要核验。配置 v1→v2 可用此合同验证宿主登记的转换及兼容测试，v1 的 `sha256:null` 仍是未绑定而非缺失。

实际转换收据与同一专属测试的执行结果均须通过 workflow 验证，版本声明本身不认证兼容。指令生成专用 manifest 使用 §16 的独立合同，没有 active/proposed 状态，不参与判官启用。登记的语法/引用约束也由 registration 判官实施；runner 只负责 JSON 传输与协议解析。配置不可解析、进程无法启动等基础故障是运行时错误，不是一个未声明的业务判官。

## 4. 唯一检查指令、快照与 CI 对等

实现并安装指定二进制后，本地与 CI 必须从宿主根目录运行同一条指令：

```sh
./.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/config.json \
  --base "$CHRONO_BASE" \
  --candidate "$CHRONO_CANDIDATE" \
  --context .chrono-harness/state/context.json
```

两个变量必须是已存在的完整、不可变 commit OID（按仓库对象格式为 40 或 64 位 hex）。不得直接传 `HEAD`、`dev`、短 SHA、工作目录别名或 CI 默认 merge ref。`base` 是此轮已获取的 dev tip；`candidate` 是拟向 dev 交付的精确提交。DELTA 比较二者的 tree，不自动改用 merge-base，base 不必是 candidate 的祖先。分支过期和合并关系由 workflow 判官处理；不允许 CI 悄悄改成全量检查。

本地/CI 必须运输相同 OID、context 与版本化配置；其报告绑定这些内容的摘要。同一指令保证调用和输入解释语义一致；相同裁决还要求完整有效输入、政策、执行物、工具链、配置和环境相同，并且判官/测试确定性求值。仅 Cargo 版本或环境摘要存在不足以证明这些前提。前提未建立时报告 parity:unestablished，不能承诺裁决相同；这不另加普通 full check 准入门。其已登记合同所需输入／证据不足仍报错。
上述“相同环境”是本轮有效输入和可移植合同的语义相同，不要求把两端 runner 的在线清单复制进仓库。CI 从 provider 的在线事实取得实际 OS、镜像、工具、路径和运行身份，写入本轮报告／artifact；本地可以生成同格式的 `state` 观察，但不得因观察不同而自动改写跟踪 policy。缺少必要在线事实时保留 `parity:unestablished` 或输入不足；不以仓库中上次运行的 host snapshot 代替当前 CI 事实。


普通 full check 在登记要求及全部选中义务通过后表示该 DELTA 满足现役登记合同；现役 registration 的 effective_inputs 仍可为 completeness_proven:false。

当前不提供独立 `parity` 比较／报告改写命令。现役输入生产者均报告
`completeness_proven:false`，普通检查报告保持 `parity:unestablished`；相同的两个报告
不能建立输入完整性或普遍确定性。检查状态归实际生产者，不提供写死宿主状态的 `spec status`。
宿主尚未登记的 compiler、SDK、网络、凭据或其他已知适用外部输入仍须补齐。

v1 明确选择干净、不可变 commit 快照这一工程边界；不另设 dirty-worktree 验证模式。工作树必须检出 candidate，索引和已跟踪文件必须等于 candidate，子模块不得隐式展开。未跟踪文件须登记并提交；只有 config.artifacts 中生成目录可忽略。仅写进 `.gitignore` 不够：判官检查所有未跟踪路径，包括被 Git 忽略的路径。生成目录里若出现跟踪文件，按普通 DELTA 文件要求登记；忽略不覆盖 Git 事实。

采集未跟踪路径时可按显式生成目录做字面、区分大小写的排除，避免先输出全部产物路径，再由调用方过滤。排除不覆盖目录同名的文件／链接、相邻未登记路径或独立的已跟踪文件／索引核对；不规范的目录写法不得归一化成更宽排除。迁移解码器前后也只比较受治理输入，允许写登记生成目录，新增未登记文件或修改候选源码仍须报错。暂存、未暂存、未跟踪的非生成物变化或 candidate 不匹配返回 `E_SNAPSHOT_DIRTY`，不静默丢弃。这些行为是 registration 判官的默认策略，runner 只传入 Git 状态事实。

仓库路径边界、特殊文件类型不支持时必须返回 `E_INPUT_UNSUPPORTED`，不能漏判。

`--config` 的磁盘内容必须等于 candidate 中同路径 blob；base 配置单独从 base tree 读取。base.root 是从固定 base tree 导出的只读文件快照；不执行其中程序。candidate.root 是实际检出的候选工作树根，不是导出的干净副本。判官将其视为只读源，构建及证据仅使用登记生成目录。只读是输入消费合同，不声称 OS 沙箱隔离；registration 对比观察与固定对象。

context 为 UTF-8 JSON：`schema_version, base, candidate, dev_tip, branch_ref, fork_point,``branch_started_at, observed_at, operation, integration_evidence`，时间均为 UTC RFC3339。

workflow 使用 context schema 2，另须显式 `run_kind: integration | delivery`；integration 前缀的交付也必须使用 delivery。registration 保留 v1 供现有有界消费者读取。operation 固定为 validate.delta；integration_evidence 为证据摘要或 null。context 的 base/candidate 必须与 CLI 相同，dev_tip 必须等于 base；CI 不接受悄悄漂移。

Git ancestry 来自完整对象图；浅克隆缺对象返回 `E_HISTORY_MISSING`，AI 补取后重跑。工作开始记录 branch_started_at，observed_at 是本轮获取 dev 的时间，不用提交时间猜分支年龄。同一轮本地/CI 使用相同 context；dev 再推进就建立新一轮输入，不能冒用旧报告。

完整报告写入 `.chrono-harness/state/report.json` 与本轮唯一的 `run-<identity>.json`，格式、原始证据与摘要保持完整。短命令 stdout 只呈现原状态、退出码、有限的具名诊断与唯一原报告路径；超出行数或文本长度的诊断明确标示省略，完整内容从原报告读取。显式旧合同 `--config` 调用仍输出与存储报告相同的完整 JSON。

报告的 report_path 指向后者；runner 将实际前序判官进程及 request_digest 传入 observations，供 workflow 核对执行身份与完成证据。CI 只负责准备工具、不可变输入并调用指令、保存报告和原样传播退出码。不得另写 CI 专属判官、skip 参数或本地宽松模式；现役生成 workflow 的 scoped 合同见 docs/ci.md；不设置虚假的绿色 workflow。

`chrono-github-full-ci/v2` 是显式完整 context 的独立 dispatch 投影，使用宿主 schema4 登记的短 check 入口，旧 provider v1 拒绝。调用方提供context 原始 UTF-8 JSON 字符串与所有引用证据；生成器不推断角色、时间、宿主布局或证据获取方式。候选配置的 Git 绑定核对 HEAD、配置、provider 源和指定 workflow修订中的投影字节，只在精确缺对象读数后补取固定 OID。

完整准备由短 check 的登记输入 action 执行，普通 `chrono-ci prepare` 手工入口已退役，调用在读取投影或发布准备输出前拒绝；内部事件准备 API 保留；未采用扩展专用的 `prepare-endpoints`／`acquire-seed` 手工入口同步退役。准备阶段保留 context 原字节，另存 Git 观察、源身份和所选短 check argv；其 `prepared` 不承担治理裁决，parity 保持 `unestablished`。实际判官仍裁决 context 其余语义与缺失证据。所有当前 full 投影以精确源路径标记归属，由共用输出预检直接核对；不解析旧 YAML 推断归属，也不自动接管只有历史通用标记的输出。不可变上下文／准备原件按路径与摘要保留，准备报告不再内嵌重复的 `context.input_bytes`；当前输出可替换，原始进程失败须保留并传播。单文件发布不认证并发或跨文件事务；此投影不自动启用完整宿主、PR/合并流程或 必需检查。

字段、重试与采用边界见 [完整 CI 传输合同](docs/full-ci.md)。

固定快照的干净检查须同时比较 candidate 树、索引 stage 和实际文件。现役源码按 Git 对象头与原始字节计算 SHA-1／SHA-256 blob 身份，逐项核常规文件类型及 Git 的 owner-executable 位、符号链接字面目标；目录祖先不得沿链接读取。它不通过工作区 diff、textconv、clean filter 或换行转换决定文件身份；配置隐藏的字节、类型与执行位差异仍须拒绝。

索引的新增／删除／mode／OID／未合并 stage 独立比较，不能由相反工作区变化抵消；既有 index flag 和 untracked/artifact 检查继续适用。核验只消费固定树／索引清单，不登记依赖、不选测试，不因缓存或 Git status 干净而跳过读取。完整／initial／scoped 检查及 worktree 创建、重建、恢复、清理复用同一文件比较实现，各自通过已有 Git 绑定读取树和索引。

Gitlink、特殊文件及非 UTF-8 登记路径尚不支持；时间戳、ACL、xattr 不是 Git 树身份的一部分，且不承诺并发原子快照。完整 Git 配置、委托程序、工具链／SDK 输入及其依赖的完整性仍须另行补齐。详见 [原始快照身份](docs/git-facts.md#literal-checkout-identity)。


### 4.1 显式单元与独立条件 job

短入口仅在原报告发布后投影，不另执行判官或业务。失败生产者与版本探针保留原始回执，解码／校验失败引用原 acquisition；完成报告发布前的执行错误另留原错误，不能称为已完成报告。保留失败时如实返回原错误与保留失败，不伪造路径；早期配置／选择错误仍为原 E_CHECK，不宣称普遍输出上界。

无论仓库规模，每个能独立验收、独立重跑的单元均生成独立 job；当前目标是一份 parent workflow，先 detector，再 job 级 if 条件单元，最后 `always()` aggregate（needs detector 与全部单元），仅 aggregate 为必需检查。边界由宿主显式登记，必要前置与完整测试义务仍须保留。各单元具有独立检查名、checkout、启动命令、边界、报告及重跑。

单元及完整测试计划的对应在宿主 `.chrono-harness/` 显式登记；不得按目录、语言或自动发现依赖分组。计划须恰有一个所属单元，漏登、重登和未知计划先报错。跨单元共用操作须逐项声明由哪些单元各自重复；本版不隐式传输产物或推断单元间依赖。

现役 config schema4 支持固定 root invocation `check`、`check --unit ID`、无值 `check --collect`；非根 bare check 保留全局 DELTA，显式 standalone 初始化绑定见 §12。

固定配置／原生 selector、bound Git、工具／环境／协议约束及 source-local/CI action 在执行前解析；`CHRONO_CHECK_SOURCE` 必须显式继承，无值／local 和 ci 各有唯一登记生产者，无混合覆盖、猜范围或手工 prepare。

runner 分别保留真实 argv/cwd 与 PreparedCheck 的配置、端点、scope、context 原字节／语义摘要及原始生产证据，执行现役判官；canonical consumers 拒绝伪造展开 argv 和 selector 不一致。

worktree v2 的 start/reconstruct 自动发布目的地 birth association，full unscoped 的 schema2 context 保留原 fork／birth，观察当前 HEAD／target／时间，引用实际历史输入，缺证据失败。local collection 由 CI owner 根据显式报告路径及独立执行物 pin 产生登记 manifest，无业务调度；provider v4/full provider v2 生成同一短 check step。原生报告收集由 `check --collect` 内部执行，不提供独立 `chrono-ci gather` CLI；传输、固定来源与 attempt、报告原件及最终判官验收义务仍保留。

旧合同显式拼写仅留给旧登记。full 独立短单元／汇总已接入显式 execution_units 与原始证据；当前实现不宣称 native 完成或完整 SPEC 验收。

现役源码扩展 `chrono-ci-check/v3` 的 `policy.units` 与 `shared_operations`。判官先完成两端 DELTA、移除／替代义务及全局所选操作图验证，再按 `--unit ID` 过滤执行；不能借单元过滤隐藏全局操作环。单元报告分别列本单元所选、全局所选、交给其它单元、所需单元及真正不需要的义务。某单元成功只承担自身结果，其失败不取消不相关 workflow。本地及该 workflow 均追加完全相同的 `--unit ID` 到登记 check 指令；v1/v2 原调用行为保留。

同一入口的 `--collect MANIFEST` 只重建当前义务并核所需原始单元报告，不能重跑业务测试。它检查完整报告集合、固定端点／配置／执行物 pin、原始判官输出、计划身份及候选操作／顺序／边界、工具观察、逐操作原始回执和成功状态；缺漏、重复、陈旧或失败报告不得全局通过。执行物 pin 由明确调用输入承担，不宣称据此证明构建产地、外部输入完整或跨环境同判。

CI 的执行结果转换与终态使用同源 Verus：完整 passed／blocked 状态分别映射为 Passed／Blocked，其余均 Failed，
原因保留原错误或既有默认值；退出只在原 Passed 或原非零退出时保留。
单元和 collection 的终态由真实结果列表产生，存在 Failed／Blocked 当且仅当失败；
库存异常及诊断失败由同一证明所有者产生对应失败记录和状态。
这些局部证明不替代 projects 原结果生产、原件准入、JSON 解码和完整覆盖门，范围见 [Verus 说明](docs/verus.md)。

`chrono-github-units/v1` 从明确的 provider 生成单个父工作流，其中各单元与汇总各有一个 job。provider 只准备固定输入、收集和运输对应 candidate/event/attempt 的原始报告并运行上述指令；最终裁决仍归判官。push／pull_request 自动汇总核实际 workflow 源和 attempt 未漂移；本地组合使用显式 manifest，父工作流不提供手动触发。启动工具按各单元显式配置，不默认构建所有项目。

完整字段、共享／重试合同、初始输入及当前采用边界见 [并发 CI 合同](docs/ci-units.md)。此扩展不自动启用完整七判官治理；产品仓库按登记将完整测试计划分配给显式单元，当前通过 `job_gating` 投影为一份 parent 内的独立条件 job；启动核心及共享操作也由宿主登记。

`job_gating` 是 `chrono-github-units/v1` scoped／v2 full 的必需调度声明。独立单元工作流拓扑已退役；单元不登记虚拟 `workflow_path`，gather 不接受已无使用方的 `wait_seconds`／`poll_seconds`。旧发布与历史证据保留原事实，不构成现役源码的兼容执行路径。检测复用 judge-ci collection inventory；full 调度复用 FILEMAP/routes 的纯声明选测及两端 assignment，SDK 输入快照仍归实际 full 准入。检测不得调用业务 check/probe、选全兜底或推断目录／语言／import；允许按宿主显式 source inputs 启动产品本身。

Git 完整树／登记 blob 在不物化全部宿主源时可读，PR 固定原生 workflow 合并提交 M 及其第一父 B，校验第二父为事件 head H；所有作业检出 M，`candidate=M` 与 `event_head=H` 分别绑定测试和 GitHub 运行归属、默认 push 完整 before/after，既有 integration `push_baselines` 保留并由 detector 一次固定。浅 blobless depth2 是可用起点，缺端点明确补取；禁止静默 HEAD^／merge-base 和 workflow paths／paths-ignore。完整 Git DELTA 不设路径数量门，超过平台 API 路径上限仍完整处理。

aggregate 的 Rust provider 必须拒绝 detection 失败及任一 required 单元非 success（含 cancelled、missing、unexpected skip）；仅 nonrequired 单元允许 skip。汇总仍经固定 `check --collect` 核原始报告及最终判官，不以 GitHub 状态代替准入。

单元重跑保留同一 parent 的原 detector／成功前置 attempt，逐项核最新实际 job 与相应 original artifact attempt；不轮询其它 workflow 或等待自己完成。投影切换统一归显式 migrate：按前一 provider 快照核精确旧投影字节和所有新目标冲突后退役旧输出，保留自定义并拒覆盖 user files。普通 init/generate/verify 只处理当前 provider 的投影，不派生另一拓扑、读取或退役其投影。

详细字段、原生环境登记、full context／external input 与历史 decoder 的未完成边界见 [CI units](docs/ci-units.md)。

full-v3/v4 的 `execution_units` 由同一 assignment/select/required 与七判官 DAG 裁决。schema4 日常入口仍为 `check`、`check --unit ID`、`check --collect`；裸 check 保留全局 DELTA，单元贡献不等于准入。汇总消费 `chrono-full-collection/v1` 的原始报告、请求、七判官绑定与回执，不执行业务或业务版本命令。

`chrono-github-full-ci/v2` 保留显式 exact-context dispatch；`chrono-github-units/v2` 在现有自动 push/PR provider 中显式登记 collection/各 unit 的 full context 路径与上传映射。bootstrap/caller 必须供应同一精确 full context；producer 核它与实际事件端点一致，原生 gather 核 workflow/attempt、下载原始上传目录，再交同一短命令裁决。

缺失或不匹配输入失败。本仓实际 full 启用、公开新二进制和远端原生 full 验收仍未完成。

local 独立 scopes 仅在当前端点、birth／role 与输入引用一致且新观察仍满足 workflow 登记的 age 判法时保留同一 context 及原 `observed_at`；超过期限自动保留旧原件并发布新的不可变观察，旧贡献不能满足新 context，当前分支仍由 workflow 拒绝 stale。裸 check 产生新观察。

unit 的 live 文件输入由 registration/input 单一实现按显式 unit plans、操作所有者、FILEMAP 依赖及 closure bindings 选取，含共享操作及治理输入；结构和引用仍全局检查，缺失／漂移／畸形／未知输入失败。capture 复用该投影；collection 核原始 blobs 与逐单元有效输入投影，不启动业务或业务版本工具。这些源码行为不证明远端 full 启用、release 或完整 SPEC 验收。

宿主自定义与更新是交付合同：单元、启动参数、平台、超时及扩展调用由宿主显式配置；安装新二进制不重置这些源。`init` 保留已有宿主源，常规修改仍从该源生成和核验。显式迁移以旧、新 provider 为输入，先核旧投影及所有目标冲突，再更新受管输出、退役不再使用的旧输出；保留新源和无关宿主文件，不推断测试归属或替宿主改登记。当前 `chrono-ci migrate` 支持 scoped v4（无初始清单）／units 到 units，以及同源地址的显式前一配置快照；独立 v4 的初始清单只允许保留同一声明的迁移，full 和 release 合同不在该迁移入口中。完整 CLI 与失败边界见上述合同；投影迁移通过不等于 CI、完整治理或跨文件事务通过。

一个测试项目可以显式登记多个测试组并分别绑定 CI 单元，不为分组另造生产／测试项目。分组须保留原测试义务，使用登记的库存判法核对当前未过滤集合：各组非空、互斥、并集完整，不能按历史数量或隐式发现规则选测。共享编译输出与库存操作仍登记互斥；原未过滤入口、各组执行边界和超时分别明确，不把分组后的多个预算声称为原单项总预算。宿主 projects 的普通测试和真实宿主迁移测试采用此分组，原断言与测试内时限保留，具体登记见 [独立 CI 单元](docs/ci-units.md)。分组不证明总工作量减少或原生时限充分。

共享启动产物须显式登记真实消费者、需求、用途与安装入口。可由已有 detector 构建一次，向所选单元及汇总传递；不新增专门启动 job，不取消实际业务检查。消费者须核原提交／配置、平台及文件身份，保留原构建报告并区分复用与本地构建。重跑使用原生产者的 artifact ID 与绑定，不能按当前 attempt 猜测或在输入缺失时静默成功。二进制传递目录与证据上传目录隔离；转移构建后，删除已无实际生产操作的编译缓存订阅，传输与安装成本从原生 CI 步骤和实际原件核对。生成器只投影登记合同，身份核验及平台适用性由宿主安装器负责，不推断语言或目录。

### 4.2 CI 缓存与增量编译（目标合同）

恢复的编译产物不能让当前构建因源码时间较旧而复用旧程序。宿主可在现有恢复入口明确采用源码时间调整：仅兼容命中时，核当前登记输入字节后，使必要输入的修改时间晚于所选产物，保留依赖和增量数据；记录实际结果并重跑原构建。精确命中、重放和未选输入不重复调整。该方法与成本由缓存登记负责，不推断源码或更换工具链。

CI 必须实际恢复和保存可重建缓存，覆盖启动工具、当前判官及所选生产／测试项目的编译工作。检查与发布 CI 均适用；检测、单元及汇总 job 按各自登记的消费者取缓存，不默认构建或缓存全仓项目。仅有名为 cache 的本地目录或 artifact 上传不算跨运行缓存已启用。

缓存政策与 provider 源由宿主在 `.chrono-harness/` 显式登记，沿现有 generate/verify 入口生成与核验 CI 投影；不得独立手改生成 YAML。逐项登记缓存 ID、所有者、生产／消费操作、准确路径、兼容输入、键的构造、允许的恢复前缀、恢复／保存时机和处置方式。缓存产物使用相对宿主的字面路径，精确引用已有登记中的同所有者、未跟踪且非执行证据产物；依赖下载缓存也须登记在宿主内。不按目录、语言或文件扫描发现缓存。执行与选测仍只消费 FILEMAP 的声明。

同一 provider 配置中的一个缓存 ID 至多登记一个保存 job。`save_caches` 显式选择各 job 的保存子集；省略表示保存全部所选缓存，空数组表示只恢复。生成／校验预检在写投影前拒绝多个保存者，报 `E_CACHE_SAVE_OWNERSHIP` 并列出缓存与冲突 job；不自动选择保存者、不增加 job 或跨单元等待。允许全部消费者只恢复；指定保存者被跳过或未满足保存条件时，本轮没有新归档。独立生产域（如不同发布平台）须各有显式缓存身份，不由程序推断拆分。

| 缓存内容 | 必须保留与复用的范围 |
| --- | --- |
| 依赖取得缓存 | 已登记包管理器的下载、索引及固定依赖内容；恢复后仍核锁定版本与内容身份 |
| 判官与启动执行物 | 当前 runner、判官及辅助工具的构建输出和已登记安装产物；每个 job 只取自身需要的执行物 |
| 各独立项目的编译产物 | 生产项目与专属测试项目各自的 target／等价输出，包括依赖编译产物、build-script 输出、指纹及编译器增量数据；保持原有输出隔离 |

Rust 的登记编译操作必须启用增量编译，并将各项目实际使用的 `target` 中增量数据一并纳入持久缓存。debug/test 与 release 的 profile、`CARGO_INCREMENTAL`／profile 配置及真实生效值均须登记；不能只缓存包下载、最终二进制，或在 CI 关闭 incremental 后宣称缓存了增量编译。其它工具链按其显式登记的增量／编译缓存合同采用，不由通用 runner 推断实现。

缓存兼容域必须绑定缓存格式、生产／消费项目及操作、OS／架构／target、实际工具链与编译器身份、profile／features／flags，以及已登记的链接器／SDK／build-script／配置／环境输入。锁文件、manifest 与显式直接依赖的构建输入也进入相应键。提交 OID 可区分保存代际，但不能作为唯一兼容判断或唯一恢复条件，使每次源码变化都丢失可用的编译缓存。键必须由实际声明输入确定，不能用不相关目录变化替代依赖。

同一 JSON 文件混合构建参数与单元登记时，可用 `json-value` 输入显式登记字面 `path` 与 RFC 6901 `pointer`。键只绑定所选 JSON 值；对象键顺序、格式与未选字段不影响键。空 pointer 选择全文，显式 null 是值，缺文件、非法 JSON／pointer 或缺所选字段均失败。全文件摘要与长度作为 `source_file` 来源读数保留在计划中，不进入所选值的身份；路径及输入／缓存输出隔离继续适用。宿主明确选择字段，程序不推断编译参数。隔离回归须同时检验无关登记变化不改变键、实际构建参数变化改变兼容域；这不证明未登记输入不存在。

依赖取得、可复用编译中间产物与可直接执行的二进制分别明确恢复条件。编译缓存可以从同一兼容域的旧源码键恢复，以登记构建命令校验指纹并增量重建；跨工具链、target、profile 或其它不兼容域不得宽前缀恢复。直接复用判官二进制还须绑定候选源码及其显式传递依赖、锁文件和全部登记构建输入，核实际版本与摘要符合当前执行物绑定。缺少身份、来源绑定或完整适用输入时，将它作为构建候选或重建，不直接执行为候选判官；缓存中的历史判官不得参与当前裁决。

恢复必须在相应 bootstrap／构建前，保存必须在编译生产者退出、写入加入且停止后。缓存恢复／保存本身也是登记消费者；采用自动清理的宿主须在整个消费与保存期间持有相应使用保护，在最后租约释放、缓存被回收前完成保存，不能与清理并发读写。精确二进制命中并经当前绑定核验后可复用；恢复旧编译缓存时仍运行同一登记构建入口，只由构建工具决定可复用部分。禁止为缓存增加第二套构建配方或因命中跳过所选判官／测试。各独立 job 保留隔离的输出路径；跨 job／重跑／跨运行共享通过登记缓存传输，不让并发生产者写同一份活目录。保存已完成的可重建编译产物可以独立于后续测试失败，但必须保留真实构建／测试状态，不发布失败或未完成构建为有效执行物。

缓存未命中、不可用或损坏须按 §7 的标准日志记录，并沿同一入口执行冷构建／重建；恢复失败保留原始异常和处理结果。候选仍不满足当前执行物绑定或构建失败时，检查非零，不能用旧缓存或命中状态填成功。缓存不得运输治理通过状态、旧判官响应、测试结果、运行 context、生命周期锁／租约或必要证据；本轮报告和原始单元证据仍由实际生产者生成并按其 artifact 合同保存。

报告必须记录缓存 ID、请求键、实际恢复键、精确／兼容命中或 miss／error、核验与重建结果、保存结果及所消费执行物身份；未知测量仍为 unknown。保存 action 成功不能证明后端归档存在；没有改变当前构建、恢复或交付决策的消费者时，不为确认保存而持续查询后端、复制线上缓存清单。需要排障时直接查原生 action 日志和线上 API。普通日志时间和命中状态是操作观察，不决定裁决。冷缓存、精确命中及兼容旧源码缓存必须执行相同选中义务、保持相同调用合同；同判仍受 §4 的完整有效输入及确定性前提约束，缓存命中不建立 parity。

## 5. DELTA 与依赖图算法

在宿主定义的治理／输入范围内，DELTA 选择充分的前提是：两状态实际语义输入及其直接/间接依赖均完整登记，且判官/测试谓词具有局部性：仅由已登记的语义输入决定，未受变更输入及其闭包影响的谓词保持不变；外部输入也属于状态。图上闭包仅是相对于给定登记图的机械事实，真实依赖完整性与局部性是工程义务，不能由合法登记推出。

该范围内未解决的必需依赖、已知缺失的 fixture、环境、外部输入或非局部影响须报输入/证据不足，AI 显式补登记或调整检查单元；不推断补边，不声称能自动发现所有遗漏，也不声称已检查每个历史对象或证明未来 Rust 实现。AI 的 declared-complete 工程声明及登记核验支持普通合同验收；普遍隐藏输入不存在的证明不是该验收前提，也不能从一次通过推出。

DELTA 是 base tree 到 candidate tree 的集合差，包含路径、blob OID 和 mode。
按字节稳定排序路径，用 Git `--no-renames` 的含 NUL 输出或等价库接口读取事实。
增加 A：base 不存在、candidate 存在；修改 M：两端均存在，blob OID、mode 或 Git 对象 kind 不同；删除 D：仅 base 存在。
重命名规范化为 D(old)+A(new)，不依赖 Git 相似度阈值；展示可附 rename 提示，不改变裁决。
同内容改名仍须检查旧归属、旧消费者、旧测试和新路径登记。
二进制文件也按 blob 比较；没有文本 diff 不等于没有 DELTA。

逐路径条目判断由 `verified_status.rs::delta_for_entry` 直接读取实际两端 `TreeEntry` 并证明；
`facts::Entry` 重导出同一类型，完整文本比较 mode、oid、kind。两端均缺失或三字段全部相同不产生记录，
kind 单独变化也产生 M；产生记录时精确保留原路径和旧新 blob／mode，缺失端为 None，存在但为空的字段仍为 Some("")。
实际 `facts::Tree` 的路径键为同根 `TreePath`，内容身份与 UTF-8 字节顺序由其 Verus
`same`／`order_spec` 定义；真实比较、构造、克隆和文本借用完成证明，保留原文且不归一化。
`facts::delta` 直接调用同根 `delta_for_trees`：借用两棵实际树的全部原键值，按已证明的路径顺序
线性合并。输出按完整 UTF-8 字节严格排序；对任意完整路径，输出投影恰为实际两端条目的零或一条
A／D／M 记录，六字段完整保留，无遗漏、额外路径或重复。枚举到实际 map 的绑定、合并及终止均有证明；
固定 vstd 的 BTreeMap `iter`／`next` 是明确采用的外部容器合同，返回完整键值及其严格顺序。
Git 命令、原始输出解析、JSON 适配及整个 Git 事实获取仍有各自未完成的证明义务。

第 1 步：读取 B/C 两份登记，解析变更文件及变更的记录 ID、字段和边。第 2 步：建立 `G = G_base ∪ G_candidate`，保留每条边来源 base/candidate/both。第 3 步：变更文件、已登记有效输入的两端变化为种子；登记变化另把变更记录目标加入种子；有类型边的增加、删除或改接必须把两端节点加入种子。例如只删除 `runner → runner-tests` 测试边，也把 runner 与旧测试作为影响种子。

第 4 步：沿已登记的有类型正向边计算影响闭包，去重但保留来源和因果路径。第 5 步：选出候选测试、判官、成本项；验证删除/迁移事实后执行候选中仍存在的测试。第 6 步：每个 finding 必须关联一个 DELTA 路径或登记变更，以及明确的依赖因果链。没有因果链的旧缺陷不应成为本次错误；可作为未裁决 context 列出。

可达节点集合不能代替有类型边、逐种子来源或执行要求。即使端点图与并集图的可达节点集合相同，也不能据此改用 candidate-only 图选测。例如 base 仅有 `project:p → test:t` 的 test-execution 边、candidate 删除该边、两端节点都已成为种子时，两图都到达这两个节点；只有保留旧边的并集仍选择旧测试。测试删除、替换或退役仍按第 6 节结算，不因节点已到达而省略。

| 边 kind | 方向与含义 | 不允许的推断 |
| --- | --- | --- |
| compile | 输入文件→项目；依赖项目→消费者项目 | 不解析 Rust import 来补关系 |
| build-input | manifest/lock/生成器/外部输入→项目 | 不把所有根目录文件当公共构建输入 |
| runtime-input | 已明确选择的资产/宿主 canonical 材料及为保留原文、校验标记而读取的已有输出字节→消费项目 | 不将运行数据伪装成编译输入，也不由运行时扫描补登记 |
| test-execution | 受影响项目/文件/脚本/输入→测试 ID | 不按文件名、测试名、Cargo 元数据猜测试 |
| judge-trigger | 文件/项目→判官 ID | 不凭扩展名选判官 |

compile/build-input/runtime-input 可沿显式项目 compile 边继续传播，但只有 test-execution 边能选测试。workflow 的显式 stability 路径/测试及条件 integration.tests 同样是白名单关系；现役 FILEMAP impact v2 将适用要求降为有来源的 test-execution 边，再交同一闭包与执行链，见 [选择合同](docs/workflow-selection.md)。

此步骤不认证分支、迁移或 integration 证据。配对关系表达所有权，不隐式产生测试边；缺配对执行边由 projects 判官报错。every-delta 判官始终被选择，judge-trigger 提供额外影响解释；二者取并集去重。依赖闭包需要读取未变文件和历史登记，这是读取上下文，不是重判历史文件。若某个声明的测试单元会验证完整项目，允许执行它；测试由 DELTA 选出且故障须说明因果。与该 DELTA 无因果关系的既存测试失败标为 `E_EVIDENCE_UNRESOLVED`，不能捏造通过或旧违规。

删除文件从 base FILEMAP 取得 owner/依赖/成本；不能因为 candidate 不再登记而丢失影响。
新增文件必须在 candidate 注册；修改文件必须在两端有可解释的注册或显式补登记记录。
完整登记表可读作上下文；初始完整性检查只要求本轮新增、变更、受影响引用闭合。
未改动、无影响的历史缺登记留给后续明确 migration DELTA；不假造全库已经合规。
schema 不可解析影响本轮输入时是错误；不能以“历史问题”忽略无法构图。

### 5.1 形式对应与保留义务

以下对应固定引用 trureturing 修订 `5cd5558013f6e02612a617b2ba5b03db7a6de0f6` 的 [TwoStateLocalityIncrementalPreservation.lean](https://github.com/the-omega-institute/trureturing/blob/5cd5558013f6e02612a617b2ba5b03db7a6de0f6/D5/S3/ConceptDynamics/Governance/TwoStateLocalityIncrementalPreservation.lean)。

模型只承载下述条件结论，不提升 Rust 实现、宿主登记或实际执行的验证状态。

| SPEC 对象／义务 | 形式对象与结论 | 仍须履行的实现义务 |
| --- | --- | --- |
| 未受影响对象的旧结论可保留 | `TwoStateLocalityIncrementalPreservation.two_state_locality_yields_incremental_preservation`：若属性具两状态局部性，声明依赖包含两端实际读取，对象自身及这些依赖值均未变化，则两端属性等价 | 状态须包括政策、执行物、工具链、配置、环境及实际外部输入；缺失值须与空值区分。真实读取的包含关系和局部性仍是前提，FILEMAP 格式正确、哈希存在或图闭合均不证明它们 |

`State` 对应固定端点，`Artifact` 对应受检查对象及其输入，`bytes` 对应包含缺失态的输入值；`reads` 是两端实际读取，`dep` 是宿主声明的依赖上界，`property` 是要保留的判定属性。使用 `Option String` 的精确应用已编译，可区分不存在与空字节。实际读取集合、局部性和依赖包含关系仍由宿主履行，不能把 `Local` 假设当作自动取得的结论。不另造仅包装既有结论的定理。

已合并的 trureturing [PR #10868](https://github.com/the-omega-institute/trureturing/pull/10868) 在修订 `168e85c372e38d926a25892a5f752c3f9ad8a173` 的 [TYPED_GOVERNANCE_ADMISSION.md §Theorem 6](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/docs/develop/theory/TYPED_GOVERNANCE_ADMISSION.md#L190) 交付种子边更新的节点集合推论，并登记对应的 [atom](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/Meta/Digestion/atoms/sha256/e041845285be68b9a306b63e427dfde5f58b09e7b8c977c1099c760f214c432e) 与[吸收映射](https://github.com/the-omega-institute/trureturing/blob/168e85c372e38d926a25892a5f752c3f9ad8a173/Meta/Digestion/backfill/typed-governance-admission/absorbed-closed/e041845285be68b9a306b63e427dfde5f58b09e7b8c977c1099c760f214c432e.yaml)。

它复用修订 `64b78db1ad18a8a90b42e13a412dfc281d5c0922` 中已冻结的 [ConsequenceClosure.lean](https://github.com/the-omega-institute/trureturing/blob/64b78db1ad18a8a90b42e13a412dfc281d5c0922/D5/S3/ConceptDynamics/DagCompletion/ConsequenceClosure.lean) 的 `ConsequenceClosure.consequenceClosure_least`、`ConsequenceClosure.consequenceClosure_successorClosed`、`ConsequenceClosure.subset_consequenceClosure`；该 PR 未新增命名 Lean 定理或冻结条目。

Theorem 6 的派生结论是：给定任意顶点类型、两端有向关系及同一个种子集合，若每条新增或删除的关系边的终点都在种子中，则并集图、base 图、candidate 图从这些种子可达的节点集合相等。它允许环和无限顶点类型，路径仍是有限路径且允许零长度。证明遇到某端缺失的边时，从其已登记为种子的终点重新出发，因此不保留原来的起始种子。

与 FILEMAP 的对应是：顶点取显式节点 ID，关系取“存在某种 kind 的登记边”，种子取本轮变更种子的节点集合。第 5 节要求变更边两端进入种子，强于该推论仅要求变更关系边终点的前提；从 Rust 的有类型登记变化到该前提的实现对应仍须核验。此结果不授权删除旧边，也不授权把逐种子解释、路径、边类型、测试选择或成本替换为节点集合。精确有类型边投影应用及下述删边反例的既有编译证据不构成 Rust 算法的形式验证。

第 5 节的删边例子也给出实现对应的边界：把有类型边投影为“存在某个 kind 的边”，或只保留可达节点，会丢掉测试选择需要的信息。`project:p` 和 `test:t` 同为种子、唯一测试执行边被删除时，两图的可达节点均为这两个节点；candidate 图没有执行边，并集图保留旧执行边。因此任何只证明节点集合相等的结果都不足以替代旧边、逐种子解释、路径、测试选择、成本或退役义务。

当前对应限于上述参数映射、冻结声明与派生推论、精确应用及反例的既有编译证据与现有行为回归。

相关 Rust 消费者是 `crates/judge-filemap/src/{lib,graph}.rs`；`crates/judge-filemap-tests/tests/impact.rs` 的 `file_record_field_and_edge_add_delete_retarget_changes_seed_both_ends`、`base_only_test_edge_preserves_removed_requirement_owner_and_cost` 和 `pairing_and_test_record_seed_without_execution_edge_do_not_select` 分别约束变更种子、旧测试要求与仅到达测试节点不执行。

上述外部数学模型不构成 Rust 相对模型的机械精化，也没有证明真实输入登记完整或任意判官确定性。

实际图内核现由同源 Verus 实现类型比较、两端并集及来源、精确名称和槽位索引、FIFO 首次发现与最短前驱、稀疏前驱树、有限原因链、传播行投影及有序原因集合。`graph::closure` 直接消费这些结果；原因集合的成员、新增／重复、长度、克隆和归一化均保全完整字节身份。逐 seed 传播使用流式状态，成功一步恰消费同一位置的完整 seed 与拓扑根，只有全部位置消费后才结束，重复终态保持位置；传播行与前驱对应同一根。真实遍历记录的选测结果恰为全部 test-execution 边目标，按完整 UTF-8 字节严格排序并去重，其他边类型不选测试。两处诊断的 seed-ID 查找按完整字节借用输入首个匹配记录，无匹配时返回 None，保全同一记录的全部字段。节点／边与原因关联、完整原因合并、seed-ID 生产及关联完整性、JSON／诊断适配、额外选测及完整 SPEC 对应仍待证明，范围见 [Verus 实施说明](docs/verus.md)。

### 5.2 按实际消费者重构依赖

依赖重构须绑定现有宿主操作、真实消费者和具体耦合问题，说明哪些调用、构建或测试受影响。AI 从最新 dev 建隔离候选，在负责该内容的位置修改代码与显式登记，沿同一检查入口验证 DELTA，核落地并清理。只报告图或建议不算交付；不为假想优化预建分析器或固定排名。

FILEMAP 只须产生第 5 节的两端有类型并集、逐种子影响闭包、真实边与有限路径见证。它不常态产生 SCC、压缩图层深、关系数、跨所有者排名、执行冲突摘要或这些辅助报告的自检；当前没有使用这些输出的生产消费者。FILEMAP 允许循环，有限闭包仍须保留环边、菱形替代路径和全部种子。执行 DAG 的无环、前置与资源／输出排斥仍由 routes 和 projects 的现役入口检查，不得以退役辅助报告省略它们；执行回执的必要自诊断保持。

每项重构固定前后提交、范围与必要对应关系，保留实际接口、行为、配对和测试义务。不得通过删除必要登记边、测试或创建空壳缩小闭包；退役按第 6 节处理。只量与当前问题有关的成本，注明输入、方法、单位及窗口，未知保持未知，不要求所有改动生成固定结构向量或统一评分。图闭合或指标改善不证明语义、输入完整、加速或最优。政策与产品同改须明确说明影响和验证成本，适用检查未完成或失败时保留原件并如实报告，不升级为已交付。

## 6. 登记变化、移除与演进

登记自身也是代码数据，其 A/M/D 进入 DELTA；不使用 candidate-only 图或“最后配置赢”。FILEMAP 新增文件行、调整 owner、登记实际 build 输入/测试边是 membership 变化。引用目标删除后仍存在的消费者报 `E_DANGLING_EDGE`；AI 修改消费者或明确登记迁移。删除测试项目必须同步迁移生产项目的专属测试及执行边，不能留下一个无测试项目。合法同时删除生产项目与其测试时，workflow.retirements 明确列出二者及原因。

旧测试计入 `retired_tests` 和成本影响；不存在的候选测试不被误标为已执行。旧测试若无合法 retirement 或 replacement，返回 `E_REQUIRED_TEST_REMOVED`。替代测试必须在 candidate 真正执行，旧成本依然可见；同一方法不永久冻结项目结构。

删除/替换判官要登记 `kind: "judge"` 的 retirement、replacement 或 null 和原因。只执行 candidate 指定的当前判官，以 evaluate 模式按新政策判本轮 DELTA。candidate 的 migration_validator 消费 registration/filemap/cost 的结果，核对 base 登记、旧边、成本、retirement/replacement、迁移映射与 integration 证据；旧谓词不永久生效。

base 只提供历史事实，不启动旧判官，也不要求取得、重建或用适配器包裹失效旧执行物。当前验证器可自主替换；其自身替换同样保留两端证据并经过新候选的 integration 验证。不自动下载猜测的插件、不把未知删除当“关闭成功”；有意规则调整是正常自主工程动作。

schema 版本变更要登记当前替代：旧/新版本、受影响消费者、记录变化与删除原因。无需维持旧接口、旧格式执行或配对历史兼容测试；验证当前合同的成功、失败和边界行为。workflow.migrations 保留版本、mappings 和 reason；仅实际数据转换时成对登记 script/test，二者必须引用已登记脚本。mappings 元素为 `{from, to}`，from/to 可为 null 表示新增/删除，但不能同时为 null；删除原因放同条 reason。

仅现役消费者确需转换已有数据时才登记一次性迁移及其行为验证；不为历史兼容创建脚本或适配器。candidate 按当前合同读取固定基线所需事实，保留原始记录、变化与摘要；缺失必要事实明确失败，不落回默认配置。新判官 ID、项目、语言和适配器都经白名单扩展；runner 无需知道当前项目全集。
主机／构建环境 policy 也遵守同一移除合同：当其真实消费者退休、迁移到 CI 在线观察或可移植合同，或审计确认只重复 CI 信息时，candidate 必须同时登记 policy retirement、替代来源、受影响消费者、删除原因和迁移验证。迁移验证核对新来源的实际消费及报告，不执行旧 host snapshot 判官；旧快照只在仍有明确保留根时暂存于 state／artifact。没有替代来源且仍是必需输入时报告 `E_INPUT_UNDECLARED`，不能用删除文件假装问题解决。

同一合同适用于所有对象：功能、字段、输入、依赖、测试、脚本、判官、CI job、缓存、报告、文档和生成投影失去必要性链终点时，登记 `retirement` 或删除，并同步移除唯一依赖边、消费者和保留条件。不能因为对象仍有引用、已经写入历史、曾经通过 CI 或删除会降低图指标，就把它继续当作必需；引用本身是待迁移关系，不是必要性证明。


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

上述新增事实字段不裁决 admissibility：config_path 是两端共同的显式根相对配置路径；checkout.tracked 是相对 candidate 的索引/工作树变更路径并集，untracked 包括 ignore 路径；index_flags 是 Git ls-files -v 的完整 {path, tag} 记录，registration 拒绝 assume-unchanged/skip-worktree 隐藏条件，runner 不改写索引。runner 绑定实际当前执行物。

registration 从固定 tree 判断支持的 entry 类型，重读对象、磁盘配置、context 和 checkout，忽略两次观察之间新增的已声明 artifact；其它变化报错。request_id 覆盖这些字段；不把 导出的干净副本 当实际 checkout。base 导出保留在 state，普通文件无写权限，不执行 base 工具。

registries.digest 精确定义为 JCS `{"base": {"<path>": <parsed JSON>, ...}, "candidate": {"<path>": <parsed JSON>, ...}}` 的 SHA-256，每端包含 config_path 与该端 config 指定的四份 registry；context.sha256 为 context JCS 摘要，evidence.sha256 为原始文件字节摘要。所有 JSON 先拒绝重复成员。

普通 full DELTA CLI 要求两个真实 commit。根提交只通过第 12 节的显式初始化配置进入单候选清单协议；不构造伪造空 base，不产生 DELTA 治理成功。登记版本迁移按第 6 节的候选转换与实际兼容测试合同执行。

impact 的版本为 `schema: "chrono-filemap-impact/v1"`，以下示例列核心字段；完整附加事实见 [FILEMAP impact 合同](docs/filemap-impact.md)。impact.edges 为平铺的 `{from, kind, to, origin}`；origin 是 base/candidate/both。seeds 是确定性去重的节点 ID 列表，附加 seed_causes 保留每个原因的 ID、节点、实际路径／记录引用和原因。

tests 是候选仍存在的选中测试节点 ID 列表，retired_tests 是候选已不存在的选中必需测试节点 ID 列表，两者确定性去重，只表达选择／删除事实，不代表执行或退役批准。required_tests 和完整两端记录、节点定义及因果来源作为附加字段保留。retired_tests 成员资格不得豁免 `E_REQUIRED_TEST_REMOVED` 或第 6 节的迁移／退役证据义务。prior_results 仅含 after 列出的前置判官完整响应；不得从运行目录猜测旧输出复用。

registration 先消费原始快照与 context；filemap 据登记生成 impact，其他判官显式消费结果。尚无前驱产物时 impact 为 null，不填造已算出的空闭包；runner 仅将直接前驱 outputs.impact 同名传入，冲突报错，其余具名产物保留在 prior_results.outputs。routes 先验证执行入口，projects 再执行测试；judges.after 中明确登记这个依赖顺序。

共享上下文的推导属于登记判官逻辑；runner 只传递具名输出，不在内部另写项目政策。所有摘要使用 SHA-256；JSON 摘要使用 RFC 8785 JCS，文件 blob 按原始字节计算。普通请求的 registries.digest 覆盖两端五份 JSON 的路径与内容，两个端点都必须存在。初始化请求没有 base 或 delta 字段，使用单份候选登记映射的独立摘要。

摘要文本格式由同源 Verus 的 `verified_identifiers::digest_format` 规定：恰为 64 个 ASCII 小写十六进制字节，允许全零。
实际 `wire::is_digest` 直接使用其已证明的文本入口；格式证明不证明 SHA-256 计算、源真实性或原件存在。

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

当前协议四态的实际类型、身份比较、`pass < warn < fail < error` 顺序、克隆和退出码由同源 Verus 实现。
两个实际协议状态的合并直接调用同源 `maximum`，结果精确保留严重程度较高的变体。
full 执行与审计、初始库存、留存报告汇总及 finding 记录共用这一实现；外层遍历与状态转换仍分别承担证明义务。
实际响应校验与留存报告审计复用同一证明根判断真实 finding 列表的状态一致性：fail／error 当且仅当存在精确 `error` 级别，
pass 不允许 `warning`，warn 必须存在 `warning`；finding 克隆保留完整字段及两个引用数组的顺序和重复。
响应头直接核完整协议／请求 ID／判官 ID 及原退出。finding 字段形状由同源 Verus 核代码和消息非空、
级别精确为 `info`／`warning`／`error`、引用数组非空；evidence 身份由同源 Verus 核
`.chrono-harness/state/` 字节前缀、种类非空和完整摘要格式。前缀允许空后缀，不代替后续文件检查。
当前实现的引用允许条件也由同源 Verus 逐项判断：普通请求接受以 `/` 字节开头的引用，
或与本请求任一实际 `Delta.path` 完整相等的文本；初始库存只接受前一类。
实际 DELTA 的克隆和比较保留六个字段，区分可选文本的缺失、空值和完整内容。
普通与初始库存的响应前置校验直接调用同源 `validate_response_fields`：先核响应头和退出码，
再逐项核完整 finding 数组的字段与引用，最后核 finding／状态一致性。
失败分别保留原 `E_PROTOCOL` 完整消息及上述优先级；成功后仍须完成后续 evidence 检查。
留存执行报告的头部／状态审计直接调用同源 `validate_record_response`：先核完整头部与原退出，
再核实际 finding 列表的状态一致性，证明原两项错误的完整消息及优先级。
这些局部判断不证明登记记录指针的语法／目标归属、完整响应组合、JSON 或证据 IO，
范围见 [Verus 实施说明](docs/verus.md)。

子进程输出 fail 却 exit 0、空 stdout、额外日志、截断 JSON、未知状态都属于协议错误。
本轮选中的当前进程崩溃、超时、不可执行、摘要不匹配、必需测试未执行都不能降级为通过。
总结果优先级 error > fail > warn > pass；基础错误阻止依赖任务，独立任务可以继续收集证据。
被阻止的任务列为 blocked，不能省略；完整通过必须有全部已选判官/测试结果。
测试操作本身不必输出判官 JSON；projects 判官调用登记操作并包装退出码、工具和结果摘要。
程序 stdout/stderr 是证据内容，不能靠查找字符串 PASS 代替进程状态。

### 7.1 标准日志体系（目标合同）

所有产品程序、判官、宿主脚本和插件适配器必须通过统一标准日志体系输出诊断、进度、告警及异常，禁止在业务代码中散落自由格式的 print／println／eprintln 充当日志。Rust 使用 `tracing` 事件／span 与统一初始化和格式化入口；其它语言使用其标准日志接口接入同一记录合同。日志字段、级别语义和格式的唯一实现归现有对应所有者，消费者复用，不各自维护拼接配方。

默认机器日志为 UTF-8 JSON Lines，每行恰好一个 `chrono-log/v1` 记录。日志格式／级别／sink 的选择及影响运行的环境变量由宿主显式登记；本地可选人类可读格式，仍由同一记录渲染。默认级别为 info，trace/debug 用于诊断，info 记录正常阶段与已处理观察，warn 记录降级／可恢复问题，error 记录当前操作未完成的失败。日志级别不是 pass/warn/fail/error 的裁决状态，stderr 非空不代表失败。

| 记录字段 | 合同 |
| --- | --- |
| `schema, timestamp, level` | `chrono-log/v1`、实际 UTC RFC3339 时间、`trace/debug/info/warn/error` |
| `producer, event, message` | 实际生产者、稳定事件 ID、供人阅读的说明；消费者不解析 message 决定行为 |
| `context` | 当前可知的 run/request/judge/operation、固定 base/candidate、阶段及输入／路径身份；未知者显式缺省，不伪造 ID |
| `fields` | 事件的具名结构化数据，包含必要的原退出、边界及缓存观察 |
| `error` | 有异常时附 §7.2 的完整错误链或已发布原始错误记录的身份引用；无异常可省略 |

未采用文件日志的宿主沿用 stderr；采用后的正常运行必须在登记位置产生可直接查看的运行日志，不能只在异常或最终报告中留下记录。命令边界记录实际生产者、进程、操作、开始和返回状态，已有阶段事件沿用统一记录。判官 stdout 保持恰好一个协议 JSON 文档。CLI 帮助／版本、登记数据输出、完整机器报告及短命令结果投影走各自已有输出合同，诊断不能混入其中。子进程原 stdout/stderr 按字节保留，不能改写成 JSON 日志后丢掉原件；适配器通过日志记录其身份、状态与证据引用，显式 participation 的原样转发合同继续成立。

文件运行日志采用按大小滚动和固定份数自动清理，份数包含当前文件。宿主明确登记目录、每份字节上限、保留份数、级别和是否同时输出 stderr；本仓采用 10 MiB／份、共 5 份，最多 50 MiB 正文。每条完整记录写入前检查大小，达到上限时轮换并删除最旧份，不按运行次数无限建文件，不另设后台清理程序。单条记录超过上限时明确失败并保留现有字节，不能悄悄截断。多进程写入与轮换由同一文件锁串行，轮换后重开当前文件，避免继续写旧文件。只处置登记的日志文件家族，路径别名、非普通成员或 IO 失败保留原异常并显式报告。

滚动日志是有界的运行观察，不作为不可变证据引用。检查报告引用的 stdout／stderr、错误原件和恢复材料仍按 §10.1 的现役保留机制回收，不参加日志轮换。运行日志回收不代表检查完成或原件已交付。初始化失败由统一应急入口输出结构化错误并阻止业务启动；运行中的日志失败保留原业务返回状态和日志异常，最终退出非零。进程突然终止、磁盘故障或应急 stderr 不可用时，不声称记录已经持久保存。

上下文随调用、异步任务和子进程边界传递，并绑定实际 request／operation，不能靠相邻行或字符串拼接猜归属。普通库层只附加上下文并返回错误；负责处理或结束该操作的边界发布一次对应事件，外层继续引用同一原始错误，避免每层重复倾倒整份输出。完整日志或原始错误写到登记 state 并由现役生产者发布身份；摘要必须给出真实路径／摘要。超时、输出超限或控制台省略时记录截断／缺失事实与可用原字节，不能把省略后的内容称作完整证据。sink 初始化、写入或证据发布失败保留其原异常，并与原业务异常一起按 §7.2 传播；尚无 logger 时只由统一应急入口向 stderr 输出结构化失败，不能静默丢日志。

### 7.2 异常传播、处理与一般规则（目标合同）

错误必须作为带类型和原始因果关系的值逐层向调用边界传播。每层可增加当前操作、输入、路径及稳定错误码，但必须保留原错误为 `source`；禁止把原错误仅格式化成字符串、新建无 source 的“失败”、置空 cause 或用通用码覆盖原生错误。Rust 使用 `std::error::Error::source` 的错误链，Python 使用保留 traceback 的重抛或 `raise ... from ...`；其它语言使用等价的 cause／inner exception。没有底层异常的直接领域拒绝可用 `source: null`，不得制造异常。

跨进程与报告边界使用独立、版本化的 `chrono-error/v1` 记录保留等价结构。每一层包含 `type, code, message, context, source`；根因保留实际类型、原始消息及已取得的原生错误码，已取得的栈／traceback 也保留。子进程失败附原 argv、退出／signal、stdout/stderr 字节或不可变证据引用，以及 timeout／overflow 等实际观察；尚未启动时不伪造进程结果。记录及引用由生产者产生，消费者核身份并展开完整链；引用不可用须明确报证据不足。

现有 finding 的 `causes` 仍表示登记图边，不兼作异常链。新记录与其传输字段须通过显式 schema／协议演进接入，不能静默塞入当前严格 v1 请求／响应。

捕获、重试、降级、恢复、转换为领域结果或忽略预期缺失也是处理事件，必须附原始异常链和实际处理动作／结果，即便最终成功。成功只描述已完成的恢复或业务操作，不删除先前失败。重试逐次绑定尝试及原异常；次数、时限、可重试条件由当前登记规则决定，不靠循环直到偶然成功。受控的预期缺失可作为结构化领域状态返回，但若由异常触发，处理记录仍保留原异常。无异常发生的正常缺失不凭空补异常。

处理过程中又发生异常时，原业务异常与新异常均保留，标明哪个是 source、哪个是清理／日志／发布等相关失败，不能在 finally／析构／日志写入失败时覆盖原根因。顶层协议错误、CLI 错误码和短摘要必须指向同一完整错误记录；只有字符串摘要、包装器退出或“已处理”标记不足以证明异常链完整。失败、超时和无结果先读取已有实际错误、输入、原输出及退出，再决定修复；未知结果保持未知。

对任意路径、文本、OS／工具错误、外部响应及其它开放或无限输入集合，按类型、结构、语义不变量和能力边界定义通用处理规则，不逐个样本、消息片段、文件名或测试 case 打补丁。只有已登记的有限协议值／错误码可以枚举；未知成员进入统一且保留原始输入／异常的规则，不把未知当通过。新特殊分支必须具有独立语义和明确适用边界，不能只覆盖曾出现的一个失败文本。检查使用等价类、成功／失败／边界预期及必要反例；通过有限样本不宣称无限集合已穷举。

现役缓存恢复可由宿主 v2 明确采用 `recover_failed_restores`：在独占的原生恢复阶段、原构建开始前，以固定计划和原观察重新核对登记，仅清理 restore 原始 outcome=failure 的登记宿主产物，保留原失败、intent、逐项结果及发布异常，再沿原构建入口执行。登记漂移、路径别名、缺失／未知原生结果、未完成 intent 或清理失败须阻断；已完成尝试的重入只返回原结果，不再次删除后来构建的产物。外部目录和 linked worktree 尚不支持；该机制不替代完整归档校验，也不证明成功 restore 的内容未损坏。报告必须保留恢复原始证据，不能把恢复成功改写为原 restore 或业务检查成功。

宿主还可显式采用 `recover_unconfirmed_restores`，须同时采用上述失败恢复及隔离边界，省略保持旧行为。同一恢复入口在原生步骤 success 却没有命中键、键为空或不在登记兼容域时，也清理对应登记输出，避免下载／解压异常被原生实现转为警告后留下部分产物。原步骤不改判；缺键仍是 miss-or-unavailable，清理不证明缓存原先损坏。

恢复按原 matched key 分类，命中 exact／compatible 或明确 skipped 保留输出；缺失／取消 outcome 及畸形 outputs 在删除前失败。策略绑定恢复计划但不使未变编译缓存失效。普通冷缓存且没有残留产物时不制造恢复错误警告；删除残留、原生失败和后续构建失败仍分别保留。该扩展不校验声称有效命中后的完整内容，也不自动完成原生损坏实验、外部目录或 linked-worktree 的所有权合同。

计划、恢复、内容核验和封存原件由实际生产者保留，原生 action 日志提供步骤观察。当前构建及发布判断不消费额外缓存传输报告，因此不持续拼装或复制该报告。失败诊断沿现有上传能力直接保留；发布诊断使用独立 artifact，不改变正式发布归档根和 collector 合同。原件按明确使用方及既有保留期限清理，不入 Git。

### 7.3 自检的发现能力（目标合同）

Harness 自身的检查、观测、缓存与资源报告也按独立生产／测试单元登记。修复已知盲点时，专属测试须先构造实际异常或等价的受控故障，验证异常确实进入原始报告及面向消费者的失败、警告或未知状态；只验证正常路径通过不足以验收发现能力。适用时先确认旧实现不能通过该回归。诊断故障由实际生产者保留，不能因业务步骤成功、检查器没有输出或查询不可用而报告正常。登记的必需报告无法生成或保存时，报告入口须非零并向上传播，工作流不得整体忽略该报告程序的退出码；已完整记录的可恢复后端异常可保留为非致命警告，两者不得混淆。

发现与修复分别验收：暴露错误不等于已消除原因，一次重跑通过不关闭历史失败。修复保持原功能与测试义务，并核对应异常的报告、恢复行为及未解决边界。已知遗漏须补入真实所有者的回归和登记；不推断依赖，不宣称可以自动发现所有未知缺陷。

观测优先复用现有执行、原始证据、报告与同一 check 入口。新增探测须登记实际消费者、所需读数、界限及成本；无适用操作则不启动探测，不以自检之名新增没有消费者的全仓扫描或常驻任务。测试和观测只能说明已验证范围的发现能力，不能从“没有告警”推出整个系统健康。

独立报告生产者的构建记录须使用明确登记的独立输出位置，不能被后续无关构建覆盖。报告保留实际程序路径、文件摘要与版本，消费者核对其构建记录与源码身份；缺记录、脏源码或身份不符不得声称完成源码绑定。路径摘要是文件观察，不证明完整编译输入或已加载内存身份。

### 7.4 任务完成所有权与等待（目标合同）

每项任务的调用方负责保存实际任务身份、加入本任务及其自有子孙，并核对最终结果；宿主负责提供完成通知或真正阻塞的等待能力。启动前必须确认当前宿主实际可用的唯一完成通道及原结果的取得方式：有主动完成通知就优先等该通知，收到后一次收取原结果；没有通知则使用任务原生的真正阻塞等待。不得同时为同一任务另开完成通道；禁止短周期反复查询状态、进程或产物，不以反复空读会话、sleep 后查询或改名 wait/watch 的短周期循环代替等待。允许授权内对原任务作长周期核实，不因此另开同时运行的完成通道或新增确认门。

真正阻塞的原生工具可以内部查询，但名称、默认间隔或包装器退出 0 均不证明符合等待要求或任务成功。缺少两种能力时，启动前报告能力缺口，不启动无法收取终态的工作；已存在的任务保留原句柄和未知状态。

完成只由匹配该任务身份的原始终态结果及真实退出产生。任务生产者必须实际等待自己启动的子孙和后台工作结束，再发布其所负责操作的终态；调用方仍须加入自己另行启动的任务。启动器、包装器、父进程或一个参与者退出，不证明真实任务、存活子孙或其他消费者已经结束。runner 只控制自己启动的操作，不据此接管宿主、调用方或其他任务的进程。所需原 stdout／stderr、退出／signal 和结果身份按 §7.1、§7.2 保留、传递并核对，不能用包装器退出 0、静默或报告文件存在代替。

等待超时、通知暂缺、通道丢失或结果不可取得不证明任务失败或终结；无法确认的结果保持 unknown。调用方保存原任务句柄、已取得结果和恢复所需身份，沿原宿主通道恢复收取或在授权内长周期核实原结果；尚不能确认原任务终结时不重复启动同一工作，不凭路径或命令名认领、终止他人进程。原失败的读取与恢复动作遵守 §7.2，保护和释放遵守 §10、§10.1；仅阻断依赖该未知结果的工作，独立已授权工作按 §1 继续。不为此另建任务控制层、守护进程或平行状态账本。

### 7.5 所有工具的本地状态与终态追踪（目标合同）

所有产品工具、判官、宿主脚本和插件适配器，每次运行都必须在宿主登记的本地 state 中留下可恢复的状态。命令返回、会话结束或机器重启后，后续调用仍能查明这次操作及其负责对象是否进入终态。状态由对应生产者维护，复用现有操作、worktree、进程及原件保留所有者；已有记录足够时直接补齐，不能再建一份平行账本。日志用于诊断，不能代替当前状态；状态也不能依赖仍在内存中的句柄、终端输出或 AI 的会话记忆。

每条记录须绑定实际操作／尝试身份、生产者、有效输入或其原件引用、负责对象的准确身份、当前已知状态及更新时间、原任务句柄与结果取得方式、原结果和必要错误引用，以及恢复／交接所需的消费者与保留条件。未产生句柄、结果或对象时明确缺失，不伪造已启动或已完成。外部副作用或任务派发前先持久登记 intent；取得实际身份后更新同一操作。记录发布失败必须报告，已发生的副作用保留为待核实，不能报成完整成功或直接重试创建。后续调用沿原身份恢复，不能覆盖其他尝试或把不完整写入解释成终态。

命令执行结果与所负责对象的生命周期分别记录。终态由该对象的登记合同及匹配身份的原始结果确认，区分成功、失败、取消及尚未确认的结果；命令退出 0 不自动终结仍存活的对象。下列要求区分命令与对象状态；本仓 PR 工具的当前采用及验收范围见 [PR 工具](docs/pr.md)：

| 工具或对象 | 本地追踪与终态要求 |
| --- | --- |
| 同步检查、生成等一次性操作 | 保存实际退出及结果；自有子孙已加入、结果已核对后，确认这次操作的终态 |
| worktree 创建／重建 | 创建命令的终态与工作树的活跃、交接、保留／清理状态分别追踪；创建成功后继续保留对象状态，不能据创建进程退出宣称工作树已完成 |
| pr-open | 保存实际仓库、PR 身份和固定提交；打开成功只终结打开操作，PR 仍按其实际 open／closed／merged 状态追踪，实际落地另按 §10 核验 |
| pr-watch | 绑定已有 PR 及原等待通道，保存已取得的原始观察和等待进度；watch 超时、中断或查询失败不使 PR 进入终态，沿原身份继续收取 |

状态追踪沿用 §7.4 的通知或原生阻塞等待，不另开短周期轮询器、守护进程或第二完成通道。无法取得结果时保留 unknown、原句柄、已取得结果和恢复条件；不得因状态变旧、PID 缺失或命令已返回而推定对象终结。只在完成本次登记的全部终态核验后转入终态保留；失败或取消也须有原结果，仍需恢复、复核或交付的引用继续保护。

终态状态像日志一样按 §10.1 的有限保留政策滚动清理，不立即丢弃，也不永久累积。宿主明确登记终态保留期、数量／字节界限及维护触发点；保留期从实际确认终态时计算。只有终态已确认、必要消费者已释放且满足登记保留条件的记录才能滚出；活跃、unknown 和仍需恢复的状态不能因轮转被删除。完整记录、索引、结果摘要及清理回执都受有限保留约束，沿现有维护入口清理；清理状态不授予删除工作树、分支或远端 PR 的权限。过期记录应明确为已回收或不可取得，不能据缺失反推对象从未运行、已成功或已终结。

#### 工作树登记、恢复与清理

worktree 工具必须把创建、重建、导入、复用、使用、交接和清理接到同一个本地生命周期记录。状态保存在存活的协调宿主，不能只留在待清理工作树内。创建前登记 intent，随后绑定实际仓库、物理附件及代际、路径、分支／HEAD、创建结果、当前使用保护，以及已有的任务／PR／落地证据；没有 PR 或尚未取得结果时明确缺失。命令返回后，记录仍须足以接续该工作树，不能只有一份“创建成功”报告。

创建／重建的 intent、原结果及对象登记须绑定实际调用 agent 的明确身份，后续操作保持原创建者不变。身份来自宿主登记的调用方输入；缺失或无效时明确为 unknown，不能用 PID、用户名、目录、分支名或当前接管者推定创建者。显式导入只能登记当前保管者，未知的历史创建者仍为 unknown；创建者身份不授予删除权限，也不替代内核租约、工作保全或真实终态核验。

执行 worktree 创建、重建、导入、复用、使用、交接及维护时，沿同一协调宿主的登记提醒当前 agent：它创建的哪些工作树尚未进入工作及清理的完整终态。提醒绑定原创建者和实际对象，列出路径、分支、当前工作／清理状态、仍未完成的责任、阻碍及下一步维护方式；已交接但仍保留或清理受阻的对象继续提示。当前对象仍活跃时提醒及时维护，在完成工作并加入自有任务后交接；不据提醒自动 finish 或删除。其他 agent 创建的对象不归到当前 agent 名下。无身份、漏登、忙碌或损坏导致观察不完整时明确说明已知范围与 unknown，不伪报“没有待维护对象”，也不阻断独立的合法操作。

提醒由现有 worktree 记录即时产生，不另建重复状态索引、后台扫描器或短周期提醒循环；提醒输出及其原件沿现役有限保留机制退出。相关操作的实际消费者必须能够看到提醒，不能只写入无人消费的日志。进入完整终态并满足保留条件的对象按本节政策滚出后，不再作为未完成对象提醒。

宿主采用的其他客户端创建或复用工作树时，也须接入同一所有者，在开始受管使用前完成登记或当前保管的明确接管。维护可核对 Git 自身登记与已有生命周期声明，逐项报告漏登、空记录、附件变化和未知所有权，按明确目标补齐当前保管；不能只处理已有完整条目，使漏登对象永久不可见。补登保留历史结果 unknown，不伪造历史创建成功、任务终态或消费者释放，也不根据目录名、年龄或分支名推断删除资格。

每棵工作树分别记录工作／交付状态与清理状态。清理记录须明确待处理对象、原尝试及结果、当前阻碍和恢复条件，并区分待清理、受阻、按政策保留和已核实移除。只回收缓存不算工作树移除，PR 合并或 finish 返回也不单独证明可删。创建、使用、交接或清理中断，以及调用方漏调 finish 时，后续入口须沿原记录恢复；生产者在发布阶段结果前持久保存尚未完成的交接／清理责任，不依赖 AI 记得补做。真实终态仍按 §7.4 核验，实际处置仍按 §10.1 排除消费者和保全工作。

保留必须指出具体仍需的源码、提交、证据或消费者，以及解除条件和后续处理入口。不能只因 bin／state 目录存在，就把整棵工作树永久留住。必要原件可由现有保管所有者迁至登记位置并核验原字节、身份及消费者引用；满足释放和保留条件后，原工作树继续进入清理。未知或仍在使用的原件继续保护，不用复制另一整套 state 代替释放。独立目标的失败保留原结果，维护继续处理其他合格目标。

实际移除后须核对物理路径及 Git 登记；可选分支处置另核请求与预期 OID，部分效果仍保持待恢复。只在工作树已核实移除、无未完成清理责任且相关消费者释放后，清理过程记录才按同一政策滚动退出。仍被保留或复用的工作树保有当前对象记录，历史尝试可按条件轮转，不能先丢状态再留下无人接续的目录。

自动生命周期内部产生的不可变回执和保留字节也须登记到同一原件保留所有者。发布时先保护所属地址的使用方引用，再写原字节；回执明确声明其实际输入依赖。账本发布后按当前字段收窄引用，仍在返回或发布报告的调用方由原件能力保护，报告接收依赖后才结束交接。终态历史滚出只释放该地址的引用；其他报告、恢复任务或使用方仍消费的原件继续保留。未明确接管的历史原件不能因启用新政策而被扫描或补写成功。

工作树代际退出时，其内核租约也须退出；先在同一地址登记已核实的退休责任，再重新排除使用方并按精确物理身份回收租约，不能删掉条目后留下无人负责的能力文件。地址没有当前／历史条目、待交接或待退休责任，且工作树已不存在也未在 Git 登记时，维护可回收所属 metadata 文件和空地址目录。清理须持该目录与 metadata 的内核排除，删除前保存有限的恢复声明并逐项核身份和原字节；中断后沿同一声明接续。未知成员、身份变化或活使用方继续保护，不按年龄清目录；真正空的登记目录可以在目录排除下直接回收。

## 8. 默认判官、规则表面与警告

当前 judges.json 登记以下判官；registration、filemap、routes、projects、cost、mixed、workflow 已实现范围有限的约定，宿主绑定仍 proposed，不存在默认隐藏门槛。

| ID | 本轮职责与典型错误 |
| --- | --- |
| registration | schema/引用/白名单/快照/输入闭合及明确消费关系；E_UNREGISTERED、E_SNAPSHOT_DIRTY |
| filemap | 两端图、类型边、归属、影响；E_DANGLING_EDGE |
| projects | 一一配对、项目隔离、所选构建/测试执行；E_TEST_PAIR、E_TEST_FAILED |
| routes | 唯一操作与 argv/证据一致；E_ROUTE_MISSING、E_ROUTE_AMBIGUOUS |
| cost | 显式成本与旧/新影响报告；未知值 W_COST_UNKNOWN |
| mixed | 规则与产品同一 DELTA 的可见成本警告；W_MIXED_JUDGE_PRODUCT |
| workflow | dev/integration、新鲜度、稳定性及登记迁移证据；E_BRANCH_STALE、E_INTEGRATION_REQUIRED |

FILEMAP.surface 显式声明整个文件类别，不通过文件内容、目录名或语言模型猜语义。membership 文件中的部分字段通过 config.semantic_fields 声明为影响判定的字段类别：projects 的 actions 新增/修改/删除涉及治理方法；FILEMAP 已有行的 surface 改动/删除涉及分类。pointer 的 `*` 仅匹配一个对象 key 或数组元素，不是文件路径通配符；行用 path/id 定位比较，不能因 JSON 数组换序误判规则变化。
仓库事实 provenance 与 surface 分类正交：FILEMAP／对应登记必须显式标为 `portable-contract`、`deterministic-projection` 或 `runtime-fact`；后者只能指向 state／CI artifact，不能成为 tracked policy。缺少 provenance、消费者、预算或释放条件时 registration 按 §2.3 报错，不以目录位置推断。

schema_version 改动按登记 schema 迁移处理。对同一文件取 base/candidate 表面分类并集，不能通过本轮重分类遗失旧语义影响。config/judges/workflow 全文件为 judge-policy；独立判官脚本可登记为 judge-implementation。

代码加对应 FILEMAP 普通成员行或真实依赖边，是通常的代码+登记协同修改，不触发混合警告。已有判定字段、动作、规则配置、判官实现或绑定二进制摘要变化属于规则语义变化。此类变化与 product/test 文件修改同一 DELTA 时，mixed 输出 warning 及两组路径、旧/新成本。warning 不升级成 一律阻断、不索要批准、不要求人为确认或另起提交才能继续。integration 是这类规则变更本身的稳定性要求，并非混合提交惩罚。

AI 先尝试在现有规则下解决任务；确需改判官时自主变更并验证，报告为什么和影响多大。从产品源码构建工具的 engine 修改只按显式 stability 登记要求 integration，本身不被当作宿主 policy。现役 mixed 输出和语义选择合同见 [docs/mixed.md](docs/mixed.md)；integration 认证由独立 workflow 消费实际结果与完成报告，合同见 [docs/workflow.md](docs/workflow.md)。

## 9. 成本与执行报告

报告必须包含 `schema_version, status, base, candidate, candidate_tree, context_digest,``registry_digest, executables, tools, environment, effective_inputs, parity, delta, impact, judges, tests, findings, costs`。
报告中的 `environment`、`tools` 和 `effective_inputs` 是本轮生产者观察的事实引用，不是跟踪 policy 的补本。完整原始 JSON、绝对路径、安装清单和 `cargo metadata` 输出只能由生产者写入 `state` 或在线 artifact；报告仅保留必要摘要、来源、owner、消费者和保留／释放状态。CI artifact 过期或本地 state 被清理后，消费者必须报告证据不足，不能从仓库里的旧 JSON 补造当前事实。


executables 为 `{path, sha256, version}` 列表；tools 另记录实际解析路径和工具版本。effective_inputs 记录两端完整有效输入的 ID、实际值摘要及来源证据；摘要存在本身不证明相等或完整。parity 为 `{status: "unestablished", compared_report: null}`；当前不生成成对比较状态。性能测量差异分别报告，不用状态声明替代实际观察。

judges/tests 记录每项的 selected/executed/blocked/retired 状态、退出码和证据摘要。这里 selected 是执行计划状态，完整结果不能停留在 selected 就返回 pass。

costs 报告每个变更文件、影响项目、选择测试及旧删除节点的成本引用和来源。四维向量分别为 CPU 时间、墙钟时间、峰值内存、IO 字节；不折算成虚构统一分数或罚税。展示 declared_before、declared_after、affected_tests、retired_tests 和 unknown 列表。不同资源的峰值不能直接相加；并行 wall 时间也不能当串行总和。可按显式串行计划估算总 CPU/IO，任何输入未知则总估计注明不完整，不填 0。

实际测量放在 measured 下，附工具、样本与时间；不能把构建耗时冒充测试耗时。当前所有成本 unknown 是诚实的初始登记；cost 判官报告 warning，不能捏造基准数据。现役有界成本输出见 [docs/costs.md](docs/costs.md)。

首增量的 `scope: "configured-judges"` 草稿也必须提供上述全部顶层字段。尚无生产者结果的tools/effective_inputs/impact/tests/costs 写 `null`，扩展字段 `unresolved` 按报告 JSON pointer给出原因；不能用空列表、零成本或配置声明冒充已观察结果。`sources` 按目标 JSON pointer列出原始响应在同一报告中的 JSON pointer。

验证通过的响应若提供同名 outputs，原值转入对应字段；多份值相同可共用，冲突或生产者给 null 则保持 unresolved，原响应完整保留。这只传递结果，不由 runner 判断专门政策、输入及其依赖的完整性或具名产物的语义完整性。findings 汇集所有验证通过的响应，逐条绑定来源；部分响应缺失时列明不完整原因，无有效响应时写 null。

executables 仍是 `{path, sha256, version}` 列表：runner 使用自身执行物和编译版本，判官使用实际 process 观察的路径/摘要；未观察的判官版本为 null 并给原因。每个 judge 的 binding 仅是配置元数据，executable_index 才指向已有进程观察，不把 blocked 或未获进程观察的绑定填成执行事实。

草稿 status/退出码只聚合配置判官与传输状态，parity 保持 unestablished；即使 scoped pass，这些未知项仍不是完整治理成功。完整报告、专门生产者及完整输入证据的义务不因此减免。

### 9.1 暴露登记与执行中的资源问题

证据的固定发布地址可以由宿主显式选择保存原件引用，不重复保存完整正文。scoped v3 的 `policy.report_publication = "retained-reference/v1"` 只适用于已准备输入的短 check；固定地址保存 `chrono-check-reference/v1` 的原件路径和 SHA-256，完整报告先独立保留，控制台指向完整原件。未采用保持旧格式，未知值、null 及旧 profile 采用须拒绝。

汇总核固定引用摘要，再按登记上传映射核原件摘要、原始留存路径、实际字节上限及既有全部报告义务；缺失、损坏、越界、链接或引用链不得通过。不删除历史原件，不改变业务失败，不新增 job 或重执行业务。宿主采用前须配套升级生产者及实际消费者；该机制只消除 check 报告固定地址与原件的正文重复，不宣称其它证据已去重。

显式采用 `retained-reference/v2` 时，固定引用格式不变，原件为 `chrono-check-report/v2`。判官 stdout／stderr 各按原始字节独立保留，以路径和 SHA-256 引用；进程身份、退出码、失败、请求与报告身份保留。报告不再另存 stdout／stderr 文本、数字字节数组或独立 response；消费者核原字节后在内存重建进程视图，并从原 stdout 解码唯一 response。

汇总继续执行既有协议、进程、执行物、计划、回执和判词校验；不能因存储投影而改变原业务失败。`report_bytes` 对每份原件累计约束元数据＋stdout＋stderr，固定引用另受相同上限约束。引用及全部原件须经过同一已登记上传映射；缺失、损坏、未登记、越界、链接、重复内联视图或超出累计上限均失败。本地与原生消费者须配套升级；旧版本政策保持原义。不宣称业务回执内部或其它采集证据已去重。

按登记执行成功不证明登记必要、消费真实或资源合理。宿主须把库源码依赖与运行所需发布资产分开登记；前置构建／下载与显式消费合同不符时报错，缺少消费说明时给出具名单元和未验证依赖，不把未知当作已合理。声明只能证明一致性；必要性须由真实消费者和适用的隔离／反例验证支持，不自动猜测或删改依赖。

发现冗余、漏检或误判后，先把实际问题写成能推翻旧行为的回归场景，再修原生产者。回归约束消费语义、完整原测试和登记投影的对应，不把当下的单元数量或全量前置固化为永远正确。独立单元的库调用测试应能在未提供发布二进制时完整执行；实际 CLI 消费仍须取得其明确选择的产物。

成本排查直接使用原生 CI 步骤、日志及线上 API，按当前问题区分测试、编译／bootstrap、缓存准备／传输、证据上传／汇总及排队，并说明测量口径与未覆盖范围。步骤时长之和不是并行墙钟时长或计费分钟；未知峰值／字节不得填零。昂贵或重复工作必须能回到具体调用、输入、消费者和必要性说明。是否需要某项工作由 AI 根据这些证据修登记，不引入人工确认门。

现役发布配方 v5 的可选 `verification_consumers` 以验证单元 ID 登记 `{need, release_assets}`；预检核资产引用唯一且其生产者恰为该单元 `needs`。报告区分 `declared` 与 `unverified`，有发布前置却无说明时输出 `W_RELEASE_CONSUMPTION_UNVERIFIED`。它不证明消费说明真实或依赖最小；本宿主逐项登记全部验证单元的实际发布资产消费。

不为已有原生运行信息持续生成第二份步骤分类、比较告警或摘要。额外资源投影及其宿主登记、专属测试已经退役；汇总实际用于运行归属、job、artifact 和原报告验证的响应继续由原生产者保留。成本调查本身也须有当前问题、实际使用方和有限保留条件，不能以本节自证常驻观察机制必要。

GitHub 当前 attempt API 视图可能包含沿用结果，新 job ID 不证明重跑；不能把不同 attempt 的行相加后宣称总成本或节省。缺读数、排队、峰值、计费和未测范围保持未知，其余单元消费核验与普遍冗余诊断仍未完成。

## 10. dev、integration 与过期分支

现役 `chrono-worktree start` 按宿主 `.chrono-harness/worktree.json`、五份登记及实际远端抓取结果创建新工作树，记录固定 base/tree、分支、开始时间和真实 Git 回执。它只生产操作结果，不产生新鲜度、integration 或治理通过判词；后续仍调用相同的登记 check 入口。创建失败保留工作和身份供恢复，不重置已有分支或删除已有目录。实际合同与当前恢复、PR、合并边界见 [worktree 合同](docs/worktree.md)。

源码增加 可选的 `automatic_cleanup`：宿主在 `.chrono-harness/` 登记版本化政策、准确产物路径／处置、存活协调锚、固定保留分支与可选删分支；未采用的旧配置保持显式维护语义。成功 start/reconstruct 自动登记实际 birth，并在新建前排空已登记终态项；finish 是调用方加入任务并确认完成／落地后的固定交接，运行时观察当前身份、固定保留 OID、产生不可覆盖终态回执并立即调用同一 drain，独立命令为 maintain。

登记消费操作通过 use 复用现有进程 owner 与小型持久使用保护，和 finish／删除同步；任意未登记后台写入仍归调用方加入。缓存处置复用现役产物 owner，不推断语言或目录。宿主须明确登记普通释放即可处置的临时产物，以及留到明确 finish 或合格年龄回收的跨操作复用缓存；普通 bootstrap／check／use 返回不得提前删除后一类缓存，finish 释放应可持久恢复且不授权删除后续代际。未解决证据、脏源码及未保留提交保留 checkout，只清准确授权的可重建输出。整树清理继续委托原 Cleanup 保留合同。原失败、部分效果、缺失／中断及漂移均据实保留，未建立空闲定时清理、掉电事务或通用证据归档平台。

主宿主政策保留 state/bin，默认 cache-only；真实宿主 terminal 事件与 Git/GitHub 落地归调用方。完整格式与限界见 [自动生命周期清理](docs/worktree.md#opt-in-automatic-lifecycle-cleanup)。

自动生命周期重试及显式 `recover`／`cleanup-fetch` 以 `chrono-worktree-retained-input/v1` 引用保留的原始证据，绑定协调宿主、相对 state 路径、摘要、长度及 receipt／原始 bytes 格式。不可变回执直接引用，部分或未封口结果的原字节独立保留；沿合同明确列出的引用核验原件，缺失、改写、绑定错误或符号链接拒绝处置。不得逐层嵌入历史正文导致重试报告递归膨胀，不把保留未知结果改判为成功，不改写既有原件。父报告按现有逐输出保留关系保护所引用的原件；具体字段及消费规则见 worktree 合同。

现役 `chrono-worktree reconstruct` 接收固定旧 base/candidate 和完整逐路径 carry/retire 计划，复用 创建新工作树后重新应用所选 DELTA；成功仅表示已得到暂存树，冲突保留实际失败与锁，旧工作不删除。AI 仍须检查变化是否必要、修复冲突及登记、提交候选，再运行现役统一检查。工具不自动判断语义必要性，也不复制旧检查或 integration 成功。

现役维护入口复用同一宿主工作树配置与登记。`recover` 绑定原失败报告、实际工作树与锁、AI 显式给出的 HEAD 和已解决 index tree；验证后释放原锁，原失败不改写。`cleanup` 按显式路径／分支／HEAD、保留分支及逐项产物处置清理已保存工作。源码现先核所有选中产物为两端登记的非跟踪真实目录，且不含候选树已跟踪路径；在自有锁内单独删除这些产物，再由有时限的 Git 移除 checkout。

逐项报告已缺失、尝试未核实或已核实缺失；文件系统删除不继承 Git 子进程时限，调用方须在宿主作业生命周期中运行。产物阶段失败保留锁及源码，协调后用既有恢复入口和显式重试继续；不承诺并发或掉电事务，也不保证任意规模的源码移除均不超时。最后核对实际移除并用期望 OID 删除可选分支；部分失败如实保留，明确允许缺失工作树的计划可继续重试。不从目录、语言、年龄推断回收对象，也不替代相同的 check 入口。

`cleanup-fetch` 只清理原失败 start／reconstruct 回执绑定的临时 fetch ref，要求固定本地保留分支包含期望 commit，并以期望 OID 删除、核对缺失及部分效果。源码现已在 checkout／锁操作及 fetch 前分别保留不可覆盖的 intent；`recover-interrupted` 核对当前工作树并解锁，`cleanup-fetch-interrupted` 按固定保留分支清理该 intent 绑定的临时 ref，均保留原结果的缺失或原始字节并标原操作结果未知。

调用 AI 须先确认原进程已停止；终态回执仍用普通维护入口，intent 不是操作成功证明。这些入口不重建损坏元数据；显式重建见下款，PR／合并编排仍未完成。格式与边界见 [维护合同](docs/worktree.md#registered-recovery-and-cleanup)。

公开 beta.11 提供 `inspect-rebind`／`rebind`：旧收据、intent 或 linked-worktree 元数据缺失／损坏时，由 AI 显式登记现有目标目录、已有工作分支及 HEAD、要采用的 index tree、原元数据成员、全新备份和 donor 路径。观察只为该范围生成文件内容／mode／字面链接身份，不发现依赖或推断丢失的原索引。

执行前绑定观察和计划、保留 intent，将原 gitfile 与残存元数据保存在登记 state 中，再用同一 Git 创建 locked/no-checkout donor、应用显式 index tree 并 repair 指针；成功须核可见工作、备份、分支及索引，随后释放锁。原操作结果始终未知；重建成功不代表检查或治理通过。

普通失败保留实际输出、阶段、备份和部分效果；已附着且仍持原锁的失败可经既有 `recover` 在 AI 解决工作后释放，源码新增 `resume-rebind`：以原 intent、原计划和明确观察的缺失／部分／失败结果，复用保留与附着步骤完成续跑；原结果与工作保持，冲突报错，完成不追认原成功，beta.15 开始包含此入口。备份 rename 要求同文件系统，不保证并发或掉电事务。

这两个入口已通过 beta.11 的原生发布测试与公开混合宿主消费验证；完整格式与边界见 [元数据重建](docs/worktree.md#explicit-metadata-rebind)。

所有交付最终进入 dev；feature 与 integration 均从当时最新 dev 创建。

宿主默认采用[并行验收、按依赖合并](docs/merging.md)：GitHub 关闭 strict，保留必需汇总检查；PR 验固定合并树，目标 push 验完整实际落地 DELTA。独立 PR 不按编号或创建时间排队，也不因 dev 单纯推进而强制追平或全部重验。检查后形成的新落地树由 push 检测并及时修复；这个默认不承诺严格同树。需要该保证的操作须另行登记并实证其合并执行方案。

下述新鲜度阈值及精确 integration 凭据是显式 full 工作流的合同；本仓仍为 proposed，不是默认 scoped CI 已启用的合并门禁。不能据此重新要求所有普通 PR 串行追平。

现役源码新增 `cleanup-remote`，按显式计划与登记 remote 核对唯一 push URL、工作分支期望 OID、固定本地及远端目标提交，复用 ancestor／same-tree 保留检查，以精确 OID lease 删除并核对远端缺失。真实退出、部分效果与显式缺失重试分别报告；不清理本地工作，不推断 PR 或治理成功。目标引用、配置与服务器观察不构成原子事务；目标在删除期间变化可导致删除后的失败。该入口已包含在公开 beta.10，合同见 [远端分支退役](docs/worktree.md#remote-branch-retirement)。

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

判官语义/绑定改动或 stability 路径变动：先创建 新的 integration 分支，组合拟交付完整变化。
integration 上执行同一 check 指令与注册稳定性测试；成功报告作为 integration 证据。
workflow 判官在显式 integration run_kind 上标记 `integration_run`，执行所需测试后生成证据，
不会递归要求本轮先提供自身成功证据；feature/dev 交付才消费已完成的 integration 证据。
纯普通变化在 feature 验证后可进入 dev；AI 可显式新增 stability 登记扩大测试需求。

integration.json 为 `{schema_version, base, candidate_tree, registry_digest, executables,``tools, environment, effective_inputs, required_tests, results_digest, branch_ref, fork_point, branch_started_at, observed_at, status}`。

绑定 base、候选 tree、两端登记摘要、执行二进制/完整工具链、有效输入与环境、所需测试及结果摘要。证据放 state，不纳入 tree，从而不发生“报告包含自身提交摘要”的循环。进入 dev 前最终 candidate tree 必须完全等于测试 tree，base 必须仍是最新获取的 dev。允许 commit OID 因合并元数据不同而变，只要 tree/base/其他绑定字段相同；报告写明映射。

若合并冲突处理改变 tree 或 dev 推进，重新建立 integration 候选并测试，不能复用旧绿灯。缺失、失败、过期或绑定不匹配证据分别为 E_INTEGRATION_REQUIRED / E_INTEGRATION_MISMATCH。证据额外保留 producer 身份和 proof 中的实际 context、plan、results、inputs 与判官进程。交付必须同时取得发布该证据摘要的成功完成 runner 报告；未完成进程留下的证据不能使用。

输入比较只规范化宿主路径与允许变化的候选 commit 身份，保留原始证据；交付仍执行选中测试。报告来自可信但可能出错的 AI/runner；摘要用于识别误用，不设计对抗 AI 的授权系统。

### 10.1 报告、证据与临时产物的自动清理（目标合同）

自动清理必须覆盖历史报告、日志、测试现场、下载／解压产物和可重建缓存，而不只清理工作树的顶层编译目录。新宿主首次采用须登记并启用有限保留政策；既有宿主通过显式迁移采用，不静默改变旧版政策或证据解释。不得仅因文件位于 `state`、曾用于验证、属于失败现场或被历史报告引用，就将整个目录永久保留。留存依据是当前实际用途、登记的消费者与有限留存要求。
Git 跟踪边界在清理前就生效：`state`、CI artifact、runner 快照、host inventory 和运行报告都属于可重建或有界证据，不得为了“方便审计”复制回 Git。发现已跟踪的运行时事实时，先固定其原始字节和消费者引用，再由自然所有者迁移到在线 artifact／忽略目录，更新 FILEMAP 和消费入口，最后按 §6 退休旧路径；迁移完成前不得声称仓库已满足本节。清理 artifact 不会把它重新变成源政策，过期后也不能以旧摘要伪造当前主机事实。


政策由宿主在 `.chrono-harness/` 显式配置；每类产生物须登记对应所有者、生产／消费操作、准确输出范围、留存理由与释放条件、保留期限、数量上限及逻辑／实际字节预算。独立产生物不得通过全局可变账本、准入锁或全库数量／字节配额相互串行；状态与互斥归准确输出、消费者或实际依赖关系。普通发布／读取／完成只处理这些显式地址；跨对象维护独立执行，局部繁忙或损坏保留原失败并继续无关对象。关系集、单个输出和单轮维护仍须有界。

清理触发点、单轮对象数／时间上限及超限处置也须登记，不使用产品内写死的宿主目录或逐个历史 case 的删除配方。生产者在发布前登记产生物身份与 intent，实际完成后发布路径、摘要、长度、结果和消费者绑定；此程序维护的清单是产生物及其状态的唯一来源，不靠目录扫描、文件名、年龄、反射或 IO 分析推断所有者、用途与依赖。嵌套测试宿主的输出也由原生产者按该登记纳入生命周期，不能借放在证据目录内逃避缓存回收。

| 产生物类别 | 自动清理与留存要求 |
| --- | --- |
| 当前运行／恢复所需状态 | 存活消费者的租约、当前对象登记／指针、未终结操作的 intent、尚需恢复或交付的实际输入按明确角色保护；租约身份保持稳定。历史副本不因与它们同目录而获得相同保护 |
| 工具及其负责对象的终态状态 | 按 §7.5 持久追踪；确认终态且消费者释放后，按登记的保留期和数量／字节界限滚动清理。活跃或 unknown 状态继续保护，摘要、索引及清理回执也不无限累积 |
| 成功检查的完整报告与原始输出 | 操作终结、登记消费者释放且保留期届满后自动回收；可保留有界的提交／输入身份、结果及处置摘要，不默认永久保存完整测试现场 |
| 失败、取消或未知结果的现场 | 保留原异常链、真实结果和当前恢复／复查所需原件；有用的回归程序与最小复现数据归其对应所有者。失败本身不授予整套现场无限留存，释放后按同一有限保留政策回收 |
| 缓存与临时产物 | 下载包、解压副本、测试仓库的已登记编译／增量输出、临时验证程序和产物副本在实际消费／保存结束后按兼容复用及容量政策自动回收；未保存源码、必要实验程序／数据和未保留提交继续保护 |
| 重复内容与会话过程记录 | 同一原始字节只保留一份内容实体，报告、stdout 捕获与下载副本通过核验过的身份引用复用；历史 brief／回答／PR 草稿等过程文件无当前用途时按有限保留政策回收，不另立长期手工权威 |

保留根只能由当前运行／恢复／交付角色、明确保存的有用成果及尚未到期的留存条目产生。消费者必须显式取得和释放所需引用及使用保护；沿合同声明的引用计算需保留的闭包，不扫描文件正文猜引用。旧报告互相引用或形成环不构成永久根；长期结果摘要和清理回执只记录身份及实际处置事实，不将全部历史原件再次强引用保留。不得截断仍需使用的原始输出、错误链或历史事实来满足预算，也不得把清理摘要冒充原检查报告。

同内容复用须核原始字节的摘要、长度与实际内容，保留各次操作的独立身份、结果和完整异常关系；相同内容不表示不同运行获得同一成功判词。新的日志／报告不得逐层嵌入历史正文、同时复制完整字符串和原字节数字数组，或在多个目录反复存整份相同产物。引用到期处置后，结果摘要必须能区分原结果与原件已回收的事实；实际仍需原件的检查明确报证据不足，不伪造可用路径。压缩只能减少存储占用，不能代替释放、去重和自动回收，也不能改写仍被引用的字节／身份合同。

普通 start／reconstruct／check／bootstrap／use 及报告发布入口在登记的时机调用同一现役维护实现；正常操作在加入消费者、完成必要缓存保存并释放使用保护后自动尝试回收。会话意外终止或未调用 finish 时，由后续普通入口消费原 intent 和实际租约状态恢复清理，不要求用户先运行手工删除或由 AI 记得 finish。固定 maintain 只作独立重试／诊断入口；复用 worktree、原进程、状态发布和产物处置的对应所有者，不新增空闲定时扫描器、守护进程或第二套清理平台。

留存期限和容量只能用于已确认释放、符合处置条件的产生物；不得以过期、低 CPU、PID 缺失或超出预算推断活消费者结束。删除前立即重验目标身份、政策／引用代际、实际排除、跟踪／索引与提交保留条件；与新的发布、引用取得及使用保护同步，不能在检查后被重新引用的窗口内删除。存活子孙、当前必要状态、源码和未保留提交不得为了腾空间而回收。受保护对象使预算无法满足时，生产者报告各类实际占用、保留原因、可回收量及登记的超限处置结果，不静默丢证据或假称达标。

清理自身须可中断、可重试，并且留存有界。发布紧凑 intent 后逐项处置，记录真实缺失／部分效果、原异常链、逻辑字节及可取得的实际占用变化；未测量值写未知，不把文件长度当成物理释放量。中断后沿原身份恢复，保留必要失败事实，不制造原清理成功或工作完成。报告登记范围内剩余量和拒绝原因；无关对象的失败不阻断合法当前任务，目标／协调状态失败仍按现役合同阻断，显式 maintain 据实返回失败。清理回执不得递归嵌入历史清理报告或成为无限增长的另一份日志。

旧宿主与未登记历史目录先由各对应所有者显式迁移：列明当前消费者、必要恢复状态、保留根、原件身份、重复关系及逐项处置条件，核原引用后采用同一维护入口。未知归属、仍有引用或身份不兼容的对象先保留并报告，不能仅凭本节批量删 `state`。允许原始内容迁到唯一保留实体并更新已登记消费者；迁移后仍核字节、原结果和实际引用，不运行旧判官、不把旧失败重写成成功。

本节是新增目标合同。现役 v2 可显式采用主检出 cache-only 登记和一个 CI 命令回执生产者的精确产生物／引用释放登记；只由同一生命周期 drain 回收已释放且白名单内的生成输出，保留必要原件和未知历史状态。主检出不获得整树删除权，已存在但未迁移的输出继续保护。逻辑长度、分配块和未知物理释放分别报告；独立 Cargo 项目保留独立 manifest、lock 和 target。该范围有限的实现仍不满足通用产生物留存、容量控制、证据去重、完整自动回收或本节全部验收；实际宿主迁移、原生 CI 与交付尚未完成。


### 10.2 兼容工作树复用、协作与交接（目标合同）

每个 worktree 的现役生命周期所有者维护本地持久状态，复用逐地址登记与原件，不另建跨对象可变成员账本。AI 开始及交接时须能查询未终结事项；日常目标入口提醒当前未完成工作、未知状态及清理保留条件。任务终态与实际清理结果分别展示，缓存已清不表示工作完成，终态保留不表示检出已移除。状态缺失、损坏或附件不符保留未知，不补造历史成功；单项错误不遮住可观察同级对象。提醒不增加人审门、定时扫描器或删除授权，删除仍即时重验既有条件。

选工作区先核已有检出的 Git 登记、实际仓库身份、分支／固定提交、工具和输入绑定、已有材料及当前操作范围；兼容且在授权隔离区内就复用，需要隔离时才创建。会话与目录没有永久一一对应关系，兼容复用也不得覆盖其他任务的未保存工作、索引或提交。只读搜索、diff 与固定修订读取可使用合适的既有检出，固定读取直接消费 Git 对象，不为读取切换共享检出的分支。有写副作用的构建、测试和验证按写操作处理。创建或续接的新 feature／integration 分支仍遵守本节从当时最新 dev 创建及固定证据的合同；复用目录不豁免新鲜度、integration 或原检查义务。

协作参与者显式声明实际语义读取和写入范围，在现有生命周期／操作所有者处协调冲突；互不冲突的编辑可并行。复合暂存／提交等 Git 操作保护实际 index 和受影响 ref；切换检出、重建等整树变换只在该操作期间排除冲突使用，不持有会话全程整树写入独占。使用保护须覆盖仍在访问资源的自有子孙，无法延续保护的子孙须在使用前重新进入。一个参与者退出或释放自己的保护不释放其他参与者／子孙的保护，也不证明整树可删除；实际任务加入归 §7.4 的宿主与调用方，清理资格归 §10.1，不按目录、进程名称或会话身份猜测。

交接须分别说明源码／提交是否已保存、自有操作与子孙是否已加入、适用检查的实际结果、交付是否已落地，以及其他消费者和清理条件是否仍未满足。保存的提交只证明保存字节；操作退出、检查通过或一个参与者交还不证明全部消费者结束或实际落地。调用方按本节核实际交付并交接后，才能调用既有生命周期完成入口；未知项保留，不以交接文字生成成功状态或删除资格。兼容后续任务可在核对已保存工作、已加入自有操作及下一任务绑定后继续复用，冲突或未知使用仍受原保护合同约束。

### 10.3 AI 的 PR 操作

独立 Rust `chrono-pr` 为现有 AI 调用方提供 `pr-open`、`pr-status`、`pr-watch`。宿主在 `.chrono-harness/pr.json` 显式声明仓库、目标分支、必需检查、gh 程序及请求／等待／输出上限；生产和专属 Rust 测试项目、文件、依赖与操作分别登记。GitHub 认证沿用现有 gh，不另建凭据或后台控制层。格式和完整命令见 [PR 工具合同](docs/pr.md)。

调用必须绑定期望 head SHA。打开前核远端同仓库分支，复用唯一同 head／base 的打开 PR，否则创建并回读身份。创建成功仅表示 PR 打开；创建或回读失败保留未知结果与原回执，再次调用先查现存 PR。工具不推送本地代码，不自动合并或清理。跨仓库 fork 和 GitHub Enterprise 不在首版范围内。

查询一次获取 PR 身份及指定提交的检查。等待复用一个原生 `gh pr checks --watch` 通道，间隔至少 60 秒，结束后重新核同一 PR／SHA；不因本地等待程序退出 0 就认定检查通过。必须有完整且无歧义的必需检查集合，所有项均为 SUCCESS 才通过；缺失／运行中保持 pending，失败／取消／跳过不通过，head 变化为 stale，API／协议错误为 unknown。截断或未知状态不能当作通过。超时加入本工具启动的本地任务，保留远端身份，不宣称远端 CI 已结束。

stdout 为版本化 JSON，绑定调用、仓库、期望／观察 SHA、检查、结论和原始进程回执，不输出子进程环境。打开成功与检查通过分开表达；合并后的检查成功也不证明落地提交或发布。源码／替身测试、真实远端调用和公开分发分别验收，不以局部测试宣称 §10 生命周期或完整 SPEC 完成。

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

空仓库没有 base，允许一次明确标记的根提交存储 spec/scaffold/草案登记。初始根提交不是一次通过的 DELTA，也不生成“所有历史已合规”的报告。随后修改都有真实 base；初次启用仍按 §6 由候选判官检查 activation DELTA 和迁移证据，并通过 integration。base.status 为 proposed 时记录 previous_enforcement:none，不能回填之前的成功；根提交之外不得伪造空 base。

草案与 active 的迁移都不执行旧判官；启用必须补齐宿主声明的治理／输入范围并满足 registration 要求，input_closure 为 declared-complete 且无 unresolved。此工程声明不要求普遍隐藏输入证明；现役判官仍可报告 completeness_proven:false。

宿主可显式登记 `chrono-initial-check/v1` 初始化配置，包含完整登记的 `host_config`、具名 `judges` 及进程上限。schema4 的 `canonical_check.initial_profile` 将它与 DELTA `profile` 分别绑定候选字节，standalone v4 provider 显式保留同一 `initial_inventory`，本地和 CI 均使用登记的 bare check。

生产者先核真实 Git 拓扑／事件：真正无父提交进入单候选清单，非根仍须真实 base 并检查 DELTA；缺绑定或 base 失败。空业务清单保留为空，不转入 unit/collection；已有非空 scoped v3 unit 初始化仍保留原显式合同。旧登记保留其 `check --config P --candidate OID --initial` 合同。

普通完整配置不自动降级到该模式；初始化配置拒绝 `--base` 与 `--context`，读取真实 commit header 排除非根提交，包括浅历史及 Git replace 覆盖造成的伪根。

初始化绑定采用 `every-initial` / `inventory`，通过 `chrono-initial-judge/v1` 只传候选端点、配置与登记摘要、实际 checkout／runner／环境观察和显式前驱结果。没有 base、DELTA 或猜测的判官，复用已有进程调用、摘要、严格 JSON 与退出码校验。默认 registration 初始化判官要求草案状态，复用登记引用检查并检查整个初始文件清单；它不执行项目测试、不证明输入及其依赖的完整性，也不启用治理。扩展判官和脚本通过该配置的显式 DAG 登记。

`chrono-initial-report/v1` 的 scope 为 `initial-inventory`，成功状态为 `complete`，base 和 delta 为 null，governance 恒为 `not-evaluated`；实际判官响应和进程回执另行保留。初始化检查成功不会修改草案登记或补写历史合规。配置、请求、报告及已验边界见 [执行合同](docs/execution.md#initial-inventory)。

当前 scoped CI 源使用 `chrono-github-ci/v4`，明确登记必需的 `facts_config`，并可声明 `initial_inventory: {path, profile}`。同一 init/generate 生成初始配置与 workflow，verify 核对二者；本地与 CI 使用登记的短命令，初始事件选择初始配置，普通 DELTA 选择 check_config。v1/v2/v3 长命令 provider 已退役，必须在发布任何生成物前拒绝。不推断判官、语言、路径或摘要。来源与输出均由宿主明确登记，采用时保留定制；生成不认证登记语义，也不启用治理。固定旧发布包及其宿主登记保留原有事实，不产生当前源码的兼容义务。

具体所有权与失败边界见 [CI 合同](docs/ci.md#owned-github-projection)。完整自举来源、工具链输入完整性、当前候选的原生初始事件采用与启用仍是独立义务。

产品自举按登记方式从指定产品提交构建，将结果放宿主 `.chrono-harness/bin/`，
写入具体版本、产品源提交、二进制摘要和工具链证据后，再调用唯一 check 指令。
源提交记录在 state 构建证据中，由绑定摘要连接；产品源码与宿主政策始终分别归类。
更新已生效二进制绑定是规则变化，须 integration；AI 可自行完成，无人审批。
构建失败、缺组件或 draft 配置不得生成成功报告；从本机取包也不得默默解析 latest。

### 12.1 中央测试版分发

产品发布统一维护在 chrono-harness Releases，宿主不跟踪产品源码包或二进制。独立 distribution/专属测试对负责显式发布清单、平台成员、版本与 SHA-256/长度锁定、采用及安装；宿主保留 `.chrono-harness/distribution.json` 与生成的 `install.sh`。本地/CI 都运行该入口，再调用同一 check。平台选择只使用登记的 OS/架构，不读取宿主语言、项目目录或依赖清单。

组合发布包同时生成固定版本的 `install.sh`，支持通过 `curl … | sh -s -- --host-root H` 首次采用，默认选择该清单的全部工具，也可用重复的 `--tool NAME` 显式收窄。Shell 只使用系统工具识别平台、取得并校验安装器及清单，再委托 Rust 的 adopt/install；宿主生成入口固定安装声明的原字节身份。安装不要求 Python、Rust 编译器、sudo 或全局配置修改；可选 CI 与宿主业务工具依赖另行登记。

原生打包固定干净源提交/树，发布组合拒绝不同源、不同成员或重复平台。每份下载先验身份；全部所选成员验证后才替换，普通替换失败尝试回滚并报告恢复位置。显式更新可由 AI 自主执行；摘要不证明来源、完整工具链输入完整性或跨平台同判。完整格式、实现范围和故障边界见 docs/distribution.md。

### 12.2 CI 驱动的公开交付

本仓持续交付的目标是：dev 的实际落地 CI 成功后，自动构建、验证并公开测试版。源码固定为该运行的完整提交和树；测试版版本由当前发布计划的版本核心、原 CI run ID、attempt 和完整源码提交确定，重跑不覆盖另一份公开版本。每个发布单元、使用的原件和最终平台包必须绑定同一发布身份。

普通按需构建继续使用已提交计划的版本。稳定版公开交付要求显式版本目标，该目标必须与固定源码中的计划一致且是正式版本；同时核对该源码对应的 dev 落地 CI。仅上传 Actions artifact 不算公开发布。发布者须核对原 CI 的工作流、事件、分支、提交、attempt 和真实成功结果，收齐全部登记平台及其通过的原报告，再复用现有 assemble 和安装入口。

公开版本不得覆盖。先在 draft 中组合和核对固定资产，全部通过后才公开；失败保留原件和明确恢复边界。重试只能继续同一源码、版本和资产身份，已公开且完全一致的版本可核验复用，冲突则失败。最后核对实际 Release、标签源码、公开资产以及真实安装结果。实现和采用状态由负责文档记录，不能以规范或单元测试代替远端验收。

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
| 第 2.1.1 节的必需证明缺失、未执行，或证明根／结果与真实生产调用不符 | 目标为 `E_VERUS_COVERAGE`，保持 `proposed`／`incomplete`；普通测试或 CI 通过不能代替证明；全仓覆盖门尚未实现 |
| 外部接口适配自身又决定准入、错误分类或状态，或遗漏接口假设／实际输入到 Verus 的调用绑定 | 仍属未闭合的证明义务，不按实现语言或外部脚本豁免；纯接口观察不能声称仓内决策已证明 |
| 暂存/未暂存/未跟踪非生成物变化 | E_SNAPSHOT_DIRTY，不偷偷只判 committed 子集 |
| 浅历史缺 base 或 fork_point | E_HISTORY_MISSING；补取后重跑 |
| 判官退出成功但 stdout 为空/非法 JSON | E_PROTOCOL，exit 2 |
| 插件崩溃/超时/摘要不匹配 | error，exit 2；不得 fail-open |
| 全部成本未知 | W_COST_UNKNOWN，unknown 明示，不编造测量 |
| 替换/退役判官，旧二进制缺失、崩溃或不支持新 schema | 当前 migration_validator 核对保留的旧事实与迁移证据；candidate evaluate + integration，无旧执行前提 |
| 无 DELTA | 报告 delta=[]，不裁决历史；输入/分支条件仍检查 |
| 首个根提交 | bootstrap/no prior base，无绿色 DELTA 报告 |
| 当前 CLI：check | **已实现**独立 chrono-ci-check/v1、显式 Git 绑定的 v2 与带 --context 的 chrono-judge/v1；完整宿主仍因 proposed/缺绑定/未实现义务非零，具体范围见 §14 |
| 当前 CLI：未知命令/信息命令多余参数 | **已实现** E_USAGE，exit 2 |
| 已退役 CLI：spec status / parity | E_USAGE，exit 2；不写状态或改写报告 |
| 当前 CLI：help/version | **已实现** 信息输出，exit 0；help 明示完整治理未实现 |
| 程序输出诊断／进度／告警或处理异常 | §7.1 标准日志记录与实际上下文；机器 stdout、结果输出及子进程原始字节合同保持完整 |
| 根因经多层包装并跨进程／报告传输 | 可从顶层记录恢复每层 source、原生信息及实际进程证据；只剩摘要字符串不满足 §7.2 |
| 异常已捕获，重试／降级／恢复最终成功 | 原异常与每次处理动作／结果仍可读取，最终状态由实际完成结果产生 |
| 处理／清理／日志发布期间再次失败 | 原业务异常与新相关异常同时保留；引用发布失败不伪造可用报告 |
| 成功报告及完整现场已释放且保留期届满 | 后续普通入口自动回收，保留有界结果／处置摘要；无需手工 finish 或删除 |
| 历史报告引用成环，已无有效保留根 | 不因历史互相引用永久保留；按声明引用与处置条件自动回收 |
| 多份报告／stdout／下载副本含相同原始内容 | 核字节后复用唯一内容实体；各操作身份、原结果与异常关系仍独立完整 |
| 临时测试宿主的 SDK／编译／增量输出位于 state 内 | 原生产者显式登记并加入清理生命周期；目录位置不授予永久留存 |
| 会话在产生物发布或回收期间中断 | 后续普通入口按原 intent／租约恢复，报告真实部分效果和未知原结果，不制造成功 |
| 已超过容量／数量预算，但仍有存活消费者或必要恢复状态 | 保护有效保留根，报告占用和保留原因并执行登记的超限处置，不靠预算强删 |
| 清理判定后出现新引用、身份漂移或未加入子进程 | 发布／引用取得／处置同步，立即重验；不能确认排除时保留目标并报告 |
| 原件已按政策回收，摘要仍在；或清理自身反复重试 | 明确原件已回收，实际需要原件的消费者报证据不足；清理摘要／回执本身有界且不递归膨胀 |
| 既有宿主有超大旧报告、未登记状态或重复 CI 产物 | 所有者显式迁移、核引用和处置条件；未知项保留并报告，不批量删除整个 state |
| 开放集合出现未见过的合法输入或失败变体 | 按类型／结构／不变量及明确边界处理，未知异常保留并传播；不依赖 case 名或错误文本补丁 |
| CI 冷缓存与后续精确命中 | 观察真实恢复／保存与当前执行物核验；所选判官及测试在两轮均实际执行 |
| 源码变化，兼容旧编译缓存可用 | 同一登记构建入口增量重建；复用指纹／依赖／增量数据，候选判官源码及绑定更新 |
| 工具链／target／profile／features 或适用构建输入不兼容 | 拒绝不兼容恢复／直接执行物，按登记冷构建或重建；不能用宽前缀冒充精确命中 |
| CI 缓存未命中／损坏／服务失败 | 保留原异常及处理记录，沿同一入口重建；构建或当前执行物绑定仍失败时非零 |
| 独立 job 并发／重跑，或编译成功后测试失败 | 活输出隔离、生产者加入后保存；可保留完成的编译缓存，原测试失败及本轮证据不被缓存状态覆盖 |
| 生产单元及其专属测试、断言和测试 helper 使用同一种语言 | 按显式配对和 DELTA 运行对应测试，不引入额外语言要求 |
| 除 Shell／Python 配对外，登记或实际执行证据显示不同语言承担测试逻辑，包括源码字符串／解释器参数／薄包装 | 配对判官拒绝并指出生产单元、语言声明及不符的测试入口；不能凭外层文件语言放行，也不声称自动识别任意隐藏逻辑 |
| Shell 脚本显式配对 Python 测试 | 允许该配对；继续核对独立归属、依赖、操作及实际测试结果 |
| 混合语言宿主分别登记各生产单元与同语言测试 | 各单元独立选测；不推断宿主语言、不合并测试项目，不引入 Rust 工具链前提 |
| 登记图含循环、共享前置、菱形依赖和不同边类型 | 有限影响闭包保留原边、全部种子和路径见证；只有 test-execution 边选测；执行 DAG 另由现役 routes 检查 |
| 同一逻辑单元拆分、改名或合并前后比较 | 固定两端与显式对应；保持测试／接口义务，比较相同逻辑种子，不靠粒度变化制造收益 |
| AI 按依赖候选拆开可独立构建／测试的单元 | 实际代码、配对、输入、操作和 CI 一并迁移；同一 check 选择真实依赖并验证原有行为 |
| 仅删登记边、必要测试或制造空壳使图指标改善 | 不授予结构优化完成；缺失登记、执行或替换证据由对应判官报错，未知保持未验证 |
| 优化某维改善但其它维退化或成本未知 | 分维记录变化和登记权衡，unknown 不填零；无实测不声称加速、等价或全局最优 |
| 自主结构优化完成或中断，出现与 DELTA 无关的旧热点 | 完成须核真实 PR／落地与适用检查；中断保留未完成状态，旧热点仅为上下文，不全仓判红或要求人介入 |
| 新功能仅有专属测试、文档配方或臆想的未来调用 | 不作为功能交付；先核真实现存问题与消费者，必需中间实现随实际接入一起验收 |
| 消费者在外部仓库或真实用户操作中 | 显式登记身份、固定版本、实际入口、输入输出用途与接入证据；证据不可取得不伪造消费或成功 |
| 任一功能、字段、依赖、脚本、判官、CI job、缓存、报告、测试或文档无法追溯到当前必要性链终点 | AI 保留未解决结论；不实现、不激活、不加入构建／CI／保留，已有对象走 retirement／迁移 |
| 必要性只写“未来使用”“方便审计”“保持一致”或只引用一次 CI 绿色 | AI 继续追问当前消费者、失败影响和可观察验收，不能以登记齐全代替必要性 |
| 对象有真实消费者但中间依赖没有独立必要性链 | 沿消费者分支补齐生产者、输入、输出用途和终点；链条未闭合前保持 unknown，不假称完整 |
| 专属测试是某功能唯一所谓消费者，但没有产品、外部接口或当前 SPEC 终点 | AI 核查真实终点；测试不能单独赋予无用途功能交付资格 |
| 必要性链终点的消费者身份、版本、入口或证据不可取得 | 保留 `unknown` 和原始缺证；不转为 active，不新增替代机制或人工确认门 |
| 受影响功能的消费者缺登记、已退役或实际调用不符 | 对该 DELTA 报具体登记／引用／证据错误；保留未验证边界，不自动猜消费者或全仓判红 |
| 启动任务前宿主有完成通知、只有原生阻塞等待，或两者均缺 | 只选实际可用的一个通道并明确原结果取得方式；两者均缺时报能力缺口，不启动无法收取终态的任务 |
| 等待超时、静默或通道丢失，原任务尚无匹配终态 | 保留原句柄、已有结果及 unknown；恢复原任务收取，不重复启动；独立已授权工作继续 |
| 启动器退出而自有子孙仍在运行，或收到另一任务的终态 | 不宣称本任务完成；实际加入自有工作并核对匹配身份、原退出和结果后才交接 |
| 固定修订只读或兼容既有检出承接后续任务 | 只读不切共享分支；复用核身份、已有工作和绑定，新分支仍从最新 dev 创建且保留原验收义务 |
| 协作编辑、index／ref 操作与整树变换发生范围冲突 | 在对应所有者协调实际读写及 Git 范围，只在冲突操作期间排除；其他参与者／子孙的使用保护持续有效 |
| 已保存提交或检查通过，但尚有其他消费者或未核实际落地 | 交接分别报告保存、加入、检查、落地和清理边界，不据单一退出／成功释放其他消费者或删除工作树 |
| 任一工具返回后会话结束或机器重启 | 从本地持久状态恢复原操作、对象身份、已取得结果和恢复条件；不依赖终端输出或会话记忆，不重复派发未确认终结的工作 |
| worktree 创建或 pr-open 已成功，负责对象仍活跃 | 分别保留命令终态与对象状态；PR 打开不等于合并，工作树创建不等于交接或可删除 |
| pr-watch 超时、中断、查询失败或结果身份不符 | 保存原观察、句柄及失败，未确认的 PR 状态保持 unknown；沿原等待通道恢复，不伪造对象终态 |
| 状态发布中断，或外部副作用成功但本地结果发布失败 | 保留原 intent、部分写入及发布错误，沿原操作身份核实；不报完整成功，不重新创建同一对象 |
| 已确认终态的状态超过保留期或数量／字节界限 | 只滚出消费者已释放且满足政策的记录；活跃、unknown、必要恢复状态仍保护，超限据实报告 |
| 终态状态滚动清理，或清理后再次查询 | 记录、索引、摘要和清理回执均有限保留；已回收明确为不可取得，不反推成功、不额外删除工作树或远端对象 |
| Git 登记的工作树没有对应生命周期条目、只有空记录，或由其他已采用客户端创建 | 维护逐项报告差异；受管使用前明确接管当前保管，历史结果保持 unknown，不猜删除资格 |
| 显式导入仍保存退休配置格式或不同产物登记的旧检出，或只查询其状态 | 导入按调用宿主现役政策绑定目标原提交的输入，保留原配置、源码、HEAD 和索引；观察登记不要求目标采用调用宿主的格式或全部产物范围。未提交输入拒绝，后续消费／迁移继续严格核现役格式及原绑定，实际处置仍核两端路径登记 |
| 显式导入 detached HEAD，包括尚有未解决索引的检出 | 保留原 HEAD、索引、源码和未知历史；按实际 detached 标志、提交和物理关联维护本地状态及提醒。未解决源码继续保护，合格清理不删除其他分支；年龄保存先保全原索引与 reflog，仅更新该检出的 HEAD |
| 工作树创建／交接／清理中断，或漏调 finish | 存活协调宿主持久保留原身份、尝试、阻碍及待交接责任，后续入口恢复；不以进程退出制造任务终态 |
| 缓存已清理或 PR 已合并，但工作树仍存在 | 分别报告工作／交付状态与清理阶段；按实际保留项、消费者及提交保存条件继续处理，不报整树清理完成 |
| bin／state 中必要原件已由所有者接管并释放，或清理只完成一部分 | 核原件与引用后继续原工作树清理；核实路径及 Git 登记已移除前保留恢复状态，活跃／保留对象的当前记录不轮转丢失 |

<a id="14-implementation-boundaries"></a>
## 14. 实现范围与限制

第 2.1.1 节的全仓证明覆盖、源码根与真实生产调用完整绑定，以及 `E_VERUS_COVERAGE` 覆盖门仍未实现。现役局部证明与结果合同只覆盖明确登记的范围，不能据此启用完整治理或宣称全部 SPEC 已证明。

§1.1 的全对象必要性审计，以及 §2.3、§7.4、§10.2 的完整迁移和行为验收尚未完成。本仓已退役没有现役调用方的主机盘点、额外安装及 CI 事实副本；scoped Git 身份改为运行内实际观察，Cargo 配置使用仓库内相对目录。已交付范围、实际入口采用与剩余缺口见[覆盖表](docs/spec-coverage.md#当前必要性与数据边界)，运行结果直接查对应 PR／线上原件。已有进程回执、受保护消费与 worktree 恢复仅证明各自已测试范围，不证明所有对象已有当前必要性终点、所有主机输入已迁移、任务级唯一完成通道、丢失通道恢复、全部自有任务加入或完整协作检出合同已实现。其余对象继续按 §1.1 和 §2.3 核对使用方、必要性、存储边界及实际验收；未验证项保持未完成，不能用规则文字或文档检查代替迁移和行为结果。

§7.5 的所有工具本地状态及终态滚动清理仍是目标合同。现有进程回执、worktree intent／result 和原件保留只覆盖各自已采用的操作，不证明所有工具均可跨会话恢复、命令与对象终态已分离，或终态状态及其索引／摘要已统一有限保留。现有 `chrono-pr` 已提供 `pr-open`、`pr-status` 和 `pr-watch`，实现与验收范围见[PR 工具](docs/pr.md)；PR v2 已在现有输出保留所有者保存调用前意图与返回后原件，并分别记录命令结果和 PR 状态。已返回且封存的 PR 操作可显式接续一次只读状态查询，沿现有保留所有者释放被接续的旧恢复引用；当前 OPEN 及独立调用继续保护。实时启动句柄及部分流已接入同一 runner，启动原件只保留环境摘要；返回后直接核对并引用原流，结果封存后交接临时发布保护；接续先核实际原件，再补完封存后中断的明确输入交接，旧结果保持不变。真实外层中断的测试验证原件可读、原结果缺失及 unknown 继续保护。无完整结果的跨中断自动收取和全部活跃原件有界轮转仍有缺口。后续须在现有所有者实现并验证其余工具接入、中断恢复及滚动清理，不能以规范更新宣称运行时已实现。

工作树的跨客户端登记、漏登核对与当前保管接管、无需记得 finish 的完整交接恢复，以及证据释放后的整树回收也尚未完成。现役维护只消费已声明项；缓存回收、保留理由或一次 finish 成功，不证明所有本地工作树已进入可恢复的清理流程。上述缺口须在原 worktree 和保留所有者处验收，不通过扩大删除范围、整目录丢弃证据或提前回收当前状态解决。

本仓旧输入分支的原生初始盘点已退役：当前检查和发布没有消费其全工具安装及主机／SDK／Python／配置盘点输出。删除对应工作流、转发与探测脚本、宿主观察声明、专属动作及测试；普通 core／所有者 bootstrap 的实际 fetch、构建和安装保留。不将旧盘点列为待激活义务；仍有用途的历史原件按现有保管合同处理，退役不证明输入齐全或历史验收通过。

§2.2 的语言声明与受影响配对检查由 projects schema v2 承载，v1 宿主不自动启用。本仓通用执行、历史转换和报告工具观察回归由 Rust 测试及子进程 helper 实现；Python 业务示例及其配对测试保留，真实跨语言接口 fixture 仍须按该节独立登记。本仓已采用 v2；示例的 v2 采用、helper 的完整语言登记及完整同语言约束验收仍未完成，声明检查不证明源码的实际语言。

§4.2、§7.1 与 §7.2 是尚未完整验收的目标合同。当前实现已有 stdout 协议隔离、原始进程字节／退出和部分失败证据保留，结构化日志与跨层异常链尚未全面采用。缓存键与 provider 投影已有候选实现，主宿主显式登记实际检查／Rust 发布作业、需求、用途和项目编译产物，生成固定版本的 restore/save action；当前运行输入与编译源码分别取有界命令观察及 FILEMAP 显式闭包。

生成成功不证明实际恢复、保存或增量复用；v2 消费者引用原操作登记，恢复前拒绝生产操作已退出计划的缓存订阅，消费者增减不使未变产物失效。原生 action 日志保留实际读数；save 步骤成功本身不能确认后端已保存。持续 GitHub 缓存后端查询、相关政策、凭据注入和专属测试已退役：查询结果不参与当前构建、恢复、封存或发布判断，需要排障时直接读取线上 action 日志和 API，不维护本地后端清单副本。额外缓存传输报告、CLI、步骤快照复制及消费者上传目录字段也已退役；普通 CI 复用失败诊断上传，发布用已有固定 action 单独上传失败 state，保留正式发布归档根。恢复、内容核验、封存和发布诊断仍使用的原件继续保留。

启动构建成功的显式子集可在后续检查失败并结束后保存；完整原生验收、损坏恢复、任意项目的测试失败后独立保存、完整适用输入及 linked worktree 全程租约仍未完成。现有 target、bin、名为 cache 的目录和 artifact 传输本身不满足缓存合同。具体实现边界与待实现项见 docs/ci-cache.md 和 docs/spec-coverage.md。

宿主管理的原生发布配方只接受 `chrono-release-build/v5`，旧格式在启动工具或创建输出前失败。明确的 build、verification、collector 单元保留 `chrono-release-unit/v1` 回执，含真实源码观察、进程 argv／cwd、原 stdout／stderr 文件、哈希和退出。本地驱动保存 `chrono-release-local/v1` 及同一单元证据。预检拒绝无效登记和已有输出；无论构建成功或失败，原生上传都尝试保留产物。目录或报告存在不代表发布成功。

当前配方明确声明 Rust 组件、逐 verifier 工具链选择、构建依赖、release-plan 资产及消费目的地；目的地须由一个 untracked 宿主产物声明接纳。staging 前拒绝 Git 已跟踪目的地、链接及源／目的重叠。验证或打包前暂存选中字节和 mode，每项操作前后核身份。漂移会阻止成功；子进程失败后，即使额外观察失败也保留原退出。这些有限时点观察不能证明期间无临时替换、硬终止／存储恢复、完整构建输入、所有测试调用 CLI 或原生 full 治理。配方约定归 `.chrono-harness/release/`，见[发布与安装](docs/distribution.md)。

`chrono-github-release/v1` 在宿主 `.chrono-harness/` 明确声明 GitHub 发布构建投影，由现有 chrono-ci init／generate／verify 选择。每个 job 写明 runner、完整 argv、时限和字面产物目录／名称，生成器不推断语言、项目结构或配方。它与检查投影共用预检／写入器，但使用独立所有权标记。verify 不修漂移，未知声明和所有权冲突均失败。构建失败保留退出，`always()` 尝试上传产物；本地／CI 使用同一登记命令。

checkout 的 source 是请求修订，生成器不认证其为不可变 OID；构建消费者负责源码身份和发布验收。生成不发布 Release，也不证明输入完整或同判。检查 schema 语义保留，宿主采用和限制见[发布 CI](docs/release-ci.md)。

runner 负责 Git／输入事实传输、真实入口 argv／cwd、有限进程观察、候选执行物绑定、协议和具名输出汇总。启动后证据发布失败时，仍将已捕获流、退出和进程身份保为失败证据。绑定 Git facts 使用 `chrono-git-facts/v2`；消费者在 Git 执行前拒绝 v1、缺失或未知 schema。能解码历史进程不代表兼容 v1 facts 请求，见 [Git 事实](docs/git-facts.md)。

registration 负责严格当前 schema、受影响引用、固定快照、登记就绪、所供保留输入验证及有限历史解码；filemap 负责显式类型化 DELTA 并集、记录、因果影响及必需测试事实；routes 负责唯一登记方法、有序计划、单次工具绑定和回执比较；projects 负责受影响双向配对、显式所有权／输出隔离、真实执行、依赖阻断和必需替代执行。通用核心没有语言或目录专用语义。这些项目及专属测试各有独立 manifest、lockfile、target。

当前 FILEMAP v2 执行及 workflow v2／v3 历史 profile 约定见[执行说明](docs/execution.md)。Config 支持 v1 及明确存在状态的 v2／v3／v4，projects 支持 v1／v2，judges 用 v1。Workflow v3 按两端精确 schema 版本选择登记的宿主 decoder，含保留旧输入语义的 config v1→v2 兼容验证；它不自动写宿主配置，严格旧 v1 Reader 会拒绝新字段。

本仓 `workflow.json` 的 `historical_profiles` 为空，旧宿主专用 decoder 及其样本、对象获取和迁移测试已退役。通用转换接口的当前登记和验证边界由[执行说明](docs/execution.md#历史格式转换与按范围-ci)维护；已完成的宿主迁移过程归 Git／PR 历史，不产生继续维护旧路径的义务。

scoped judge-ci 经明确适配器复用 routes 计划及 projects 执行／回执逻辑。当前宿主操作顺序只由 FILEMAP 管理，scoped 选测与环境的适用范围仍按当前合同标明。ci 负责 workflow 生成与事件准备，instructions 独立。bootstrap 使用登记的 Rust 工具链和 locked build action；构建、验证、CI 与存储总成本未测。

scoped `chrono-ci-check/v2` 必须明确 `policy.facts_config` 为 `.chrono-harness/` 下路径，在获取对象前选择候选 full-v3 或 schema4 Git 绑定。schema4 可只声明工具并显式采用 null 版本预期；同一运行的实际执行物身份和原结果由读取器产生并逐调用保护。需要预期身份的消费者须另有 §1.1 的必要性终点，不能为了记录环境而维护主机白名单；显式预期仍严格核验。同一 Reader 读取两端、原 commit parents、index／dirty 事实、可选 full-registration 解释及执行后事实。

原始 commit parents 的物理头选择直接使用同源 Verus `commit_parent_records`：只以 LF 分行，
在首个空行停止，仅选精确 `parent ` 前缀，完整保留原后缀、顺序和重复。
空后缀、CR、NUL 和非 UTF-8 字节均不在选择层改写或丢弃；续行及正文不冒充 parent 头。
返回完整序列和终止由 `commit_parent_lines` 及实际循环证明；`facts::commit_parents` 仍负责
原 UTF-8 解码、完整 OID 检查与错误分类，这些外围组合及 Git 获取仍有独立证明义务。
切片和 Vec 存取使用固定 vstd 的明确外部合同，独立预期和原运行测试沿同源入口验证。

完整 OID 的文本准入由同一标识符根的 `full_oid` 实现，`facts::full_oid` 只重导出。
成功当且仅当完整 UTF-8 字节恰为 40 或 64 位小写十六进制且不全为零；
失败完整保留 `expected full nonzero lowercase commit OID`，不归一化或截取输入。
独立 Verus 预期及普通运行测试共用实际实现，核边界、非法字节及原错误全文。
文本借用／诊断构造沿用固定 vstd 的外部合同；不据此宣称 parent 解码／收集组合或 Git 获取已证明。

Git tree／index 库存的局部准入也直接使用同一标识符根：空输入或末字节 NUL 才通过终止检查，
否则完整返回 `incomplete Git inventory`，且该检查先于 UTF-8 解码。
标准 UTF-8 接口成功时完整借用文本，失败由 Verus 返回 `non UTF-8 Git inventory`。
NUL 记录选择、首 TAB 分割和 metadata 的 ASCII 空格拆分也使用同源 Verus 循环。
记录选择忽略全部空项，保留非空项的顺序、重复和完整字节；metadata 保留首尾及重复空格产生的空字段。
首 TAB 缺失时保留 tree／index 各自原行错误；成功时完整保留 metadata 和其后全部 path 字节，包括后续 TAB。
以完整输入字节递归定义结果序列，证明实际数组的数量、每项完整字节和循环终止。
仅在已证明的 UTF-8 字符边界使用固定 vstd 的 `str::split_at`，不另信任仓内拆分函数。
tree metadata 恰为三字段；index metadata 还要求 stage 恰为一个 ASCII 字节 `0`、`1`、`2` 或 `3`。
字段数／stage 失败分别完整返回 `invalid Git tree metadata`／`invalid Git index metadata`，
且仍先于路径和 OID 校验。不额外限制其余字段，不改写空字段。
这些成功／失败、拆分结果及原错误由同源 Verus 及独立预期拥有；
标准安全 UTF-8 解码仍是明确外部接口；index 调用组合及文件系统状态比较仍待证明。

tree 的实际条目构造、map 更新和重复路径拒绝由同源 Verus `insert_git_tree_entry` 拥有。
新 map 恰为原 map 在该完整路径插入原 mode／OID／kind，其他全部键值保留；
成功当且仅当原 map 不含该路径，重复时完整返回 `duplicate Git tree path`。
保留原 insert 顺序：重复条目已经替换，外围随后丢弃整棵失败候选树。
不在此状态函数增设路径或字段限制；原 metadata、路径及 OID 检查仍在调用前执行。
独立 Verus 预期与普通运行共同核完整状态、原错误及空／前缀／NUL／Unicode 身份边界。
固定 vstd 的 map 插入合同只在本仓已证明的 TreePath 比较法则下使用。
tree 的完整逐行循环由同源 Verus `parse_git_tree_rows` 拥有：依次执行首 TAB、
metadata、路径、OID、重复检查；返回全部且仅有的有效条目及完整字段，或首个失败。
前缀规格保留原顺序和两两不同的完整路径，map 规格双向对应所有前缀行并核实际数量。
标准 map 的完整严格有序迭代合同连接实际键与全文身份，不假定新分配 String 的实体相等。
动态路径错误保留原借用路径，证明后的分支只向明确外部标准 Debug 格式化接口转交全文；
固定错误原样返回。原字节终止检查、标准 UTF-8 解码与 NUL 拆分仍在外围按原顺序调用。
独立 Verus 预期同时在普通 Rust 核完整 map、字段、数量、首错、重复及原路径。
index 重复 stage、文件系统状态及全仓覆盖门仍待证明，不据此扩大完整覆盖结论。

候选 profile／facts-config 字节须匹配固定 blob；构建后绑定失败和后续失败都在 `evidence.git_facts` 保留结构化进程原观察。可选 `registration_config` 独立采用原有登记／canonical 调用语义，单独 Git 绑定不要求完整登记。v1 拒绝新字段（含 null）。选测、initial 规则、操作环境、协议及唯一 check argv 保留 scoped 合同，base enforcement 写明真实端点 schema。

这批历史实现时产品宿主尚未采用 v2；此能力不证明 Git／委托输入完整、full 治理或同判。

context 可通过 retained_inputs 引用两固定端点的外部文件原字节、按内容寻址的 blob 及环境快照。独立 chrono-inputs capture／pair 生产者和流式验证见[输入说明](docs/inputs.md)。registration 核摘要、当前候选字节、缺失／空变量及候选观察；哈希和声明不证明未披露输入完整。正向项目 fixture 在无 manifest／lock／root、自定义 action 和任意文件位置下执行登记方法。

仓库 ci-tests 的真实 Cargo 执行由 scoped 迁移消费者覆盖。本宿主仍 proposed／incomplete，full 治理仍非零；普通 Cargo 执行不证明 compiler／SDK 或未披露输入完整。

当前 scoped CI provider v4 必须明确 `facts_config`，为每次事件准备读取／fetch 选择候选 full config v3 的 Git 绑定。网络观察前核候选政策字节和检出身份。有限 batch probe 只接受精确请求 commit 或精确缺对象响应；只有后者允许 fetch。畸形输出、错误对象类型、执行失败都保留原进程证据且不 fetch；fetch 成功后再探测并核 commit。context 携 `git_facts`，准备失败保留观察、不虚构完成 context。

旧 scoped provider v1／v2／v3 已退役；v4 可明确加入 initial 清单投影，不会推断。固定旧发布包的登记不产生当前源码的兼容义务。绑定不证明网络／凭据／Git 配置完整或当前候选的原生采用，见 [CI 说明](docs/ci.md)。

CI 事件准备允许明确 `push_baselines` 前缀及 origin 基线 ref：匹配的 integration 创建／修复 push，以完整候选对比固定观察 dev 顶端；未配置 push 仍用 before／after，见 docs/ci.md。PR 以同次运行固定 workflow merge commit 的第一父提交为基线、该合并提交为候选；必须恰有两个父提交且第二父提交与事件 head 完整身份相等，否则失败。事件载荷中较早的 base SHA 不决定选测，依赖 job 消费 detector 已固定的两端。准备不认证 workflow 义务。workflow 用真实观察和完成报告核有限分支、integration、退役、迁移合同，见 [workflow 说明](docs/workflow.md)。

完整 Cargo／SDK／外部输入、超出明确根清单 profile 的 initial 采用／bootstrap 来源、原生 full CI／同判、AI 协调与陈旧重建全流程、PR／merge／落地及最终完整 SPEC 审计仍未完成。单个执行者的本地行为检查不证明独立评审、原生 CI 或落地；指令生成的平台／崩溃／并发限制仍有效。原编号要求和每项验收目标全部保留。

## 15. 实际参考经验

只读参考 trureturing 提交 `fbac8071cc862c8a43a247db2d8101dc61f655ea`：

- [Meta/FILEMAP.toml](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/FILEMAP.toml)：已有 resources/materials/require 登记；本设计保留显式影响，进一步逐文件化、区分边类型。

- [Meta/engineering-projects.json](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/engineering-projects.json)：明确 production、专属测试、references/build_inputs/execution_inputs；本设计避免从源码推断测试。

- [harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/.github/scripts/harness-gate.sh) 与 [local-harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/local-harness-gate.sh)：实际入口传入 base、候选和报告；本设计进一步固定同一 argv，去除本地/CI 入口差异。

- [judge-content-address.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/workflow/judge-content-address.sh)：判官地址包含 source/runtime/architecture/sdk；本设计同样绑定执行物与工具证据，避免复用错报告。

这些是经验来源而非兼容承诺，不复制已有复杂体系，不读取其运行时配置，不把它当依赖。
上述参考材料不因引用而被本产品修改；第 2.1 节的通用数学成果通过 trureturing 独立工作树与 PR 交付。本产品没有宿主全局 hook、权限门禁或人工流程。

另只读核对 trureturing 固定提交 `1239bd2a0ed6e9d54aabb5a42af94234e9eb61ae`，补充采用范围如下；上述 `fbac8071cc862c8a43a247db2d8101dc61f655ea` 示例保持各自历史来源：

- [CLAUDE.md §1.2、§2.10](https://github.com/the-omega-institute/trureturing/blob/1239bd2a0ed6e9d54aabb5a42af94234e9eb61ae/CLAUDE.md)：复用唯一对应所有者、保留结果与真实边界的原则；本产品的权威、记录与留存仍由 §1、§7.1／§7.2、§10.1 定义，不建立重复账本。

- [CLAUDE.md §8.6–§8.8](https://github.com/the-omega-institute/trureturing/blob/1239bd2a0ed6e9d54aabb5a42af94234e9eb61ae/CLAUDE.md)：采用真实任务及自有子孙加入、宿主完成通知优先和唯一等待通道的经验，形成 §7.4；不以该参考的外部状态轮询例外、宿主命令或预算授予本仓权限，本仓授权内的长周期核实以 §7.4 为准。

- [CLAUDE.md §6.1](https://github.com/the-omega-institute/trureturing/blob/1239bd2a0ed6e9d54aabb5a42af94234e9eb61ae/CLAUDE.md) 与 [golden-ledger-repo-spec.md A20](https://github.com/the-omega-institute/trureturing/blob/1239bd2a0ed6e9d54aabb5a42af94234e9eb61ae/docs/develop/spec/golden-ledger-repo-spec.md)：采用兼容既有检出复用、语义读写／index／ref／整树操作范围及保存、加入、检查、落地的区别，形成 §10.2；不复制主检出丢弃工作权限、额外 clone 禁令、CLI／路径、阈值、数学或语言体系。

这些固定源码是设计参考数据，不是本仓执行权威或运行依赖，也不证明新增目标已实现、宿主已采用或实际交付完成。

## 16. 宿主指令生成（已实现）

`chrono-instructions init --host-root H` 从内嵌产品 catalog/default manifest 采用独立宿主数据，默认只生成中文根正文与 AGENTS 字面相对链接。通用方法由双语内容叶子与普通空文本 aggregate 组成，既有 core 稳定入口及 core.general 保留；旧主题入口只组合原职责。聚焦 skill 选择修复产生处及其证据／复用前提。无需运行时 checkout、自动语言推断、网络翻译、包解析或新平台。

`core.ownership` 显式组合投影条件与消费义务，`core.behavior` 显式组合同语言测试要求与实际 CI 事件；单独选择旧入口仍提供原职责。默认新宿主和本仓根指南、完整指南的内容范围及测量口径见 [采用说明](docs/methodology-extraction.md#实际消费者边界)；观察结果归本轮 state／artifact，不维护长期读数表，也不设通用运行时预算门。

当前 schema 2 / atomic-rules/relative-alias/v3 的 output plan 显式声明输出身份、路径、格式、locale、根引用及必要元数据。不选布局时确定性 DFS 依赖先行、共享 atom 每输出仅一次。可选 catalog.layouts / output.layout 保留旧字段语义；具名双语标题与显式内容放置组织阅读，不赋予权威或执行顺序。选择时校验标题语言、深度及非空闭包恰好一次覆盖，全部预检后才写入；引用/循环/重复身份与选中闭包缺翻译报具体错误。

产品默认和本仓根 manifest 显式选择现有内容叶子，以 workflow 的需求、审计、重构、开发四阶段组织，验证／SPEC 迭代与交付／清理归开发下级。完整 general 布局和 core.general 保留供自定义 Markdown／skills；本仓 general-zh 选择完整库，聚焦 skill 保持原平铺。清理原子只承载便携生命周期责任，宿主操作、部署及恢复限制归登记 host_context 与现役生命周期文档。

source variant 是 inline 或宿主 .chrono-harness 下的 file，原 UTF-8 字节保留。程序不认证翻译语义等价或组合的语义完整性。

根受管块保留宿主块外原文、唯一原文来源 和预期整份比较语义。Markdown/skill 为带身份起止标记 的整文件投影，未拥有/畸形现有文件预写入拒绝。skill frontmatter 从首字节开始，元数据显式验证。当前 manifest 是唯一管理计划，删条目/改名保留旧输出，由授权 AI 显式退役；不建立历史总表 或扫描删除器。

init/generate 只接受当前 schema 2 / atomic-rules/relative-alias/v3 manifest；不支持的格式在写入前报错，保全源、manifest 和根文件，不自动迁移旧格式。新宿主 init --methodology 将显式方法绑定为 file atom，未声明语言时为 und，保留原 UTF-8 字节。init --locale 只绑定新根；省略选项保留采用数据，显式冲突报错。全计划预检、路径/链接/实际别名检查、父先子后建目录及多输出回滚复用现有 publisher；新建 manifest 最后发布。当前 macOS 普通 IO 为已测边界，无跨平台、崩溃/并发保证。

完整 schema、组合配方、CLI、所有权与恢复合同由 [docs/instructions.md](docs/instructions.md) 单一维护。

[来源与采用说明](docs/methodology-extraction.md) 和 [来源许可](docs/licenses/methodology-attribution.md) 保留固定来源、修改声明、当前所有者及分发义务，不是运行时输入或政策权威。历史逐条迁移、审读与统计归 Git／PR，不在当前树维护副本。派生内容经过修改、泛化和翻译；生成只校验内容传递，不能证明语义完整或替代适用的内容核验。

自举由同一工具生成中文根、英文 Markdown 与聚焦 skill；产品资产/宿主采用数据/生成投影均显式登记，不是第二份手工政策。五份完整治理登记仍 proposed、input_closure 仍 incomplete；指令专用 manifest 校验与内容图解析不代表治理判官或 AI 遵守。另行采用的 CI slice 及其实际 DELTA 范围见 §14 和 docs/ci.md。

`automatic_cleanup/v2` 以目标独立的 metadata 租约与附件代际 SH/EX 租约，在后续普通生命周期与已采用 check/bootstrap 中恢复未调用 finish 的会话中断，包含正常返回且没有遗留 use token 的情况。EX 取得后，准确的两端登记 dispose 产物以专属不可覆盖 intent、原缺失／失败输入和新真实回执重试；finish-dispose 复用缓存须等明确 finish 释放或合格年龄回收；不创建持久自有 Git lock，不设 terminal，不删除源码、分支、未保留提交、bin/state 或证据。

旧／缺失／漂移身份保留，整树 finish 保留真实交接合同。描述符由原进程 engine 的子副本与登记 Rust 转交方法明确传递，不推断通用继承。本地真实 Cargo test／原生子进程回归验证独立父子进程的租约转交，完整采用仍需已提交组合与支持平台验收；不从父进程退出、PID 缺失、年龄或空闲推断子孙终止。源与宿主政策同时修改，兼容协调者部署、旧生产者排除、已提交组合、支持平台验收、真实宿主切换和 Git/PR/发布仍归调用方。

现役本地执行扩展由 projects 唯一调度 routes 已合并的 DAG。显式有限 cap 只限操作数，不推断 CPU／语言，不限工具内部线程；前置未终结等待，失败只阻断后继，冲突 ready 项可让位于独立项。回执保留原字节与失败，结果按规范计划顺序输出，线程与已知嵌套 runner 进程由原 runner 引擎收束。FILEMAP 声明缺失／未知／歧义在执行前失败；两端 cap、claim、冲突消费者与输出登记变更进入 DELTA，collection 仅重建核原登记和证据。

Unix 匿名继承描述符承载已知 launch tree，超时后有限清理不延长执行额度；不承诺跨调用锁或关闭继承描述符的未登记外部进程树。本宿主 cap 2 与独立输出／workflow inventory 共用 claim 属于宿主政策；未从 owner suites 声称完整 canonical／native 成功、交付或完整 SPEC 验收。

`execution_scheduling.priority` 可显式登记启动优先顺序，只含不重复的已登记操作 ID；空／未知 ID 或类型错误在业务执行前拒绝。就绪项先按该列表、再按未列项的规范计划顺序选择；依赖、失败传播、cap 和资源／输出排斥仍同时生效。优先级不新增选择义务、不推断耗时，也不阻止不冲突的就绪项前进。完整列表进入计划身份及留存核对；缺省或空列表保留旧序列化身份。列表重排、修改和删除按两端登记影响所有计划消费者，报告顺序仍为规范计划顺序。

Unix runner 的中断恢复由同一 process engine 的单个已加入 observer 承载：exec 前固定 child 与 launcher 的活身份，macOS 用 kernel exit event，Linux 用 pidfd；只有两者的实际终结与已登记子树完成才能协调丢失 destructor 的 slot。PID 消失、已发信号或外层 exit 0 都不是完成回执；代际固定阻止旧事件／上下文消费复用 slot。

清理仍限原一秒；中断 owner 的存活子进程终止后仍保留清理失败，未知完成不成功。进程 exit 0 后的 receipt／清理失败在 CI 摘要投影为 failed、exit null，原 receipt 保留真实 exit 0、诊断与绑定；collection 保留原失败而不重执行。宿主 cap、计划、选择义务及本地／原生验收仍由原登记与调用方负责。

observer 以与执行时限相同的系统单调时钟记录实际内核退出观察。监控恢复时已过期限，仅当该观察严格早于原期限且立即取得真实子进程退出状态，才保留原退出结果；观察缺失、过晚、时钟失败或仍未退出均不授予额外等待。取消、输出上限与收束失败仍独立生效。时间戳是退出的观察上界，不是内核记录的精确退出时间；observer 自身延迟时不推断更早退出。共享布局身份独立钉版，嵌套执行须配套运行时，不兼容继承描述符直接拒绝。

exec 前身份登记失败保留 observer 的实际 kernel errno；child 经已有私有 context descriptor 以有界、核返回值的写入发布固定 stage／reason／errno 记录，pre-exec 所有错误路径不分配，只返回 raw OS error；诊断文字由 parent 在 spawn 失败后构造。诊断缺失或部分写入不替换实际 errno，无 OS errno 的 owner 失败使用 EINVAL。

已有 send loop 在原一秒 handoff 内重试 EAGAIN、EINTR、ENOBUFS，耗尽后保留最后 send errno；parent 区分 acknowledgement poll timeout 与 short read。启动失败仍不制造已执行 process／exit／完成回执，不延长原一秒 handoff／清理额度或注册执行时限。Linux pidfd syscall 显式采用已有 `libc::pid_t` ABI 类型；编译修复不证明 Linux 运行行为。

observer 接收端的 recvmsg 失败（EAGAIN／EINTR 除外）与不完整 handoff 在原共享内存中保留有界计数及最近的原始 errno／msg_flags。启动失败报告只追加本次 launch 观察窗口内的增量；原 spawn errno 与 child 诊断保留。共享 observer 的读数不归因到某个 child，也不将无读数解释为接收端正常。诊断由 parent 渲染，pre-exec 不增加分配、文件或等待额度；共享布局钉版，所有参与执行物须配套更新。受控接收端描述符耗尽验证能暴露该故障，不证明既有超时皆由同一原因造成。

已登记 v2 birth 的封口中断由原 worktree owner 在正常入口取得实际排除、核原附件与政策后恢复；保留原缺失／部分结果及独立恢复回执，重复中断重用固定回执，不据恢复制造原成功或 finish。新消费在已发布缓存尝试后开启新代际、保留旧 intent/result；后续回收可绑定当前 HEAD。bootstrap 消费原 managed_command_failed 分类，原非零构建与后续生命周期拒绝及报告引用分别保留。

自动清理政策的已登记 v2 内核附件可以显式迁移：从存活协调宿主调用 `chrono-worktree migrate --path <已登记工作树>`，在目标 metadata 与附件独占租约下核已提交的两端同政策／配置、原物理附件、有效 birth 和两端产物登记。原登记及回执不改写，另保留旧新输入原字节与固定提交的不可覆盖迁移记录；后续普通操作核整条关联并消费最终绑定。迁移不删除、不更新 Git、不设完成；active 开新缓存代际，retained 保持已保留状态并要求新的 finish。

v2 可显式登记创建年龄政策：严格超过复用阈值时保全当前源码、原索引及 checkout reflog 的独有提交到真实分支，再回收可重建缓存；严格超过删除阈值时才可请求检出回收。主检出、当前调用者、活跃／未知使用方、必要证据及未知创建时间继续保护。年龄不独自授予删除权，不替代实时所有权、身份、Git 锁、源码保全与原处置检查；回收不宣称任务成功。已有意图及中断结果不覆盖，原状态和清理提醒继续反映实际结果。宿主可选择 7200／14400 秒；实际采用须由兼容所有者消费已提交政策并核宿主行为。

缺失／损坏原证据、未知租约、未封口 birth、待执行终态删除及协调锚／状态地址／政策地址搬迁不自动解释。发布迁移记录但尚未登记的中断允许按同一绑定重试；普通入口只消费当前登记格式。当前 binary 部署、实际宿主切换、未登记写入加入及落地验证仍由调用方完成。

所有者升级允许保留完全缺失且已排除消费者的已登记工作树：原路径、原 Git metadata 及 inventory 的路径／分支关联均缺失，原独占租约、空 use、适用的有效 birth／政策回执与账本字节须贯穿发布并重验。报告明确当前 HEAD 未知；不退役该记录、不改分支或任务状态，后续本地状态继续提醒处理。部分附件、未知所有权和活消费者仍拒绝升级。

已存在的关联检出切换本地分支或进入 detached HEAD 时，共享所有者部署以本次固定 inventory 的当前引用和 HEAD 核原物理附件及原登记租约，持续排除真实使用方并重核 inventory。部署报告分别保留登记分支和当前引用，不迁移登记、改写工作或授予处置权；普通消费、finish 与清理继续核原身份。部署期间引用、HEAD、附件、租约或登记漂移仍拒绝。

显式生产者登记迁移须支持仍有实际消费者的旧分页 v2 数据，保留绑定的 owner／逐项原字节和原消费者状态，在持续排除原写入者后启用当前登记。不释放诊断、不宣称历史完成，不把旧保管声明变成新的整目录删除资格；准备／归档中断可重入，未知与漂移拒绝，普通入口只读当前格式。

Git main 的已登记历史可重建产物可经同一生命周期所有者 `migrate --path <main> --adopt-cache <state-input>` 接管当前 custody。`chrono-main-cache-custody/v1` 输入显式绑定物理 main、当前完整 HEAD、逐项 dispose 路径、`current_consumers_released:true` 与当前工程原因；操作方须实际加入或排除旧的未登记消费者，不据目录、年龄或静默猜 release。

目标 metadata／附件独占租约保护 active main；源码、index、bin／证据、未知路径及活的已采用消费者仍拒绝。不可覆盖的独立接管回执保留原声明、附件与所有权 DELTA，历史 producer outcome 保持 unknown，原 producer／disposal outcome 保留，不制造 terminal。现有 `reconcile_uses` 可追加中断消费历史并移除过期 token，mutable enrollment／uses 不保证字节不变。

已托管项重入不增接管历史，未登记的独立对象不否决已选择的合法兄弟；发布中断可按同绑定重试。接管本身不删除，新 pending 代际由现有普通入口／maintain drain 回收；后续普通构建及中断恢复继续使用同一租约。该局部源与行为合同不证明 native、全 SPEC、PR 或发布完成。

已结束历史复制路径消费的精确证据别名，可由 worktree 现役维护所有者执行 `retire-evidence`。当前 custodian 输入绑定物理 checkout、固定 HEAD／branch、明确结束消费、权威声明原件及逐项路径／长度／SHA256／保留原件／所有者／原因；等同字节不独自授权处置。复用目标独立 metadata 和附件租约，核现役单一 enrollment、Git 锁、干净源码与真实产物登记，不迁移历史源码或执行旧判官，不清理其它登记项。

逐文件稳定身份／原件核验后删除，独立失败不阻断合格兄弟；独立原 intent/result 日志保留部分效果，重入只报告实际缺失，不制造原成功或 release。`removed` 只计完全核验的删除完成；unlink 后的核验失败可记 failed，因此 removed=0 不证明没有删除效果。回执依赖精确引用的 retained declaration／authority，不保证自包含副本。历史 manifest 可由当前 custodian 明确结束 replay，原字节仍保留。

当前 CI gather 保留下载报告原字节，不再改写旧内联报告的硬链接；宿主采用 retained-reference/v2 在发布处避免复制报告正文，失败 native／transport 原件继续保留。该有限行为不证明通用全历史回收、完整自动证据终点、native parity、PR／发布或持续目标完成。


当前独立工作树的生命周期登记按物理工作树地址分别存储和互斥，不得使用跨工作树的可变成员账本或全局准入独占锁。一个工作树的登记占用、损坏或清理失败只阻断该目标及显式依赖，维护继续处理可用的独立目标。普通 start／reconstruct／use／check／bootstrap／finish 只维护本目标，不把其他工作树或 fixture producer 的清理作为前置；跨目标回收使用显式 maintain，继续处理可用的独立目标。共享执行程序的替换须排除其全部实际使用者；普通操作对该程序持共享保护。当前登记的历史格式不构成兼容义务，切换须保护并迁移仍在使用的原状态。详见 docs/worktree.md。

## Verus 当前实施范围

规范要求由第 2.1.1 节维护。下文只说明当前局部实现，不表示完整覆盖门已启用。
实施按可独立验收的小批次推进，未迁移义务必须明确保留。当前实际 runner 的 OID、CI 单元 ID
及相对词法路径判断已使用同源 Verus，执行结果与规格充要相等，且循环终止。
执行器命令、不可变输入传输、登记及路由计划的资源上限共用同源 Verus 判断：
超时为正，输出为 1 字节至 64 MiB（含两端）；独立边界直接调用真实实现，整 crate 证明为使用方计划的必需前置。
普通与初始库存请求的头部判断也使用同源 Verus，精确匹配各自协议和模式，并要求 judge ID 非空；
名称不额外受 ASCII ID 限制。请求封存、JSON／摘要及外层组合仍待证明。
证明操作通过 action／Method 的可选 `output_contract` 明确登记 Verus 版本和完整 commit，
不按工具名、操作名或语言猜测。合同字段须非空；只变更合同也必须进入两端 DELTA 比较及真实计划身份。
routes 在核过同一原回执、原 stdout 字节摘要和完整执行身份后，一次解码其类型化 Verus 报告，
由同源 Verus 准入：原进程退出 0 且无进程失败，参数严格为登记的完整验证／编译形状，
报告版本／commit 精确匹配、success 为真、两类错误标记为假、errors 为零、verified 大于零，且验证整个 crate。
缺失／错型报告、过滤参数、额外参数、输出与 dep-info 路径不符均失败；普通操作不解析业务 stdout。
本地执行和离线 collection 共用这项判断。此合同不单独证明源码／dep-info 绑定、编译器正确性或完整输入。
本地 judge-ci 与离线 collection 另共用同源 `bind_operation_output`，按完整操作文本选择唯一登记方法，
核对同一原回执的操作、完整参数、实际退出及失败，再共同准入原报告、编译库、完整依赖和固定候选源码。
只有明确声明证明合同的成功操作才返回原报告与源码的类型化证明条目；普通成功、失败或阻断不产生条目。
对声明为 passed 的普通操作，返回成功且无证明条目，当且仅当存在唯一登记方法、完整原操作和参数匹配、
原进程退出 0 且无失败，并且没有编译库或依赖观察；该充要合同也由同源 Verus 证明。
标准 JSON 解码只关联原字节和类型化字段，不决定准入；方法查找、比较与全部准入分支归 Verus。
这项组合仍不建立文件到证明根的全仓覆盖，也不代替编译产物与实际生产调用的完整对应。
实际遍历边的原因归并使用同源 Verus：键集合恰为原键与本次出边的并集，
每个完整有类型边只增加本次实际起点 ID；重复槽位／ID 不增加成员，无关边保留原原因序列，循环终止。
节点原因、完整关联与外层组合仍待证明。
实际登记的八种 NodeKind 由同源 Verus 拥有，复制与相等保全变体身份。
FILEMAP 的五种边必须通过同源 Verus 的完整起点／终点类型关系；独立表覆盖全部320种组合，
包括 Script 可作 build／runtime／test 输入起点而不能作 judge-trigger 起点，以及 Test／Judge 不能作这些输入的边界。
registration 和 filemap 的实际计划强制验证其生产／独立根；JSON 节点提取与外层组合仍待迁移。
其他逻辑、完整 SPEC 对应与全仓强制覆盖尚未完成。详见 [Verus 实施范围](docs/verus.md)。
