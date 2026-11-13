# CI generation 的有限依赖调查

本包服务 SPEC §2.1.1 的完整同源证明与实际编译绑定。使用方是本次有限调查的下一位工程工作者及包结束后的一次 sshx review triplet。当前仍未建立全类覆盖依据；本包准备登记，不提供证明成功、生产准入或交付结论。

## 所有者与唯一入口

操作及完整参数只由 [projects.json](../projects.json) 维护；runner 所有者负责真实依赖导出，ci 所有者负责真实 generation 导入和缺失证明输入。FILEMAP 仅增加本文的数据分类和三个操作的已有 state 输出占用，没有把实验加入正式执行计划。没有新增程序、执行器、工具、生产合同、证明豁免或 CI 任务。

下一 assignment 只通过以下受保护入口逐项执行，不能直接启动 Verus、使用未提交参数或换用其他操作：

```sh
.chrono-harness/bin/chrono-worktree use --operation <下表操作ID> --path /Users/macstudio2/chrono-spec-current-dev
```

从该工作树根运行。调用方须先保存本包的固定提交并记录 OID，工作树干净；源码与基线 `04cab6acace80dbce489c31e00dc9a505fdf37fb` 相同，仅有本包登记和说明增量。登记解析取目标 HEAD 的 Git 字节，未提交新增 action 不可用。本轮不 commit，也不运行入口。调用方保管既有登记、租约及结果交接，不要求重新部署、迁移、finish 或 maintain 共享协调者。

源码依据是 `crates/worktree/src/start.rs::registrations`、`automatic.rs::registered_command/action_command/program_command`：`use` 精确选择唯一 project/script action，将目标作为 cwd，使用目标配置中登记的工具及环境；它保留原进程，不调用生产 verifier 的扩展参数准入。`target_policy_inputs` 仅核 worktree 配置和 cleanup 政策；本包不改两者。实际生命周期状态可能另有阻断，未通过本包验证。普通 `use` 会写已有协调状态，须由具有既有生命周期权限的调用方安排，不能把本 assignment 的协调宿主只读限制当成绕行理由。

## 固定输入与执行前提

工具沿用已安装的 `0.2026.04.24.f8e1704`，commit `f8e17045c4ac17a02e566b8997070f2eecc255be`，Rust `1.95.0-aarch64-apple-darwin`。不 bootstrap、下载、Cargo 构建或运行 cargo-verus。三个 action 都是一次直接 Verus 调用，保持 `--no-cheating`、`--no-external-by-default`、整 crate、原两线程、JSON 报告、编译和 SHA-256 dep-info；不增加 rlimit 或超时预算。`use` 的实际限制来自所选配置的 `protocol`，当前 macOS 为 2400 秒、每流 67108864 字节，不取正式 test 计划的 900 秒／1048576 字节，也不把这项既有区别说成调高限制。

runner 根为真实 `crates/runner/src/lib.rs`；consumer 根为真实 `crates/ci/src/verified_generation.rs`，仍无条件包含 `generation.rs`。不改源码、分类、Serde 派生的现有 cfg、生产入口或具体回调。consumer 根不能代表 `lib.rs` 的四个实际 prepare 分支；后续完整生产闭包仍需验收。

两根使用 manifest 的 edition 2024；runner 的 `env!("CARGO_PKG_VERSION")` 按当前 manifest 固定为 `0.1.0`，通过 rustc `--env-set` 传入，不修改环境政策。Rust 1.95 随附 `unstable-book/compiler-flags/env-set.html` 说明该值进入 dep-info；其在固定 Verus 转发链上的实际支持仍为 **ASSUMED-UNVERIFIED**。不支持时保留原诊断，不能临时加第二个执行器或原地重跑修配方。

runner 的普通依赖选择来自现有 `crates/ci-tests/target/debug/.fingerprint/chrono-harness-1b904f53053eaf74/lib-chrono_harness.json` 的十二个依赖指纹逐项对应，避免混用另一个 profile。直接宏输入选固定 Verus 发行物的 `libverus_builtin_macros.dylib`；普通缓存库仍可能关联原 Cargo 宏。宏、工具链和库的实际兼容性未知，不能从同名或相近版本推导。Serde 缓存已启用 derive／rc，serde_json 缓存已启用 float_roundtrip／raw_value／unbounded_depth；这些是实际诊断输入，不能冒称仅有 ci manifest 的最小 feature 集。

下表记录准备时读取的直接输入。执行者在进入入口前核对它们及固定 HEAD。源码以固定提交和干净树绑定；成功编译后另核 dep-info 中全部实际源、include 数据、宏及库。`-L dependency=crates/ci-tests/target/debug/deps` 还会解析传递缓存输入，其完整身份和 Rust sysroot／SDK 闭包当前未知。缓存输入缺失、变化或编译器报告不兼容时不重建、不换库、不重试相同失败，保留具体缺口交回有限调查。下表的摘要只固定字节，均不授予证明资格。

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

## 三个候选及预先承诺的判法

每项最多执行一次，总额不超过三个不同固定工具候选；必要行为操作额度一项继续保留，本包未选择或启动行为操作，不能因此重跑 272 项测试。三个输出分别使用 `libci-generation-dependency-20261114-runner`、`-consumer`、`-missing` 前缀，位于已有 `.chrono-harness/state/`，各有 `.rlib`、`.rlib.d`、`.vir`。另有刻意不存在的输入 `libci-generation-dependency-20261114-absent.vir`。

执行前用文件系统观察核对这十个确切地址都不存在，保留观察。任何地址已有独有工作就停止依赖操作，不覆盖或删除、不换名冒充新候选。缺失输入必须保持缺失；不删除成功 VIR 制造负例。三个 action 参数的输出名称不同只用于保护各自原件。

| 顺序／操作 ID | 真实输入和差异 | 预期、可证伪结果与依赖条件 |
| --- | --- | --- |
| 1 `investigate.ci-generation-dependency.runner` | 真实 runner 整 crate，加精确十二项 extern 和固定 crate 名；同次严格验证、编译、export。其内部 identifiers／diagnostics 普通库没有现成完整证明资格，本包没有伪造 import。 | 观察原编译／Verus 对真实依赖及仓内适配逻辑的处理。预期可能出现实际语法、类型、宏、库兼容或未证明逻辑错误，不预填具体错误码。必须保留原错误所在源码及输入。若原进程非零、失败、流不完整，或报告非严格整 crate 成功，停止 2／3；失败导出即使存在也不能继续当作成功依赖。 |
| 2 `investigate.ci-generation-dependency.consumer` | 同一真实 generation 根，serde／serde_json 精确普通库；runner 的 extern 和 import 同时指向候选 1 本次产生的库与 VIR，不使用普通缓存 runner。 | 仅在 1 的原严格成功报告、编译库、dep-info、VIR 和身份都可读取并绑定到本次调用后执行。此条件只允许诊断消费，未赋予传递依赖完整证明资格。预期检验真实 import／extern 对应，并可能暴露 Object／Target／Plan 的 Serde、std、闭包或具体回调缺口；实际错误保持原样。仅成功解析文件或编译不算完整生产证明。 |
| 3 `investigate.ci-generation-dependency.missing` | 与 2 相同的根、库、工具和输入，只把 runner import 改为上述不存在的 VIR 地址，隔离输出；仍保留对应的 runner extern。 | 依赖 1 完整结束且 2 已结束，输入和库均未漂移。预期原进程非零并明确拒绝缺失证明输入。若 2 本身未到达有效导入／验证，3 的失败只能算缺失输入诊断，不能证明正常 consumer 的资格或失配拒绝。若 3 退出 0／报告成功，原样记录工具接受缺失输入这一反例，不在包装层伪造拒绝。 |

1 失败时不运行 2／3，不偷换为普通 runner 库以完成计数；2 失败时可以运行具备上述输入条件的 3，但不能把其他编译错误当作缺失 VIR 的拒绝证据。未消耗的额度不是追加第四个候选、重复调用或抬预算的授权。实际不支持的参数也是本轮原失败，不另开帮助或版本探测来规避次数。固定 import／export 语法的静态依据是此前保存的固定工具帮助原件 `/tmp/chrono-generation-verus-bound-help-20261113.stderr`，它不证明行为。

## 输入、输出和退出合同

stdin 沿现役 `use` 不提供业务输入；输入是 action 原 argv、目标 cwd、所选配置的有效环境、固定源码与工具和上述库。配置环境继承项可能含工具链／SDK 选择，下一执行者须从原进程核有效输入，不能仅核命令文本。没有增加或读取新权限。外部接口仅是现役文件／进程／Verus 工具边界；没有将 `decode`、路径判断、retention、发布、具体回调等仓内函数登记为外部合同。

stdout／stderr／退出码、signal、超时、失败原因、进程及工具身份、原流完整性和子孙加入证据，以 `use` 的 `managed_process`、`managed_process_original`、原回执及引用原件为准。原 stdout 的 JSON 应报告固定版本／commit、整 crate、success、encountered-error／vir-error、verified／errors；不能手填成功。编译和 export 可能先于最终验证成功，任何单独文件均无资格。读取 `.rlib.d`、库与 VIR 的实际字节、摘要、长度及生产调用对应；没有 dep-info 或源输入无法闭合时保持 unknown。

三个实验 action **没有** `output_contract`，未被登记为生产 proof root，也未加入正式测试计划：现役生产 `verified_verifier` 仍只接受原十二项形状。实验原报告只能供工程调查和后续所有者实现消费，不能作为正式 DELTA 的证明贡献。这里不添加一个宽松验证器、成功转换器或新的准入规则。

原 Verus 非零就是失败观察；现役生命周期包装会报告 `managed_command_failed` 并非零返回，不保证等于子进程退出码。包装 0 仅表示受保护调用正常完成，仍须读取子进程原报告；生命周期或 IO 错误不冒称 Verus 失败。工具未启动、无原结果、通道丢失或超时均保持未知，不重启任务。使用唯一原生等待通道，等待自有子孙后一次收取原结果，不另建轮询器。

完整依赖资格还要求 identifiers、diagnostics 及全部仓内传递逻辑的严格成功证明、源码／库／证明身份和真实生产调用闭合。本包仅把它们作为未取得资格的现役诊断输入；即使 1／2 成功、3 拒绝，也不能断言这些条件满足，更不能把仓内程序作为外部合同。缺失输入拒绝只覆盖本负例，不等于内容错配拒绝、全部编译输入完整或全类覆盖。后续若需要内容错配探测，必须在已有总额度内重新取得明确的不同输入登记；本包不预造第二个依赖或变体。

## 保留、交接与退出

本包不是新产品范围。源码和登记由 ci／runner 现有所有者保管；原进程由现役生命周期原件保管；调用方保存本有限包的实际摘要、结果引用和未知项，交给 B 结束后的同一次 mandatory review triplet。A 登记准备结束不是正式评审点，不在此启动其他工作者。

容量是这一次有限调查的三个调用及至多九个编译／证明产物；无循环或定期生产者。总字节与验证耗时未测，不能宣称节省。已有 `.chrono-harness/state/` 为 evidence-retain，缓存为 finish-dispose；本包不改 retention／cleanup，不把年龄当删除权。调用方在保留原有失败的同时，保护本次实际原件及其引用输入，直到 B 和后续评审／修复已消费；2026-11-21 复核保留需求，复核日期不自动授权删除。

评审不再使用本包且失败已转为可复用判据或必要回归后，现有所有者可在核对活使用方、证据引用和保留义务后删除三个临时 action、对应占用、本文及其登记。原产物另按现役所有者和同样的保留条件回收，不删除源码或普通可复用缓存，不改已提交的七文件行为内容。未满足退出条件就保持有限调查未完成；不新增总账或清理器。

调用方的 Git 交接清单只有 `.chrono-harness/projects.json`、`.chrono-harness/FILEMAP.json` 和本文。提交前核净增量，尤其原七文件中的 FILEMAP 只追加本包记录；其余六文件必须逐字节等于固定基线。本轮没有运行机器判官、Verus、构建、行为测试、正式 DELTA、integration／dev 或落地检查；这些未执行项不写成通过。全 SPEC goal 继续 active。
