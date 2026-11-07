# 可选的 Cargo 项目检查

本仓的 diagnostics 库与其路径依赖消费者使用登记的锁定 Cargo 构建。独立的受保护构建产物没有接入这些消费者；专属 guard、主机快照、vendor 准备和启动 profile 已退役。普通构建、格式检查及 diagnostics 的真实行为测试继续由原计划执行。

`judge-cargo`／`judge-cargo-tests` 是独立的生产／测试项目对。通用 registration、projects 不依赖它或 TOML。宿主通过登记二进制、参数、政策输入和 FILEMAP 边采用此适配器。Go／TS／脚本宿主不会因为出现类似 Cargo 的文件或目录名，就必须采用 Cargo 政策。

公开 beta.16 分发下文的 v5 工具链清单能力。其原生发布在 macOS arm64、Linux x86_64 执行适配器专属测试；这不证明编译器／SDK／委托输入完整，也不启用产品宿主。

政策须位于宿主 `.chrono-harness/` 下，并登记在 FILEMAP：

```json
{
  "schema": "chrono-cargo-projects/v1",
  "projects": [
    {
      "project": "library",
      "manifest": "units/library/Cargo.toml",
      "lockfile": "units/library/Cargo.lock",
      "output": "outputs/library/",
      "ancestor_manifests": []
    }
  ]
}
```

每行明确把已登记项目连到其拥有的 manifest／lock 输入和已登记生成目录。不从通用项目字段解释 Cargo 含义，也不从源码根推断输出位置。`ancestor_manifests` 须明确列出适配器要检查的已登记 Cargo 祖先文件。适配器与已登记祖先路径核对；漏声明就报错，不自动增加输入。

这项有限的结构检查拒绝 workspace 合建／关联、workspace 依赖继承、manifest／lock 或输出缺少所有者，以及找不到唯一 Cargo 项目和明确 FILEMAP compile 边的路径依赖。它检查普通、dev、build 和按 target 生效的 TOML 依赖。registry／Git 依赖在此检查中失败；要验证其保留输入和真实解析结果，须采用下文的受保护操作。两种模式都不证明编译器／后端、构建脚本、链接器或 SDK 输入完整。

完整检查协议的绑定方式为：

```text
chrono-judge-cargo --protocol chrono-judge/v1 --policy .chrono-harness/cargo/projects.json
```

将它登记在 `registration`、`filemap` 之后，并让 `projects` 依赖它，使其先于操作运行。完整适配器验证请求身份、登记视图、保留输入和已提交政策字节，只检查传入 FILEMAP 明确影响范围内的 Cargo 项目。其他语言不受影响，文档变更也不会重判历史 Cargo 缺陷。政策、manifest、lock 和祖先文件的变更都须明确连到消费者。错误保留实际政策路径；通过输出列出已检查项目，并明确不证明输入完整。

维护完整登记数据的宿主，也可把以下命令登记为一个普通前置操作，明确选择项目：

```sh
chrono-judge-cargo check --host-root HOST --config .chrono-harness/config.json \
  --policy .chrono-harness/cargo/projects.json --project library --project library-tests
```

命令不自行选项目，何时运行由调用方的 FILEMAP 计划决定。完整协议和独立模式共用检查函数。外层标准 runner 继续负责干净快照、工具绑定、操作回执及前置失败处理。这项有限适配器不启动 Cargo 子进程。

## 受保护的 Cargo 操作

将下面的普通操作登记为项目 action，通过已有 FILEMAP 执行计划选择：

```sh
chrono-judge-cargo run --host-root HOST --config .chrono-harness/config.json \
  --policy .chrono-harness/cargo/inputs.json --operation test.library
```

通用 runner 调用登记的保护程序。保护程序负责 Cargo 配方：验证政策与保留输入，观察配置的 Cargo 二进制及版本，运行锁定／离线 metadata，将解析结果与声明比较，通过后才运行已声明 Cargo 消费者。在 metadata 后和消费者后，都会再次核对输入身份。普通完整检查和按范围检查不需要 Cargo 专用回调或字段。

保护程序沿消费者明确的 `compile` 源码依赖，以及这些源码的直接文件、工具、环境和输入关系确定输入范围。提供运行时或构建产物的项目，不会把制造该产物所需的输入带给消费者。产物、工具绑定和前置回执仍由各自登记的执行负责人核验；源码依赖及所选配置、工具链、包输入仍保留原有前后身份检查。

**产品与宿主政策曾同步修改：**这个边界移除了本地 diagnostics 经运行时工具引入的无关构建输入。当前本机消费者绑定自身的 1,125 项文件输入，包含现有 vendor 文件、明确配置和工具链输入。同语言回归要求运行时／构建生产者关系下的真实 Cargo 操作成功，同时继续拒绝 compile 依赖中的输入变化。候选 bootstrap、标准变更和真实 CI 验证仍须完成；总运行及存储收益未测，完整宿主输入范围仍未闭合。

受保护的 `run` 通过共享登记获取模块，接受直接宿主配置及已有 `chrono-git-configs/v1`、`v2` 选择器。登记 action 仍须精确点名调用入口。`registry_binding` 报告该入口、实际直接政策及选择结果。两个文件原字节及取得的登记，都在启动子进程前、metadata 后、消费者后检查。选择器不能证明另一宿主的工具链或输入合格；未知身份、选择或绑定不符，都在 Cargo 启动前失败。

政策入口还可使用 `chrono-cargo-input-policies/v1`，只含 `schema`、`configurations`。后者把明确的有效配置路径映射到直接 `chrono-cargo-inputs/v2`–`v6` 政策路径。选择使用已取得的登记身份，不探测平台或挑选可用工具链。两个路径都须是宿主 `.chrono-harness/` 下的字面路径，两份文件都须登记并连到真实消费者。缺映射／文件、嵌套选择器、别名、路径穿越或通过直接目标参数绕过选择，都会失败。操作继续只点名一个入口。新增的 `policy_binding` 报告两份原摘要和有效配置；每次子进程结束后都保留并核对两份原件。

**本仓 diagnostics 曾同步修改产品与宿主政策：**旧原生政策移到 `cargo/diagnostics-native.json`；稳定入口 `cargo/diagnostics.json` 为固定原生配置选择该文件，为 macOS 27 选择 `cargo/diagnostics-local.json`。本地政策绑定实际所选 Rust／SDK 根，并通过 `manifest_root` 绑定原清单根。核对原摘要或缺失声明后才迁移本地输入路径。内容不同的 rustix 索引保留原所有者精确字节，不能冒充所选 Cargo home 的索引。

这些提案声明不证明输入完整或同判。新增成本为一次有限政策读取和身份复核，总验证成本未测；原期限和受保护操作继续适用。

保护程序的登记开发构建与其他清单判官一样使用优化级别 1。大型 sysroot／SDK 清单仍执行原有全部文件、目录、别名及边界检查，只改变编译后的执行成本。受保护操作仍使用各自登记的期限。实际外层消费者须在所选宿主证明性能符合期限，不能只凭构建 profile 或小型回归判断。

### 目录清单如何复核

验证器从明确文件声明一次算出物理父路径。每项绑定先检查父路径，再分路检查文件；每个叶项都须是非链接普通文件，SHA-256 须匹配。随后核目录子项和字面别名，最后复核全部物理父路径。这样避免每轮为每个文件重复拼父路径，但不跨轮保留物理观察结果。

在初次准备或一轮输入不变检查内，完整通过的绑定可把已验证的规范根文件路径和预期摘要借给后续绑定。同路径、同摘要的重叠声明复用该轮内容观察；摘要不同就拒绝。每项绑定仍检查自身声明的物理路径、目录子项和别名。准备阶段这些绑定之间没有子进程，只有完整通过的绑定才贡献观察结果。

每轮复核从空文件表开始，因此每次子进程结束后仍能发现持续存在的父路径替换、文件修改和别名漂移。表借用现有声明，每个不同的规范根文件路径至多一项，不跨子进程边界保留内容观察。分配／查询成本未测。这不是文件系统快照保证，两次观察间修改又恢复的变化仍不在合同内。

每项绑定沿用两个有界线程，检查声明文件，对本轮尚未验证的文件计算摘要。返回验证结果或启动受保护子进程前，两个线程必须都结束；任一线程错误使该轮失败。所选宿主每轮声明的逻辑文件量为 14,466,038,905 字节，包含重叠清单。去重后的实际摘要字节数和外层性能须由宿主测量。预期摘要、子进程边界和执行限制不变。

## 配置与包声明

配置政策使用 `chrono-cargo-inputs/v3`，必需字段如下。固定 v2 清单合同仍支持原绝对 `CARGO_HOME` 要求，没有祖先政策选项。

| 字段 | 要求 |
| --- | --- |
| `root` | 明确的包 ID，其 `project` 是 action 所有者 |
| `projects` | 与上文结构政策相同的 Cargo 项目记录 |
| `metadata` | 已登记 Cargo 工具和 metadata 调用的 `{tool, argv}` |
| `operations` | 登记操作 ID 到真实 Cargo `{tool, argv}` 的映射 |
| `target` | metadata 与消费者共用的明确目标三元组 |
| `configuration_files` | 明确的 `{path, input}`：根／Cargo home 查找文件、参数及递归 include 路径；选 `inventory` 时还含祖先。`input` 为已登记文件输入 ID，缺失须明确写 `null` |
| `configuration_ancestors` | v3 必须选 `inventory` 或 `absent`；v2 只用原字面清单 |
| `packages` | 针对此根和配置声明的完整包解析结果 |
| `timeout_seconds`、`output_limit_bytes` | 各子进程限制，须为正，输出最多 64 MiB |

保护程序从明确宿主根调用 Cargo。须声明根下 `.cargo/config`、`.cargo/config.toml`，以及明确配置的 `CARGO_HOME` 内 `config`、`config.toml`。v3 接受绝对 home，或相对同一宿主根解析的 home，不使用保护程序调用者 cwd，也不推断 HOME。

选 `configuration_ancestors: "inventory"` 时，还须声明根的每一级严格祖先下两个查找文件，以及按顺序传入的每个 `--config` 和递归 `include` 目标。路径可相对宿主根或为绝对路径，解析位置须与登记输入一致。路径缺失也须写 `input: null`，漏字段报错；可选但不存在的 include 同样处理。这些都是宿主明确声明，不用于推断 FILEMAP 边或选测。

选 `configuration_ancestors: "absent"` 时，宿主声明**每一级严格祖先**的两个查找文件都不存在。保护程序核这个全称声明，拒绝任何存在的文件、符号链接或重叠清单记录；已登记文件也无隐含例外。需要祖先配置就选 `inventory`。这种选择避免把检出深度和祖先名写进本可相对描述的政策，不会发现或登记新输入。根查找文件、Cargo home 文件、参数和 include 仍各需明确记录。

报告新增 `configuration.ancestor_absences` 列出真实检查位置，并在工具观察、metadata、执行后再次确认缺失。这些观察不提供保留的前后端点快照、不发现未登记的语义读取，也不证明完整输入同判。

存在的文件须登记输入、预期 SHA-256 和到消费者的明确依赖路径。缺项、多项、重复行、存在状态或身份错误、未声明 include、include 循环都拒绝。符号链接文件或父路径组件（含 `..` 前的链接）不在支持范围。Cargo 的旧 `config` 优先于 `config.toml`；两者存在状态和身份都绑定，但被遮蔽的新文件不解析。适配器核 include 成员，Cargo 自己合并值和应用优先级。

工具观察、metadata、消费者后，都复核存在输入及声明缺失。这能发现跨边界持续存在的修改，不能发现观察间修改又恢复的写入。

例如 `{"path":".cargo/config.toml","input":null}` 只表示该路径，不能代表其他必需路径。早期 v1 迁移须用这份完整明确清单替换 `configuration_inputs`，保留原输入 ID／摘要／边，并更新 schema；v1 继续拒绝。v2 迁到 v3 须明确选祖先政策：`inventory` 保留原行；`absent` 须移除严格祖先行，并确认所断言文件确实不存在。相对 home 和输入位置是宿主选择，不自动改写。现有 v2 政策含义不变。完整宿主采用仍为提案，有限的 diagnostics 范围见下文。

每条包记录包含 `id`、`name`、`version`、`source`、`project`、`manifest_input`、`inputs`、`checksum`、`features`、`dependencies`。本地包设置 `project`，`source`／`manifest_input`／`checksum` 为 null，`inputs` 为空；manifest、lock、源码所有权和输出目录来自明确 Cargo 项目行及通用登记。

外部包的 `project` 为 null，明确声明精确 Cargo registry 或 Git 来源身份，含 Git revision。`manifest_input` 须是 `inputs` 成员，后者列出实际包目录内每个已登记文件。registry 包要求 lockfile checksum；Git 包在 Cargo.lock 没有 checksum 时用 null。输入路径和 SHA-256 明确保留在 `config.environment.inputs`。

重命名并启用 feature 的 registry 包示例：

```json
{
  "id": "dependency",
  "name": "fixture-dep",
  "version": "1.0.0",
  "source": "registry+https://github.com/rust-lang/crates.io-index",
  "project": null,
  "manifest_input": "dependency.manifest",
  "inputs": ["dependency.manifest", "dependency.lib", "dependency.checksums"],
  "checksum": "<exact Cargo.lock checksum>",
  "features": ["bonus"],
  "dependencies": [
    {"package": "leaf", "name": "fixture_leaf", "kinds": [{"kind": null, "target": null}]}
  ]
}
```

这只是字段示例，checksum 和输入 ID 须换成真实声明。依赖的 `package` 指声明包 ID，`name` 为 Cargo 解析后的依赖别名。`kinds` 保留普通（`null`）、`dev`、`build` 和 target 条件。feature 集和依赖须与真实 metadata 完全一致。包 ID 由宿主选择；Cargo 随位置变化的 ID 按精确 name／version／source 映射，绝不用来生成边。

政策、包输入和配置输入都须通过 FILEMAP 的 compile、build-input 或 runtime-input 路径明确连到消费者。清单只比较真实文件与白名单，不增加输入、依赖或测试。未列文件、文件缺失、不支持的文件系统项、摘要或输入位置变化、锁 checksum 错误及输入未连接都失败。真实 metadata 另核包位置、登记 target 源、独立 workspace 根／成员和声明输出目录。

metadata 与消费命令须使用同一个登记工具、精确 manifest／target、锁定／离线模式、feature 选择及有序 `--config` 文件参数，保留参数顺序。支持 `build`、`check`、`test`、`run`、`bench`、`doc`、`clippy`；metadata 要求 format version 1 和 `--filter-platform`，消费者要求 `--target`。

初始参数语法还在适用位置支持 `--frozen`、`--features`、`--all-features`、`--no-default-features`、`--release`、`--profile`、`--lib`、`--bins`、`--tests`、`--all-targets`、`--examples`、`--benches`。未列标志、内联配置赋值和 `--` 后的参数都拒绝，支持它们须明确扩展适配器。某个已接纳组合是否合法仍由 Cargo 判断，保留真实失败。

外层登记 action 须用上文精确 root／config／policy／operation 参数调用保护程序。`chrono-cargo-run/v2`（v5 工具链政策用 `v3`）报告实际工具观察、metadata／消费进程的原 stdout／stderr、摘要和退出，以及检查过的配置清单、输入 ID、摘要、缺失路径和已解析文件。

解析结果被拒绝时保留成功 metadata，不产生消费回执。测试失败保留真实退出码（包括 101）。测试本身成功后输入变化，保护仍失败。进程超时／输出失败是错误；普通预检／验证失败退出 1，执行成功退出 0。报告是操作结果，不是新的 judge 协议通过结论，也不是完整输入凭据。

真实 fixture 覆盖重命名／传递 registry 依赖、feature／依赖种类不符、固定 revision 的 vendored Git 依赖、配置顺序、测试失败、执行后修改。完整和按范围 runner fixture 使用同一登记 action，文档变更不执行无关历史 Cargo 缺陷。专属 `configuration`、`inputs`、`input_consumer` 测试覆盖 Cargo home／祖先配置、递归 include、旧文件优先、明确缺失、路径别名、修改，以及完整／按范围拒绝和不执行行为。

产品构建并分发此可选二进制，完整宿主登记仍为提案。每份保护报告都写 `input_closure_complete: false`：包 metadata 和已声明保留文件尚未补齐编译器／后端／链接器／SDK、构建脚本读取、Cargo 凭据和其他服务输入。配置清单覆盖 Cargo 1.95 查找及 include 规则，不覆盖子进程的一切输入。其余输入声明、采用和完整原生同判仍待完成，现有核心检查不能证明这些缺口已解决。

## 明确绑定编译器

宿主可保留全部 v3 声明，增加 `cargo_input`（外部输入 ID）和 `compiler: {tool, input}`，选择 `chrono-cargo-inputs/v4`。`tool` 是另一个已登记编译器工具；两个输入 ID 都须存在、有预期 SHA-256，并通过 FILEMAP 明确连到消费者。探测版本前，保护程序核每个实际调用路径及字节与输入一致。每个工具都须声明预期版本。

报告保留 Cargo `tool` 观察，并新增 `compiler` 观察。v2／v3 保留原合同，不宣称编译器绑定，且拒绝 v4 字段，包括明确 null。

有效宿主环境须将 `RUSTC` 明确设为编译器绝对输入路径，将 `RUSTC_WRAPPER`、`RUSTC_WORKSPACE_WRAPPER` 都设为空字符串。metadata 和执行共用该环境。v4 拒绝对应三个 `CARGO_BUILD_*` 别名，也拒绝任何已解析 Cargo 配置（含递归 include）声明这六个环境名或 `build.rustc`、`build.rustc-wrapper`、`build.rustc-workspace-wrapper`。

这样编译器选择只有一个声明来源，不猜冲突声明中的宿主意图。被 Cargo 旧文件规则遮蔽的配置仍不解析，但保留字节。

v4 支持 `build`、`check`、`test`、`run`、`bench`；`doc`、`clippy` 需要额外工具合同，仍可通过原 v2／v3 范围使用。工具观察、metadata、消费者后都核保留输入。消费者成功但持续修改绑定工具时，保护失败并保留真实消费回执。瞬时并发替换不在前后观察保证内。

这只绑定提供给 Cargo 的执行物，不补齐其委托进程、编译器库／sysroot／后端、链接器、SDK、构建脚本读取或其他外部输入，`input_closure_complete` 仍为 false。产品宿主尚未采用 v4。专属直接及完整／按范围测试使用真实 Cargo 编译，检查所选工具轨迹、前后不符、配置拒绝及只改文档时不执行。

## 明确登记工具链清单

宿主可保留 v4 声明，增加 `toolchain`，选择 `chrono-cargo-inputs/v5`。v5 不发现编译器环境，每项都须明确输入 ID，并有到 Cargo 消费者的 FILEMAP 边。

| 字段 | 要求 |
| --- | --- |
| `sysroot` | `{root, manifest}`；root 是精确所选目录，manifest 输入使用 `chrono-input-directory/v1` 或 `v2`，`RUSTFLAGS` 必须恰好为 `--sysroot=<canonical root>` |
| `backend` | 明确的后端文件输入 ID 列表，可空；列出的 ID 须存在并连到消费者 |
| `linker` | `{tool, input, environment}`；使用独立登记工具、存在的输入 ID，以及选择规范链接器路径的环境变量 |
| `sdks` | `{root, manifest, environment}` 目录清单；每个环境变量须选择精确规范根 |
| `build_script_inputs` | 构建脚本外部读取的明确、存在输入 ID 列表 |

`chrono-input-directory/v1` 列出每个保留普通文件及 SHA-256；空清单、重复路径、符号链接和身份不符都在 Cargo 前失败。v2 保留这些文件项，另要求 `symlinks` 数组。每条链接明确 `path`、字面相对 `target` 和 `kind`（`file` 或 `directory`），链接路径须有物理父目录。

目标可在清单根内经过其他已声明别名和相对父组件。文件目标须有登记摘要，目录目标须含登记文件。未声明别名、越出根、重复路径、目标类型错误、解析循环或跟随超过 64 条链接都失败。v1 拒绝 `symlinks` 字段，包括 null。目录清单仍是明确子集，不是发现的依赖，也不证明 SDK 无其他输入。

消费者后，保护程序复核原根路径的规范目标、全部目录文件、字面链接目标、声明输入和所选路径。不同字面链接即使指向同一文件也不等价。消费者成功但改变声明别名或重定向目录根时，保护失败，保留原操作结果。递归 Cargo 配置不得提供第二个链接器或 sysroot 选择来源。

v5 以 `chrono-cargo-run/v3` 报告工具链证据和链接器工具观察；v2／v3／v4 保留原含义并拒绝 v5 字段。

这是可选且范围有限的清单，不证明后端库、链接器委托、SDK metadata、构建脚本、Cargo 凭据、OS 状态或网络响应完整，`input_closure_complete` 仍为 `false`。宿主明确登记并启用前，完整采用仍是提案。专属测试通过普通 Cargo 操作覆盖真实 Rust sysroot、链接器、SDK 清单、构建脚本输入、选择冲突和目录／输入变化。

## v6 原生目标与 v3 目录来源

`chrono-cargo-inputs/v6` 明确要求 `target_mode: "native" | "explicit"`。原生模式保留 Cargo 原目标目录布局，不注入 `--target`；核绑定 rustc `-vV` 的 host 与声明 target 一致，拒绝竞争的 Cargo build-target 环境／配置。v2–v5 含义不变。登记的具名测试集、过滤器和 libtest 尾部参数仍是实际消费的字面 argv。声明 `CARGO_NET_OFFLINE=true` 可提供离线选择，在线或冲突选择仍拒绝。

目录清单 v3 增加精确子项名称行，含根 `.`、空目录和只有别名的目录。v1／v2 含义不变。可选 `manifest_root` 可明确把未改原清单迁到另一所选现用根；来源错误、字节变化、别名越界、循环和子项变化仍拒绝。观察器原件由已有保护程序消费，包括操作后复核。阶段观察保留失败发生位置，不替代任何检查。

## 本仓已退役的额外保护链

runner 的额外 guard 没有接入正式检查，已连同专属实验和输入退役。原观察 JSON 中由获取测试实际读取的 34 项项目预期已迁入原 Rust 测试，断言保留。

diagnostics 是独立 Rust 库。runner、judge-ci 和专属测试通过 Cargo 路径依赖编译其源码，不消费独立 `build.diagnostics` 的产物。当前需求由普通锁定 Cargo 构建和真实行为测试满足，额外的环境保护未形成必须保留的验收终点。因此 `build.diagnostics` 恢复直接 `cargo build --locked --manifest-path crates/diagnostics/Cargo.toml`；diagnostics 单元复用已有 core 安装验证，再执行原未过滤测试。

**可见宿主政策与 CI 消费修改：**删除 diagnostics 专属 Cargo 政策、Git 选择器及两个直接配置、两个 bootstrap profile、观察声明、本机 Rust／Xcode 快照、vendor 配置和准备声明。同步移除 macOS 27 的 1,125 项独占输入声明、专属 file-input binding、3,430 条配置级边及三个公共 guard 依赖。Git、bootstrap 和三个独立工具输入仍有现役消费者，保留各自声明；其他普通 Cargo 环境绑定不受此次退役影响。

专属宿主组合测试和私有离线领域测试随功能退役；通用 bootstrap 的工具漂移、安装验证、普通 Cargo 领域及通用 Cargo guard 的成功／失败／边界行为测试继续执行。runtime-state 输入测试使用自含 fixture，保留原全部断言。这里只退役宿主采用，不据此声明通用 guard 产品能力必要性或完整输入已验收。

历史失败和原件由既有保管者按原义务保留；删除入口不表示历史实验通过，也不删除其他工作树或 state。验证走候选 bootstrap、两端 DELTA 的正式检查、PR CI 和落地后验；总运行、CI、存储收益未测，输入完整性与同判仍未建立。
