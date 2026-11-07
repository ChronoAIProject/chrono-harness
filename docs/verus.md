# Verus 实施范围

本仓按批次把实际决策和状态逻辑迁到 Verus。完整目标是所有 SPEC 与仓内程序逻辑都有证明，
外部接口只采用明确登记的假设。目前只完成下列局部范围；全仓强制覆盖门和其他程序的迁移仍在实施。
Python 实现暂缓，不把未迁移程序列为已证明。
新增规格、决策、状态维护与独立边界预期直接写在 Verus 中，登记整 crate 证明并由实际消费计划强制执行。
新增未证明逻辑不能以“以后迁移”豁免；全仓存量覆盖门仍未完成。

## runner 标识符、路径与结果状态

`crates/runner/src/verified_identifiers.rs` 是实际 OID 和 CI 单元 ID 判断的唯一实现。
OID 必须恰为 40 或 64 个小写十六进制字节，且不全为零；ID 必须非空，
每个字节都属于 ASCII 字母、数字或 `._-`。循环证明返回值与各自规格充要相等，包含边界和终止。
`facts::full_oid` 与 `units::id` 直接消费这些结果。

摘要格式也由同一根的 `digest_format`／`valid_digest`／`valid_digest_text` 规定和实现：
必须恰为 64 个 ASCII 小写十六进制字节，允许全零。`wire::is_digest` 直接重导出文本判断，
现役响应、原件、输入和工作树使用方消费同一实现。字节循环证明完整输入的充要条件和终止，
文本入口证明借用完整 UTF-8 字节；不把格式证明当作 SHA-256 计算、真实性或原件存在证明。
独立根核 0／40／63／64／65 长度、全零、数字和 a–f 端点、任意位置的非法字节，
以及完整混合文本、NUL、换行、大小写和 UTF-8 序列。复用原六项 runner 整 crate 操作；
`str::as_bytes` 与切片长度／索引使用固定 vstd 的外部合同，没有新增受信仓内实现。
最多读取 64 个字节，使用常量额外状态；这是算法估计，完整构建、验证、CI 与存储成本未测。

普通请求和初始库存请求的头部判断也直接使用同一根的 `valid_request_header`。
返回真当且仅当协议和模式分别与调用方给出的精确值相等，且 judge ID 非空。
普通请求绑定 `chrono-judge/v1`／`evaluate`，初始请求绑定
`chrono-initial-judge/v1`／`inventory`；不额外限制 judge ID 的字符。
独立根核两个有效组合、交叉协议／模式、空名称、大小写、版本后缀、NUL、换行，
并证明任意非空名称均接受，包括 Unicode。复用 runner 现有两项标识符整 crate 证明，
没有新增验证进程。请求封存、JSON／摘要和外层组合仍待证明。
比较最多读取协议和模式的完整字节，名称只查空值；构建、验证和完整运行成本未测。

`crates/runner/src/verified_paths.rs` 是 `relative_path` 的实际判断实现。
它精确描述当前 POSIX 词法路径合同：拒绝空、绝对路径、开头的当前目录、任意完整父目录分量，
以及 NUL、换行、回车和反斜杠；保留原本允许的内部 `.`、重复分隔符和末尾分隔符。
调用方的 `str` 保证 UTF-8 有效性。规格与执行循环证明全部字节的拒绝条件、分量边界及终止；
不据此主张文件系统解析、符号链接或目录穿越的完整安全合同。

`crates/runner/src/verified_status.rs` 是实际 CI 结果 `Status` 的唯一类型定义。
四种状态 `Passed`、`Failed`、`NotRequired`、`Blocked` 的比较精确保持变体身份，
克隆保持原状态；独立根逐项列出全部 16 对比较预期。实际报告与汇总通过 runner 的公开类型使用同一实现。
原 Serde 派生和 JSON 名称保留，Rust 回归核四种名称、往返及非规范值拒绝；
序列化适配、外层状态转换和完整响应合同仍待证明，不把派生实现计为已证明。
同一模块也拥有真实 `CheckResult`，其克隆证明精确保留 ID、状态、原因和可选退出码。

独立 Verus 根直接读取生产源码，声明成功、失败和边界预期。Cargo 消费同一源码并擦除 ghost；
Rust 测试不能代替证明。其他 runner 逻辑、调用适配器与完整 SPEC 对应仍未证明。
把错误长度的 OID 改为接受，会直接违反返回值后置条件（6 verified / 1 error）；
正确源码按 SHA-256 恢复，再运行正式检查。

实际启动准备判定也使用同源 `verified_consumption::matches_preparation`，证明命令、宿主配置、
唯一先行 verifier、执行工具和项目登记必须精确对应。消费关系判官读取显式 preparation，
再核原共享 core 合同；解码和外层组合仍待证明。独立根与 Rust 回归核缺前置、重复、
把校验放入普通操作、工具／schema／命令／项目错绑及缺文档。registration 原计划增加两项整 crate 证明。

## 协议状态与 finding

实际 `wire::Status` 也归 `verified_status.rs`，保持 pass／warn／fail／error 的外部编码。
身份比较、`pass < warn < fail < error` 的完整顺序、克隆和退出码 0／0／1／2 均由同源 Verus 证明；
真实 `Finding` 的克隆保留 code、level、message 及两个引用数组的完整字符串、顺序和重复。
`wire::validate_response_at` 和 `full::validate_record_response` 直接消费同一 `findings_match_status`，
只读取原 finding 的完整 level 字段：
fail／error 当且仅当存在 `error`，pass 不允许 `warning`，warn 必须存在 `warning`。
该循环证明完整列表的存在量词及终止，不由调用方提供预先计算的标志。

独立根列出全部 16 对状态比较、四个退出码及全部 error／warning 存在组合，
另核空列表、info、排列和重复、大小写／Unicode／NUL 级别、其他字段含误导文字及完整克隆。
现有协议进程测试继续核四态真实 JSON、退出码与错误状态拒绝。
finding 格式、引用准入、响应身份、证据 IO、JSON 解码和外层组合仍待证明。
状态判断为常量工作；finding 扫描为 O(F) 条记录，使用常量额外状态，不增加进程。
这是当前算法估计；完整运行、构建、验证、CI 和存储成本未测。

## 实际图内核

`crates/judge-filemap/src/verified_graph.rs` 是实际有类型边比较、两端边并集、名称索引、
FIFO 遍历、稀疏前驱树、消费者行投影和原因集合的唯一实现。
`graph::union`、`graph::closure` 和原因链查询直接消费这些实现；Cargo 擦除同一源码的 ghost。

证明保留完整 UTF-8 字节身份、边类型、两端来源、每个槽位的目标及 seed 根索引。
FIFO 首次发现生成确定的最短跳数和前驱；传播行逐槽对应原名称和完整出边，包含平行边、环和自环。
返回名称恰好存在有限根路径，稀疏前驱集合精确等于首次发现槽。
前驱树按名称字节严格有序；原因链查询终止，缺失、错目标、环及自环有限失败。

节点及遍历边的原因状态使用 `CauseSet`。其私有字符串向量按完整 UTF-8 字节严格有序，
二分定位、成员关系、精确新增／重复信号、集合和长度更新、首项、克隆及归一化均有证明。
重复插入保持原序列；归一化保全输入的精确字节键集合。JSON 仍使用有序字符串数组，
反序列化后调用已证明的归一化。没有新增仓内受信字符串排序接口。

逐 seed 调用使用同源 `SeedPropagations` 流式状态。每次成功恰推进一个位置，
保持原 seed 的 ID、node、reference、reason，传播行和最短前驱对应同一位置的拓扑根；
只有全部位置消费后返回 `None`，重复终态不再推进。实际闭包直接消费该状态，移除外层 `zip`。
独立根核空输入、重复 seed／ID、不同根、完整 Unicode／NUL 字段；根数量不匹配不满足构造前置。
状态只保留拓扑／seed 借用和一个游标，不同时留存所有 seed 的传播行；完整成本未测。

实际 `TraversedEdge` 也归同一 Verus 根，克隆保留完整边键和原因集合。
`selected_test_targets` 只从这些原记录的 `TestExecution` 边选择目标，
输出按完整 UTF-8 字节严格排序，集合恰等于全部执行边目标，重复记录和重复目标不增加成员。
空输入以及仅含 compile／build-input／runtime-input／judge-trigger 的输入均不选测试；
名称内容、源节点名称和原因集合不改变边类型判断。循环有终止证明。
`Closure::selected_tests` 直接转发此结果，再用原标准库转换接口返回 `BTreeSet<String>`。
独立根核全部边类型、空输入、重复目标、完整 Unicode／NUL 名称与记录克隆；
现有 Rust 消费者回归保留原返回类型、JSON 与选测预期。

外层节点／边关联、seed-ID 映射、完整原因合并、JSON／诊断适配、
外层额外选测和完整 SPEC 对应仍未证明。局部图证明不证明实际登记完整，也不代表全仓覆盖完成。

## 执行资源上限

`crates/runner/src/verified_limits.rs` 负责执行资源准入的唯一判断：超时为正 `u64`，
输出为 1 至 67,108,864 字节（64 MiB，含两端）的 `usize`。
`valid_execution_bounds` 和单独的 `valid_output_limit` 分别证明返回值与规格充要相等。
实际执行器的命令检查、不可变输入传输、registration 的默认／逐操作上限和 routes 的计划检查直接调用它们；
使用真实字段类型，无中间数组、整数转换或新增外部接口假设。
生产根覆盖任意类型有效的参数；独立 Verus 根直接调用实现，核零值、最小值、最大值、越界一字节、
`u64::MAX` 超时与 `usize::MAX` 输出。原 Rust 消费者测试仍负责外围计划和进程行为。
runner、registration 和 routes 的登记计划把两项整 crate 证明放在原构建和测试之前。
判断使用常量时间与额外空间，不增加运行时进程；构建、验证、CI 和存储的实际成本未测。
外层解析、计划组合、启动状态和完整 SPEC 对应仍待证明。

## CI 离线汇总的结果集合

现役 `judge-ci` 的执行结果转换和终态也由同一证明根拥有。
`operation_result` 精确保留原操作 ID 与错误文本，只把完整 `passed` 映射为 Passed、
完整 `blocked` 映射为 Blocked，其余状态全部映射为 Failed。
原错误缺失时采用既有默认原因；空错误和 Unicode／NUL 错误均原样保留。
有原退出时，Passed 保留任何退出码，其余状态只保留非零退出；
零退出后的回执／基础设施失败不伪造成非零子进程退出，原回执继续保留真实零值。
来源是当前 projects 原结果的直接字段转发；projects 的状态生产、回执准入与 JSON 适配仍待证明。

`response_status` 直接读取完整真实 `CheckResult` 列表：返回 Failed 当且仅当存在 Failed 或 Blocked，
否则返回 Passed，包含空列表；ID、原因、退出码及重复不参与该终态规则。
本地／单元执行和离线 collection 共用同一实现，循环证明完整遍历和终止。
库存异常由 `inventory_failure` 同时生成原原因的唯一失败记录和失败状态；
诊断发布失败由 `diagnostic_failure` 追加真实失败记录并返回失败，保全已有全部记录及顺序。
独立 Verus 根核四态、未知／大小写／NUL 状态、缺失／零／正负及整数端点退出、
默认／空／Unicode 错误、首尾失败、重复与空列表，以及两类失败状态与实际记录的对应。
复用现有 CI 两项整 crate 操作，不增加证明进程、依赖或外部假设。
终态最多遍历 N 条记录、使用常量额外状态；完整成本未知，不据此主张全仓覆盖门完成。

`crates/judge-ci/src/verified_collection.rs` 是离线汇总实际消费的结果 ID 集合比较。
期望列表由同一证明根构造，精确包含 `ci.inventory`、登记操作和未要求测试。
返回真当且仅当实际 ID 与此期望列表互相包含；构造和两个方向的比较循环均证明终止，
空集合、排列、缺项、额外项、缺库存标记及完整 Unicode／NUL 身份由同源独立根核对。
集合含义不依赖顺序或重复次数；响应协议继续单独拒绝重复结果 ID。
实际汇总直接消费同一证明根对真实 `CheckResult` 的状态判断：操作成功须有同一条精确 ID、
`Passed` 和 `Some(0)` 的记录；库存标记须是 `Passed`／`None`，所有未要求结果须为 `NotRequired`。
未要求状态不额外限制退出码，完整 ID 覆盖仍由集合合同承担；各遍历均证明终止。
独立 Verus 根核字段分散在不同结果时拒绝、四种状态、缺失／错误退出、空结果及逐字段克隆。
实际结果 ID 的借用提取也由同一根证明：输出逐项等于真实记录的 ID，保留顺序、重复、完整 Unicode 与 NUL，
集合合同直接关联原报告字段；调用方不再提供结果 ID 投影。
登记操作和未要求列表的字符串借用同样由 `borrow_texts` 证明等长且逐项相等，
实际调用方直接消费；独立根核顺序、重复、Unicode、NUL 和空列表，并组合验证真实报告覆盖。
未要求列表直接保留原有序唯一 `BTreeMap` 键遍历的向量，不再重复构造同序集合；JSON 数组合同保持。
原报告、流摘要、JSON 解码、登记操作／未要求列表的外层提取和组合仍有各自未完成义务。

比较使用借用字符串向量，不再为集合比较克隆和排序结果 ID。
实际与期望长度为 A、E，最长键为 L 字节时，最坏比较成本为 O(AEL)，
借用向量占 O(A+E)；完整运行、构建、验证、CI 和存储成本未测，不宣称性能收益。

## 所有者可执行文件发布

`crates/worktree/src/verified_owner_publication.rs` 是实际发布身份比较、原件保全、意图读取／准备、
所选文件替换／重入和最终绑定检查的唯一判断实现。绑定包含 inode 身份、SHA-256 和长度；
完整阶段还绑定目标路径、保留阶段路径和旧文件绑定。发布意图和阶段验证分别要求精确 schema，
阶段验证另绑定原报告摘要。不可变原件按完整字节比较，保留 NUL 和非 UTF-8 字节身份。

原件已有时保留，随后仍核其绑定；缺失时只允许从精确旧绑定创建。无意图却有准备文件时拒绝，
有意图则读取原件；所选文件必须是精确旧绑定或精确已发布绑定。两绑定相等时先执行旧绑定分支。
文件观察核 inode、长度、修改秒及纳秒全部不变；保留路径仅证明登记的字节前缀。
全部执行结果与规格充要相等，独立根逐字段核身份／内容／长度漂移、原件及意图有无、重入、
错误 schema／摘要、完整阶段错绑、任意字节和整数边界。

Rust 所有者在原准入和全部受影响登记的独占租约内直接消费这些判断，再执行硬链接、同步和原子替换。
原 Rust 行为测试继续验证真实文件系统、失败恢复及租约排除。Serde 派生只承担明确的编码接口，
JSON 解码、哈希、metadata／inode 观察、路径解析、Git、文件系统操作和外层部署／采用组合不在此局部证明内；
不据判断证明声称整个文件系统事务已证明。

## 固定执行入口

宿主 bootstrap 的 Rust 迁移正在实施，尚未替换当前 Python 入口。
`.chrono-harness/ci/native/src/verified_bindings.rs` 拥有工具绑定来源、摘要形状、
字节与版本比较、可重定位路径、重复旧路径退役、继承编译器和两个 wrapper 的判断。
版本前缀另要求完整空格分隔；Cargo 可接受精确版本字符串，rustc 必须保留版本后的空格。
独立根按成功、缺输入、冲突和边界声明预期；生产根和独立根都登记整 crate 证明。
普通 Rust 适配已直接调用这些判断来解析声明、查询 Rustup 和观察真实工具；
JSON 解码、字段提取、路径和环境处理、进程与文件 IO，以及完整启动组合仍未证明。
Rust 回归另核真实 Cargo／rustc 的身份、重定位和继承选择；它不替代证明，
也不证明共享安装、冷启动和发布调用方已迁移。

本地和 CI 使用宿主上下文登记的 bootstrap 与 canonical check。
`test:runner-tests` 先运行八项整 crate Verus 操作及编译，registration 先运行四项，
routes、filemap 与 CI 汇总各先运行两项；同一计划内共用的证明操作只执行一次，
worktree 的 core／adoption 两个消费计划共用两项发布证明，再执行各自原 Rust 构建和测试。
独立根直接读取生产源码，声明成功、失败和边界预期；
Rust 测试另外验证真实消费者，不替代证明。

官方 Verus 固定为 `0.2026.04.24.f8e1704`／Rust 1.95.0，强制
`--no-cheating --no-external-by-default`，没有函数过滤或新增证明预算。
现有通用 bootstrap 消费明确登记的 HTTPS 下载、SHA-256 校验和解包操作。
七个证明单元先核共享 core，再安装工具。
当前归档只适用于 macOS arm64。

## 此批使用的外部接口

本批新增生产逻辑采用下表的具体外部接口，合同来自固定 Verus／vstd；
类型或库名本身不授予其他仓内逻辑豁免。没有新增 `external_body`、`assume` 或证明过滤。
其余存量证明根和程序使用的外部接口仍须完整清点，不据本表主张全仓覆盖。

| 外部接口 | 本批用途及采用的合同 |
| --- | --- |
| `str::as_bytes` | 借用完整 UTF-8 字节，返回切片视图等于 `str::spec_bytes()`；后者由固定 vstd 定义为字符串字符序列的 UTF-8 编码。摘要判断读取原字节。 |
| `String::as_str` | 借用原字符串，字符视图相等；finding 判断读取真实 level。 |
| `str::is_empty` | 返回真当且仅当字符序列长度为零；请求头只要求 judge ID 非空，不附加 ASCII ID 限制。 |
| 切片 `len`、`<[T] as Index<usize>>::index` 及 `<usize as SliceIndex<[T]>>::index` | 长度和逐项读取对应切片视图；本仓循环另行证明索引界限、完整遍历、返回条件及终止。`T` 在本批为 `u8` 或 `Finding`。 |
| `<String as Clone>::clone` | 复制完整字符串；本仓 Finding 克隆证明各实际字符串字段均保留。 |
| `<Vec<String> as Clone>::clone` | 保留长度和逐项克隆关系；本仓 Finding 克隆证明两个引用数组的完整字段、顺序和重复均保留。 |
| `str::to_string` | 生成完整字符串；CI 结果 ID、原错误及默认原因均保持完整字符／UTF-8 身份。 |
| `Vec::new`、`Vec::push`、`Vec::len`、`Vec` 的 `Index<usize>` | 空序列、追加后保全原前缀、长度及逐项读取；本仓证明 CI 终态循环界限、完整遍历、记录对应与终止。 |
| `Option::as_ref`、`Option::map` | 无回执保持 None，有回执仅转发同一原记录的 `process.exit_code`；不在适配器中计算状态或选择退出。 |

实际字段借用使用固定 vstd 的 `String::as_str` 合同；协议和模式比较复用已证明的
`equal_text`／`equal_bytes`，其 UTF-8 转换采用 `str::as_bytes` 的完整字节合同、
切片长度／索引合同和固定 vstd 的 UTF-8 编解码引理。没有新增仓内受信实现。

`Status`／`WireStatus`／`CheckResult`／`Finding` 的 Debug 与 Serde 派生是明确保留的外部编码接口，沿用原 JSON 名称和字段合同，
不据这些派生主张解码、协议校验或仓内决策已经证明。工具下载、摘要、进程与 IO 的现有接口继续保留原验收义务。
`TraversedEdge` 的原 Debug／Serde 派生同样只承担外部编码。
选测返回转换明确保留固定 Rust 1.95.0 的
`<BTreeSet<String> as FromIterator<String>>::from_iter`：外部合同为保全输入字符串的精确字节集合，
按标准 `String::Ord` 排序并去重。此处没有自写受信实现；该标准库转换未由本仓机械精化。
Verus 证明覆盖转换前的完整选择、去重和顺序，现有 Rust 回归另外核实际返回值。

选测复用 `CauseSet`。遍历 E 条记录、选择 U 个不同目标、最长键 L 字节时，
按字节比较和字符串句柄移动估算，向量插入的最坏成本为
O(E(1 + (L+1)log(U+1)) + U²)，再有原标准库集合转换。
这是当前算法的上界估计，不是 Verus 机械成本证明。临时向量保留 U 个目标字符串及句柄，
不增加运行时进程；完整运行、构建、验证、CI 和存储成本未测。

finding 克隆使用固定 vstd 对 `<String as Clone>::clone` 和 `<Vec<String> as Clone>::clone` 的外部合同，
前者保留完整字符串，后者保留长度及逐项克隆关系；本仓实现证明各实际字段均保留。

**产品与宿主政策同时变更：**实际判断、状态类型与图改为同源 Verus，FILEMAP、工具、启动及执行计划登记十六项证明。
固定宏依赖通过 runner 传播到现役依赖方的独立 lockfile，没有升级其他包。
runner 判断仍为线性字节遍历，未增加运行时进程；`CauseSet` 定位需要 O(log C) 次字节比较，
新增插入最多移动 C 个字符串句柄，存储为 O(C)。原图索引与遍历成本继续保留；
构建、验证、CI、存储及完整运行成本未测，不宣称性能收益。
