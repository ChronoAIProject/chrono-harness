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
