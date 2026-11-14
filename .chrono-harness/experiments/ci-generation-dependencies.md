# CI generation 的依赖与 runner 适配器候选

本文服务 SPEC §2.1.1 的完整同源证明和实际编译绑定。runner 所有者负责真实借用 JSON、捕获 map 和 seen set；ci 所有者负责 generation 的真实生产闭包。当前交接是完整直接 adapter family 的源码候选，**不是严格证明、依赖准入、正式 DELTA 或交付结果**。全部 SPEC 目标仍未完成。

## 当前候选及边界

基线为干净 `ff006c4e08b7585ac484e583299d65a60d36c35e`。候选保留三处原源码所有者、真实调用、借用和顺序；不改 CAPACITY、lifetime、cfg、program 分类、错误优先级或生产选择。源码库存仍是下面七个具体实现块和 `verified_git_binding` 内两个既有泛型选择适配器，没有发现另外的直接实现或本地实现生成宏。这不是全部 proc-macro／cfg 宇宙的证明。

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

S 从干净 `986004dc222143d373babf2318907f32f1a9ce59`／tree `fafde2b6eaa6b2d018fae4044760eb96a20a4bb4` 准备本节候选，不重查 R 已解决的普通 proxy／import 类。目的仍是七个实际适配器及 generation 所需的真实外部 JSON／全文合同。**S 的初始登记不完整；T 在派发前被拒绝，U 的当前修正仍待封存和原生准入。没有构建或运行正反向输入，端到端能力为 ASSUMED-UNVERIFIED。** 不据此宣称严格成功、完整 family、正式 ProofAdmission 或交付。

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

U 只准备候选，未重跑原输入或任何 status／use、准备／证明操作。已有原生 CLI 的实际登记解析从固定提交读取，正式 check 还要求干净候选；本轮没有另造脏目录解析器，也不把 JSON 解码或源码核对称为原生登记通过。调用方先封存完整变更，再以 `.chrono-harness/bin/chrono-worktree use --operation investigate.verus-tool-contract.extract --path /Users/macstudio2/chrono-spec-current-dev` 核验新候选的实际准入；成功后才按上表逐项执行 patch、build、consume、reject，每项等待并核原结果，任一前置失败只停止其依赖项。旧 T 输入不重跑。

正式 DELTA 仍走 `.chrono-harness/bin/chrono-harness check`，所选证明和原未过滤测试由登记负责。当前正向 action 的 bash 转交、extern／新组件及缺少 typed 资格仍受现役工具合同限制；补入计划成员不使其自动满足 `root_planned` 或实际编译绑定，不能添加虚假 `output_contract`。候选原生登记／extract 准入尚未验收，后续任何前置或工具资格错误仍是失败。新增组带来额外正向验证义务；完整构建、运行、CI、存储和维护成本未知。

### 执行前固定的预期、范围与剩余义务

正向根必须整 crate 零退出、验证成功且实际编译出新库和 dep-info。其函数没有把实际外部调用换成仓内假实现：get 的完整成员关系、Index 缺键无文本、object 的真实成员数、String→str 借用和集合插入均由调用后的证明义务使用。边界分别要求：缺键得到 None；存在但非文本时 get 为 Some 而文本为 None；任意全文相等（包括不同存储、Unicode、NUL、换行）有相同成员关系；insert 的 fresh 与此前全文成员关系互补并保全实际旧 set。JSON 模型前提只是外部观察条件，不证明 JSON 构造或解析；原 `json_adapters.rs` 的实际空／null／错类型／前缀／组合 Unicode／重复插入预期继续保留，由后续原未过滤操作消费。

反向根必须在确实消费同一新库的前提下，因仓内 `assume` 被严格检查拒绝；缺文件、类型错误、其他前置失败、无输出或超时均不是这一预期的通过证据。反向原非零结果继续是失败原件，不安装“失败就成功”的包装。当前组件尚未构建，因此两个结果均未知；本轮真实工具探针数为零。准备后由调用方封存并另行委派，不运行 canonical／bootstrap，也不把上述待执行项记成已完成。

本候选只试验一个真实工具库载体：源码层面的 family 依据是所有所需声明由真实 vstd 所有，消费者仍走未放宽的严格检查；它**尚未**提供全 family 的实测成功。可证伪条件是：新组件完整准备成功后，实际使用这些合同的正向严格根仍拒绝或无法证明关系，或相同严格路径不再拒绝仓内 assume。失败须先读原输入／stdout／stderr／退出，不重跑 R 的普通 import 拒绝，也不逐项修改仓内编译参数来伪造覆盖。

真实 serde_json／标准文本模型仍是逐项列出的外部假设，不是已机械精化的库实现。正向 raw 成功也不证明全部 runner／adapter-tests／generation、完整生产编译绑定或 ProofAdmission。后续仍须在原所有者完成唯一合同的实际消费迁移、68 个受影响未分类文件与六个旧整文件映射、完整 producer／argv／cwd／env／rlib／dep-info／VIR／宏及必要传递输入资格、generation 四生产投影与 prepare／effect／recovery、正式 DELTA、integration／dev 和实际落地。

产物沿现有 state 的 evidence-retain 与 check-original 所有者保管；不新增保留生产者、总账或清理器。使用方是工具合同工程、runner 资格工程、调用方与 mandatory review。原失败保留到调试／验收义务完成；新组件及原件在这些使用方结束、已保全必要证据且无活使用方后由调用方通过现有生命周期／保留所有者回收，2026-11-21 复核用途，不因日期自行删除。新增运行总成本未知；本轮只量取上表输入字节，没有用预算、重复运行或降低目标换取成功。

R 的普通 proxy／import 源码调查没有待执行的新登记操作；S 的五项真实组件操作仍按上表前置关系等待执行。验证总成本未知。全部受影响源码的实质证明、generation 四个真实投影与 prepare／effect／recovery、正式 DELTA、原生 integration／dev、平台与生命周期义务继续未完成。保留使用方和复核条件沿用本文既有声明；本节不新增运行账本、执行器、缓存或永久证据副本。
