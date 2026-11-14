# CI generation 的依赖与 runner 适配器候选

本文服务 SPEC §2.1.1 的完整同源证明和实际编译绑定。runner 所有者负责真实借用 JSON、捕获 map 和 seen set；ci 所有者负责 generation 的真实生产闭包。当前交接是完整直接 adapter family 的源码候选，**不是严格证明、依赖准入、正式 DELTA 或交付结果**。全部 SPEC 目标仍未完成。

## 当前候选及边界

本阶段 AP 的固定源码基线为 `f6a982cb1e542f229d96aacbca6b6e630aef7e49`／tree `0b11a81f8cc5693963a64264fae472d64b031616`，工作树和分支沿用调用方指定的隔离候选。以下 adapter 语义表及原断言继续有效；本阶段只准备 runner 自属 Cargo 依赖生产和消费前拒绝，没有执行构建、测试、证明或业务。当前源码尚未封存或验收。

最近一次实际 runner 是 AN：11 份登记 extern 不存在，并报告不能找到 vstd；304 个编译错误后真实退出 1，`success:false`、整 crate、0 verified／0 errors。原件是协调宿主 `.chrono-harness/state/worktrees/use-29600-1794645686633816000-0.json`（509,533 字节／`877328db933c6b087df4993508d46701a46a8c445328fe044172b37092ccd023`）。为何丢失仍未知，不能把级联诊断当成 CAPACITY 或 AC 损坏的证据。AO 已核 runner 借用测试项目私有 target，且 runner／ci-tests 的 serde_json feature 与 profile 不同；版本号、旧 hash 表或 fresh 都不能建立生产／保管关系。

调用方随后用已核本树工具完成 status／migrate，原 enrollment EX 排除仅覆盖该协议登记的旧租约使用方；没有处置或 terminal handoff。AN 历史 managed descendant join 仍 unknown，`report_handoff_pending:true`、18 个未知所有者及原件继续保护。本阶段不重复生命周期操作。AC 的 1,557 verified、AE 的 11 verified 及严格 reject 事实继续保留，不扩大为 runner 成功；组件不下载、解包、patch 或重建。AP 的实际源码候选和未支持关系见末节。

| 具体实现及实际使用方 | 候选规格不变量 |
| --- | --- |
| `facts_binding::JsonBindingReferences` → `declaration` 的唯一 tool/input 引用 | count 是原切片长度；每个有效槽位的 id 是该行完整可选文本；规格的越界位置为 None，执行索引仍须证明界限。 |
| `facts_inputs::JsonGuardReferences` → guard declaration/reference | 同一借用切片投影；不复制、截断、归一化或重排 id。 |
| `facts_configs::JsonSelectorRows` → `collect_identity_paths` | 原顺序、对象成员总数、完整 digest/path、get 的键存在性；null 与缺键不同，错误类型仍为 None 文本。 |
| 同一 `JsonSelectorRows` → `validate_identity_registry_paths` | 同一原切片的完整可选 registry_path 文本序列。 |
| `facts_configs::JsonTargetRows` → `resolve_target/select_git_target` | count 是原长度；Digest 只选 sha256，Path 只选 path；保留 None、原槽位和完整文本。键选择仍在仓内匹配分支。 |
| `CapturedRegistry` → `identity/select_captured_target` | has_path 当且仅当实际 map 某原键的全部字符等于查询；不查当前 Git 或执行物。 |
| `SeenSelectorText` → identity hash/path 去重 | seen 为实际 set 成员的全部字符；record 保留旧文本集合并恰好插入 text，fresh 当且仅当此前不存在；执行仍调用一次原 insert。 |

`json_value_specs.rs` 是这些所有者共享的外部接口规格提案，不包含仓内选择／错误／状态判断。`lib.rs` 的隐藏文档重导出只供独立预期直接消费真实实现，不引入测试替身。Default 从派生改为同样的标准空树集合构造，以便声明空状态。

**源码与宿主登记同时修改：**FILEMAP 将四处当前生产源登记为 program，共享外部接口源为 specification，真实生产根为 `crates/runner/src/lib.rs`。独立 `runner-tests/tests/json_adapters.rs` 为 program，使用候选 runner 的同次库和 VIR。两操作进入原 runner-tests 计划并声明原 state 输出占用；原全部操作、线程、期限和行为断言保留。根登记表达义务，不能证明根已成功或获得准入。

## 外部接口及当前阻断

规格提案逐项对应固定缓存 serde_json 1.0.151：`value/mod.rs::as_str` 只借用 String 全文；`as_object` 只借用 Object；`get` 在非对象／缺键时为 None，存在但为 null 时仍为 Some；`value/index.rs::Index::index` 在缺键／错误类型时返回 Null，其 as_str 为 None；`map.rs::len` 返回原有限成员数。通用索引方法保留原泛型，只有本类实际使用的 &str 键解释为对象成员；不为其他索引类型编造已解释语义。

标准切片长度／索引、Option::map/is_some 和树集合 insert/contains_key 复用固定 vstd。该 vstd 的树集合合同要求 `obeys_cmp`；借用 String → str 的 contains_borrowed_key 及 &str set 与全文集合的对应没有当前可用的完整文本精化。提案只为固定标准文本键声明这些外部模型事实，不假设仓内状态更新正确。固定 Rust 1.95.0 的源码文档中，`core/str/traits.rs::Ord/PartialEq` 使用完整字节，`alloc/collections/btree/set.rs::insert` 返回 map.insert 的 is_none，`map.rs::contains_key` 返回 get 的 is_some；这支持所需合同的现实语义，不证明 Verus 模型或工具准入。

**严格入口仍被外部合同承载方式阻断。**固定工具帮助原件 `/tmp/chrono-generation-verus-bound-help-20261113.stderr` 明确把 assume、admit、external_body 和 assume_specification 列为 `--no-cheating` 禁止项。固定发行物 `builtin_macros/src/syntax.rs` 的 ProofAxiom 分支会把 axiom 转成 external_body；候选不能凭“外部接口”名称取得例外。当前源码提案中的 external type／function specs 和文本键 axioms 尚无已准入的独立外部规格载体；不声称它们会被严格根接受，也没有运行来产生新的拒绝记录。

最小负责方差异：固定工具／外部接口规格及依赖准入所有者须提供能在现役严格根消费的、与这些真实直接接口相符的固定规格输入，并绑定源、工具、库及 VIR；若现役工具没有这样的路径，保持阻断并由该所有者修正方法。这里不改工具、不关闭 no-cheating、不给仓内 adapter/projection/选择/状态函数 external_body、不把仓内库当外部公理。源提案中的接口假设尚未机械精化；它不是可合入的已证明实现。

现有阻断仍是 `verified_verifier` 只准入十二项严格参数。完整方法中的 edition、extern、import/export 等输入不能只按白名单丢弃，再投影成旧形状。E 没有扩展准入；已证实的输出登记错误及未完成的方法类见本文末节。正式汇总继续拒绝未支持的方法／缺失编译输入，raw use 的报告不能替代正式证明贡献。

## 独立成功、失败和边界预期

预期由原六个 trait 合同、原始 E0046 及上述直接接口语义确定；不声称执行了修改前的新用例。新 Rust/Verus 驱动直接调用两个引用 adapter、selector 的两视图、target 的两字段、真实 captured map 和 seen set，使用独立提供的完整文本／字段预期。Rust fixture 只提供真实 JSON／标准集合值；严格驱动按这些值满足其输入合同证明，fixture 解析本身仍是外部接口，不能由前提冒称已证明 JSON 解析器。

成功预期包括原行数／顺序和完整借用文本、存在但值为 null 的键、不同存储的相同文本命中、每次插入保留 prior。失败／边界包括空切片／空 map／空串、非对象、缺键、null、数字／布尔／数组字段、额外字段总数、前缀、组合 Unicode 差异、NUL／换行、首次插入 true 和重复插入 false。运行预期不符时必须 panic，不由 ghost 断言替代。原 `verified_git_binding` 的独立抽象合同、真实 `git_inputs`／`registry_snapshot` 等消费者用例继续经原未过滤操作验收；七个 E0046 的旧失败原件不被新测试替代。

## 操作、输入与等待

完整参数只由 [projects.json](../projects.json) 维护；不复制另一套运行配方。调用方先提交可审阅候选、记录固定 OID 并保持工作树干净，然后沿唯一受保护入口执行所选现役 operation：

```sh
.chrono-harness/bin/chrono-worktree use --operation <已登记操作ID> --path /Users/macstudio2/chrono-spec-current-dev
```

| 操作 | 前置、用途与结果边界 |
| --- | --- |
| `investigate.ci-generation-dependency.runner` | 真实 runner 整 crate；登记完整 Verus argv，包含缓存 extern、edition、manifest 版本和直接路径 `--export`。输出新 `libci-generation-dependency-20261114-runner-adapters` 的 rlib/dep-info/VIR，保护此前 runner 原件。先观察全部具体 adapter、直接外部规格声明、其余严格错误和 CAPACITY；没有严格完整成功则不把产物交给依赖证明。 |
| `verify.runner-json-adapters-tests` | 只有上一操作完整成功且同次源码、库、dep-info、VIR 与工具身份可绑定时，导入真实 runner 证明并消费新独立驱动。输出 `libci-generation-dependency-20261114-runner-adapter-tests` 的 rlib/dep-info；不使用普通缓存 runner 代替。 |
| `investigate.ci-generation-dependency.consumer` | 前置仍为 runner 同次严格完整成功；同一 generation 根消费新的 runner-adapters 库/VIR。它仍不代表 ci/lib.rs 四个实际生产 prepare 分支。 |
| `investigate.ci-generation-dependency.missing` | runner 和 consumer 已结束、其余输入未漂移，刻意不存在的原 absent VIR 仍不存在；缺失输入诊断不能证明完整依赖准入。 |
| 原 `test:runner-tests` 计划及 `test.runner-tests` | 保留所有原证明、格式、未过滤行为测试与新 JSON suite；正式 DELTA 还按前后端登记闭包选中其他受影响所有者。证明失败只阻断其依赖，普通 Rust 行为结果不提供缺失的证明保证。 |

新增生产／独立根已声明固定 `output_contract`，但 raw use 不执行生产 verifier 的扩展参数准入。C 必须区分原工具结果与正式类型化证明。原 consumer/missing 仍无 output_contract，不提供正式证明贡献。不得改变原 `--no-cheating`、`--no-external-by-default`、两线程、整 crate、编译及 checksum；不增加预算、过滤或重复执行。

C 的有限范围由固定候选的实际登记 DELTA 冻结，适用必需操作各一次；严格诊断至多两个有依据的不同候选。该诊断额度不删除适用行为验收。实际任务失败先读原 stdout/stderr、退出/signal 和输入；同一失败不原地重跑或改候选。子孙及原命令只沿宿主原生等待通道加入；没有结果保持 unknown，不另造轮询或执行器。

沿用已安装 Verus `0.2026.04.24.f8e1704`、commit `f8e17045c4ac17a02e566b8997070f2eecc255be`、Rust `1.95.0-aarch64-apple-darwin`。不 bootstrap、下载或无实际必要重建。use 沿所选 protocol 当前为 2400 秒、每流 67108864 字节；正式 runner-tests 计划仍为 900 秒、1048576 字节，两者不能冒称相同。此前 dep-info 已观察 `CARGO_PKG_VERSION=0.1.0` 且 argv 未被命令行拒绝；这不证明完整转发、sysroot/SDK、feature 或传递闭包。

下表是原固定工具与直接缓存输入；其摘要只固定字节，不授予证明资格。C 在执行前核对固定候选与这些输入，读取实际 dep-info 闭包；不扫描替换缓存库。任一新输出已有独有工作则停止，不覆盖／删除或换名重复启动。此前失败的旧 runner 输出及原 stdout/stderr继续保护。

| 直接输入路径 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `.chrono-harness/bin/verus/verus` | 3218528 | `f11f8a863103a3c8fcaf27e6189edfdba31081516591365b5e29b0a66f570451` |
| `.chrono-harness/bin/verus/rust_verify` | 17422224 | `09e1b379773cda9e64e2eb80c495ef0fc64b6684e4ea285f3eb5cf64759e51de` |
| `.chrono-harness/bin/verus/version.json` | 261 | `2a6c07dbfaf80cf85b9f749ec5be969948903a71b6ba763a21e59cd039fe4e90` |
| `.chrono-harness/bin/verus/vstd.vir` | 20228161 | `429a7cd94c89b9ca62250da0e46c144d7509fc7cd6d0f8b2b9efe056d382e0f3` |
| `.chrono-harness/bin/verus/libvstd.rlib` | 7335848 | `37af4b28b147c0bb0ffea529e8210e65cd8e8e8ff66c0aee19f49cba69d36581` |
| `.chrono-harness/bin/verus/libverus_builtin.rlib` | 576416 | `030d256085e0369cf808e72452e876e30d311e394e9238dff3f42e8ad37558f0` |
| `.chrono-harness/bin/verus/z3` | 23365455 | `bbb24b8fed27552f7fb7cfdf0ed9101e6e24fd0cc0b9cfb945233b3f2ed333c5` |
| `.chrono-harness/bin/verus/libverus_builtin_macros.dylib` | 11137840 | `9e85058d1dd0a4a55b9f370561c7fb45a7fe56f3ec9720009bcff5323d33db5d` |
| `crates/ci-tests/target/debug/deps/libchrono_identifiers-cf6a9639509b2b9f.rlib` | 113032 | `71b8e0e140eaa02a13bd92b8d79d985029e4cee2fb53ab9901d8db072230af74` |
| `crates/ci-tests/target/debug/deps/libserde_json-f23d5e320f77b40e.rlib` | 5218544 | `dc1e620dbcececdb9f28177e8dc48034ead1fbf6dc899ebe366b7244d303f9de` |
| `crates/ci-tests/target/debug/deps/libserde-4a4b19d5de75786c.rlib` | 999864 | `ceced5f61184de4c73f1d865ee052c0a9e612c700af818d4dbcf3f2710aa7020` |
| `crates/ci-tests/target/debug/deps/libserde_jcs-3bd63443869f0222.rlib` | 398160 | `00ccdca7c8a61f35d853c45030b421eae7b70e7bbbc6064aadf99a5691832f9e` |
| `crates/ci-tests/target/debug/deps/libminiz_oxide-7aba2a2b6a95b832.rlib` | 1256848 | `0b9b6253fdcded45fd9b77cf7e335649c9afd4c6540a439a3cea7bb8a27a61f5` |
| `crates/ci-tests/target/debug/deps/libtempfile-8640da8d6bd8ae6d.rlib` | 1240168 | `2f53134ba8448517573eef20eec484df7ea4c54b496fed2a67b0c6ad97d19efe` |
| `crates/ci-tests/target/debug/deps/libsha2-e186601b8af342ac.rlib` | 547728 | `61ee55248ff9c5a432b9ceb846038f771e4b540c614e0970e83a66de9f2c82cf` |
| `crates/ci-tests/target/debug/deps/liblibc-f778b8c2624adc32.rlib` | 2475552 | `a061d98df20953bc0a7ea1c5200d53078b175a3b293460759e3237814ef1d919` |
| `crates/ci-tests/target/debug/deps/libsha1-c90c88a1621cc9ce.rlib` | 212416 | `93e4c1ab5bb4cd53d7391bd0cb894aba07f9db10052f6e1abf59bf1d92bb2945` |
| `crates/ci-tests/target/debug/deps/libchrono_diagnostics-045e0f0016a85516.rlib` | 6719120 | `f711f3a6bd485af0fa7500d4ed005ad53c254b53010b5d614469f622982f5564` |
| `crates/ci-tests/target/debug/deps/libtracing-add697f9f59934e7.rlib` | 729992 | `3c7c784df4908c6a620cecd3af91219ac7a1dad9bea4cc1cd2486cade5aabcae` |

## 成本、剩余目标和保留

本 B 未执行构建、Verus、机器判官、行为测试、bootstrap、lifecycle 或 Git/GitHub 写入；只读取固定源码／原件并编辑、格式化候选。没有新运行成功状态。候选的 JSON 外部规格承载、标准文本键精化、具体方法证明、整 runner 严格编译及 proof admission 都未完成；CAPACITY 因果未知，未作修改。全部仓内传递依赖、cfg/proc-macro 闭包、CI 四生产分支与真实 prepare 调用、正式 DELTA、integration/dev 与落地仍由后续所有者负责。

运行不新增 JSON 副本或扫描；规格序列、map 和 set 投影均为 ghost，record 仍为一次原树插入。增加的编译、证明、CI、存储及全生命周期总成本未知；原线程和限制保留，不预填收益。

本文与临时产物的使用方是后续负责外部接口承载的工程工作者、C 和同包 mandatory review。调用方保管实际结果引用，直到缺口得到调试／验收并被使用方消费；2026-11-21 复核保留需求，不因日期自动删除。已有 state 为 evidence-retain，编译缓存仍复用；未用的产物不建立成功状态。退役调查 action／占用前须核实际使用方、原失败和保留义务；新生产／测试源码及相应必需证明入口不能随临时证据一起删除，不新增总账、清理器或批准关卡。

## E 输出登记修复与未完成的方法类

E 的固定起点为 `01b9d7ebed083908ec0e3a49fe23c106e8b0da08`，工作树为 `/Users/macstudio2/chrono-spec-current-dev`，分支为 `feature/ci-generation-state-20261113`。本节只交接已知 export 登记修复，**不代表完整方法类、依赖证明、正式 DELTA 或 GoalArtifact 满足**。E 不执行 F；caller 冻结提交后另行派发。

**登记政策发生变化：**projects.json 的三项 `--export` 值从 `name=path` 改为原定直接路径，`--import name=path` 不变。旧值会被固定程序完整当作路径，候选值才与原定产物身份一致。产品准入保持原拒绝边界，没有新增或放宽方法支持；没有同时交付产品逻辑修改。修正依据是 D 缓存的固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be` 的 config.rs 和 import_export.rs，分别为 SHA-256 `3f3cef57fae0a9f1a59a3055c3305abbfdfa58208787330edd96038dc85877bd`、`061967beddaf0b29e3e27810e2727b88d1ee5ce5f5f65a7e6220febf114a37a8`。不重新下载或将 help 文本当成实现。D 实际只到达 CAPACITY erasure-context panic，未到 export、没有 rlib／VIR，因此本修复没有伪造运行前的失败观察。

### 所有者、输入与实际用途

| 对象／负责方 | 输入、使用方、用途与替代方案 | 登记操作与验收边界 |
| --- | --- | --- |
| projects.json／repository | 固定工具语义和已有完整 argv；runner、adapter tests 和 ci generation 的在研严格证明消费原定 rlib／dep-info／VIR。保留现有产物路径，只纠正 export 值；不能用普通 Cargo 缓存替代证明。FILEMAP 原有 routes／projects／registration 触发和 runner-tests、routes-tests、native 消费边仍负责选中影响。 | 本文上表的 runner、adapter tests、consumer、missing；consumer／missing 没有 output_contract，不能取得正式证明贡献。 |
| verified_verifier.rs、verified_sources.rs／runner | 已有十二项严格方法、原回执、原报告、库和完整源 checksum；本地及离线 judge-ci 经 source_binding 共用 ProofAdmission。真实扩展输入没有生产者／VIR 资格载体，继续拒绝。不能以语法投影替代输入资格。 | 原 `verify.verus-result`、`verify.verus-result-tests` 和 `test.runner-tests`，均由原 `test:runner-tests` 计划负责；E 没有修改这些源码或操作。 |
| docs/verus.md／repository；本文／ci | 固定工具源码、D conclusion 和候选登记；caller、F 与同包 review 使用，以区分输出修复、未完成方法类和完整目标。替代方案是重复复制运行配方，未采用。 | 文档沿现有 FILEMAP 行登记，当前无机器执行消费边；不能为选测虚构消费者。 |

以上对象的构建、验证、CI、存储、维护总成本为 unknown，沿 FILEMAP 的 `unmeasured` 保留。没有新增执行项目、判官、总账、依赖、预算或路径。现有 FILEMAP 行与消费者边均仍准确，不为无变化的行制造登记差异。

### 完整方法类仍缺少的资格

当前真实 runner 的 46 项、adapter tests 的 25 项、consumer／missing 的 30／30 项，必须由 runner 资格所有者统一解释真实 source root、crate／edition／env、emit／`-o`、extern／import／export 与搜索路径。现役 `[8]`／`[10]` 选择仍只在十二项形状内成立；E 不宣称已经迁移这些路径，也不将新方法静默设为通过。

扩展支持所缺的不是额外 flag 白名单，而是可消费的资格输入：每个直接／必要传递库及宏的生产者身份、固定源与有效编译输入、同一成功严格整 crate 原报告、实际 rlib 与 dep-info checksum；import 还需与同次成功生产者、对应 extern 和当前工具绑定的实际 VIR。缺失、失败、未知或变更的任一输入必须带具体错误阻断其依赖闭包。VIR 的存在、SHA、库名、缓存命中或 import／extern 声明都不能产生这些资格。

现有 routes Receipt 和 typed ProofAdmission 没有逐依赖生产者／VIR 字段。`.chrono-harness/ci/native/src/proved_build.rs`、`verified_cargo.rs` 及 runner-build.json 的当前生产路径绑定 identifiers 库和最终 runner 二进制，仍应复用；其合同没有建立这四项扩展 action 的全部库、宏、传递输入或 VIR 资格。普通非 vstd proxy／external_body 在固定 no-cheating 下未被准入，不能假设 external spec 库 import 已解决 JSON／全文载体。E 没有足够依据为缺失载体填入成功，故只交付确定的 export 登记修复；完整方法类保持未完成，后续由原资格与固定工具／外部接口所有者继续处理。

### F 的独立预期与执行边界

这些预期在 E 静态交接时定义，**修改前和修改后均未执行**；它们不是已通过的测试，也没有新增承重程序逻辑。纯登记文字修复不新增重复形状测试。F 仍按固定候选的两端 FILEMAP DELTA 选择原方法，每项适用操作一次，不改线程、期限、输出限制、未过滤行为或原断言。

| 场景 | 独立预期与当前状态 | 原登记使用方／方法 |
| --- | --- | --- |
| 当前 basic roots 成功／失败／边界 | 唯一方法、完整 argv、零退出、固定完整报告、源 checksum 和实际非空库同时符合才接受；缺报告、错误根、输出不符、失败或超限继续拒绝。原 Rust／Verus 独立预期保留。 | `verify.verus-result`、`verify.verus-result-tests`、`test.runner-tests`，通过 canonical DELTA 及其原 `test:runner-tests` 计划执行。 |
| runner 46 项 | 真实根应为 `crates/runner/src/lib.rs`，rlib 为既有 `libci-generation-dependency-20261114-runner-adapters.rlib`，dep-info 为该 rlib 加 `.d`，export 为原定同 stem `.vir`。当前方法资格不完整，正式 ProofAdmission 必须拒绝，不能因原报告或缓存库而通过。 | `investigate.ci-generation-dependency.runner`；D 的未变 CAPACITY 失败不得原地重跑，输出值修正不证明消除了该失败。 |
| adapter tests 25 项 | 真实根为 `crates/runner-tests/tests/json_adapters.rs`；导入同一合格 runner 的 rlib／VIR，输出原定 runner-adapter-tests rlib／dep-info。当前生产者未成功、方法资格未完成，保持 blocked。 | `verify.runner-json-adapters-tests`；只在合格生产者具备后执行，不能用旧 272 结果代替。 |
| consumer／missing 30／30 项 | 根均为 `crates/ci/src/verified_generation.rs`，分别使用原定 consumer／missing 输出。consumer 必须有同一合格 runner 输入；missing 的 absent VIR 仍须确实缺失。两者仍没有证明贡献合同，普通成功不等于完整 GoalArtifact。 | 原 consumer／missing action；生产者与必要前置合格后才执行。 |
| export 达到写入阶段 | 固定工具将给定直接路径作为实际目标，不从 `name=path` 拆分；若进程随后失败，即使 VIR 已写出也不能成为成功证据。D 未走到该阶段，当前真实结果 unknown。 | 同一 runner／consumer／missing 原调用；不另启诊断。 |
| 依赖、生产者或原件缺失／变更；参数冲突／未知 | 新方法不能取得 ProofAdmission；当前缺少完整绑定能力，不能把本行算作已经实现的扩展成功支持。完整修复须在原所有者增加直接消费实际实现的 Rust／Verus 行为与证明预期，并由同次 DELTA 验收。 | 同一 runner 资格入口、本地／离线 source_binding、routes 和实际编译消费者；这仍是后续未完成义务。 |

### 未完成目标、原件与保留

完整 GoalArtifact 仍未满足：完整方法解释与所有准入入口迁移、严格 JSON／全文外部载体、全部适配器和仓内传递依赖证明、整 runner／实际编译绑定、CAPACITY 原因、四个真实 Projection 分支及 prepare／generation／effect／recovery 闭包，均未由本登记修复替代。正式 DELTA、原生 integration／dev 交付及落地由 caller 后续分别核实。本 E 不启动 F 或 review，不修改目标状态。

D conclusion 报告的 managed 原件和 20 份补充材料按 caller 已核结果继续保护。旧 6af74eb canonical 与两流目前缺失，责任方及恢复为 unknown；同字节新 stdout 不能恢复其保管链，也不阻断本次独立 export 修复。E 不删除、覆盖或重跑原失败，不修改 state、缓存、bin 或生命周期。

本 flight 的固定源身份与限定 patch 由 caller、F 和同包 mandatory review 消费，沿既有负责方保留到真实使用方消费并完成验收／调试义务；2026-11-21 仅复核需求，不是自动删除授权。E 只交接静态编辑，没有运行构建、bootstrap、Cargo、Verus、业务测试、机器判官或生命周期；独立预期及完整验证成本仍为 unknown。

## R：固定工具对导入外部合同的检查边界

本节服务完整同源证明与实际编译绑定，修正“另建普通外部规格库再 import 或许可以满足当前严格根”的未决判断。检查起点是 `e7854f5e7c1787b0c7430fc7c900c72f1aec7d4f`，工具仍为上述固定 Verus／Rust。这里只核对该版本的官方源码和现有所有者，没有新工具运行结果；不修改 SPEC、生产方法、严格参数、分类、预算或准入规则。

SPEC §2.1.1 允许逐项登记真正的外部接口假设。下面的限制来自固定工具实现，**不能把工具缺口解释成 SPEC 禁止这些假设，也不能给仓内适配、选择或状态函数增加豁免**。

### 声明、准备与严格使用是三件事

固定上游 commit 为 `f8e17045c4ac17a02e566b8997070f2eecc255be`，tree 为 `30bee75a9c8563d45ed3c1f1e99c8b0a04001773`。以下引用全部固定到该 commit：

| 机制与权威位置 | 当前版本的实际作用与边界 |
| --- | --- |
| [assume_specification 参考](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/docs/guide/src/reference-assume-specification.md) | 支持为外部 exec 函数声明合同，声明可位于另一个 crate；文档明确它是无证明的假设。这说明声明语法受支持，不说明严格使用方允许它。 |
| [rust_to_vir_func.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify/src/rust_to_vir_func.rs#L1565) 的 `handle_external_fn` 调用及 `FunctionX` 构造 | 声明映射到真正的外部函数，但保留非空 `proxy`；`owning_module` 仍是声明所在模块，不因目标是 serde_json 或标准库就变成 vstd。`external_trait_specification` 派生的方法也经过此路径。 |
| [import_export.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify/src/import_export.rs#L56) 的 `export_crate` | 可导出公开合同，按模式移除部分函数体；不清除 `proxy`、`attrs.is_external_body` 或 `owning_module`。导出发生在后续严格检查之前，文件存在不证明生产者成功。`CrateWithMetadata` 仅含 Krate 与原文件位置元数据，没有生产者成功或完整编译输入资格。 |
| [verifier.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify/src/verifier.rs#L2808) 的 `construct_vir_crate` | 合并导入和当前 VIR，随后以**当前使用方**的 `no_verify`／`no_cheating` 调用 `well_formed::check_crate`，不把已导出合同视作严格检查例外。 |
| [prune.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/vir/src/prune.rs#L839) 与 [well_formed.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/vir/src/well_formed.rs#L1952) | 这里以 `module=None` 做 crate 级处理；保留当前 crate 及可达导入，函数调用会标记其真实目标可达。函数保留检查所需标记，`check_crate` 对结果中的每个函数调用 `check_function`。未使用合同可能被裁掉，但不构成真实使用成功；把实际使用的合同移到导入库不能避开检查。 |
| [well_formed.rs:1471](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/vir/src/well_formed.rs#L1471) | `no_cheating && (is_external_body || proxy.is_some())` 产生 `external_body/assume_specification not allowed with --no-cheating`；唯一该分支例外是 `owning_module.is_vstd_path()`。此条件不检查外部合同是否符合现实语义，也不读取宿主逐项登记。 |
| [ast_util.rs:58](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/vir/src/ast_util.rs#L58) | `is_vstd_path` 要求 `CrateId::Vstd`，不是普通库中的 `vstd` 同名模块。冒用工具的 vstd 身份或替换安装产物不属于本次支持路径。 |
| [no_cheating.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify_test/tests/no_cheating.rs#L89) 的既有独立预期 | 上游明确预期 `--no-cheating --no-verify` 仍拒绝 `external_body`；`--no-verify` 只取消求解阶段，不取消上述构造／检查。这里读取了预期源码，没有执行上游测试。 |

因此，先用普通声明生成库／VIR、再让严格生产根 import，仍没有得到这些合同的受支持严格载体；准备阶段即使不声称证明，也不会改变使用阶段的判法。外部类型声明本身与函数合同须分别判断：不能仅从 `external_type_specification` 的存在推出所有类型声明被 `no-cheating` 拒绝；但只有类型描述不足以证明真实 JSON 方法的返回关系。当前宏还把 bodyless spec 和 broadcast axiom 降为 `external_body`，这些模型声明同样不能凭另一个普通 crate 的身份取得例外。

这是一条针对普通外部规格声明及其原生 export/import 的共同检查条件，覆盖本类直接 JSON／全文合同，不是逐成员补丁，也不是对所有 Verus 版本或未核对机制的不可行性证明。可证伪条件是：在固定工具、未冒用 vstd、未清除假设标记且保留严格整 crate 参数的前提下，真实使用方成功消费所需合同；只有语法接受、非严格准备成功或文件存在都不满足它。

### 现有可复用合同与尚缺的现实关系

固定 vstd 已有 `String::as_str`／Deref、字符串全文 view、切片、Option 和树集合合同；这些是真正的导入工具合同，不是本仓函数证明。`string.rs` 的 `String::as_str` 保留全部字符，`std_specs/btree.rs` 的合同仍带 `obeys_cmp` 和借用键模型前提；本轮没有找到可直接替代 JSON／全文补充关系的已安装合同。

仍需承载的真实外部接口是 `serde_json::Value::{as_str, as_object, get}`、`<Value as Index<I>>::index`、`serde_json::Map<String, Value>::len`，以及标准 String／str 树键的顺序、String→str 借用查询和相同全文的集合成员关系。现有 `json_value_specs.rs` 已集中表达这些提案，无需复制。实际使用方仍是 `JsonBindingReferences`、`JsonGuardReferences`、`JsonSelectorRows` 的两个视图、`JsonTargetRows`、`CapturedRegistry` 和 `SeenSelectorText`；七个实现及其仓内组合继续承担完整证明义务，不能成为准备阶段的假设。现有独立 `json_adapters.rs` 继续保留成功、失败、空值、null／缺键、全文和重复插入预期。

runner 的 `require_registered_input_evidence` 另外仍拒绝非空扩展编译上下文；它不读取上述 Verus 实现，也没有可由普通 import 填充的完整生产者资格记录。工具承载限制和宿主准入缺口是两个独立义务。即使将来前者解决，仍须绑定完整 argv／环境、实际源／编译生产者、rlib、dep-info、VIR、宏和必要传递输入；不得把“明确允许的外部假设”写成“已证明的仓内依赖”。

### 当前交接与能够改变结论的负责方修正

本轮没有发现能满足上述固定严格使用条件的新增受支持操作，因此没有新增实验源码、执行登记或尝试已知会被该共同条件拒绝的生产根；没有重跑旧 CAPACITY 失败，也没有建立新的成功原件。现有操作继续按原前置阻断，不以本节替代运行结果或类型化准入。当前结果属于源码可行性证据，raw 严格使用、正式 ProofAdmission 和实际交付均未新增。

能够改变此结论的修正归现有 Verus 外部接口／runner 资格所有者：工具必须提供有明确语义的外部合同输入，使严格使用方可以接受**真正的外部接口声明**，同时继续拒绝仓内函数、适配判断和状态维护的假设；或提供实际涵盖本类接口、可在固定严格根使用的工具合同。当前 `well_formed::check_function` 只有 vstd 例外，不能仅改 projects 的声明或把普通规格库称为工具合同来实现这一点。在固定工具身份不变时，这项能力尚未找到；需要改变工具／方法的方案须先显式说明原默认严格方法与新作用范围，不在本轮静默采用。

后续若取得这样的受支持机制，先在现有规格所有者准备最小真实外部合同和独立 Verus 成功／失败／边界预期，显式登记准备与严格使用的不同作用，封存干净候选后由调用方委派受保护 operation。准备成功不算证明，严格使用成功也只算 raw 工具观察；完整准入修复随后仍须复用 runner／routes／本地与离线使用方的现有所有者。新增证明不得丢失严格参数、整 crate、原线程、预算或原断言。

## S：真实 vstd 源码候选与严格使用实验

S 从干净 `986004dc222143d373babf2318907f32f1a9ce59`／tree `fafde2b6eaa6b2d018fae4044760eb96a20a4bb4` 准备本节候选，不重查 R 已解决的普通 proxy／import 类。目的仍是七个实际适配器及 generation 所需的真实外部 JSON／全文合同。**U 的登记修正已封存，V 的原生 extract／patch 成功；W 修正实际继承的 PATH 后，build 已启动编译器，但因补丁重复定义 vstd 别名而失败，组件完整验证／编译及严格正反向结果仍未取得，端到端能力为 ASSUMED-UNVERIFIED。** 不据此宣称严格成功、完整 family、正式 ProofAdmission 或交付。

**可见变更警告：同时增加工具库源码补丁、Verus 实验程序和宿主实验方法登记。** 原安装工具、产品源码、正式证明操作和准入判断均未修改。旧安装 vstd 没有这里的 JSON／全文模型关系；候选是真实 vstd 源码的一项显式扩展。新增构建、证明、CI、存储和维护成本尚未实测，不因候选使用原二进制的版本字符串就沿用旧库资格。

### 原生依据与组件身份

固定上游 [vstd_build/src/main.rs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/vstd_build/src/main.rs) 直接把真实 `vstd/vstd.rs` 交给 `rust_verify`，使用 `--is-vstd`、`--compile`、`--export` 并等待子进程。其参数循环不接受额外 extern；[vargo 的 rebuild_vstd](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/tools/vargo/src/commands/build.rs#L450) 也明确未转发 verus_args。因此不把 serde_json 参数塞给这个不支持的前端，不重建编译器；projects 的准备操作直接复用该前端已经调用的原生 Verus 能力，完整验证并编译真实 vstd 源码。没有 `--no-verify` 或自制执行器。

[原生 wrapper](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/verus/src/main.rs#L97) 从自身旁边的 `verus-root` 和 `rust_verify` 定位组件；[driver](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify/src/driver.rs#L193)、[externs](https://github.com/verus-lang/verus/blob/f8e17045c4ac17a02e566b8997070f2eecc255be/source/rust_verify/src/externs.rs#L47) 和上述 R 的 import 证据说明标准根怎样同时使用邻接的 `libvstd.rlib` 与 `vstd.vir`。严格实验通过已有 bash 工具作单次 `exec` 转交，以使用独立目录内的原生 wrapper；不增加脚本、工具绑定、环境替换、等待器或新的运行框架。这个源码依据不是实际运行观察。

组件名称为 `vstd-f8e1704+chrono-json-fulltext.1`。`vstd.patch` 只给**完整真实 vstd 根**加入内部 self 别名和指向现有 `crates/runner/src/json_value_specs.rs` 的模块路径；没有另造同名库，没有把适配器／仓内判断移入工具，也没有修改序列化 VIR。所有合同仍只有这一份现有源。独立目录固定为 `.chrono-harness/state/verus-tool-contract-20261114/`，从已有发行包解压，绝不覆盖 `.chrono-harness/bin/verus/` 或共享产物。`--is-vstd` 仅用于这个实际 vstd 源码构建，两个仓内实验根均不使用它。

| 固定输入／新源 | 当前身份与用途 |
| --- | --- |
| 原发行包 `.chrono-harness/bin/verus.zip` | 25,543,507 字节；SHA-256 `023e0a6e67bb9927f43a8757ebdec318b50cbfe1fec4a1c320c53d5fd71008e7`。复用缓存，不下载编译器。 |
| 发行包内的真实 vstd 源码 | 本轮以固定上游 tree 的明确文件表核对已安装的全部 108 个 vstd 文件，共 1,429,187 字节，Git blob 身份全部一致；不是靠目录名认定其来源。根 `vstd.rs` 的旧 SHA-256 为 `a5317ab99820e48f7a1fab6f298732bab29047ed6e2ca8cf4e4e5290be5c230a`。调用方解压后须再核对实际被编译的字节。 |
| 补丁后的根与合同源 | 根的预期 SHA-256 为 `4cca22df78489ce34a5fb3c71c8aa688d0ceb46c5e339d09947d31d6349f6b8a`；所引用合同源为 `a41dd58bc38e1d7dbc2a7bc55ae4bcb15b35ab22f1db821863fceb421b2df4dc`。补丁和实验根随调用方封存的新候选绑定，不能只读将来漂移的工作树。 |
| Verus／Rust、宏及 serde_json | 沿用本文前表的原二进制、Rust 1.95.0 和 serde_json 1.0.151 缓存 rlib。额外核对原 `libverus_state_machines_macros.dylib` 为 `958d50e815616624702567343d539f0cd4b6109141f9a38da4d4cda47201bb5a`。 |
| 新库、VIR 与 dep-info | 尚未产生。成功后须记录实际新字节、完整上下文、源与外部依赖；原 vstd 库／VIR 的摘要不能替代。解压得到的旧 rlib／VIR 绝不是构建成功证据。 |

本轮读到的固定原生源码 SHA-256：`vstd_build` 为 `d3472fb12e8ebd2ab984f890a928c1cd7c3e3211489e81d5c25ab88dc4fa74ee`，wrapper 为 `0ff5ef2d7e0064ed55a1e70446ba32f3af0cf09a3abd7d674af7eea97b030409`，driver 为 `237cb3700aa97d98fc7aab6ba74ceb7f579d432d6a5b4990b30f8c5cd7b5a22c`。来源均为上述固定 commit；只取得必要源码数据，没有运行这些源码或更改安装组件。

本轮也读取实际缓存 serde_json 1.0.151 的 `value/mod.rs`、`value/index.rs`、`map.rs`，SHA-256 分别为 `6331d59ea362a74b6cd3d302d08114a63a73a384ff56fef737de29edea2f8d91`、`f1404ed12e795a5553da4bdf72835c441a5e0cc0f6e794fee875eaa887d65319`、`b5a2fd1ee865e6cfd83ef2da0a2e1057f892465ac7ede8a78a11354a60526f33`：get 转交原 Index，str 索引仅查询对象，缺失的不可变索引返回静态 Null，文本／对象借用只匹配对应变体，len 转交原 map。固定 Rust 1.95.0 源文档中，String 在原 `Vec<u8>` 字段上派生比较，str 的 Ord／PartialEq 比较全文字节；BTreeSet::insert 返回底层 map 插入是否原来缺失，BTreeMap::contains_key 转交 get().is_some()，并明确要求借用键与存储键顺序一致。这些源码关系支持本次外部接口假设的现实解释，不是 Verus 模型精化或消费者成功的替代证据。

### 所有者与唯一待执行入口

完整 argv 只由 projects.json 维护。五项操作均有现有 `.chrono-harness/state/` 产物占用，沿本文原 `chrono-worktree use --operation` 入口执行。原生工具库的准备不提供仓内证明贡献；正向程序和反向外部工具输入的调用均保持 `--no-cheating`、`--no-external-by-default`、两线程、整 crate、编译和 checksum。所有实验 action 均无 `output_contract`，不伪造现役不支持的 typed 资格。原未过滤测试计划、线程、时限、行为断言和生产证明操作均保留。U 补充的正向组及现有 runner 单元归属见下节；反向原非零结果不进入成功证明计划，不新增缓存订阅。

| 操作／现有负责方 | 必需输入、顺序与实际使用方 |
| --- | --- |
| `investigate.verus-tool-contract.extract`／host-bootstrap-launcher、host-bootstrap-native | 调用方先提交候选、确认干净且持有 use；核对原发行包、独立目标目录及两个使用根的输出路径均不存在，不覆盖独有工作。原生 tar 在独立目录准备原工具和完整源。 |
| `investigate.verus-tool-contract.patch`／同一工具所有者 | 只消费上一项的完整解压结果及已封存 `vstd.patch`；原生 patch 禁用 fuzz、禁止交互，非零保留原结果，不原地重试。补丁后核对根与合同实际路径／字节。 |
| `investigate.verus-tool-contract.build`／同一工具所有者 | 消费补丁后全部 vstd 源码、唯一 JSON 合同源、原宏／工具链与固定 serde_json rlib；原生 `--is-vstd` 完整构建允许工具库原有显式假设，不冒充仓内严格证明。只能在原任务零退出、完整验证及编译结果和新库／VIR／dep-info 身份都取得后交给下面两项。 |
| `investigate.verus-tool-contract.consume`／runner-tests | 消费同次成功新组件。`consume.rs` 实际调用 `as_str`、`as_object`、`get::<&str>`、泛型 `Index<&str>`、Map::len、借用键查询和 set::insert，证明完整返回关系、状态变化及下面的边界。不是未用 import 或 fixture 回调。 |
| `investigate.verus-tool-contract.reject`／runner-tests | 仅在上项严格成功后，以相同组件与严格路径检查 `fixtures/reject.rs` 中模拟仓内函数的 `assume(false)`；该故意非法输入应使 Verus 非零退出，原诊断须确实指向该函数的 `assume/admit not allowed with --no-cheating`。失败状态原样保留，不能转成成功 ProofAdmission。 |

当前 FILEMAP 将 `consume.rs` 保持为 program，绑定其实际整 crate 操作和下节正向执行组；故意非法的外部 Verus 输入 `fixtures/reject.rs` 按 fixture 数据合同登记为 data、`roots: []`，仍由同一 runner-tests action 真正传给 Verus。`vstd.patch` 仍是原生 patch 的输入数据，不包含新判断程序，登记为 data。现有合同源的 specification／runner 证明根和真实工具准备使用边保留。补丁及正反向输入／输出关系由本表明确登记，不靠扫描、新项目或成功包装推断。完整 DELTA 和正式证明资格仍待验收，不能据 raw 登记声称覆盖已闭合。

### U：完整实验关系的登记修正与待验收方法

**可见方法变更警告：修正实验分类、执行组、影响边和单元归属；不修改产品判断、分类判官或工具资格规则。** S 原来将正向程序和故意非法输入都登记为成功证明根，却没有把两项操作放入任何 execution plan。现在只有真正需要成功证明的 `consume.rs` 是 program；反向 fixture 保留独立的非法载荷和严格拒绝预期。此修正只适用于 S 已明确要求非零的外部工具输入，不豁免产品、适配器、测试驱动、状态判断或独立 Verus 预期。

现役分类门拒绝同一路径从 program 降为 data。为纠正 S 的输入表示，原 `reject.rs` 的全部 322 字节原样保存在 `fixtures/reject.rs`，SHA-256 仍为 `aeeb242ba37a121eb817855517ef8930f918d5591005c2c053026a11e119aa60`；原 action 只迁移该输入路径，原输出路径、所有参数及 `assume(false)` 均保留。旧路径不再登记，新的 fixture 行有实际 runner-tests 运行输入边。没有移走或隐藏任何应成功证明的仓内实现；外部 Verus 仍会看到相同函数并必须真实拒绝它。

| 当前登记／实际使用方 | 作用和边界 |
| --- | --- |
| projects 的 runner-tests `test_groups` | `runner-tests → execute` 保留原未过滤操作；`runner-tool-contract-consume → investigate_tool_contract_consume` 绑定真实正向 action。没有反向成功组或新项目。 |
| FILEMAP `test:runner-tool-contract-consume` | 单项计划只含 `investigate.verus-tool-contract.consume`，默认上限沿用 runner-tests 的 900 秒、1,048,576 字节；这不是实测用时或总预算。consume／补丁的执行边与 runner 到组的配对边明确选择该根，成本为 unmeasured。原 `test:runner-tests` 计划原样保留。 |
| check.json 的现有 `runner` 单元 | 同一单元同时拥有原测试和新增正向组，保持全部计划恰有一个单元负责；不新增原生 job、生成投影、缓存、共享操作或执行器。正向组是新增的相关验收义务，不代表 CI 已运行或支持该工具合同。 |
| extract／patch／build 的三个 action 与 claim | 仍归 `script:host-bootstrap-launcher`，五项完整 argv／产物 claim 均保留；它们是上述表中逐项核验的显式准备操作，不伪装成测试组。正向单项计划不会自动准备组件；调用方须先取得同次成功准备结果。 |
| reject action 与 claim | 仍归 `project:runner-tests`，直接消费真实 fixture，并原样保留非零 stdout／stderr／退出状态。它不需要成功证明根或成功计划；独立拒绝验收由本节既定预期和调用方消费原件，不转换成 ProofAdmission。 |

T 在封存 `b11d7b936d30320f185c8538c56f741f6e5786c6`／tree `e7f2d25f6382ae17dfc1394dcbc90b0c65ac535c` 上的原生 `status-7081-1794624176321927000-0` 和一次 extract use `use-14172-1794624253437090000-0` 均退出 2：`filemap: .chrono-harness/experiments/verus-tool-contract/consume.rs: E_VERUS_COVERAGE: proof root operation is not registered`。原件分别为 `.chrono-harness/state/worktrees/status-7081-1794624176321927000-0.json` 与 `.chrono-harness/state/worktrees/use-14172-1794624253437090000-0.json`。两个报告各保留 25 项已加入的 Git 读取进程；未取得 use、未派发任何实验操作、未启动 tar／Verus。raw positive／negative 均未知；这不是预期的 assume 拒绝，也不是历史 cleanup identity 失败。原件按本节既有使用方和 2026-11-21 用途复核条件保留，不复制日志或另建账本。

调用方将 U 候选封存为 `aa05ad5a9db737b37d59358328ae7d241628b48f`／tree `224b3fcd56aac6b10afca4b51dd54639e0c3f9d1`。V 在干净候选上按登记入口各执行一次 extract、patch、build：`use-97286-1794626151313205000-0` 与 `use-9426-1794626320608184000-0` 的原命令及 use 均退出 0；`use-13366-1794626388739537000-0` 的 Verus wrapper 退出 128，use 退出 2。三项原报告及各自 `.json.process/` 的 stdout、stderr、launch、result 由实际生命周期所有者保存在 `/Users/macstudio2/chrono-harness/.chrono-harness/state/worktrees/`；操作 cwd 和政策候选始终是本工作树，未编辑协调者源码或状态。三项任务均已等待结束并核对原件；旧 T 输入、canonical 和 bootstrap 未重跑。

extract 的全部 168 个解压文件匹配缓存发行包，108 个真实 vstd 源文件共 1,429,187 字节；patch 只改变根，摘要符合上表，合同路径解析到唯一现有源。V 的 build 原 stdout 为空，完整 470 字节 stderr 报告 `verus: rustup not found, or not executable`，随后 `failed to execute rust_verify`。其 managed PATH 摘要为 `06328d66d5306076888bcacdaad76a6ea49dbbe27f4b731b5a86f3276111ff06`，不含已有 rustup；该失败原件继续保留。

W 在干净 `8a095581b8adb1830f63f801f46e6c9638a1f463`／tree `225e1e1d1a6f22564d74da252cd648873fb93448` 上复核同一组件、工具及缓存字节，只恢复一次 build：`use-42526-1794627191763564000-0`。实际 managed 环境与 V 相比仅 PATH 的值摘要变化；新 PATH 以 `/Users/macstudio2/.cargo/bin` 开头，保留其余继承项，摘要为 `8db253e7df6f9b8c0cdb83f1d3969e39c4ebcd6eb1165a85fceff833c06310d0`，环境摘要为 `afb19c677b75cfab1662ba3378ceb107f78ee7e2ae7650e3ba5a84e24d21c362`。intent、launch、result 及报告中的环境绑定一致，原 argv、工具、线程和预算不变。首个可解析 rustup 的 SHA-256 仍为 `63752a1919a3a4793173ec70f8f212621a7ce38ba6a246095984482f4de20f9f`。启动前直接读取当前配置推算的环境含 SDKROOT，而原生 managed 结果没有该项；实际判定采用后者，不把启动快照冒充 managed 环境。

新 Verus 原退出为 1，use 为 2；479 字节 stdout 报 `success:false`、`encountered-error:true`、`verified:0`、整 crate，535 字节 stderr 报 Rust `E0254`：补丁根第 29 行的 `extern crate self as vstd` 与上游第 155 行的 `pub use crate as vstd` 重复。原报告 SHA-256 为 `3815461da388e04f710329324fe3452320dbd78956ee6638227917c8ef6502d8`，stdout／stderr 分别为 `16e02ae17ca34dfd13d276a12ea0e96cdad307fc2732b75bb490a1873eab6f42`／`13bf746b112089d87e9ca9830bdfc9e53595f5403e6f2ed1655c95d4e6fab199`，沿同一协调者原件目录保留。managed 原件及报告内全部 42 项唯一进程均已加入。该结果证明编译器已启动，不证明组件验证／编译成功，也不是反向 assume 拒绝。最小修正归现有工具源码准备所有者：补丁复用上游已有 self re-export，去掉重复插入的别名；由调用方封存新的补丁／根身份并安排后续执行，W 不修改源码或重跑。

正式 DELTA 仍走 `.chrono-harness/bin/chrono-harness check`，所选证明和原未过滤测试由登记负责。当前正向 action 的 bash 转交、extern／新组件及缺少 typed 资格仍受现役工具合同限制；补入计划成员不使其自动满足 `root_planned` 或实际编译绑定，不能添加虚假 `output_contract`。当前候选已通过普通操作的原生登记准入并实际派发 extract；这不证明正式证明计划合格。新增组带来额外正向验证义务；完整构建、运行、CI、存储和维护成本未知。

### 执行前固定的预期、范围与剩余义务

正向根必须整 crate 零退出、验证成功且实际编译出新库和 dep-info。其函数没有把实际外部调用换成仓内假实现：get 的完整成员关系、Index 缺键无文本、object 的真实成员数、String→str 借用和集合插入均由调用后的证明义务使用。边界分别要求：缺键得到 None；存在但非文本时 get 为 Some 而文本为 None；任意全文相等（包括不同存储、Unicode、NUL、换行）有相同成员关系；insert 的 fresh 与此前全文成员关系互补并保全实际旧 set。JSON 模型前提只是外部观察条件，不证明 JSON 构造或解析；原 `json_adapters.rs` 的实际空／null／错类型／前缀／组合 Unicode／重复插入预期继续保留，由后续原未过滤操作消费。

反向根必须在确实消费同一新库的前提下，因仓内 `assume` 被严格检查拒绝；缺文件、类型错误、其他前置失败、无输出或超时均不是这一预期的通过证据。反向原非零结果继续是失败原件，不安装“失败就成功”的包装。W 的 build 失败后，rlib／VIR 仍逐字节等于解压旧库，dep-info 不存在；六项正反向输出仍不存在。consume／reject 均未尝试，仓内严格探针为零，两个结果保持未知。PATH 前置已在 build 实际观察通过；当前阻断为补丁重复别名。调用方须先封存必要的源准备修正，后续按 build → consume → reject 的原登记依赖取得新原件，不重复成功解压，不把单项计划当作自动准备组件，也不在当前失败输入上重跑。

本候选只试验一个真实工具库载体：源码层面的 family 依据是所有所需声明由真实 vstd 所有，消费者仍走未放宽的严格检查；它**尚未**提供全 family 的实测成功。可证伪条件是：新组件完整准备成功后，实际使用这些合同的正向严格根仍拒绝或无法证明关系，或相同严格路径不再拒绝仓内 assume。失败须先读原输入／stdout／stderr／退出，不重跑 R 的普通 import 拒绝，也不逐项修改仓内编译参数来伪造覆盖。

真实 serde_json／标准文本模型仍是逐项列出的外部假设，不是已机械精化的库实现。正向 raw 成功也不证明全部 runner／adapter-tests／generation、完整生产编译绑定或 ProofAdmission。后续仍须在原所有者完成唯一合同的实际消费迁移、68 个受影响未分类文件与六个旧整文件映射、完整 producer／argv／cwd／env／rlib／dep-info／VIR／宏及必要传递输入资格、generation 四生产投影与 prepare／effect／recovery、正式 DELTA、integration／dev 和实际落地。

产物沿现有 state 的 evidence-retain 与 check-original 所有者保管；不新增保留生产者、总账或清理器。使用方是工具合同工程、runner 资格工程、调用方与 mandatory review。原失败保留到调试／验收义务完成；新组件及原件在这些使用方结束、已保全必要证据且无活使用方后由调用方通过现有生命周期／保留所有者回收，2026-11-21 复核用途，不因日期自行删除。W 的新 build 原报告为 450,870 字节，managed 双流共 1,014 字节；这是该任务的文件字节口径，不是总存储或运行成本。未单独测量构建 CPU、峰值内存或全流程用时，完整构建／CI／存储／维护总成本仍未知；没有用预算、重复运行或降低目标换取成功。

R 的普通 proxy／import 源码调查没有待执行的新登记操作；登记准入类已有真实正向观察，PATH 前置已在 build 观察通过，组件构建当前受重复别名的真实编译错误阻断。consume／reject 上的环境与严格行为尚未观察，工具载体和编译资格类仍未成功。验证总成本未知。全部受影响源码的实质证明、generation 四个真实投影与 prepare／effect／recovery、正式 DELTA、原生 integration／dev、平台与生命周期义务继续未完成。保留使用方和复核条件沿用本文既有声明；本节不新增运行账本、执行器、缓存或永久证据副本。

## X：E0254 源码准备修正与保留组件恢复入口（未执行）

本节服务 SPEC §2.1.1 的真实工具合同载体准备，只修复 W 已观察到的重复 self namespace。上文 S／W 的旧补丁与根摘要保留为失败输入事实；当前 canonical 输入及保留组件的接续方法以本节为准。X 的起点仍为 `8a095581b8adb1830f63f801f46e6c9638a1f463`／tree `225e1e1d1a6f22564d74da252cd648873fb93448`，原 W 文档的全部 53,291 字节原样保留为本文前缀。本次没有修复 state、启动构建、consume／reject、bootstrap 或正式检查，也没有 Git／GitHub 变更。准备完成不代表机制验证、完整批次、review、SPEC 或交付完成。

**可见变更警告：同时修正工具源码准备数据和宿主操作登记。** `vstd.patch` 不再插入 `extern crate self as vstd;`，继续加入唯一 JSON 合同模块，复用完整上游根已有的 `#[cfg(not(verus_verify_core))] pub use crate as vstd`。不改上游 re-export 的条件，不移入仓内适配器、判断、驱动或独立预期。新增的恢复补丁只删除失败组件中的这一行；没有新判断程序、脚本、信任合同或证明豁免。FILEMAP 为该原生工具输入登记 data、现有所有者、明确使用边、unmeasured 成本及输出占用；projects 只用既有 bash 转交系统 patch。原 build／consume／reject 的参数、线程、时限、断言和严格要求全部保留。新增操作的实际准入、执行及验证成本尚未观察。

### 当前输入身份与等价的目标根

| 输入／输出 | 固定字节身份及用途 |
| --- | --- |
| 当前 canonical `verus-tool-contract/vstd.patch` | 315 字节；SHA-256 `c3d6bf110dc6df25cb0675503ae65020caabb9d51e00faaae86303fa80f72e39`。只供原 `investigate.verus-tool-contract.patch` 消费未打补丁的完整发行源；不能施到本次已 patched 的保留组件。 |
| 恢复输入 `verus-tool-contract/vstd-repair-self-alias.patch` | 341 字节；SHA-256 `59039a8690787d7abe71efa016cc4d6b1287c23cd4b4a318d33b5785ef3f3aa7`。前后各保留两行非空上下文，固定到旧补丁模块，只删除 27 字节的重复别名行。唯一使用操作为下述 repair-self-alias。 |
| 保留根的 expected before | `.chrono-harness/state/verus-tool-contract-20261114/vstd/vstd.rs`，4,563 字节；SHA-256 `4cca22df78489ce34a5fb3c71c8aa688d0ceb46c5e339d09947d31d6349f6b8a`。X 只读复核仍为此值，没有产生恢复结果。 |
| 目标根的 expected after | 同一路径，4,536 字节；SHA-256 `be4d10cf44a5a4f2217b51cc28b79b0599f030200b944cda8c6eb5aff343f401`。此值由保留根在内存删除该行计算；也与固定发行根加当前 canonical 五行插入得到的字节相同。它是执行前预期，尚不是实际恢复结果。 |
| 保留副本的 expected after | 同目录 `vstd.rs.pre-e0254`，应与 expected before 的 4,563 字节及 SHA-256 完全相同；由系统 patch 的固定 simple backup 选项产生，X 未创建。 |
| 唯一外部合同源 | `crates/runner/src/json_value_specs.rs`，4,357 字节；SHA-256 `a41dd58bc38e1d7dbc2a7bc55ae4bcb15b35ab22f1db821863fceb421b2df4dc`，未改。根中的相对模块路径仍解析到这一份源。 |

固定发行包仍为上表的 25,543,507 字节／`023e0a6e67bb9927f43a8757ebdec318b50cbfe1fec4a1c320c53d5fd71008e7`。X 只读取归档，不重新解压；逐一比较归档列出的 168 个文件与现存组件，只有 `vstd/vstd.rs` 不同，且没有额外文件。原发行根为 4,282 字节／`a5317ab99820e48f7a1fab6f298732bab29047ed6e2ca8cf4e4e5290be5c230a`。后续在保护范围内重新比较同一固定归档的明确成员，除目标根外的 167 个文件必须保持原字节；恢复后仅允许增加上述原根副本。身份因此由固定归档加唯一根替换及副本确定，不按目录名推断，不另建清单或组件身份账本。仍须核对无符号链接替代、根与父目录归属、合同路径、旧库／VIR 和缺失 dep-info；静态目标摘要不赋予任何库成功资格。

### 唯一恢复操作与执行合同

保留组件的恢复归原 `script:host-bootstrap-launcher`／`host-bootstrap-native` 所有者，登记 action 为 `verus-contract-repair-self-alias`，operation 为 `investigate.verus-tool-contract.repair-self-alias`。完整 argv 只由 projects.json 负责；FILEMAP 沿用 `.chrono-harness/state/` claim，不加测试计划、集中锁、工具绑定或执行器。它是当前失败源码准备的恢复，不是历史格式兼容，也不替换未打补丁源的 canonical 准备入口。

调用方先封存候选并核对干净输入、候选字节身份、原 W 报告和所有原任务已 joined；确认没有仍在消费／修改该组件的任务。继续使用本工作树、原 native use 和已修正的 managed PATH，执行中不得有独立修改者；新任务的原环境仍须按实际 managed 原件核对，不能拿 W 环境摘要代替观察。caller 在保护范围内紧邻执行重新核对本节 before 及归档关系，确认 `vstd.rs.pre-e0254`、`vstd.rs.rej` 和其他备份／拒绝文件不存在，无独有工作会被覆盖。任一前提不符即保留 unknown 并交原所有者，不施补丁、不重新解压、不改 state 来凑身份。

封存后，恢复活动只走这个登记入口：

```sh
.chrono-harness/bin/chrono-worktree use --operation investigate.verus-tool-contract.repair-self-alias --path /Users/macstudio2/chrono-spec-current-dev
```

原生系统 patch 禁止交互、fuzz 为 0，并以 `-b -V simple -z .pre-e0254` 保存修复前原根；系统手册明确这些参数的含义，显式 simple 也排除环境选择编号备份的差异。它只消费恢复补丁，不消费完整 canonical patch。未增加程序分支或成功包装。原生 patch 返回 0 只说明补丁操作返回成功；调用方必须等待并核对原 stdout／stderr／退出、目标根、副本及其余成员，才可记录源码恢复完成。不能从 patch 0 推出 build 或严格证明成功。非零、未知或字节不符时停止依赖工作，保留可能的部分修改、副本、拒绝文件及原回执，不原地重试或手工回滚；use 的实际外层退出也须单独核对，不能由本文预填成功。

恢复验收后，下一 assignment 才可沿原 build → consume → reject 的登记依赖继续；不重复已成功的 extract／完整 patch，不在未变化的失败输入上重跑 build。build 仍须取得真实完整 vstd 验证、编译和新的 rlib／VIR／dep-info；严格正向必须消费同次组件并成功，反向必须实际因仓内 assume 被拒绝且保留非零原结果。任何另一类失败先读取原业务证据，再交自然所有者，不扩大本次准备 assignment。

### 静态核对、失败原件与未完成目标

本次仅核对 JSON 可解析、登记操作唯一及 claim／数据边、补丁行数与精确上下文、两种输入到同一目标根的字节关系、相对合同路径，以及净改动。没有为 patch 文本增造镜像测试；这些核对不是正式 DELTA、原生恢复／构建、同源证明或消费者验收。新组件实际身份、恢复回执、严格正反向结果及完整成本仍未知。全部 SPEC 目标与既有证明／绑定／generation／交付义务保持未完成，等待调用方的后续 assignment。

必要失败仍是 W 的 `use-42526-1794627191763564000-0.json`，位于原协调宿主的既有 worktrees 目录，450,870 字节／SHA-256 `3815461da388e04f710329324fe3452320dbd78956ee6638227917c8ef6502d8`。X 只读核对该原生业务报告及其带摘要的 stdout／stderr：Verus 退出 1，success=false、verified=0、整 crate，Rust E0254 明确指向第 29／155 行的重复 namespace。它仍不是反向严格拒绝。原报告、双流及恢复时系统 patch 产生的原根副本，供工具源码准备、runner 资格、调用方及 mandatory review 使用；沿现有 evidence-retain／check-original 所有者保管，2026-11-21 复核用途，调试和验收未完成不删除。使用方结束、必要证据已保全且无活使用方后，才由调用方通过既有生命周期回收；X 不写保留状态，不运行清理或新增永久副本。

## Y：保留组件恢复通过，完整构建遇到新的名称解析失败

Y 在干净 `daa6cf1ac58f2a5ab09be3a25c211c3445cd57f9`／tree `5cfa3c0fbc1dbab72d9e1fa1e3b7a10c0d1ae140` 上执行本节两项操作，各一次。执行前复核 X 的五项候选输入、W 原失败及已加入状态；紧邻恢复再次核对根、路径归属、无符号链接替代、目标登记无活跃 use、备份／拒绝文件不存在，以及固定归档的全部 168 个成员。未重复 extract 或完整 patch，未改 installed 工具、生产源码、登记、flags、fixture、预算或线程。

| 登记操作 | 原生业务原件与实际结果 |
| --- | --- |
| `investigate.verus-tool-contract.repair-self-alias` | 协调宿主 `/Users/macstudio2/chrono-harness/.chrono-harness/state/worktrees/use-39712-1794629301535784000-0.json`，531,122 字节，SHA-256 `5e4334de8b573c8dec6fad9a9802804cb810a5eafda3e3da2c7bb1fd823acac5`。use 退出 0，managed patch 退出 0，56 项唯一进程全部 `owned_work_joined: true`。原 `.json.process/stdout` 为 29 字节／`25fc5f9634a44785bc9133d7d5c06978c7d547f283993b4036fd8f24b910e435`，stderr 为 0 字节。 |
| `investigate.verus-tool-contract.build` | 同目录 `use-45161-1794629422647616000-0.json`，451,546 字节，SHA-256 `535447d3192d8615f487d6f03bbd3c512b0f4375e1e2d92afa3b144a8002c56e`。use 退出 2，Verus 退出 1，42 项唯一进程全部 joined。原 stdout 为 479 字节／`16e02ae17ca34dfd13d276a12ea0e96cdad307fc2732b75bb490a1873eab6f42`，stderr 为 829 字节／`d0921b7cc773c060d9052842a7c2d9887159d0abb9c2d7a65809ea60884c014b`。 |

恢复实际 after 为 X 预期的 4,536 字节／`be4d10cf44a5a4f2217b51cc28b79b0599f030200b944cda8c6eb5aff343f401`；原生 patch 产生的唯一新增成员 `vstd/vstd.rs.pre-e0254` 为 exact before 的 4,563 字节／`4cca22df78489ce34a5fb3c71c8aa688d0ceb46c5e339d09947d31d6349f6b8a`。无 reject 文件，其他 167 个发行成员保持原字节；build 返回后再次比较仍相同。因此源码恢复通过，但没有获得新库成功资格。

两项新任务均从实际 intent／launch／result 核对同一环境摘要 `33d877154a810bb9608dee0a7c724da03df4bdbf07f8d136919c9cb705274a5d`；managed PATH 摘要为 `c482ed2161401984bf8cb381c7812af22e9bc1f98baa05cf536251331c8dbada`，与报告中的实际 PATH 值对应，首项为 `/Users/macstudio2/.cargo/bin`，现有 rustup 字节仍符合 X 身份。程序／参数绑定一致，保留两线程、2,400 秒及每流 67,108,864 字节。Verus 原输出确认固定 commit、Rust 1.95.0、整 crate、`success: false`、`encountered-error: true`、`encountered-vir-error: false`、`verified: 0`、`errors: 0`；这里的 errors=0 不代表 Rust 编译成功。

新 stderr 的 E0433 指向唯一合同源 `json_value_specs.rs:10` 的 `use vstd::prelude::*`，随后第 12 行 `verus!` 宏无法解析。上游根第 154 行的现有 `pub use crate as vstd` 只说明可通过 `crate::vstd`／`$crate::vstd` 引用，不能由 X 的静态等价根推得该合同源的裸 `vstd::` 路径可在实际构建中解析。最小修正归现有工具源码准备与 runner 合同源所有者：建立同一合同源在真实 vstd 模块中的名称／宏解析关系，并重新封存与验收；具体修法尚未实测。Y 不在 state、生产源或 flags 中消除该新错误，也不重跑。

原 rlib／VIR 仍为发行包旧摘要，dep-info 未产生，六项正反向输出均不存在。`consume` 与 `reject` 各零次，因 build 失败未获前置资格；严格正向与 `repository_assumption` 的严格拒绝均保持未知，当前错误不是负向成功。完整组件验证／编译、同次严格正反向、typed compiler-input qualification、全部仓内适配器与独立预期证明、CI 四个真实 generation 分支、实际编译绑定、正式 DELTA、integration／dev 和落地仍未完成；全部 SPEC goal 保持 active。

交接前的原生 `status --path` 退出 0，原件为同目录 `status-61282-1794629624596918000-0.json`；32 项进程全部 joined，目标当前 uses 为空、工作仍 active。状态仍提示 `report_handoff_pending: true`／`recover-result-handoff`，并保留既有 cleanup identity 提醒；这不撤销已核实的原命令结果，也不证明生命周期交接或清理完成。按本 assignment 不执行 finish／maintain／migrate，原样交调用方继续处理。

成本仅测原 managed `launch.json` 至 `result.json` 的文件修改时间窗口：恢复 0.401030784 秒，失败 build 13.806015047 秒；包含原件发布开销，不是 CPU 时间、完整 use 用时或成功构建成本，不据此估算全生命周期收益。失败与恢复原件、before 副本供工具源码准备、runner 资格、调用方及 review 消费，沿 X 的现有保管与 2026-11-21 用途复核条件保留；调试、验收和真实使用方结束并保全必要证据后，交调用方走既有生命周期释放。本轮未作 Git／GitHub 写入，也未执行 bootstrap 或正式检查。

## Z：唯一合同源的现役名称绑定准备（未执行）

Z 只准备同一合同源在现役编译上下文中的名称／prelude／宏绑定，服务 SPEC §2.1.1 的完整证明与实际编译目标。固定起点为 `daa6cf1ac58f2a5ab09be3a25c211c3445cd57f9`／tree `5cfa3c0fbc1dbab72d9e1fa1e3b7a10c0d1ae140`；进入 Z 时本文已有 Y 的全部 66,615 字节，SHA-256 `6f6cba87988d3bf6af1c1a9f2ccd190d829d6aacddcc91dd2a9fc4ee5c9dd5fd`，原样保留为前缀。Y 的原业务 stdout／stderr 已直接读取：完整构建失败，verified=0，E0433 和宏解析错误均发生于唯一合同源，不能充当严格反向拒绝。本节的源码准备不是新运行成功、review、正式 DELTA 或交付；全部 SPEC goal 仍 active。

### 共同机制与当前使用方

合同源在 ghost 编译中用 `use crate::vstd::{self, prelude::*};`，同时绑定局部 `vstd` 名称及同一库的 prelude，覆盖文件内所有 `vstd::laws_cmp`／`vstd::std_specs` 路径。`verus` 宏改为始终直接从已有 `verus_builtin_macros` 导入，避免把宏可用性寄托于 glob 导入。runner 根只在既有 ghost 编译条件下用 `use ::vstd;` 把真正外部库绑定到 `crate::vstd`；真实 vstd 根继续复用上游已有 self re-export。普通 Rust 擦除编译不要求 vstd crate，继续使用 Cargo 已锁定的同一个宏依赖。cfg 只控制证明库的导入，没有隐藏合同声明、仓内适配器、状态、驱动或独立预期。

| 现役上下文／登记入口 | 唯一源与名称对应、实际用途及限制 |
| --- | --- |
| 真实 vstd 内嵌模块／`investigate.verus-tool-contract.build`，edition 2021 | 保留根的 `#[path] pub mod json_value_specs` 仍直接读本仓同一文件；`crate::vstd` 是根已有的 `pub use crate as vstd`。原 prelude 导出 Map／Seq、View 和相同 proc macro。完整真实库的外部合同承载方式及新 rlib／VIR 仍须实际构建验收。 |
| 仓内 runner 整 crate／`investigate.ci-generation-dependency.runner`，edition 2024 | `src/lib.rs::json_value_specs` 读同一文件；根的 `::vstd` 从 Verus 提供的 extern prelude 取得真实库，再由合同模块显式导入。`facts_binding::JsonBindingReferences`、`facts_inputs::JsonGuardReferences`、`facts_configs` 的 selector／target／captured map／seen set 仍调用 `crate::json_value_specs`。这只修正名称关系；当前 runner 仍本地包含外部假设，尚未迁移为消费新工具合同，也未取得严格证明资格。 |
| 独立仓内预期／`verify.runner-json-adapters-tests` | `tests/json_adapters.rs` 仍经 `chrono_harness::json_value_specs` 和真实公开适配器消费同次 runner 库／VIR；不复制合同或实现。runner 前置尚无完整成功，不派发该操作。 |
| 严格工具消费者／`investigate.verus-tool-contract.consume` 与 `reject`，edition 2021 | 正向根经独立组件的 `vstd::json_value_specs` 使用同一源的已编译合同，反向仍是真实 Verus 输入 `fixtures/reject.rs::repository_assumption` 的 `assume(false)`。正向 program 和故意非法 data 的原角色、原字节、整 crate 与严格 flags 都保留；二者尚未派发。 |
| 普通 Rust／runner 的 Cargo manifest 与原 `test.runner-tests` | Cargo 已登记、锁定 `verus_builtin_macros = 0.0.0-2026-04-20-1748`；ghost 库导入被擦除，宏导入及原执行代码继续存在。原未过滤用例仍直接使用 runner 真实适配器；本轮没有编译、格式化工具或测试结果。 |

共同性质是“宿主根明确绑定实际 vstd，合同模块明确导入它，宏明确绑定已有 provider”，不按逐个诊断增加 cfg 分支、代理库或补丁。依据为本机固定 Rust 1.95.0 的 Reference：`paths.qualifiers.global-root.edition2018` 规定 `::name` 从 extern prelude 解析；`paths.qualifiers.crate` 规定 `crate` 从当前 crate 根解析；`items.use.forms`／`items.use.self` 支持一次绑定父模块和成员，`items.use.glob.shadowing` 允许显式导入覆盖同 namespace 的 glob 名称；`names/preludes` 说明 `--extern` 与 extern prelude 的关系。对应权威入口为 [Rust paths](https://doc.rust-lang.org/1.95.0/reference/paths.html)、[use](https://doc.rust-lang.org/1.95.0/reference/items/use-declarations.html)、[preludes](https://doc.rust-lang.org/1.95.0/reference/names/preludes.html)。本轮直接读本机该工具链文档，没有下载。固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be` 的真实 `vstd/vstd.rs:154` 和 `vstd/prelude.rs:19,37` 分别提供 self re-export、宏与 Map 导出；Y 的编译器诊断也明确建议导入 `verus_builtin_macros::verus`。这些是名称机制依据，不是新源码已编译或已证明的证据。

### 净改动与输入身份

| 源码 | before → after |
| --- | --- |
| `crates/runner/src/json_value_specs.rs` | 4,357 字节／`a41dd58bc38e1d7dbc2a7bc55ae4bcb15b35ab22f1db821863fceb421b2df4dc` → 4,342 字节／`3b96fe12eae53aa04968ff1fb37812ef56728c7b609b5333110f57b120f9709a`；只改 import，`verus!` 起至文件末尾的全部合同字节仍为 SHA-256 `1026f156cf7223b6292474a86206f6a2b5dbcfd15683b25d403810181e8ae34f`。 |
| `crates/runner/src/lib.rs` | 103,588 字节／`9957a35d5e030b09bc9d88341add4ac3e53ddb1ce24126496a17ec335b197fb8` → 103,735 字节／`b14086671d1b2aefaac7f574afd170a40e11b2c12d6bdefc40004ee685da804b`；只增加注释及条件 import，去掉这一个精确新增片段后摘要回到 before。 |

FILEMAP／projects 不需新增节点、操作或路径：两处源码原有 owner、verification 根、成本和使用边不变；`project:runner → test:runner-tool-contract-consume` 的现有边已选择正向组，runner-tests 原计划继续包含整 runner 证明、独立 adapter 预期及未过滤测试。FILEMAP 仍为 274,011 字节／`e1e32c95a8343cde62e07fc9a6f98366fb05a67a005a22a6f5f2b9818773f056`，projects 仍为 156,897 字节／`b7c669b6fcab825b3029120d1167e8e8ad9c14d9b80c64e4c27990e6af6428d3`。本轮只更新既有实验方法说明，没有修改政策、证明判官、成功合同或实际编译参数。

**不需要组件源码恢复动作。**保留根仍为 Y 已通过 alias 恢复后的 4,536 字节／`be4d10cf44a5a4f2217b51cc28b79b0599f030200b944cda8c6eb5aff343f401`，其 `#[path]` 实际解析到上述唯一已编辑源；原根副本仍为 4,563 字节／`4cca22df78489ce34a5fb3c71c8aa688d0ceb46c5e339d09947d31d6349f6b8a`。没有修改 state、canonical patch 或已成功的 repair-self-alias，没有新恢复层。固定归档仍为 25,543,507 字节／`023e0a6e67bb9927f43a8757ebdec318b50cbfe1fec4a1c320c53d5fd71008e7`，仅核摘要，没有重新解压或下载。本轮不重做 Y 已核的 168 成员比较；调用方在下一次受保护运行前按既有方法紧邻核对，不以本轮静态摘要代替运行时输入。

### 后续唯一正式路径与尚未验证项

调用方封存本次三文件候选，记录固定 OID／tree／源码身份并保持干净；先确认原任务已 joined、组件无活跃使用者、原失败及 before 副本仍受保护。下一 assignment 复用原 `chrono-worktree use --operation <ID> --path <work_target>`，顺序仍为 `investigate.verus-tool-contract.build` → `consume` → `reject`，完整 argv 只归 projects。无需再次 extract、完整 patch 或 alias repair；也不改变已观察通过的 PATH 方法。build 的必要输入为保留的完整 vstd 源、当前唯一合同、原 Verus／宏／Rust 1.95.0／Z3、固定 serde_json rlib 及原传递依赖；使用原 std／alloc features、两线程、2,400 秒和每流 67,108,864 字节。

build 必须原命令及 use 零退出、整 crate 验证成功，并取得实际新 `libvstd.rlib`、`vstd.vir`、`libvstd.rlib.d` 及其源码／工具／依赖绑定后才能消费。正向必须在同次组件下整 crate 严格验证、实际编译和零退出，保留全部原行为义务；反向必须使用同一组件并真实非零，诊断确实为仓内 `assume/admit not allowed with --no-cheating`，类型／名称错误或其他前置失败不算符合预期。原 `--no-cheating`、`--no-external-by-default`、线程、计划期限和输出限额均不变。统一绑定的可证伪条件为任一已列实际上下文仍发生本合同的 vstd／prelude／宏解析错误；更强合同载体主张仍须完整构建和严格正反向实际行为另行验证。

本轮静态核对仅包括：两处精确 import delta、合同正文不变、现有明确选择边和 JSON 可读取、模块路径同源、保留根／备份／归档摘要、原失败双流，以及 dep-info 和六项严格探针输出仍不存在。没有运行受治理操作、Verus、Cargo、bootstrap、check、consume／reject 或任何 Git／GitHub 写入；没有声明格式化、证明或实际消费通过。原发行 rlib／VIR 仍不具备新成功资格，未跑结果保持 unknown。纯导入转发不新造重复形状测试。

runner 当前严格根仍包含本地接口假设，实际改为消费新组件及 typed compiler-input qualification 的差距保留；完整仓内 adapters／driver／独立预期、CI generation 四个生产投影与 prepare／effect／recovery、实际编译绑定、正式 DELTA、原生 integration／dev 和落地仍未完成，不把本次准备当作这些义务已满足。后续源码和资格迁移仍归原所有者，正式 DELTA 仍走既有 `.chrono-harness/bin/chrono-harness check`，不能用本轮静态观察替代。

必要保留沿 X／Y 的 evidence-retain／check-original 所有者：Y 原失败、恢复前根副本及当前候选供工具合同工程、runner 资格工程、调用方和 mandatory review 消费，2026-11-21 复核用途；调试／验收义务未完成不删除。使用方结束、必要证据保全且无活跃使用方后，由调用方走已有生命周期释放。本轮不 finish／maintain／migrate，不增加保留生产者、清理器、缓存或中心化总账。成功构建、证明、实际消费、CI 与总生命周期成本仍未知；不据少量静态读取估算收益。Z 自有工具命令均同步取得终态，无后台任务或委派；终态交接及交付生命周期仍由调用方负责。

## AA：名称绑定修正后的真实组件构建（后继阻断）

在固定候选 `035e27d8ba06ee69455a6c87d9d72dbf3da3a540`／tree `2ff8fe4e18fabab3abc9fe8c959dee47cabc3db9` 上，复用原受保护入口，仅执行一次 `investigate.verus-tool-contract.build`：`use-31874-1794631113734805000-0`。use 退出 2，原 Verus 退出 1；原报告、intent／launch／result 和双流均已核对，42 项唯一进程全部 joined。原 argv、整 crate、std／alloc、两线程、2,400 秒和每流 67,108,864 字节上限不变；没有重复解压、打补丁、下载或修改源码／登记／工具。详细身份、原件路径和有限保留用途见 runner attempt `6af7e5350400a6a725a20142/attempt-1` 的 `observations.json`，原报告由协调宿主的 `.chrono-harness/state/worktrees/use-31874-1794631113734805000-0.json` 保管。

原 stdout 为 403 字节，报告 `success:false`、`encountered-error:true`、`encountered-vir-error:true`，没有报告 verified／errors 数量，不能沿用旧运行的 0。原 stderr 为 852 字节：唯一合同源第 25 行 `ExJsonIndex` 的 `external_trait_specification trait bound mismatch`，真实 trait 比代理多出 `serde_json::value::index::private::Sealed` bound。本次未再报告 Z 所修的名称／宏解析错误，但没有取得完整证明或编译成功。固定 serde_json 1.0.151 的实际 dep-info 所指 `src/value/index.rs:37` 确为 `pub trait Index: private::Sealed`，第 135—140 行在私有模块定义 sealed 合同；这是当前错误的源码依据，不是另一个已验证修法。

实际 managed PATH 与报告有效值的摘要匹配，首个可解析 rustup 是既有安装；intent／launch／result／报告的程序、参数与环境绑定一致。原 stdout 确认 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be`、Rust `1.95.0-aarch64-apple-darwin`。当前合同、根、备份、归档、依赖及旧 rlib／VIR 摘要未变。新增 `libvstd.rlib.d` 为 41,485 字节，SHA-256 `321872a749c9008d810b629eeff133223159f5499d285af2d232006b448a0919`；其 105 条源码校验和共覆盖 1,428,366 字节，均与当前文件相符。它在 VIR 失败前已产生，不能赋予旧 rlib／VIR 新成功资格，也不构成完整编译输入资格。

由于 build 前置失败，没有派发 consume 或 reject；两者的源码、全部断言、预算、线程、整 crate 和严格 flags 原样保留。这里不是 `repository_assumption::assume(false)` 的严格拒绝结果，不能生成成功 ProofAdmission。下一修正归唯一 JSON 外部合同及工具源码准备的现有所有者：核对固定 Verus 对 sealed 外部 trait 的受支持表达，使真实泛型 `Value::get`／`Index` 合同与实际 supertrait 一致；私有 bound 不能仅靠补一个不可访问路径认定修好。保留同源合同、真实适配器与独立预期，不增加仓内信任豁免；修正并固定新输入后仍沿 Z 的 build → consume → reject 前置关系验收。本 assignment 不修该新错误、不重跑。

结束时原生 targeted status 退出 0，32 项进程全部 joined，目标 uses 为空、工作仍 active；`report_handoff_pending:true`／`recover-result-handoff`、既有 cleanup identity 提醒和 18 项 unknown creator 仍交调用方处理，不授予删除权。managed launch 至 result 原文件 mtime 窗口为 22.273100335 秒，含发布开销；双流共 1,255 字节，非 CPU、峰值内存或全流程成本。原失败、新 dep-info 和观察由既有 evidence-retain／check-original 所有者保留，供工具合同工程、runner 资格工程、调用方及 review 消费，2026-11-21 复核用途，调试／验收义务结束并保全必要证据后再由调用方释放。没有执行正式 DELTA、bootstrap、Git／GitHub、finish／maintain／migrate 或委派；完整编译输入资格、真实 warehouse 消费迁移、全部生产路径及适配器／独立预期证明、integration／dev 落地和完整 SPEC 仍未完成，全 goal 保持 active。

## AB：现役字符串字段读取迁移准备（未执行）

起点沿 AA 的固定 HEAD／tree；进入本节前本文为 80,914 字节、SHA-256 `0bdb33f9b7ec206a301c0c2b78c5195cc8f5e3f1eb8176e5639ef24abdff507d`，全部原字节保留为前缀。AB 只准备源码与必要登记，不执行任何受治理操作。AA 的 trait 匹配失败、未报告 verified／errors 数量、旧 rlib／VIR 无新成功资格及 105 条源码校验和 dep-info 的失败资格不变。

**先定义行为承诺，再改实现。** 对任意 `&Value` 和任意完整 `&str` 键，字段读取必须返回原对象的同一成员借用；非对象（含数组）或缺键返回 None；存在但 null／错类型仍返回 Some，文本观察才返回 None。文本读取保留空串、Unicode、组合字符、NUL、换行和全文，不把前缀或归一化文字当身份。对象数量仍区分非对象与空对象；借用树键、集合 fresh／旧状态保持原合同。原 `consume.rs` 的所有函数签名、requires／ensures 和 assert、`json_adapters.rs` 全部原字节及非法 fixture 必须保留。新仓内分支必须在同源 Verus 中证明；上述承诺由原独立预期及严格正向根验收，本轮不手填成功。

选择公开 `Value::as_object` → `Map<String, Value>::get<Q>` → `Value::as_str`，把共同分支放在 runner 的 `json_fields.rs`，生产适配器与严格正向根直接包含同一程序源。这个程序不进入 vstd，不使用 `external_body`。唯一外部合同仍在 `json_value_specs.rs`；Map::get 保留真实泛型 Q、`String: Borrow<Q>` 和 `Q: ?Sized + Ord + Eq + Hash`，只对现役 str 查询补完整成员模型，其他 Q 保持不透明。原 Value::get／Index 的通用声明不是产品要求；其必要的字符串行为由以上公开 API 覆盖，不能仅删除探针。尚未迁移的其他仓内 JSON 调用仍是整 crate 证明义务，不因本节取得资格。

### 使用方、固定依据与最小表示

| 实际使用链／当前输入 | 原 API → 候选 API／模型；现役可观察验收 |
| --- | --- |
| `facts_binding::declaration` → `unique_binding_reference` → `JsonBindingReferences::id`，工具／输入行及 `id` | `Value["id"].as_str` → `json_fields::field_text`，仍对应 `field_text(row, "id")`；缺失／歧义引用判断及原 reference 独立预期。 |
| `facts_inputs::declaration` → `git_guard_reference` → `JsonGuardReferences::id`，guard 输入行 | 同上；保留 guard 唯一引用的原输入、全文和顺序，未把整个 declaration 宣称已证明。 |
| `facts_configs::identity_targets` → `collect_identity_paths` → `JsonSelectorRows::row`，sha256／path／registry_path | 三项文本与存在性改用同源 helper；`as_object`／Map::len 仍给 fields。`field` 区分缺键和 Some(null)，原 selector 独立预期保留。 |
| 同一 identity 链 → `validate_identity_registry_paths` → `SelectorRegistryRows::registry_path` | registry_path 的完整文本改用同源 helper；原 registry 路径预期不变。这是同一 struct 的第二个现役 trait 实现。 |
| `facts_configs::resolve_target` → `select_git_target` → `JsonTargetRows::text`，digest／path | 两种动态选择键都交同源 helper，仍对应原 `text_at`／`field_text`；原 target 独立预期保留。 |
| 捕获配置解析 → `select_captured_target` → `CapturedRegistry::contains_path` | 原 BTreeMap::contains_key、`string_map_borrow`、String 顺序合同全部不变；原不同存储／全文／前缀／组合 Unicode 预期不变。 |
| `identity_targets` → `collect_identity_paths` → `SeenSelectorText::record` | 原 BTreeSet::insert、`str_set_membership`、fresh 与完整旧 set 状态证明不变；原重复／空串／先前成员预期不变。 |
| 严格正向根 `observe_field`／`observe_index`、missing／present_nontext 等原边界 | 根以 `#[path]` 包含生产 `json_fields.rs`；field 返回原 Value 借用，原 Index 路径的文本投影仍是 field_text。所有原签名、前后条件、断言及树集合证明保持不变；不是把探针改成只看文本或仅空值。 |

未发现仓内在研或现役调用者要求本合同按抽象 `I: serde_json::value::Index` 实现通用算法；被迁移的调用均为 str 字段键。**这不表示仓库没有其他 Value::get／Index 调用。**例如三个 facts 文件的外层解析仍有具体字符串、`&String` 和数组索引调用，保持原执行代码；它们及完整 warehouse 的证明／合同迁移仍未完成，不能把此处的消费者清单扩称全仓已封闭。`json_adapters.rs` 三个真实 Rust 边界用例及 Verus 驱动全部 10,272 字节不变；它们最终仍须由原未过滤操作验收。

固定 serde_json 1.0.151：`value/index.rs:37,85,119,135,213` 分别给 sealed trait、str 对象查询、引用转交、私有定义和 Null 回退；`value/mod.rs:305,366,492` 给 get 转交、对象与文本变体；`map.rs:73—86` 给公开 get 的完整 Q 签名与原 map 转交。三文件摘要沿 S，AB 重核；str 对数组／非对象返回 None，因此新路径与现役字符串语义相同，保留全部成员借用而不构造替代 JSON。

固定 Verus 的 `rust_to_vir_func.rs:982—1055` 比较完整泛型签名和 predicates，`2593—2626` 只过滤 Destruct、特定 SliceSealed、内置 Sealed；`attributes.rs:641—662` 没有任意私有 sealed bound 豁免参数。其缓存源码 blob 分别为 `e6f51ee2695116a2fe5abb2f93693f1fdf3ce414`／`6957de2bcd1ea161a2cde82acf647a58bbea24ff`，AB 按既有固定 `f8e17045c4ac17a02e566b8997070f2eecc255be` tree 表核字节相符。原 assume_specification 权威文档也要求包括泛型／bounds 的相同签名；因此不把 `Value::get::<&str>` 单态假设当支持路径，不补不可引用的私有路径，不推广内置 SliceIndex 特例，不用 extension／sealed 属性掩盖 mismatch。这里未证明未来工具普遍不可能支持该 trait。

公开 Map::get 的所有 bounds 已有固定 vstd 负责：`std_specs/core.rs:90,122` 的 Hash／Borrow 和 `cmp.rs:31,116` 的 Eq／Ord，三个缓存源码均匹配上述固定 tree 的 blob；`std_specs/hash.rs:963—978` 已用同类 `Q: ?Sized`／Borrow／Hash 的泛型 get 合同。只补 serde_json 的外部成员观察，复用成熟 Rust API 与现有 Verus 泛型方式；没有编译器 fork、第三方依赖、新 trait 框架或仓内受信包装器。新 helper 仅含不可变查询及文本投影两个必要分支，供以上当前使用方共用，防止把逻辑复制进各适配器和探针。原对象／文本、Map::len 和全部树集合模型保留。

### 候选身份、静态范围与后续验收

**可见警告：产品源码与宿主 FILEMAP 方法登记同时变更。** FILEMAP 只增加 helper 的 program 行、runner 原整 crate 证明根及 runner／runner-tests／严格消费边，成本仍 unmeasured；未改政策判法、projects、原 argv、预算、线程、分类约束或成功条件。新 helper 通过 runner 根的私有模块及正向根的同源 path 消费，不加入 vstd patch。新增仓内逻辑没有 external_body／assume／admit、cfg 隐藏或证明范围豁免。

| 文件 | before 字节／SHA-256 → after 字节／SHA-256 |
| --- | --- |
| `crates/runner/src/json_value_specs.rs` | 4,342／`3b96fe12eae53aa04968ff1fb37812ef56728c7b609b5333110f57b120f9709a` → 3,768／`49cbbc1615e471987d61a563b560a40d257ab5f07c30a88960ae06eafa594b33` |
| `crates/runner/src/json_fields.rs` | 原不存在 → 806／`02037b054f66041341605c64c00e43d88a934a6c5a48b1afa7454c515ead97cc` |
| `crates/runner/src/lib.rs` | 103,735／`b14086671d1b2aefaac7f574afd170a40e11b2c12d6bdefc40004ee685da804b` → 103,752／`fa77c66c80fdef155d251e8d63c6e37871a5beaf7e76561949e5c1873952dcb3` |
| `crates/runner/src/facts_binding.rs` | 37,295／`51bc92fa661cf4125c687ef64aba9261a44c37fc4d94c6559cda88fafa46d92b` → 37,267／`928bf085f71266d05ad86d97007d01dfd02f2693e0d97d5ddc41ca852f364fe5` |
| `crates/runner/src/facts_inputs.rs` | 4,556／`23d50449bc1c8e6b210cfe3015fd4d9968d2830e1e513e7bce1fe58ef6e2026c` → 4,528／`0a4a7475ffb3b2dea47bc76b86197d86bc3b3ffb813e2621a685695a814f164f` |
| `crates/runner/src/facts_configs.rs` | 17,344／`9e2ced63be3c985da2ae684ed5810c69f12d69dd49b53eac6dd2f80e39c7fbfd` → 17,432／`134e3fef236db4387862fc47448682c9128678a2cca65346b77ee7410c4a608d` |
| `.chrono-harness/experiments/verus-tool-contract/consume.rs` | 2,875／`19e99403b1d47774c3503b479db142c92e668668a584843a51272c722b1e8f67` → 3,026／`ac06b5ccd086fd99a62dd0a93ec2bf12b2450045b0f70e81b322bfaa7933b428` |
| `.chrono-harness/FILEMAP.json` | 274,011／`e1e32c95a8343cde62e07fc9a6f98366fb05a67a005a22a6f5f2b9818773f056` → 274,471／`91042394c44a9ea1bba526a8b8fbd3c039e1c7cb70ba73179133d27132107a82` |

本文自身最终字节与摘要由 AB result envelope 列出，避免自引用摘要。静态核对限于净改动反向重建原摘要、完整 AA 前缀、JSON 可读取、唯一 FILEMAP 行／原证明操作、真实模块路径、独立预期与 fixture／projects／组件根／备份／失败 dep-info 保持原摘要，以及上述固定源码对应。没有运行 Verus、Cargo、fmt、build／consume／reject、check、bootstrap 或生命周期操作，不能报告这些成功。Map::get 的新签名匹配、全部库求解、helper 与适配器证明、普通 Rust 擦除编译及实际消费者行为仍未知；不得用源码推断声称修复已验证。

AA dep-info 原字节继续保留；它记录的唯一合同仍为旧 4,342 字节／`3b96fe12eae53aa04968ff1fb37812ef56728c7b609b5333110f57b120f9709a`，与本候选 3,768 字节／`49cbbc1615e471987d61a563b560a40d257ab5f07c30a88960ae06eafa594b33` 已不同。它只解释 AA 的旧输入，不能描述新候选，更不能取得成功资格。新 helper 不属于 vstd 源；后续 consume 自己的新 dep-info 必须绑定它。

调用方先封存本次九文件输入及干净候选；下一 assignment 沿现役 `chrono-worktree use --operation <ID> --path <work_target>`，依次 build → consume → reject（ID 沿 Z）。无需 extract／patch／repair，不直接改 state。build 仍消费唯一合同、保留完整 vstd 根、固定 serde_json rlib／传递依赖、原工具／宏／Rust／Z3，保留 std／alloc、整 crate、两线程、2,400 秒及每流 67,108,864 字节。必须完整原生证明与编译成功、原程序及 use 零退出，并取得同次新 rlib／VIR／dep-info 与输入绑定后，才取得下游执行前提。旧发行产物及 AA dep-info 均不能填补该前提。

consume 必须在同次新组件下，用原严格 flags 整 crate 证明并实际编译，dep-info 必须含本仓同一个 `json_fields.rs`；所有原后置条件、边界断言和状态承诺必须通过。任一 bound／名称／类型错误、返回关系或原断言失败均证伪候选；失败保留原输入／双流／退出，不抬预算或重复运行。reject 必须同组件下真实非零且确为仓内 `assume/admit not allowed with --no-cheating`，前置错误不算严格拒绝。fixture 保持 data 身份及原非法 Verus 输入，不宣称其成功证明。runner、原 adapter-tests 和未过滤测试仍须由正式 DELTA 共同验收，不能以新正向根成功代替。

必要保留归原 evidence-retain／check-original 所有者，使用方仍为工具合同工程、runner 资格工程、调用方与 review；AA 失败及 dep-info 保留至调试／验收义务结束，2026-11-21 复核用途，随后按已有生命周期释放。没有新增永久证据生产者。新函数、模块包含和登记的完整构建／证明／CI／存储／维护与运行成本未知；既有预算不变，没有测量收益。全部 warehouse 迁移、typed compiler-input 资格、完整 CI generation 生产与恢复链、正式 DELTA、原生 integration／dev 及落地验收仍是 active 全 goal 义务。本轮不交付、不清理；自有静态命令均已取得原生终态，无后台任务或委派（一次无关范围过大的只读文件名枚举已中止并取得 exit 130），交接生命周期由调用方负责。

## AC：公开 JSON 查询的真实组件与严格消费（后继阻断）

固定候选为 `8fe9b108fdbf92d50c6e330d139a6314cc215170`／tree `e89a6990b065e2e8b480a9fde775e05566aced84`；操作前工作树干净，AB 全部 93,135 字节／SHA-256 `1f022ac01e504d1bbbc14c73bce712a3aa4722e1eb3f3417f7be4654e1439ce6` 保留为本文前缀。本轮只沿登记 use 各执行一次 build 和 consume，没有改源码、登记、fixture、工具、线程或预算；consume 失败后没有派发 reject。

以下原件均在协调宿主 `/Users/macstudio2/chrono-harness/.chrono-harness/state/worktrees/`；双流路径为对应报告名加 `.process/stdout`／`.process/stderr`。

| 操作／原报告 | 原生终态与原件身份 |
| --- | --- |
| build：`use-53527-1794633613411773000-0.json` | use／Verus 均退出 0；55 项唯一进程全部 joined。报告 713,756 字节／`5c56b582b3269b409581125d5a87be3ce5e03f416673eb2831c836beb48c8486`；stdout 675,537 字节／`8f203d5bf564e6a705fb7b4008131201e864b46ef48855a894af2efaae5112a1`；stderr 0 字节／`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。 |
| consume：`use-10071-1794633984433939000-0.json` | use 退出 2、Verus 退出 1；42 项唯一进程全部 joined。报告 452,054 字节／`62cf6165ba340bcef4645c4f7e6bc59ed96f45fa1e9c9083049f8b500deaa9f0`；stdout 403 字节／`8defb0303f480d60e4ef3ad65cd0b36a42dd9a2cd7428d4661818e0649598b32`；stderr 285 字节／`4789328f54ab95fe24b712983f13b7f5abe91992a141d43d14a826bf8493abe5`。 |

build 原 stdout 报告 `success:true`、整 crate、1557 verified／0 errors；保留 `--is-vstd`、std／alloc、两线程、2,400 秒和每流 67,108,864 字节。新组件目录仍为 `.chrono-harness/state/verus-tool-contract-20261114/`：`libvstd.rlib` 为 7,594,176 字节／`89345c0e49264471c945c01f551b6cf36efb5e4404739076db812183b5a2efca`，`vstd.vir` 为 25,375,402 字节／`371c8f5d93d25dce8274c7e498dc36c04e501cf24214996da35d4dc2b0356c96`，`libvstd.rlib.d` 为 41,485 字节／`1367c34867875d31e5995dcc1746dcc4bcb1b640266799ce681ed8f21d2c4d4d`。同次 dep-info 的 105 条源码校验和全部匹配当前字节，包括唯一 JSON 合同的 3,768 字节／`49cbbc1615e471987d61a563b560a40d257ab5f07c30a88960ae06eafa594b33`。这是完整真实工具库的构建和编译成功，仍不授予仓内程序假设豁免或完整 typed 资格。

consume 实际使用上述同一组件和原严格 flags，诊断为 `consume.rs:68` 的 `json::standard_string_key_order()`：`cannot call a broadcast_forall function with 0 arguments directly`。原 stdout 为 `success:false`、`encountered-error:true`、`encountered-vir-error:true`，未报告 verified／errors 数量。没有生成 consume rlib；失败过程产生的 VIR／dep-info 只作诊断，其两条源码校验和确含同一仓内 `json_fields.rs` 和 consume 根，不能取得成功证明／编译资格。组件三项产物在 consume 后保持同一字节。此错误不是非法 fixture 的 assume 严格拒绝，正向验收失败、反向未执行，不能生成成功 ProofAdmission。

实际 use 原件中的 PATH 含 `/Users/macstudio2/.cargo/bin`，其 SHA-256 `beeaa76832fc57cab0dc8451b2055535577fe62f8f8c60db3e335f693ca94469` 与两项 managed result 的 PATH 摘要相同；实际 rustup／Rust 1.95.0-aarch64-apple-darwin、固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be`、宏、Z3、serde_json rlib 均已核身份。操作前组件相对原发行包的 168 个文件中 167 个相同，唯一差异为已登记 vstd 根；没有重复 extract／patch／repair、下载或修改 installed／shared 工具。更完整的传递输入资格仍未建立。

下一自然差异归现有 JSON 外部合同、严格消费及适配器所有者：修正零参数广播事实的启用方式，保留全部签名、前后条件、独立断言与完整集合状态，再封存新候选按原前置链验收；本轮不修新错误、不重跑。原生定向交接 status 退出 0、32 项进程全部 joined，仍报告 active、`report_handoff_pending:true` 及旧 cleanup 失败提醒；这些不是新工具失败或删除授权，恢复交接与生命周期由调用方负责。

详细原件、源／工具身份、失败 dep-info 和文档 delta 由本次 runner attempt `6af7eebecb95ffcd0a113db0/attempt-1` 的结构化结果引用，不复制 worker 日志。按原生 launch／结果产物文件时间近似计，build 从 launch 到 rlib 写入为 220.639 秒，consume 从 launch 到 result 为 5.123 秒；这不是 CPU、完整 use／CI 或生命周期总成本，原 stdout 未报告耗时，未测收益。必要原件仍归既有 check-original／evidence-retain 所有者，供上述工程、调用方及 review 使用，2026-11-21 复核用途，调试／验收义务结束且无使用方后才由调用方释放；本轮不 finish／maintain／migrate／删除。raw 严格正反向、typed compiler/source/context/report/rlib/dep-info/VIR／宏／传递输入资格、warehouse 和全仓 generation／adapter／独立预期证明、正式 DELTA、原生 integration→dev／落地及完整 SPEC 均未完成，全 goal 保持 active；本节不是正式 review 或交付。


## AD：零参数广播事实的实际消费者修正准备

本轮只准备源码；AC 全部 98,348 字节／`80cfb0d6d2eba5262b9ab4d50ec7d479083e7709cbaa4d87fd1fa810e51af1ae` 原样保留为前缀。固定工具仍为 Verus `0.2026.04.24.f8e1704`／commit `f8e17045c4ac17a02e566b8997070f2eecc255be`。其同次构建源码 `vstd/laws_cmp.rs:106–112` 用 `broadcast use` 启用零参数泛型事实，`vstd/layout.rs:157、312` 用同一方式启用零参数 `layout_of_primitives`；这些源均在 AC 的 105 条 dep-info 校验和中。复用成熟语法，在实际消费者的函数体作用域启用原事实，让后续树集合操作也处于该作用域，不改事实声明或增加包装器。

| 原零参数事实 | 直接消费者（四处全部修正） |
| --- | --- |
| `standard_string_key_order` | `consume.rs::borrowed_fulltext`；`facts_configs.rs` 中 `CapturedRegistry` 的 `RegistryMembership::contains_path` |
| `standard_str_key_order` | `consume.rs::fulltext_insert`；`facts_configs.rs` 中 `SeenSelectorText` 的 `SelectorSeen::record` |

共同性质是两个事实没有值参数、分别给出 `obeys_cmp::<String>()` 和 `obeys_cmp::<&str>()`。四处均改为函数体中的 `broadcast use <原路径>;`，紧邻原 `proof` 块之前；原有带参数的借用／成员事实调用继续保留。工程检索覆盖已登记 generation／runner／独立预期根及其相关源码；该合同只有这两个零参数广播事实，未发现其他直接调用。`json_adapters.rs::captured_membership`／`record_text` 经真实适配器消费其合同，不需重复启用；其余 JSON 适配器、共用 `json_fields`、generation 根和独立预期均无同类直接调用。检索不用于选择正式检查。

| 必要源码 delta | before 字节／SHA-256 → after 字节／SHA-256 |
| --- | --- |
| `.chrono-harness/experiments/verus-tool-contract/consume.rs` | 3,026／`ac06b5ccd086fd99a62dd0a93ec2bf12b2450045b0f70e81b322bfaa7933b428` → 3,042／`c515184b707dde87e8740af1790859848cf3a7e0b32fa87353c18fe47e3debb3` |
| `crates/runner/src/facts_configs.rs` | 17,432／`134e3fef236db4387862fc47448682c9128678a2cca65346b77ee7410c4a608d` → 17,448／`e9906a606628b7933095e5c6a06cef4fd0ed5e422248a3e04a056b932898a0b6` |

静态逆变换恰好还原两个 before 摘要：全部签名、requires／ensures、assert、全文 Unicode／引用／树集合状态、运行语句和独立预期原字节均保留。FILEMAP 已把两源登记为 program，并分别绑定严格 consume 和 runner 整 crate 根；现有 runner 编译／测试边及独立预期计划继续适用，FILEMAP、projects、政策和操作 argv 均未修改。没有新增测试或验证机制。

同次构建 dep-info 的 105 条源码校验和仍全部匹配；两个修改源均不在工具库输入中。唯一 `json_value_specs.rs` 仍为 3,768 字节／`49cbbc1615e471987d61a563b560a40d257ab5f07c30a88960ae06eafa594b33`，共用 helper、独立预期及 reject fixture 均未改。AC 三项成功组件产物仍精确匹配该节字节／SHA；本轮前后固定工具、根和备份、补丁及上述输入的静态摘要不变。因此没有使 AC 完整 build 的复用前提失效；不再 extract／patch／repair／build／下载，也不改 installed／shared 工具。完整传递输入的 typed 资格仍未知，这些字节核对不提供该资格。

调用方固定并提交干净候选后，下一 assignment 仅沿原 `chrono-worktree use --operation <ID> --path <work_target>` 执行 `investigate.verus-tool-contract.consume` → `investigate.verus-tool-contract.reject`，每项最多一次，前置失败停止后继。原登记仍要求整 crate、`--no-cheating`、`--no-external-by-default`、两线程、原预算、实际编译和 SHA-256 dep-info。consume 必须完整证明原合同并实际生成成功 rlib，新的 dep-info 必须绑定同源 helper 与新消费者；仅消失本次 VIR 错误不足以通过。reject 必须使用同一组件真实非零，诊断确实是 `repository_assumption` 的 `assume(false)` 被 no-cheating 拒绝；其他失败不得算负向通过。若输入／工具／组件身份漂移，先报告复用前提失效，不能悄悄重建或更改假设。

本轮未执行编译、证明、探针、格式化、bootstrap、check 或任何受治理 operation；严格正向／负向仍未验收。没有 Git／GitHub、state 写入、生命周期操作或委派；全部自有静态命令取得原生退出状态，无后台任务。成本和新语法实际接受结果未测。typed 完整资格、warehouse、完整仓内源证明与实际编译绑定、正式 DELTA、integration→dev 和落地仍待完成，全 goal 保持 active。本 chunk 不触发正式 review；有限修复批次及适用检查完成后仍须 mandatory review triplet。必要失败原件沿既有 check-original／evidence-retain 的使用方、2026-11-21 复核及释放条件保留，旧 cleanup／handoff 未知交调用方处理。


## AE：复用成功组件后的严格正反向结果

固定执行候选为 `7e06910a43a0689bde3633f93540129e915b2dbe`／tree `586105e979541cafd68389722840adcf25c4ba2c`。调用方已独立核定完整索引、提交字节和干净树；本轮定向原生 status 前后都观察到同一 HEAD，操作及依赖采集结束前未改任何跟踪文件。本文原 103,345 字节／SHA-256 `396341036fce087900ea5d87924ac1a68f8a04d4becc3dd60e7960a9caa428b3` 全部保留为前缀。本节是唯一文档追加，不是正式 DELTA、review、warehouse 准入或交付。

**本轮 raw 严格正反向已验收。** 沿登记 `chrono-worktree use --operation <ID> --path /Users/macstudio2/chrono-spec-current-dev`，consume、reject 各一次；没有重跑 build／extract／patch／repair，没有改源码、登记、fixture、flags、两线程、2,400 秒或每流 67,108,864 字节。复用 AC 成功 build 原报告及三个组件产物：前后字节与 AC 相同，105 条构建源码校验和全部匹配，11 项既有固定缓存输入匹配；工具／宏／Z3／Rust 字节未变。操作前目标 uses 为空，原登记租约可非阻塞独占取得并释放；AC 的 55 项及原失败 consume 的 42 项唯一进程均有原生 joined 记录。没有新增执行、等待或保留机制。

原件仍由协调宿主 `/Users/macstudio2/chrono-harness/.chrono-harness/state/worktrees/` 保存；下表双流路径为报告名加 `.process/stdout`／`.process/stderr`。

| 操作／原报告 | 原生结果及原件字节／SHA-256 |
| --- | --- |
| consume：`use-44453-1794635710202422000-0.json` | use／工具均退出 0，55 项唯一进程全部 joined；报告 531,713／`3e51eae38537e98be9eaaca24af6383e969efa9b01f56ff8ad61f6ae9c4a6f1f`；stdout 14,731／`c74cd588d7f900bedfdcb40b3c7e9079526829631740b161443e2c10ab80cb5f`；stderr 0／`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。 |
| reject：`use-62365-1794635842740187000-0.json` | use 退出 2、工具真实退出 1，42 项唯一进程全部 joined；报告 450,739／`57bf00b7ea08587af11edecb1092af62f49b38209290e9d1aa456dabae42951a`；stdout 768／`ba66f1c530acff1a6e3af8127c225ac9b02f1b826a2a194c23ab60dbed83241d`；stderr 230／`ab5b92862665748efc8d56da967c64299cd6cfd89d4252d5c3e6e5029a631b12`。 |

consume 原 stdout 报告 `success:true`、整 crate、11 verified／0 errors，保留 `--no-cheating`、`--no-external-by-default`、`--compile` 和 SHA-256 dep-info。原先不存在的 rlib 由本次成功操作生成；本次三个产物如下，reject 后字节未变，不能与 AC 失败 VIR／dep-info 混用。

| 本次产物（`.chrono-harness/state/` 下） | 字节／SHA-256 |
| --- | --- |
| `verus-tool-contract-20261114-consume.rlib` | 171,752／`7e0764bd6aba78fe71dae614dbfcd82f3ddb50b5fcfae48962a79b78448b2ac1` |
| `verus-tool-contract-20261114-consume.vir` | 51,738／`283d5bbdaa3f0a3a1ce14b9f3db49aab40115fd1004fc58f8eb86cdffdc9c94a` |
| `verus-tool-contract-20261114-consume.rlib.d` | 945／`f53fb925697d6d4edfead050e86315aeed4890af94f5c9ca391a5c9b45e51c93` |

新 dep-info 的两条源码校验和精确绑定当前 `consume.rs` 的 3,042 字节／`c515184b707dde87e8740af1790859848cf3a7e0b32fa87353c18fe47e3debb3` 和同源 `json_fields.rs` 的 806 字节／`02037b054f66041341605c64c00e43d88a934a6c5a48b1afa7454c515ead97cc`。原函数签名、前后条件、全文／借用／对象／集合状态断言和独立预期均未改；不是仅凭旧错误消失判成功。实际 managed result 的 PATH 摘要为 `ff91a4c3ca52b0bb71eaa4c0bb040da0aae7936dcaca80286dab41f4f8622f7e`，与原生 effective PATH 的实际字节及 result 相符，确含现有 `/Users/macstudio2/.cargo/bin`；相对 AC 的 managed 环境只有 PATH 字节变化。两次新操作实际环境相同，intent／launch／result 环境绑定一致，原 stdout 均报告固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be` 与 `1.95.0-aarch64-apple-darwin`。

reject 的原诊断为 `assume/admit not allowed with --no-cheating`，明确定位 `fixtures/reject.rs:9:13` 的 `proof { assume(false); }`，原 stdout 也点名 `verus_tool_contract_reject::repository_assumption`，不是名称、类型或其他前置错误。原 stdout 保留 `success:false`、`encountered-error:true`、`encountered-vir-error:true`；未报告 verified／errors 数量，不补 0。故意非法 fixture 按现役 data 合同消费，其真实非零不包装成成功 ProofAdmission，也不豁免仓内真实逻辑。

交接定向 status 原报告为 `status-68507-1794635918875622000-0.json`，退出 0、32 项唯一进程全部 joined；目标仍 active、`report_handoff_pending:true`，旧 cleanup 失败仍由调用方处理，不据此推导删除权。本轮全部自有原生任务已终结，没有 finish／maintain／migrate／删除或委派；未直接调用 Git、改索引／引用或访问 GitHub，受管入口内部的 Git 读取仍见原报告。按各自 launch→result 文件时间近似计，consume 为 9.783 秒，reject 为 1.158 秒；这是业务原件时间窗口，不是 CPU、完整 use／CI 或生命周期总成本，后者未测。

详细输入／工具／组件、原报告／intent／launch／result／双流摘要、joined 与环境绑定由本次 runner attempt `6af7f6a6079db12198010da3/attempt-1/result.json` 引用。AC 原成功 build 与失败 consume 的报告、双流和必要诊断仍按既有 check-original／evidence-retain 保留，已核旧身份且诊断消费完成的失败临时 VIR／dep-info 由原生产者正常替换；没有另建永久备份或总账。原件供工程、调用方和 review 使用，2026-11-21 复核，调试／验收义务结束且无活使用方后才由调用方释放。

下一差异回到既有生产所有者：完整 typed compiler／source／context／report／rlib／dep-info／VIR／宏／传递输入资格、warehouse 迁移、全部仓内 adapter／generation／独立预期证明与实际编译绑定、适用行为验收、正式 DELTA、原生 integration→dev 和落地仍须完成。特别是本轮未执行 `facts_configs` 的 runner 整 crate 证明与其行为操作，不能把严格实验根成功推广为这些范围成功。全 SPEC goal 保持 active；本 chunk 不触发 formalreview，有限批次及适用实际工作完成后仍需 mandatory review triplet。

## AG：唯一工具合同接入真实 runner（源码准备，未执行）

本轮基于调用方封存的 `9feb4e84a703d9dd07cfb11ded8cbf921e9045e6`／tree `d357bb0695452c8cf805bc1706b889712ce528b4`，只准备实际合同归属和工具消费接线。`runner/src/lib.rs` 在 `verus_keep_ghost` 下重导出 `vstd::json_value_specs`，普通 Rust 仍从原地址擦除同一合同源的 ghost 声明。这个条件只选择外部合同所有者；`json_fields`、七个实际 trait 实现及独立 `json_adapters` 的程序、断言和状态维护全部保留，不新增仓内 `external_body`、假设或副本。

**产品与宿主政策同时修改：**macos-27 的工具表增加仅供本轮调查消费的 `verus-json-contract`，直接指向本工作树 `.chrono-harness/state/verus-tool-contract-20261114/verus`；默认正式 `verus` 不变。原 `investigate.ci-generation-dependency.runner` 与 `verify.runner-json-adapters-tests` 选用此工具，并从同目录选用 `libverus_builtin_macros.dylib`。原生 wrapper 按自身位置取得已成功构建的相邻 `libvstd.rlib`／`vstd.vir`；不经 bash 隐藏 compiler 身份，不新增 recipe、安装或构建入口。固定版本字符串只用于既有版本观察，不证明这个扩展组件已具正式资格。

FILEMAP 只为该 macOS 工具选择增加 runner、runner-tests 的明确输入边及原 runner-tests 的执行边。`projects.json` 和 `macos-27.json` 补登记为声明数据：它们提供操作／工具选择，不包含程序实现或准入算法；没有把任何 Rust program／specification 改成 data。全部已有程序根、计划、顺序、输出占用、两线程、strict flags、whole crate、compile、checksum、正式 900 秒／1 MiB 和 raw 调查 2400 秒／每流 64 MiB 原样保留。Linux 尚未登记这个本地 macOS 工具；共享 actions 在该配置下会触发现有 `E_REFERENCE`，是正式跨平台验收前必须解决的缺口。本轮不宣称跨平台采用，也不能在那里用旧默认库替代本组件。

复用前提仍是 AC 三个成功产物、105 条构建源码校验和、唯一合同及原固定缓存依赖不变；本次静态读取全部匹配。合同源仍为 3,768 字节／`49cbbc1615e471987d61a563b560a40d257ab5f07c30a88960ae06eafa594b33`，共享 helper、七个实现和独立预期均未改。成功 AC build 和 AE consume／reject 不重跑；不 extract／patch／repair／下载或重建，不写 installed／shared 工具。保留原行为套件、全部断言、失败原件与现有缓存／retention 所有者。

调用方封存并核干净候选后，下一 assignment 沿上文唯一 `chrono-worktree use` 入口，先执行 `investigate.ci-generation-dependency.runner`，取得原 stdout／stderr／退出、全部自有进程 joined、严格整 crate 成功以及同次 rlib／dep-info／VIR，核对实际工具、宏和依赖身份后，才执行 `verify.runner-json-adapters-tests`。后者必须导入前者同次库／VIR，完整保留全文、Unicode、借用、对象和集合状态预期。新失败保留原件，仅停止其明确后继，不重跑成功前置、不换缓存 runner 或放宽预期。后续 generation 的 consumer／missing 仍选旧工具，本轮不执行；迁移并绑定同一组件前不得消费新 runner 库。

**正式资格仍未建立。**现有 source-only guard 继续拒绝扩展参数；两个 action 原 `output_contract` 不授予 `ProofAdmission`。合同源在 FILEMAP 的 runner 根登记目前只是尚未满足的证明义务：ghost runner 改为导入工具合同后，不能声称该源出现在 runner 的源码 dep-info；其实际生产者是 AC 的完整 vstd 根及原 105 条 checksum。现役格式不能承载这条完整工具来源关系，本轮不虚改证明根、不把成功 build 加回计划迫使重建，也不改准入逻辑。typed compiler／source／context／report／rlib／dep-info／VIR／宏及传递资格仍由原所有者后续完成。

本轮仅作编辑及静态字节核对；没有运行构建、证明、探针、格式化、bootstrap、check 或 Git／GitHub，没有委派或生命周期变更。纯转发接线不新增镜像测试；候选须另验 runner 整 crate、独立预期、原未过滤 Rust 行为及按两端登记选择的正式 DELTA。新增工具选择观察、构建／证明／CI／存储及完整维护成本未测，不能由旧实验耗时推算。generation 四个真实生产分支、原生 integration→dev／落地及全部 SPEC 仍未完成，goal 保持 active，不触发 chunk formalreview。沿用 AE 的原件使用方、2026-11-21 用途复核及释放条件，活动工作树、`report_handoff_pending` 和旧 cleanup 失败继续保护，由调用方交接。

## AM：CI generation 早期 const THIR 的原位编码准备（未执行）

本节承接旧说明中修正后仍未验证的状态，准确记录 AK 的实际 ICE 与 AL 的事实／假设。固定候选仍为 `feature/ci-generation-state-20261113` 的 `a6218ce61a52f638576537dbb0a605d973eedfce`／tree `60bbc7a2c6a61dad66031e1133728f095043ad4c`；本节只准备源码和登记，不代表严格证明、实际编译、阶段通过或完整 SPEC 交付。

AK 的实际 ICE 属于稳定的 `verus-erasure-context-before-const-thir` 机制族：严格 `investigate.ci-generation-dependency.runner` 整 crate 操作在 erasure context 建立前请求 Verus-aware 本地 body，固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be` 在 `source/rustc_mir_build_additional_files/verus.rs:156` 触发 panic。AL 亲核的原报告为 `use-68541-1794643059119974000-0.json`，runner 真实退出 1，`success:false`、`verified:0`、`errors:0`、`whole_crate:true`、`encountered_vir_error:false`；没有 rlib 或 VIR。该证据证明当前 runner 失败及触发族，不证明所有 const/static/const fn 成员都会触发，也不证明下一成员或整类已修复。

AL 已核实的实际 compiler-input 域是 `crates/runner/src/process_ownership.rs:30` 的 `const CAPACITY: usize = 4096`：它同时提供 `Shared.slots: [Slot; CAPACITY]` 和 `Tree::slots`／`descendant` 的数组类型长度，并被共享映射尺寸、4096 个槽初始化、根哨兵、容量耗尽、祖先／取消遍历、上下文边界及 generation×capacity×2 的 cookie 编解码使用。共同 const/static/const fn lowering、unerased body 保留及 VIR `encoded_const` 入口已由固定 Verus 源码核得；AL 未核得展开后完整 HIR 查询轨迹，也未证明该成员是唯一首次触发点，均保持 `ASSUMED-UNVERIFIED`。

采用最小原所有者方案：在该文件顶部以现有项目惯用导入包住原位定义：

```rust
verus! {
const CAPACITY: usize = 4096;
}
```

本候选保留 `4096`、三处类型级数组及所有运行时消费者、Shared 布局、根哨兵、cookie／代际、耗尽／祖先／取消／回收行为、lifetime、cfg、线程、预算和工具身份；没有改成 data、cfg 隐藏、external_body、手填宏属性或新增机制。其他 const、static、局部值和生成项没有足够的实际触发证据，本节不改。

需保留的行为包括 `process_ownership.rs` 的 `Tree::create`／`Launch::prepare` 成功根路径、容量耗尽错误、`cancelled`／`descendant` 的根哨兵和循环边界、`inherited` 的上下文范围拒绝，以及 `ExitObserver` 的 generation／cookie 代际检查。这些位置是当前实现，不是独立测试证据；本阶段未执行消费者行为验收，不为纯表示改动新增重复形状测试，原未过滤测试义务保留。FILEMAP 同步把该源登记为 `program`，真实根为 `crates/runner/src/lib.rs`，操作为 `investigate.ci-generation-dependency.runner`，并保留原 compile／judge-trigger 边。

这是产品源码与宿主登记同时变化的可见警告：产品侧只改变统一 const lowering 的表示，宿主侧补齐现有真实 runner 根登记；分类准入、严格 flags、操作参数和证明义务没有放宽。新增证明、编译、CI、存储和维护成本仍未测。

下一次原严格 runner 的可证伪验收必须沿原登记一次执行并取得同次完整 `success:true`、整 crate、无 panic／其他错误、rlib、dep-info、VIR、完整源码 checksum 及工具／宏／依赖绑定；任何同族下一成员 panic、后续解析／类型／证明错误、partial dep-info 或缺少任一绑定都继续拒绝。只有该 runner 合格后，才可按既有依赖派发 adapter 操作；raw 成功仍不替代 typed qualification、真实 CI prepare／generation／effect／recovery、正式 DELTA 或 integration／dev 落地。

本节未执行构建、Cargo、Verus、测试、格式化、bootstrap、check、Git／GitHub、生命周期或其他 worker；未修改工具、缓存、state、失败 `.d`、既有成功组件、JSON 合同或独立预期。剩余完整 SPEC、runner 同源证明和实际编译绑定、adapter／generation 传递依赖、typed qualification、正式 DELTA、integration／dev 交付与落地均保持未完成；全 goal 保持 active。阻断记录：trigger=`严格真实 runner 在 erasure 初始化前请求 CAPACITY 的 THIR`；trigger_actor=`固定 Verus f8e17045 查询执行程序`；trigger_path=`ordinary-operation`；mechanism_family=`verus-erasure-context-before-const-thir`；独立 recorded occurrence=`use-68541-1794643059119974000-0.json` 及其 stdout／stderr 原件。


## AP：runner 自属依赖生产候选与尚不支持的装载闭包

**产品与宿主政策同时变化，且均未执行验证。** `proved_build` 内新增专属依赖生产 helper，复用原执行器、双流／退出原件、原子发布及 check-original 保留；没有新执行器、等待器或保管总账。普通 Cargo `metadata --locked --filter-platform` 和 `build --lib --locked --profile=dev --target <登记原生target> --jobs 1` 使用 runner manifest／lock、自属 target、现役 bootstrap 工具绑定和已有 Cargo 源缓存。普通编译不以前置完整 runner 证明为条件；这次编译也不提供仓内源码证明豁免。

准备操作实际选择 Cargo resolve 根、完整库目标与原 JSON compiler-artifact，从同次 root dep-info 选择直接装载的编译实例，复用现役 checksum 判法绑定库、源码和传递二进制；记录宏／构建脚本产物及 Cargo 明确声明的唯一 static archive。原 metadata、Cargo stdout／stderr、实际 argv、工具／配置／环境摘要、产物和来源缺口归原构建负责方保存。缺文件、错 target、重复／缺根、缺成功终态、失败记录、未知形状、错 checksum 或多实例均拒绝；不扫描 target 找 hash。独立 JSON 接口样本是测试数据，不是生产成功回执。

runner-build 的四个当前消费者模板保留完整 strict flags、整 crate 根、lifetime、原 import／export／emit 和输出地址；只有 ci-tests 私有固定路径改成由同次 Cargo 生产记录解析的 extern 名字。generation consumer／missing 同步指向 AC 组件；工具宏仍由真实工具所有者负责，不把普通 Cargo 编译的宏当成同一实例。现役 projects 只保留各活动一个入口，正式 source-only／typed qualification guard 与 ProofAdmission 均未放宽；包装 action 及普通 Cargo 记录不能成为正式证明贡献。

| 负责位置 | 输入、输出、实际使用方与验收 |
| --- | --- |
| runner Cargo 项目 | manifest／lock、Cargo 实际 feature／profile／target 及源码；自属 target 中的实际编译库给原严格 runner／adapter／generation。原 `build.runner` 的 identifiers 证明和二进制合同保留。 |
| host-bootstrap-native `proved_build` | 已有绑定、profile、Cargo resolve／JSON／dep-info；生产原件、解析的 extern 和不完整项。新 Rust 选择／拒绝逻辑由专属未过滤测试消费；Cargo-Verus 整 crate 所有者及测试根另登记真实操作，两份 manifest 显式采用 `package.metadata.verus.verify=true`，不能只靠转发 flags 冒充执行了证明；这些根尚未证明。 |
| runner `verified_cargo` | 实际 Cargo 事件；流只允许一个根、成功终态最后出现。现有 Verus 根与配对的独立预期共同承担该新状态判断的证明，未运行。 |
| FILEMAP／projects／macOS 工具表 | 显式源、fixture、根、操作、前置、工具及输出占用。原测试、断言、线程及预算保留；新增检查的必要性终点是新解析／拒绝逻辑的实际消费者，不据目录推断选测。 |
| 原生命周期和 retention | target 仍为 finish-dispose，state 仍 evidence-retain；准备原件沿 check-original，保留 `runner-dependencies:<目录>` 使用方。调用方在最后实际消费或拒绝诊断结束后释放；2026-11-21 复核用途，不自动删除。 |

**本候选没有实现完整装载资格，当前严格消费明确拒绝。** 这不是已修复／可通过的准备入口：真实准备完成采集且 Cargo 退出 0 时，也只封存 `observed-incomplete` 并返回非零，原 Cargo 退出及双流不变；采集提前失败仍保留已产生的原操作和双流。fresh 的原 compiler producer、宏／build-script dep-info、原生编译源／工具调用、compiler backend／target libraries／linker／配置／SDK 仍缺可消费的完整绑定；现有 Cargo/rustc 入口摘要不足以填补它们。严格消费核原件及当前字节后保留具体错误，不运行尚无资格的 Verus，也没有可手填的 readiness 开关。这些关系仍须在原构建／资格所有者补齐，不能把本候选称为完整生产闭包。

AC 另有可复核的输入缺口：固定成功报告 `use-53527-1794633613411773000-0.json` 的实际 argv 消费旧 serde_json 实例，但 AC 的 41,485 字节 dep-info 只有源码关系，没有 serde_json 二进制输入／checksum 绑定。其旧 rlib 目前缺失；新 runner 同版本库不能证明可替代。候选固定原报告和原 dep-info 身份，从实际 producer argv 取输入关系；缺原字节、不同编译实例或无二进制绑定都拒绝。仍需要原 custodian 提供该实际输入实例及传递域的合格原生产关系，或者下一阶段决定产生绑定新当前实例的新组件；两者成本与可行性均 unknown。本阶段不为沿用历史 hash 建兼容分支，不重建成功组件。

独立预期已写入原 Rust 测试及 Verus 配对：成功选择实际 root／extern 与完整字节；缺终态、重复根／生产者、缺文件、源码漂移、越界 target、缺 native archive、错误／模糊实例必须失败；fresh 和 native archive 存在仍带来源 unknown。AC 的同字节但无二进制绑定、不同编译实例及原 extern 缺失均有独立拒绝预期；extern 展开须完整保留 strict／import／export；门后委托也有 Rust mock 独立核对完整 argv、原双流和零／非零退出，不真正调用 Verus。没有观察这些新增用例修改前失败，也没有 red／green 或运行通过声明。

调用方先核源码 diff／身份／whitespace，固定候选并封存。后续唯一生产入口仍是本树 `chrono-worktree use --operation <已登记ID> --path /Users/macstudio2/chrono-spec-current-dev`：首先 `prepare.ci-generation-dependency.runner`，由现役薄 launcher 的 frontend profile 从候选安装本树前端，再执行专属准备，不部署共享 owner；其后消费者直接复用同一本树前端和生产原件。当前准备预期非零且闭包不合格，故不能派发 runner／adapter。只有来源缺口补齐并另获阶段授权后，才按原 runner → adapter → generation consumer／missing 顺序执行，每项一次，前置失败停止明确后继。原 raw 调查预算仍为 2,400 秒／每流 67,108,864 字节、两线程；正式 native 配对原 900 秒／1 MiB 未抬高，新增全 crate 证明与 fixture 均未执行。Linux 仍缺 AC 组件采用及新 Cargo-Verus 工具登记，不能声称跨平台成功。

本阶段没有 Cargo、Verus、构建、测试、格式化检查、bootstrap、判官、业务 use、生命周期、Git／GitHub 写、共享部署或 worker。新增编译／证明／IO／CI／存储／维护成本均 unknown；复用现有缓存不是已测节省。全部自有源码命令均同步终结，没有后台任务；AN 历史 join 不追认。CAPACITY 家族仍无合格 runner 结果；完整仓内逻辑与独立预期证明、typed compiler／source／context／report／rlib／dep-info／VIR／macro／transitive 资格、真实 CI prepare／generation／effect／recovery、正式 DELTA／三评审、feature→integration→dev 两段交付与落地，以及完整 SPEC 其余目标仍未完成，goal 保持 active。


## AQ：当前依赖观察与正式资格分界

**产品和宿主政策同时修改。** runner 的实际 Cargo JSON／dep-info、根库发布别名、host 宏输入由原 `proved_build::dependencies` 负责；组件生产仍使用已校验 Verus 归档、同一外部 JSON 合同源码、现役 tar／patch／Verus 与原执行器。`runner-build.json` 只记录可移植配方；原 AC 报告、本机路径与一次运行的摘要不再充当跟踪政策。新组件另建受保护 state 槽位，原 AC／AE 不覆盖。当前组件原件由 `check-original` 的逐对象使用方保护，调用方在最后实际消费和诊断结束后释放，七日内复核；可重建 target 仍复用 finish-dispose。

prepare 的成功只表示已完成登记的依赖观察和当前输入组件生产；fresh、host 无 checksum、构建脚本、native、backend／SDK 等资格缺口如实留在 observation，不授予正式资格。四项登记 raw 调查消费同一原件及实际依赖字节，缺失、漂移、歧义或错绑继续失败。raw 不要求尚未实现的 typed 资格作为永久启动门；`verified_verifier` 的 source-only guard、`ProofAdmission`、整 crate、严格 flags、lifetime、全部仓内证明义务保留。正式 DELTA 仍会拒绝缺失资格，raw 退出 0 不能代替它。

新增承重逻辑和独立预期仍归原 Rust 源／配对测试及已登记的整 crate Verus 根，登记不等于已证明。适用入口为候选 bootstrap、正式 DELTA，以及现役 `chrono-worktree use` 的 prepare、raw 消费和独立验证操作；全部保留原线程、时限、输出限制及未过滤测试。新增构建、证明、IO、存储及维护成本 unknown；实际结果归 ignored 原件，不在本文手填通过。

## AR：恢复分类连续性并修复现役证明源码

**产品与宿主登记同时修改。** `verified_verifier` 修复证明表达式的 Vec 索引类型，以同值 UTF-8 字节表示当前 `dependency=` 前缀，并补齐参数、输入证据错误和 import 见证的证明；`verified_sources` 在原循环内证明 push 前后的已登记源码及普通文件模式保全。原执行判断、错误、source-only／typed `ProofAdmission` guard 和规格义务保留。配对独立预期保留全部原输入与行为断言，只补字面量、字节边界和递归语法的证明事实。

FILEMAP 精确恢复固定基线 `71d5e0d2fb624d83ee7530582bba8e55bb47bdd7` 中、当前路径仍存在的二十项 verification：十二项 program、八项 data。实际 canonical 随后采用 `e307951ea6ebe94c7f91300a315249079fccbe6b`，完整比较又发现 `pr-tests/tests/lifecycle.rs` 一项 program 分类缺失，故同时恢复其原证明义务；原根及操作不变。当前 snapshot 根未包含该 lifecycle 源，不将分类恢复称为源码覆盖，也不采用基线中按 ghost 条件排除其普通测试的做法。恢复现有义务不等于这些根已经成功证明，也不以空根或降为 data 绕过检查。两组变更分别由现役 Verus／Rust 操作和 canonical DELTA 验收；范围、原退出及双流由 AR 原件记录，普通测试或 raw 成功不授予正式资格。

工具域仍有实质缺口。固定 Verus `f8e17045c4ac17a02e566b8997070f2eecc255be` 的类型检查先于 erasure context 初始化；默认严格域中的普通及后续生成 const 会在早期 THIR 请求中命中原断言。既有宏的双表示只处理自身输入，现有 derive 处理也不能证明 serde／tracing 生成逻辑。此次有界源码审查尚未核实支持完整普通／生成域的统一修法，没有逐常量改写、工具 guard 绕过或重跑不变输入赌绿。native 的真实 derive 类型义务、runner JSON 适配器缺失的真实模型参数及其仓内解析逻辑证明，也仍由原所有者承担，不能假托外部 IO 合同。

原 runner／native 严格失败及其显式后继阻断继续保留；恢复条件是同域完整源码和生成项在原整 crate、lifetime、strict flags、两线程及预算下证明，并绑定同次实际编译产物。现有依赖 receipt、匹配组件、AC／AE 与必要失败原件继续由原保留负责方保护；不重建匹配组件、不重新下载全部工具、不清空 target。AR 诊断与验证原件供调用方和工程所有者使用，2026-11-21 复核用途，调试和验收义务结束且无使用方后再释放。

证明修复、未过滤行为测试、候选 bootstrap、正式 DELTA、原生 integration→dev 和落地分别核实；本文不把局部成功记作交付。新增验证／存储与完整生命周期成本仍 unknown，完整 SPEC goal 保持 active。
