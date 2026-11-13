# 独立 CI 单元

`chrono-ci-check/v3` 明确把每个登记测试计划分配给一个单元。按范围 `chrono-github-units/v1` 和完整 `/v2` 必须明确登记 `job_gating`：一个父工作流内运行一个检测 job、按需运行的独立单元 job、始终运行的汇总。各单元仍有独立检出、名称、runner、bootstrap、超时、上下文、上传和重跑；产品不解释宿主语言或布局。

## 显式测试清单

```sh
chrono-ci test-inventory --host-root HOST --config CONFIG
```

宿主登记 `chrono-workflow-inventory/v1` 配置。`projects` 和 `project` 点名项目登记及唯一项目；`unfiltered_action` 点名未过滤 action，`groups` 将分组 ID 对应到 action，并须与项目的 `test_groups` 相同。`binaries` 为每个 action 声明测试二进制的名称和顺序，`tools` 明确绑定 action 使用的工具。程序只执行这些 action 的 `--list`，另外列举未过滤 action 的 ignored 清单，不从清单推导依赖或选测。

`timeout_seconds`、`output_limit_bytes` 约束既有 Rust 进程执行器；子进程继承调用方环境，租约描述符由执行器传递。`report_directory` 接收各次列举的 stdout／stderr 原字节和 `receipts.json`，保留 argv、原退出、失败、耗时及流摘要。全部校验通过后写 `inventory.json`，绑定配置／项目原字节摘要、完整清单和各分组。退出 0 并输出数量摘要表示本次验证成功；配置、列举或覆盖失败时输出 `E_CI` 并退出 1。成功报告须结合本次退出与摘要消费。

## 本仓如何划分测试

本仓 workflow 专属测试项目明确登记三组，分别分配给 `judge-workflow`（核心）、`judge-workflow-units`（直接完整单元测试）、`judge-workflow-short`（完整短命令测试）。action 明确选择成员，清单保护核覆盖范围，不推断依赖。三组合计覆盖原两个测试二进制，保留 fixture、profile 和业务限制。发布仍使用不带过滤的 `actions.execute`／原 `test.judge-workflow-tests`，旧测试身份通过 `test_groups` 绑定独立的核心过滤 action。

workflow、CI 和 inputs 专属测试项目的 dev profile 使用 `opt-level=1` 并去掉调试信息，保留调试断言和溢出检查。迁移样本会把实际辅助程序纳入固定 Git 快照；调试信息没有测试使用方，却会随 Git 原始输出进入请求和报告。由 Cargo 在产物生产处移除这部分内容，原有迁移、原件复用、坏证据拒绝、测试集合、线程、超时和报告上限保持不变；本地与原生 CI 仍须分别核对实际执行。CI／inputs 的采用来自实际 fixture 超时后的成本审计，超时根因和完整成本收益仍待核，不以构建设置本身宣称修复。

judge-ci 专属测试根也采用相同优化和调试信息选择。原诊断失败已执行到嵌套进程准备，但未在一秒中断前启动业务进程；原辅助执行物为 12,669,384 字节，当前测试没有符号调试使用方。构建设置用于减少真实测试程序的执行及产物成本，不单凭此归因或宣称超时已修复。原一秒中断、三秒业务期限、就绪握手、原件、全部断言和未过滤操作保留，仍须核实际结果。

worktree 的 detached 年龄回归与已有具名分支回归一样，先完成复用与源码／索引保全，再通过正式 migrate 采用原 60 秒删除年龄。仍使用真实创建时间，迁移不得重置年龄；不把复用步骤耗时当成应已进入删除阶段。

各组保留构建前置，并通过登记的 `workflow.inventory` 操作运行 Rust `chrono-ci test-inventory`。宿主只维护 `.chrono-harness/ci/workflow-inventory.json` 配置。配置明确绑定原 action、分组表、Cargo 工具及二进制列表。真实未过滤清单及各分组须非空，各组互不相交且合计精确覆盖；ignored 清单须为空；列举失败保留原退出和输出。清单程序比较当前集合，不永远固定数量；迁移时的历史名称基线只是另一项实现观察。清单解析、完整且互不重叠的分组、登记漂移和子进程失败由 Rust `ci-tests` 配对验证，不再设置独立 Python 测试单元。

共享前置和清单操作明确分给三单元，各有上下文／报告；已有 detector 和 always 汇总仍是原生入口，汇总是唯一必需分支保护检查。调用方在已提交候选上真实运行前，不宣称分组原生耗时或激活验收。

完整 action 和三个分组均显式采用 2 个 libtest 线程。每个用例还会启动自己的子进程，执行器的 cap=2 只限制操作数；明确线程数可避免 libtest 按宿主 CPU 数量继续扩大单个操作的并发。测试集合、分组过滤、依赖、资源声明、断言和业务时限沿用原登记。实际耗时与本地并发失败的恢复仍须通过候选标准检查和原生 CI 验证。

worktree 测试分 `worktree`、`worktree-adoption`。采用组明确点名四项宿主入口集成测试：参与式检查、脱离父进程的 Rust 消费者、短检查、原生 Cargo 子进程。核心组排除同样四个名称。旧 Python bootstrap 组合及独立入口测试随实现退役；当前 bootstrap 委托与拒绝边界归 Rust bootstrap 专属测试。两组和发布用的未过滤 action 均显式采用 8 个 libtest 线程，限制单个测试进程内部的并发；宿主操作仍按 cap=2 调度。当前测试和期限保留。

断言失败原件在最后一个使用方退出后即可回收的测试，复用现有 Rust 锁能力隔离工具，在独立测试进程中执行。其他测试的 fork 会短暂继承本进程的文件描述符，包括尚未 exec 关闭的 CLOEXEC 描述符；这段临时继承不能用来判定目标使用方未释放。原件保留、过期后的单轮回收和已有源码保全断言保持，并在回收失败时输出实际维护结果；不增加重试、测试时限或生产权限。

分页登记迁移的替换检测也使用同一隔离工具：释放原分配者后，迁移必须能够取得 metadata 写锁，才可到达测试注入的登记替换边界。保留替换拒绝、原登记保全和后续迁移仍拒绝的全部断言。

`worktree.inventory` 用同一 Rust 清单程序及 `.chrono-harness/ci/worktree-inventory.json`，要求当前未过滤列表被非空、不相交、完整覆盖。两组共用原构建前置，明确声明共享 target 和清单资源，因此本地不能并发写同一 target；原生 job 各有检出、报告、重跑。分组不保证任意宿主在任意外部负载下都满足时限。

本地调度在共享构建就绪后，优先执行 worktree 清单、核心组和采用组，再执行 workflow 清单与各组。这样可让耗时较长的 worktree 链与其他项目检查重叠；现有依赖、cap=2、资源声明、动作、线程和时限均保留。优先级只影响已经就绪的操作，不跳过前置或放宽并发冲突。

projects 测试使用单一 `judge-projects` 单元和原未过滤 `execute`。固定历史宿主解码器／迁移测试已退役，原 `judge-projects-migration` 和双组清单移除。当前 `consumer`、`execution`、`languages` 二进制保留断言、默认并发及 900 秒上限，新测试二进制由未过滤 action 纳入。CI／发布消费同一 action；旧断言明确退役，不能拿剩余测试证明旧行为。

宿主 bootstrap 和 release 专属测试已在各自 target 从生产 `main.rs` 编译 CLI，测试计划不再前置各自的生产构建。实际前端启动仍使用原生产构建及其缓存；release staging 测试仍保留 `build.cache`。原未过滤测试、断言、profile、线程和时限保持，完整成本收益未测。

judge-ci、CI 和 projects 专属测试都在各自 target 编译同一份 judge-ci 生产 `main.rs`，通过 Cargo 提供的 CLI 路径交给真实 runner。三项测试计划不再前置 `build.judge-ci`；普通单元不再恢复或保存 judge-ci 生产 target，已无使用方的 `check.judge-ci` 缓存退役。detector 启动和公开发布仍按原登记构建生产 judge-ci，并保留各自缓存。发布验证不再为这三项测试依赖、下载或暂存 judge-ci 资产；各项其余真实资产消费保留。原未过滤测试、断言、线程和时限保留，总成本未测。

workflow 专属测试在自己的 target 编译唯一的 workflow 生产 `main.rs`，通过 Cargo 提供的路径复制并运行真实 CLI。三个 workflow 测试计划不再前置 `build.judge-workflow`，对应单元也不再恢复生产 target；`inputs` 仍消费生产 CLI，保留该构建及缓存，并负责保存缓存。测试根保留原 profile（含 `strip="debuginfo"`）、测试、断言、分组、线程和时限；完整成本收益未测。

**宿主构建登记与测试安装同时修改：**CI、inputs、registration、filemap、projects、workflow 六个测试项目也从唯一的 runner 生产 `main.rs` 编译测试 CLI，保留原复制、探测、身份绑定及真实检查调用。对应八个单元不再前置 `build.runner`，退役已无使用方的 `check.runner` 缓存及其订阅；detector 启动、正式生产构建和发布打包继续使用各自登记。原未过滤测试、断言、线程、分组、cap=2 和时限保留。验证使用正式生成、canonical check 及两段 PR／落地检查，完整成本收益未测。

**生产与测试构建配置同改：**CI、inputs 及各自专属测试项目的开发构建均显式采用 `opt-level=1`，与 runner 的日常构建一致。Cargo 只采用构建根项目的 profile，不继承依赖项目的 profile，因此四个根分别声明，让真实 CLI 和测试 CLI 中的共享原件编码、解码及状态维护采用相同优化选择。Cargo 的开发配置继续启用调试断言和溢出检查，原未过滤测试、断言、线程和时限保留。实际测试、原生消费和完整编译／运行成本须按候选结果核验，配置本身不证明超时原因或收益。

CI、projects、workflow、worktree 测试不再共用整套 fixture 生产者 claim。每个保留 fixture 分配拥有自己的登记和排除，普通命令不排空无关分配。workflow／worktree 清单操作及精确输出空间继续声明真实冲突，在原 cap=2 下调度。显式维护保留忙碌对象，继续可用兄弟对象；跨调用的检出／输出隔离仍归调用方。总耗时、存储、CI 成本未测。

现役单元 provider 只生成父工作流。独立工作流拓扑及其虚拟单元地址、等待／轮询参数已退役。四个现役外部示例的当前采用与迁移见[示例索引](examples.md)；历史运行仍查原提交与 CI，不要求现役源码继续支持旧拓扑。缺少调度声明或带已退役字段的配置会在写入输出前拒绝。公开新版本和宿主升级仍须分别验收。

## 短命令与单元输入

本仓 schema4 使用固定 `check`、`check --unit ID`、无值 `check --collect`。裸 check 执行全局 DELTA。本地 worktree 生产者固定登记远端 target／HEAD，CI 所有者用明确报告路径和独立观察的 runner／judge 身份产生汇总清单。

本地汇总向判官共享清单／分配模块查询 DELTA 必需单元，只读这些登记路径。不提供无关缺失／过时报表；必需报告缺失、过时或失败仍失败。空 DELTA 不需报告。汇总不补跑单元。旧生产观察保留，当前声明输出可替换，但不保证崩溃／并发事务。provider v4 将事件准备和原生收集放进同一短命令，下文旧宿主／示例保留登记的明确 v3 写法。完整独立短范围使用 full-v3／v4 单元和同一七判官 DAG。

短准备绑定所选原生 provider 源码字节及 `artifact_directory`。各单元在自己上传根下保留上下文、载荷、生产者证据、获取回执、不可变报告；汇总在自己的根保留 gather／manifest 原件。原件按路径及 SHA-256 寻址、复用，不在每份回执内重复正文。当前可变输出只是结果视图；provider／上传不符、原件缺失或变化均失败。上传只需所选目录，不需无关单元。

原生收集为短报告清单每项增加 `artifacts`：`source_directory` 为生产单元上传根，`directory` 为明确下载根。汇总用这个绑定解析原相对路径，保留所观察的生产检出／执行物身份，不发现目录或递归猜证据。历史明确报告保留原清单合同。

### 完整单元的离线证据

full-v3／v4 宿主通过原生七判官入口使用同一明确单元表和选择器后缀。清单可为 `chrono-full-collection/v1`，或兼容的 `chrono-ci-collection/v1` 形状，包含 `unit`、`path`、`sha256`，可选明确执行物身份及原生 `artifacts` 映射。runner 和七判官身份从固定候选登记及绑定调用取得。

完整汇总保留可重建的封存请求模板、精确进程／响应，以及按地址引用的原上下文／输入／证据字节。它按当前声明合同核每个导入判官和回执，不启动业务工具或其版本命令，原单元根和业务执行物可以已不可用。完成报告保留原证据依赖范围，供后续集成凭据消费。

integration、delivery 上下文都支持只提供贡献的单元，无需先有完成凭据；全局完成只由汇总判定。full-v3 报告上限约束报告 JSON 和 manifest，外部保留数据按上传根流式处理，含嵌套完整汇总证据。旧内联原件仍支持。原件缺失、替换、失败、过时或矛盾均拒绝。

普通不分单元的完整检查保留本地数据引用，流式核身份；可传输单元／报告上限不限制这些输入文件。provider 生成和产物获取仍归 `chrono-ci`，真实完整宿主启用尚待完成。

完整短准备经过汇总仍保留精确本地上下文。本地 manifest 生产者向 FILEMAP／routes 义务模块查询必需单元，绑定七判官执行物。最终 manifest 发布前，registration 为历史转换消费明确原报告输入，不重跑解码器。原短获取回执和寻址字节留在完整可传输证据中。

本地按范围准备只在端点、origin 角色、birth 关联、保留输入引用仍匹配当前生产事实，且新观察仍满足 workflow 年龄条件时复用上下文。有效期间固定原 `observed_at`，让独立报告绑定同一上下文。到期后，同命令保留原件，发布新不可变观察；旧贡献不能完成新上下文，workflow 拒绝过时分支。裸 check 产生新观察，不重建分支。

registration／input 统一负责真实单元验证、`chrono-inputs capture --unit ID`、汇总比较的文件输入投影。它使用单元明确计划、操作所有者、FILEMAP 前置和绑定依赖范围，含共享操作及治理消费者。结构／引用仍全局检查。无关 SDK 缺失不要求该单元保留或读取；所选／共享／治理缺项、漂移、错误快照、未知输入仍失败。

汇总核原单元数据，将各单元有效投影与当前义务比较，不启动业务或版本工具。完整原快照仍支持。原生 Go／TS 执行及完整宿主启用尚未验证。

`chrono-github-units/v2` 将完整范围接到已有自动 push／PR provider。v1 仍按范围，拒绝新增字段。v2 保留汇总、单元、gather 合同，要求 collection schema `chrono-github-ci/v4`，增加一个明确映射：

```json
"full_contexts": {
  "collection": ".chrono-harness/state/collection/full.json",
  "units": {
    "service": ".chrono-harness/state/service/full.json",
    "client": ".chrono-harness/state/client/full.json"
  }
}
```

单元键须与 full profile／工作流完全一致，每个映射文件是所选上传目录下的独立输入。已有 bootstrap 提供精确 schema2 上下文及保留输入，准备核 base／candidate 匹配真实事件端点。

收集核完整上下文摘要、七判官身份，保留 workflow／attempt 证据，写 `chrono-full-collection/v1` 和明确来源／下载根映射。失败运行仍失败，下载原件保留。各工作流独立构建／重试，汇总不运行业务或版本命令。唯一登记 CI 生产者的 `--config` 为所有短调用选择该映射，不推断角色、创建时间、凭据或布局。

本产品宿主当前采用按范围 v1 和明确 job gating。干净候选准入、真实原生完整采用、公开二进制发布及完整 SPEC 验收仍各自处理。

**产品与宿主登记同时收窄：**未发现当前使用方的 `native_adoption` 扩展、种子发布／获取／转发及专属治理采集已退役。删除专属 Cargo target、三条 FILEMAP 文件登记和 CI 的直接 `time` 依赖；其他所有者仍使用的传递依赖保留。普通 scoped 原生 CI、full 上下文／汇总、共享启动以及输入 capture／pair／compose 继续由现有入口负责。完整输入、full 采用和同判仍未建立，构建、CI、存储及维护总成本收益未测。

<a id="conditional-jobs-and-the-required-aggregate"></a>

默认合并采用[并行验收、按依赖合并](merging.md)。PR 的被验候选是固定 workflow 合并提交 M，基线是其第一父，第二父须为事件 head H。`chrono-ci-detection/v2` 分别记录 `candidate=M` 和 `event_head=H`，汇总对测试报告使用 M，对原生运行、job 和 artifact 的 `head_sha` 使用 H。不能把两种身份互换。

## 条件 job 与唯一必需汇总

父工作流用原生 concurrency 替换同一 PR 的旧运行。组名绑定工作流路径、事件、PR 号；非 PR 事件用唯一 run ID，所以 push 的各次完整 before／after 不会因取消排队而丢失。父工作流内各单元独立调度。

本宿主通过目标为 dev 或 integration/** 的 PR 检查固定合并候选，端点身份按上文分别绑定；dev push 用完整 before／after 检查实际落地。integration 分支推送不另行触发主 CI，需要候选检查时创建 PR，草稿 PR 也支持。各宿主自行登记触发分支；本仓的[独立发布](release-ci.md)按自己的登记触发。

最小生成结果是 `collection.workflow_path` 的单个父工作流。分支保护只要求汇总 job，即 `collection.name`，本仓当前为 **chrono / collection**；生成器不改分支保护。

`detect` 为每个登记单元输出布尔值。单元 `needs: detect`，在 job 层 `if` 判断该值；无关 job 不分配 runner，也不 bootstrap 或准备 SDK。`aggregate` 使用 `if: always()`，依赖 detector 和全部单元。不生成工作流级 `paths`／`paths-ignore`，也不用 matrix fail-fast 联动单元失败。

provider 的必需 `job_gating` 字段登记 detector 的平台、时限、bootstrap 和源码获取范围。本仓的当前声明见 [.chrono-harness/ci/units.json](../.chrono-harness/ci/units.json)，沿已登记的 Rust bootstrap 启动，不在本文维护第二份配方。

此形式要求 collection provider v4。内部生成命令 `chrono-ci detect --host-root . --config SOURCE --github-output "$GITHUB_OUTPUT"` 自读事件及原生绑定，不加日常标志或手动准备。实际检查仍为 `chrono-harness check`、`check --unit ID`、`check --collect`。

短汇总同时负责原生传输／状态拒绝和最终判官准入。GitHub 状态成功不能代替成功原报告；不为跳过单元造报告。空 DELTA 仍用空清单执行结构／全局检查，业务操作为零。

### detector 读取范围

按范围检测和本地汇总共用 judge-ci 的清单、两端影响和 `units::required`。调度不检查检出文件是否已完整准备，真实检查仍要求干净候选和标准入口。registration 调度入口共用历史 profile 匹配，在获取解释器或执行转换前拒绝依赖解码器的历史；真实检查／汇总保留转换准入。

检测读取完整 Git 树和登记 blob，不执行业务测试或版本探测。源码 bootstrap 只需准备声明的产品源码输入。上文稀疏路径是本 Rust 产品宿主选择；发布二进制消费者可只准备宿主登记／安装器／工作流路径。detector 不发现 SDK、import、目录或语言边界。

### 共享 startup

本仓 detector 为单元和汇总生产登记的核心启动工具。缓存 profile 单独准备 `chrono-cache`；core profile 构建并安装四项当前启动工具：`chrono-harness`、`chrono-judge-ci`、`chrono-ci`、`chrono-worktree`，并安装已校验的 Verus 原压缩包 `.chrono-harness/bin/verus.zip`。消费者安装所选 profile 后，在登记的准备位置验证原报告和已安装内容，再进入标准检查。单元从该包解压 Verus，不再各自下载、校验和构建 bootstrap 前端；本地 core 仍完成完整安装。

当前宿主没有启用调用 `chrono-inputs` 的 `native_adoption`，登记 action 也不调用它，因此 core 不预先获取、构建、安装或传递这个程序。inputs 专属测试由 Cargo 从生产 `main.rs` 编译测试 CLI，不再单独构建生产 inputs 项目；workflow 的三个测试组仍按自己的计划构建并消费生产 inputs 程序，公开发布资产保留。宿主工具表及专属依赖边同步移除这项闲置安装的声明，避免把产物的存在当作使用理由。

**实现与宿主政策同时变更：**共享文件允许普通非零权限 `0001`–`0777`，包含无执行位的压缩包；不把数据文件改成可执行文件。现役 Rust 共享入口及其证明采用此范围。安装仍核全部来源、摘要、大小和原权限，使用时拒绝字节或权限漂移；特殊权限、零权限、损坏或缺失仍失败。压缩包随现有 startup 产物传输并沿用保留期限。逐单元外部下载已删除，传输、存储和完整 CI 总成本仍未测。

worktree 两个测试组由 Cargo 从唯一 runner 生产 `main.rs` 编译测试 CLI，不再另行构建生产 runner，也不恢复其缓存。测试根保留当前锁定依赖和开发配置，原测试、分组、线程及限制不变；测试锁与生产锁的部分构建依赖版本不同，不据此声称执行物字节相同。完整成本收益未测。

judge-ci 测试也从同一生产 `main.rs` 编译 runner 测试 CLI，两个真实 CLI 调用使用 Cargo 提供的路径。该测试计划不再前置 `build.runner`，对应单元不再恢复生产 runner 缓存；生产 runner 及其他使用方保留。测试根继续使用原锁和开发配置，原断言、测试线程和时限不变；测试与生产的编译配置不同，不声称执行物字节或性能相同，完整成本收益未测。

cost 和 mixed 两个测试项目也从 runner 唯一生产 `main.rs` 编译同名测试 CLI，共享 Host 从各自 Cargo 提供的路径读取并安装它，继续绑定实际字节摘要。两个计划不再前置 `build.runner`，对应单元不再恢复生产 runner 缓存；filemap、cost 和 mixed 判官的实际构建及调用保留。registration 按下文采用测试 CLI。原锁、开发配置、未过滤测试、断言、线程和时限不变，完整成本收益未测。

**测试构建与宿主登记同改：**CI、registration、cost、mixed、filemap、projects、inputs、workflow 八个测试项目均从 registration 唯一生产 `main.rs` 编译测试 CLI，现有 fixture 按 Cargo 提供的路径复制并调用真实程序。对应十个单元的计划不再前置 `build.judge-registration`，退役已无检查使用方的 `check.judge-registration` 缓存及订阅；发布验证移除对应构建依赖、下载与资产消费。Linux／macOS 生产发布仍构建 registration，保留其共享 target、源码输入和真实 Cargo 下载缓存。原未过滤测试、协议检查、断言、线程、分组、时限、锁和开发配置保留；测试与生产的编译配置不同，不声称执行物字节相同，完整成本收益未测。

runner 和 diagnostics 使用普通锁定 Cargo 构建，core 按当前消费者安装所需工具。项目 action、单元、缓存和发布集合由各自登记维护。

诊断单元直接验证导入 core，执行原格式、锁定构建和行为测试；不再准备专属保护工具、私有离线领域或目录观察。共享工具的验证／导入合同保持如下。

可选 `job_gating.startup` 使用 `chrono-shared-startup/v1`，声明字面宿主 `directory`、产物名、固定 `download_action`，以及按明确 job ID 索引的 `consumers`。每个消费者有非空 `need`、`output_use`、字面 `install` argv。已有 detector bootstrap 是唯一生产者，向 `GITHUB_OUTPUT` 写不透明 `binding`，不加 job。

provider 在检测前上传声明目录，将生产 binding 和真实上传 artifact ID 作为 detector 输出。消费者按该 ID 下载，包括后续 attempt 只重跑失败 job 时；空 ID 不能触发下载全部。安装器须拒绝 binding 或传递文件缺失，无成功豁免。

传递目录不能重叠证据根，避免每份普通证据归档都复制二进制。未列消费者或未采用 startup 的配置保留原行为。兼容性／安装检查由宿主负责，provider 不推断语言、布局或平台兼容。

本仓 bootstrap 可写工具链 home 也在已有 `.chrono-harness/cache/` 产物根，是可重建执行输入，不属于上传的 `.chrono-harness/state/`。原身份及生产回执留在 state。共享来源验证同一缓存位置，不要求为汇总报告传递编译器文件，见[bootstrap 边界](ci.md#this-repository)。

本仓的共享启动实现由 Rust 维护，produce／install／verify 均从登记的 `bootstrap-native.sh` 进入，复用已有前端与所有者保护。它绑定干净主源码 commit／tree、平台、配置／profile 字节、原 bootstrap 报告、文件摘要／大小／权限。替换文件前验证整份传递，恢复原文件权限，在明确导入来源旁保留精确生产报告。安装不冒称本地构建；前端自身的源码准备另有真实构建报告，使用时仍拒绝已安装内容变化。

绑定缺失／改变、源码／配置／平台不符、符号链接、内容损坏或发布失败都会停止 job。这只覆盖登记启动输出，不证明编译器／SDK 输入完整、对抗性认证、linked 所有权或多文件断电原子性。真实传递／重跑验收与 fixture 验证分开。上传／下载／安装有明确资源类别，实测成本计入复用评估。

### 端点、状态与原生产身份

PR 用固定 workflow merge commit 的第一父提交作 base，该合并提交作 candidate；必须恰有两个父提交且第二父提交等于事件 head。事件载荷中的旧 base 不决定选测。默认 push 用完整 before／after，含多提交。明确 `push_baselines` 和创建规则保持原义；detector 只观察一次配置远端基线，将固定输出交依赖 job。

足够时获取从不含 blob 的 depth-two checkout／fetch 开始，缺端点 OID 明确用 `--depth=2 --filter=blob:none` fetch。深度二不保证任意历史。full 单元／汇总为分支／分叉判官保留完整历史获取并过滤 blob，detector 仍从浅获取读精确端点树。

端点缺失／不可得、有父首次基线、删除但无候选、登记错误都失败，不用 HEAD^、merge-base 或全选回退。改名视为两端已登记路径的删除／新增，保留两边依赖范围。变更文件数在选择前从完整 Git 树取得，不设数量门或截断。

GitHub [原生路径过滤](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#git-diff-comparisons)文档有 3000 文件上限，[PR files API](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests-files)最多 3,000，[compare API](https://docs.github.com/en/rest/commits/commits#compare-two-commits)最多 300。detector 不从它们取文件列表，这些也不是通用运行上限。

Rust provider 拒绝检测失败／取消／缺失，以及任一选中单元失败、取消、缺失或意外跳过。只有 detector 声明非必需的跳过可接受。gather 核精确父 run／仓库／当前 attempt／事件／工作流源码，依赖完成后分页读 job 历史，不轮询其他工作流，也不等自己的父运行结束。

原生 needs 已确认成功，但完整 job 列表中的唯一最新执行仍是已知非终态且 conclusion 为 null 时，gather 只额外读取一次该精确 job ID 的详情。详情须保持 id／run_id／run_attempt／head_sha／name 不变，并实际为 completed／success；仍未完成、失败、身份变化或传输失败都拒绝。已终结的列表结果保持原判定，不通过重读掩盖失败。两次响应沿现有 transport 保留；正常终态列表不增加 IO，无轮询或预算调整，额外读取的总成本未知。

独立重试保留原 detector 及成功前置 attempt。非必需跳过由 `needs` 绑定，无需 job 执行 metadata 或产物。每个选中单元须在同一父 run 有成功的最新 API 状态；生成 `production` 输出寻址真实原 artifact、上下文摘要、报告摘要及生产 attempt。

API job 身份和 `run_attempt` 可能描述沿用工作，不能据此造产物名。gather 将原 context／run／attempt／job 和两个摘要与 production 输出核对。后续失败／跳过不能用早先成功替代；父 attempt 漂移、产物不符／缺失均失败。原进程字节、源码 SHA／revision、事件／端点、报告摘要、执行物身份及上传／来源根映射继续绑定并判断。

同一父运行的一次 gather 只读取一份完整分页产物清单，以及该固定 revision 的工作流原字节；逐个单元仍核唯一产物、仓库／run／candidate、生产 attempt、上下文和报告摘要。观察只保存在本次调用内，不跨运行共享。全部下载与原件检查结束后、发布 manifest 前，重新核父运行身份和 attempt；该核验仍计入末个产物原有的累计期限与流上限。没有选中单元时不读取产物清单或源码。

Rust 行为回归验证单／多／零单元及重复汇总的请求次数，跨页末个产物缺失、歧义或错误绑定，以及最后下载后父 attempt 漂移时拒绝发布，并保留已下载报告原字节。这里只约束显式启动的传输进程；CLI 内部网络请求、完整执行耗时和计费成本仍需实际观察。

按范围 provider 在生产环境登记 `CHRONO_CI_DETECTION`、`CHRONO_CI_NEEDS`、`GITHUB_RUN_ID`、`GITHUB_RUN_ATTEMPT`、`GITHUB_JOB`。可选完整原生适配器则记录真实获取值，在业务继承外转发，保留独立 job 差异。生成汇总传精确 `needs` 对象，Rust provider 独自拥有接受条件，YAML 和 bootstrap 不另设白名单或选择算法。

### 拓扑迁移

单元初始化要求完整、明确的条件父工作流声明，不补造 detector 配置；已有现役 provider 的宿主定制保留。

full-v2 调度在 SDK 快照存在前复用纯 FILEMAP 声明影响、固定语义文档读取、routes `global_selection`／分配验证及 `units::required`。它不提供 schema2 完整上下文、原 birth 回执、输入快照、流式产物或历史转换观察。最终完整汇总仍核真实有效输入和七判官原件。

外部输入变化若带来额外最终义务，缺证据会拒绝；声明调度不声称已知这些快照。需解码器的历史登记报具体调度边界，不执行历史代码或猜测。真实完整原生上下文／快照准备和启用仍交调用方。

普通 `init`／`generate`／`verify` 只拥有当前 provider 输出，先核这些输出、保留宿主来源、拒绝无所有权冲突。verify 报漂移不修复；它们不推导其他拓扑、检查或退役旧工作流。

现役配置的地址或来源变更用已有明确 `migrate`，提供精确原 provider 快照和新来源。写入前核原生成结果／新冲突，保留定制，只退役原快照拥有的输出，同时更新宿主 FILEMAP 和明确消费者。该入口不解码已退役的独立单元拓扑；旧发布宿主的升级须由其负责方保全并迁移原输出。

准备和源码检查只绑定真实的 `collection.workflow_path`。单元配置没有独立工作流地址，gather 没有等待／轮询参数；汇总在原生 needs 完成后取得原报告，不等待自身所属的父运行结束。

这样去掉普通生成时额外渲染／读取旧拓扑的逻辑；运行和 CI 收益未测。[可复制条件宿主](../examples/ci-host-job-gating/README.md)使用安装工具和固定短命令。Go／TS／mix 的公开二进制采用已迁到同一父 workflow 和短命令，真实交付及范围见[示例索引](examples.md)；历史配置不形成现役源码的兼容执行义务。

本仓通过 projects.json 的 `ci.generate-job-gating-example` 操作维护该示例的生成工作流；标准 CI 计划通过 `ci.verify-job-gating-example` 调用同一 Rust 生成器，校验已提交投影。执行时沿用登记的 `chrono-worktree use` 保护。

## 分配与执行

保留原 scoped 字段，设 `schema: chrono-ci-check/v3`，增加明确政策：

```json
{
  "units": {
    "service": {
      "tests": ["test:service-tests"],
      "report_path": ".chrono-harness/state/service/check.json"
    },
    "client": {
      "tests": ["test:client-tests"],
      "report_path": ".chrono-harness/state/client/check.json"
    }
  },
  "shared_operations": {
    "build.shared-input": ["client", "service"]
  }
}
```

分配须恰好覆盖完整候选计划登记一次。未知测试、漏分、重复、报告冲突、共享操作未声明都在业务前失败。示例共享操作只有在两个登记完整计划都含它时才有效；无共享用 `{}`。共享表示各隔离检出执行声明前置，不传产物，也不隐含工作流依赖，此版本不实现跨工作流产物依赖。

两端登记仍决定 DELTA，含删边、操作变化、测试替换；单元分配变化选中受影响计划。按请求单元过滤前，判官核完整所选操作图，不能用过滤藏全局顺序冲突。每单元通过已有 routes／projects 引擎执行所选完整计划。替换成功由拥有替换计划的单元负责，汇总要求该单元成功证据。

即使单元 ID 新增或改名，输入投影也按候选单元明确计划；基线还保留原先分给该 ID 的历史计划，两种分配使用同一声明输入范围。不要求无关单元业务输入作为回退。

旧明确宿主的本地与生成单元工作流使用同命令：

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID --unit service
```

`--unit`／`--collect` 可选、互斥、要求 v3，传输为 `chrono-ci-judge/v2`，旧调用保留 v1。不带选择器的 v3 仍是全局 scoped。宿主使用 `registration_config` 时，调用须与登记标准命令完全匹配，只追加所选后缀。initial 用 `--initial`、不带 `--base`，仍要求无父候选。

证据区分 `selected`、`global_selected`、`assigned_elsewhere`、`required_units`、`not_required`。单元成功不证明全局完成，分给别处的义务不能标不需要。并发单元使用不同检出，此版本不提供共享检出产物锁。

<a id="offline-collection"></a>

## 离线汇总

在登记 `.chrono-harness/state/` 下提供清单：

```json
{
  "schema": "chrono-ci-collection/v1",
  "reports": [
    {
      "unit": "service",
      "path": ".chrono-harness/state/imported/service.json",
      "sha256": "REPORT_SHA256",
      "runner_sha256": "RUNNER_SHA256",
      "judge_sha256": "JUDGE_SHA256"
    }
  ]
}
```

摘要须是完整小写 SHA-256。必需单元恰好各一次；登记但无 DELTA 的单元可不提供，提供也须核验。旧明确宿主运行：

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID \
  --collect .chrono-harness/state/collection/manifest.json
```

汇总重算当前全局义务，核报告摘要、端点／profile／选择器、预期执行物身份、原 judge 输出、计划身份及精确候选操作／顺序／上限。用共享回执验证器核原进程字节和回执。缺失／重复证据、过时绑定、改写输出、失败单元不能通过。不执行业务测试；空 DELTA 可明确传空 reports。

### 执行证据与控制台

当前 scoped 执行声明 `execution_evidence_schema: "chrono-ci-executions/v2"`。每条执行拥有一份回执，进程沿用无损 `chrono-retained-process` 编码，去掉重复行级进程及文本／字节别名。汇总先展开并核原流摘要，再比较原回执摘要、方法、计划、结果。

只接受当前证据 schema，现役生产／消费一起迁移。schema 缺失／未知、重复进程视图、编码缺失、流损坏或回执摘要变化均失败，不增加执行／输出上限。

完整构建、CI 与存储成本未测；具体运行读数直接查原 CI／state。

回归要求原样保留含无效 UTF-8、NUL 的 stdout 和 stderr，并在既有传输上限内完成生产与离线汇总。

大型原件控制台回归仍要求超过 1 MiB 的原文件，使用确定性原生二进制流，精确断言大小／摘要，保持不加额外工作及原件不可变检查。

scoped v3 报告用紧凑 JSON，避免字节数组每字节缩进一行；解析内容、原进程字节／摘要不变。短调用 `check`、`check --unit ID`、`check --collect` 输出原状态／退出、有限身份诊断、精确不可变报告路径，明确标注省略的诊断行／文本。消费者从已有路径读完整报告。

旧明确 `--config` 调用保留 stdout 与存储 JSON 完全一致。历史报告保留原字节／摘要，包括缩进版本。控制台展示在发布后进行，不运行业务。生产失败／版本不符指向保留获取或探测原件；完成发布前的执行错误指向保留原错误，不冒充完整报告。保留失败同时保留原失败、说明写入错误，不声称产物存在。早期配置／选择错误仍用原 `E_CHECK`。

### 原报告引用格式

scoped v3 可采用 `policy.report_publication: "retained-reference/v2"`，只保留一份完整不可变报告。固定报告路径仅写 `{"schema":"chrono-check-reference/v1","original":{"path":...,"sha256":...}}`，不含结论；短控制台直接指完整原件。本地／原生 manifest 绑定引用精确字节。

汇总经声明产物传输读原件，核摘要／保留路径，继续全部报告、进程、计划、执行物检查，不跟随引用链。原件缺失、改变、符号链接、未声明或过大都失败，小引用／成功下载不能替代证据。

采用须匹配 runner／collector 二进制及准备后的短入口，明确端点调用在业务前失败。省略政策仍在固定路径写内联报告。已退役的 `retained-reference/v1`、未知值、null、scoped v1／v2 采用都在业务派发前拒绝。固定路径读取器须先支持引用，再采用。上传整个登记目录，含引用和原件，不能只上传小引用。它只减少重复发布报告，不去重其他获取、进程或 gather 证据。

引用的原件为 `chrono-check-report/v2`，保留请求、runner 身份、进程 metadata、退出、失败、保留路径。judge 精确 stdout／stderr 另按原件机制保存，`judge.stdout_original`／`judge.stderr_original` 各含 `{path, sha256}`。

报告不再有独立 `response` 或内联 judge `stdout`、`stderr`、`stdout_bytes`、`stderr_bytes`。汇总将两流摘要与引用／进程 metadata 核对，临时恢复进程视图，从原 stdout 解码响应。原协议、执行物、计划、回执、结论检查都保留。业务／传输失败保留原状态和字节，有限控制台指向报告。

`report_bytes` 约束每单元原报告 metadata 加 stdout／stderr 总字节，固定引用另受同大小上限。全部原件经声明传输解析，流缺失／损坏不能回退生产者本地副本。上传整目录，采用前升级全部实际读取器。拒绝内联流视图，只消费已核原字节。此存储合同不去重 judge 输出内业务回执或其他获取证据。

### 读取上限与身份

宿主可独立声明汇总读取上限：

```json
"collection_limits": {
  "manifest_bytes": 1048576,
  "report_bytes": 134217728
}
```

两值须为正字节数，小于平台 `usize::MAX`。省略对象保留 64 MiB，按文件或上述 v2 原件加流总量计。提供对象就须同时有两字段，不接受未知字段／null。它是 scoped v3 扩展，旧二进制拒绝，采用前先装支持版本。

每份原单元报告单独读验，上限按真实序列化字节，不按压缩下载或解析内容，也区别于 judge stdout 上限。等于可接受，超过时在解析前报路径、上限名和实测字节。有限读取器也阻止文件读取期间增长超限。

上限是资源政策，不保证内存足够或输入完整。按实测原报告调整，不改旧证据以凑摘要／大小。汇总仍核全部必需报告，不补跑业务。报告须匹配当前候选及配置；改上限也改配置身份，不能沿用旧配置报告，应为新候选产生匹配报告，另保留旧结果。

执行物身份是调用方明确输入，汇总只核报告绑定，不独立证明构建来源或外部输入完整。工具原观察可来自不同根／平台，这不证明环境等价。

<a id="github-projection-and-transport"></a>

## GitHub 生成与传输

在 `.chrono-harness/ci/` 放 `chrono-github-units/v1` 来源：

| 部分 | 声明内容 |
| --- | --- |
| `collection` | 当前 `chrono-github-ci/v4` provider，含共同事件、action、runner、generator、检查 profile，以及汇总的路径／名称／bootstrap／runner／产物 |
| `units` | 键与检查 profile 完全相同；每项声明 `name`、`runs_on`、`timeout_minutes`、`bootstrap`、`context_path`、`artifact_directory` |
| `gather` | GitHub CLI `program`、`inherit_environment`、`credential_environment`、字面 `environment`、进程 `timeout_seconds`／`output_limit_bytes`，以及明确 `manifest_path`／`download_directory`／`report_path` |

### 可选下载重试

`gather.download_retry` 明确声明 `max_attempts`（1–10）、`delay_seconds`（1–60，且小于 timeout_seconds）、非空无重复 `http_statuses`（400–599）。省略保留一次下载和原目标布局，null／未知字段失败。例如可选三次、间隔两秒、状态 `[429, 500, 502, 503, 504]`。

政策用于单元产物和共享原生种子，不改 API 查询处理，不重跑业务单元。只有进程非零、没有进程上限失败、stderr 恰有一个无歧义且在白名单中的 `HTTP nnn:`，才允许重试。全部尝试／延迟共用原下载期限，每条输出流共用原字节上限。预算耗尽、未列／歧义状态、超时、输出溢出都停止。

每次配置尝试写独立 `attempt-N` 目录，保留原进程、部分文件及实际 retry／completed／failed 行为，恢复成功也不删失败原件。只有成功目录提供报告输入，固定 artifact 身份、摘要、工作流源码、run／attempt 和最终判官检查继续适用。恢复耗尽不发布可接受 manifest，不能使失败单元通过。

单元产物沿用登记 `gh run download`。原生父工作流先核唯一未过期产物的名称、ID、归档端点、run、候选源码，再获取原上传目录。汇总仍核固定 producer／run／attempt／context、报告引用、原报告和 stdout／stderr 摘要、工作流源码及最终运行观察。下载不是业务通过结论。

每个单元产物的同一期限从 metadata 前开始，覆盖下载、延迟、语义核验、工作流源码及最终运行观察；全部获取进程共用 stdout／stderr 额度。本宿主保留每产物 60 秒、每流 8 MiB，报告读取上限不变，共享种子沿用原下载预算。超时不可重试，失败原件保留，不发布可接受清单。

**产品与政策曾同步退役：**逐成员 HTTP range 读取器、Python 测试／fixture、两个平台发布 job。完整产物下载已服务当前汇总，不再需要第二套传输。Rust 回归继续覆盖报告／流损坏、原生产身份、重试恢复、累计时间及双流预算拒绝。

删除 range 传输减少 helper 进程，完整耗时、网络／存储成本、原生交付仍须候选执行后测量。完整上传原件和原保留／确认要求仍为必需。

### 缩小成功报告的上传范围

本宿主现在上传各单元已有 `.chrono-harness/state/units/<id>/` 报告根。短准备已在该根保留报告、上下文、获取、原 stdout／stderr。成功汇总不消费临时测试宿主或其复制执行物。

可选 `failure_artifact_directory` 在 job 失败时保留更广诊断目录，用同一固定上传 action、`failure()` 和独立 `diagnostics-` 产物前缀。本仓选 `.chrono-harness/state/`，保留失败 fixture 原件，不向成功汇总传其正文。普通报告上传及交付确认保留自己的产物／映射；目录相同时不重复上传。

不加下载器、打包命令、权限或超时预算。原生报告根消费与仅失败上传须分别验证，成功 job 不能证明后者。

### 成本排查

检查时长和缓存传输情况直接查原生 Actions 步骤、日志及线上 API。AI 只为当前排查选定运行、输入、单位与测量窗口；原件按实际调试／验收义务保留，不持续生成分类报告、比较告警或重复摘要。

GitHub 当前 attempt 的 API 视图可能含沿用结果，新 job ID 不证明任务重跑。步骤秒数之和不是工作流墙钟、CPU 或计费时间；缺读数和未测成本保持未知，不能跨 attempt 相加后宣称节省。

汇总检查仍取得并保留实际用于运行归属、job、artifact 和报告核验的原响应。只退役额外的资源投影，不以成本观察替代原生失败、传输准入或最终判词。

### 配置、凭据与父工作流

bootstrap 列表为字面 argv，只登记单元所需工具，生成器不推断包、语言准备或构建依赖。报告／上下文须在本单元上传目录，父工作流路径明确，各 job 名称唯一。生成前预检输出，不覆盖无所有权文件。文件系统失败如实报告，不声称多文件事务。移除路径须由宿主与登记一起明确退役。

```sh
chrono-ci init --host-root . --config /path/to/units.json
chrono-ci generate --host-root . --config .chrono-harness/ci/units.json
chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

init 使用 `.chrono-harness/ci/units.json`，保留已有来源；要求已登记 v3 检查 profile，不造分配。已有单工作流、完整上下文、发布 provider 保留原含义和输出。

push／pull_request 准备将生成 provider、工作流、profile 绑定候选，记录精确标准 argv 和执行前 runner／judge 身份。汇总工作流有 `actions: read`，`check --collect` 的原生输入程序在内部收集报告，按登记接收 `GH_TOKEN`；独立 `chrono-ci gather` CLI 已退役。凭据环境键须明确声明，传输报告省略其值，保留非秘密进程原字节、退出／失败和真实环境身份。不得把凭据作为字面传输值。

gather 根据固定 detection 和原生 needs 选择必需单元，核原生产 attempt、父运行和工作流源码，下载原始产物后再核父运行身份。运行缺失、歧义、变化或失败都使汇总失败，下载原报告保留。manifest 在同一标准 `check --collect` 成功前为 `gathered-unjudged`。

单元失败不取消无关 job，重试单元不重跑其他已成功单元，之后重试汇总按原生产声明收集报告。自动汇总支持 push／pull_request。需要本地组合时使用明确 manifest 和本地汇总命令，父工作流不暴露手动触发。

重试不覆盖已有汇总输出，原生重试用新检出。工作流源码／端点不符仍错误，包括准备时的固定端点与登记不一致。

源码行为测试用独立克隆宿主中的真实候选二进制，覆盖并发、离线汇总、合成 GitHub 响应。公开 Go／TS／mix [示例](examples.md)使用独立原生工作流，已核本地／integration／PR／dev、真实取消及选择性重试、业务失败同时其他单元通过、恢复后零业务选择。首次清单仍是独立合同。

本产品宿主的条件单元及汇总归 [.chrono-harness/ci/units.json](../.chrono-harness/ci/units.json) 维护，core 安装范围归 [bootstrap-core.json](../.chrono-harness/ci/bootstrap-core.json) 维护。各所选计划保留明确的构建／检查／测试，共享操作明确列在 profile，只在隔离检出重复。

CI 生成器消费者在已提交 push 事件上执行生成单元的真实短 `chrono-harness check`，由登记输入程序准备绑定候选的工作流源码／上下文，再读取原报告。普通 `chrono-ci prepare`、`prepare-endpoints` 和 `acquire-seed` CLI 已退役；事件准备由登记输入程序调用内部 API。`crates/ci-tests/tests/units.rs` 核字面生成命令及所选操作，避免生成结果静默偏离本地命令。

Go／TS／mix 的接入方式与验证限制见[示例索引](examples.md)，原 CI 记录直接查各宿主的 Actions／PR。固定旧发布的有限采用不证明当前候选；有效输入完整、各 provider 版本宿主采用、确定同判及完整启用仍分别验收。

<a id="host-customization-and-updates"></a>

## 宿主定制与更新

使用宿主登记来源路径。各类定制的所有者如下，不要求业务目录或语言惯例：

| 修改 | 宿主来源 | 应用方式 |
| --- | --- | --- |
| 命令、测试、依赖 | 项目／脚本 action，FILEMAP 边／计划 | 更新声明，运行标准 DELTA 检查 |
| 单元边界、共享操作 | 检查 profile 的 `policy.units`／`policy.shared_operations`，provider `units` | 每完整计划只分一次，重新生成拥有的工作流 |
| SDK、runner 镜像、超时、启动 | provider `bootstrap`／`runs_on`／`timeout_minutes` 及明确 bootstrap 数据 | generate、verify，再验证改变的真实事件 |
| 工作流路径、移除单元 | 旧新 provider 来源 | 用下文 migrate 核所有权、退役旧输出 |
| harness 版本 | 公开分发锁和所选工具表 | 采用新固定版本，保留宿主 CI／SDK 来源 |

[Go／TS／mix 示例](examples.md)仅改分发锁和 README 演示最后一行，无需改 Rust 或给生成器加语言分支。schema／行为变化仍须明确迁移，版本号增加不自动证明兼容。

宿主拥有 provider JSON、检查 profile、分配和 bootstrap；改这些来源，工作流只是生成结果。单元可通过字面 bootstrap argv、完整计划调用登记脚本／插件，无关工作流可并存。生成器不替宿主选择语言、SDK、布局或依赖。

init 逐字保留已有来源，含 runner、超时、bootstrap 参数、上下文、产物设置，传入默认值不替换它。安装固定新版本后，对同一来源 generate／verify；生成验证与真实标准检查／原生事件验收分开。

公开 beta.13 提供 `chrono-ci migrate`。先准备新 profile、分配、provider，再迁移已验证旧工作流：

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/ci/github.json \
  --config .chrono-harness/ci/units.json
```

前一来源可为当前 scoped v4（无初始清单）或 units，目标须明确配置 units。写入前验证全部旧生成文件及新冲突，精确保留目标 JSON，写新输出，只删过时旧工作流；旧来源地址不同且字节匹配明确输入时才退役它。旧生成结果被手改或新路径冲突，均在写入前失败；无关文件保留。

初始清单不能迁入单元汇总。同一所有者支持[独立 v4 保留清单迁移](ci.md#standalone-initial-host-transition)，仍拒绝完整治理和发布 provider 迁移。

同址更新须在编辑前保留旧来源（运行临时文件放 `.chrono-harness/state/`），明确提供原地址，保持嵌入工作流命令绑定：

```sh
chrono-ci migrate --host-root . \
  --from .chrono-harness/state/previous-units.json \
  --previous-config .chrono-harness/ci/units.json \
  --config .chrono-harness/ci/units.json
```

这也退役明确删除／改名的工作流路径，保留新来源和旧输入文件。JSON 列写入／退役路径和输入摘要。同步 FILEMAP／明确消费者，运行标准 DELTA。迁移不推断／编辑登记政策，成功不证明治理或 CI。文件系统失败报告已完成变化，无跨文件事务／并发隔离；保留原结果，协调报告路径后再重试。完成迁移后普通生成幂等。

gather 保留下载报告原字节，不用硬链接替换报告。本仓采用 `retained-reference/v2`，引用原报告和流，发布时避免重复正文。失败下载、原生失败、原 context／preparation／transport 证据仍各留在获取尝试。

当前 profile 在原保留 judge stderr 采用 [CI 进度诊断](diagnostics.md#scoped-ci-execution-progress-adoption)。完整单元汇总和上传后确认仍要求原最终报告，进度记录不能满足或豁免。单元失败／不完整时，先保留生产启动、实际 judge 流，再期待最终汇总；从原件关联 request／candidate／profile、plan／operation、PID／所有权。

此采用的原生验证仍需已提交候选和登记 DELTA 单元；局部 Rust fixture 不证明原生超时已修复或已合并落地。

输入生产失败可能只留下获取原件，没有最终报告文件。上传成功只导出已有证据，不造报告或通过结果。交付拒绝明确点名缺报告或无法核的原件，保留退出 2、原 outcome 和 delivery 根，不释放其他消费者或确认未生产证据。

报告位置／原件缺失、引用摘要变化和失败 outcome 确认的回归，由原未过滤操作 `test.runner-tests` 执行；标准选测继续使用完整 DELTA 计划，不维护独立过滤入口。
