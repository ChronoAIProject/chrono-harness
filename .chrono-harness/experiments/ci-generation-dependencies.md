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

现有阻断曾是 `verified_verifier` 只准入原十二项参数形状。E 现在由 runner 所有者把带 edition、extern、import/export 的完整调用按固定选项语法投影回同一十二项语义合同；未知、缺值或冲突参数仍拒绝。该投影只闭合方法、源根、输出和 dep-info 的一致解释，外部库／VIR 的完整同源证明仍是后续义务，不能把 raw use 的成功报告冒充正式证明贡献。

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
