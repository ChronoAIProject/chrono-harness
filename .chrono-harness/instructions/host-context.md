# chrono-harness 宿主上下文

本仓维护 Rust harness 产品与独立宿主采用数据。SPEC.md 是尚未全部实现的产品合同；当前生产者、调用与限制见 README.md、docs/spec-coverage.md 及对应专用文档。30 个独立 Cargo manifest/lock/target 构成生产／专属测试配对，没有根 workspace。

产品默认归 assets/instructions/catalog.json、default-manifest.json；本宿主独立拥有 .chrono-harness/instructions/catalog.json、manifest.json 和本上下文。产品资产是编译／测试输入，宿主数据及已存在投影是运行输入；FILEMAP 的 owner、输入、消费者与边仍显式登记。升级二进制或重复 init 不覆盖既有宿主选择；采用新默认须明确编辑宿主数据。

当前指令 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。115 个 atom（99 个双语内容叶子、16 个聚合）及原 requires 保留。产品与本宿主根明确选择 21 个叶子及 workflow 五节短流程；完整 core.general/general 仍可用于自定义输出。本宿主 general-en 输出仍含 99 个叶子，repair-skill 仍为原 7 叶子闭包。CLAUDE.md 是生成的中文根，AGENTS.md 是字面相对链接 CLAUDE.md；修改宿主源后使用登记入口，不手改投影：

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

现役 FILEMAP v2 是唯一执行计划源。routes 合并显式所选操作、核规范调用并绑定工具；projects 核独立配对、输出隔离与实际回执。scoped CI 复用该执行链，仍保留专用选择／环境边界。ci.verify 属于真实 ci/ci-tests 配对；有限历史转换及方法替换由登记的 candidate decoder 消费原字节，不执行旧判官。

本宿主采用 chrono-ci-check/v3 的 18 个显式单元，并在 units.json 显式采用 chrono-job-gating/v1：一个完整 Git DELTA detector、18 个 job-level if 独立单元和一个 always() aggregate，共用一份 parent workflow；只要求 aggregate 的 chrono / collection。当前宿主采用 config schema4；short check 固定读取 .chrono-harness/config.json 的 canonical_check.profile 和 inputs.local/ci action。候选仍须已提交、检出干净；CHRONO_CHECK_SOURCE 未设或 local 选择登记的 worktree 生产者，ci 选择登记的 CI 事件生产者，不猜范围或回退。启动构建 runner、judge-ci、ci 和本地生产者 worktree；业务构建仍归独立单元计划。当前入口：

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-harness check
.chrono-harness/bin/chrono-harness check --unit ID
.chrono-harness/bin/chrono-harness check --collect
```

本地固定取得登记 remote 的 workflow target 与 clean HEAD。单元、共享操作及报告位置归 .chrono-harness/ci/check.json，parent／job 与启动参数归 units.json；无值汇总沿同一 check 入口消费登记的 manifest，CI owner 从显式报告路径及独立执行物 pins 生产 manifest，不重执行业务。生成的 scoped CI check 步骤使用同一短命令，在命令内调用现役事件准备／原生报告收集，不增加手工 prepare。integration push 保留观察的 origin dev tip 规则，detector 一次固定给所有依赖 job；汇总核 exact parent/current attempt 与最新 job 状态；实际原始 producer／artifact attempt 归 generated production output，API carried attempt 不充当生产证据，支持成功前置不重跑。已退休的 generated workflows 保留历史语义，旧 provider 未显式采用 job_gating 时行为不变。本次只核本地源码／Git fixtures，尚无当前 topology 原生 Actions 或远端保护规则变更。现役 scoped 宿主采用未启用 proposed full 治理。full-v3/v4 独立单元短入口及七判官原始证据汇总已有源实现；chrono-github-units/v2 通过显式 full_contexts／上传映射连接现有自动事件及 gather。本宿主仍采用 scoped v1，未启用 full；full unscoped 可以从 start/reconstruct 自动发布的目的地 origin receipt 生产 schema2 context，无历史证据时具名失败。macOS 宿主显式绑定 /usr/bin/python3、Python 3.9.6；工具链版本及操作来自登记，不能以本地默认值替代。具体合同与既有原生证据可选择读取 docs/ci.md、docs/ci-units.md。

五份 full 治理登记仍 proposed，enforcement=not-implemented，input_closure=incomplete；Cargo/编译器/SDK、配置及其它适用输入、执行物绑定仍有未解决项。本宿主尚未采用 Cargo guarded policy；schema4 的 scoped/provider 绑定当前 /usr/bin/git 版本与字节，18 个单元及汇总 workflow 登记 runs_on=macos-26。开发工具目录未显式绑定；macos-26 原生 scoped 匹配已在合并的 PR104 验证，完整宿主启用与 full 原生 CI 仍未完成。chrono-inputs 只捕获声明文件／变量的两端快照，不补依赖、不改预期摘要、不启用治理。普通 full check 在登记要求及所选义务通过后表示 DELTA 满足现役登记合同；declared-complete 是 AI 对宿主范围负责的工程声明，现役 registration 仍报告 completeness_proven:false。已知缺输入、必需依赖未解决、缺绑定／快照和漂移仍须失败，不要求普遍隐藏输入证明或 VM。详见 docs/inputs.md、docs/cargo-projects.md、docs/git-facts.md。

本地/CI 同命令已经采用；普遍同判仍以完整相同有效输入和确定性求值为条件。独立 parity 比较器要求更强 completeness_proven:true 和完整观察，当前未建立；它不另加普通 full check 门，也不从成对读数证明普遍确定性。完整宿主启用、full 原生 CI、端到端交付及完整 SPEC 验收仍未完成。worktree 生产者支持显式创建／重建／恢复／清理，冲突协调及 PR／合并／落地生命周期归调用方，不能把阶段成功报成完成交付。

详细历史与限定仍在可选 docs/ 下，来源／许可资料不是必读政策。指令生成不证明 AI 遵守或翻译等价；成本仍 unmeasured。Git 生命周期按当前任务授权，不从本文推断提交、推送或启用门已获授权；不改全局配置、参考仓库或其它工作树，不把过程转录写入产品源。

本宿主现将同一个 judge-workflow-tests 项目显式绑定为 core、full_units、full_short 三个 test_groups／CI 单元；成员由当前 inventory 动态核定，原 test ID、生产／测试项目配对、未过滤 execute／release 操作、历史测试正文与限制保留。注册所有者统一解析组 ID、本项目 action 与终端操作；FILEMAP 完整登记各组边、计划、成本及共享前置。宿主 workflow-inventory guard 从真实未过滤／过滤 libtest 列表核非空、无忽略、互斥且全集相等；长期核当前集合，不冻结历史数量。当前变化同时含产品合同与宿主政策，验证成本包括受影响 owner 专属测试、三组完整执行及独立库存核验。原生时限充分性、发布平台／公开分发和 main/examples full 启用仍归调用方后续验证；本地通过不证明激活。

本宿主在 worktree.json 显式采用 cleanup.json 的 automatic_cleanup。协调锚明确选择现有 Git owner inventory 的 main worktree；state 固定在该存活宿主的 .chrono-harness/state/automatic-cleanup/，不扫描同级目录。当前政策逐项采用已登记输出目录，保留 bin 与 state，默认仅回收已 finish 且无 managed use 的缓存，未解决证据不删除 checkout；不允许 finish 自行处置证据。调用方负责加入后台任务与实际落地，并从协调宿主运行固定 `.chrono-harness/bin/chrono-worktree finish --path <工作树>`；独立重试为 `.chrono-harness/bin/chrono-worktree maintain`，消费登记操作为 `.chrono-harness/bin/chrono-worktree use --operation <操作> --path <工作树>`。start/reconstruct 已接入同一 drain 与新生登记，不是定时空闲清理。旧工作树只按明确 `import --path <工作树>` 迁移；旧配置缺 automatic_cleanup 仍 opt-out。源与宿主政策同改，验证成本为 worktree owner 全套真实 Git 用例、受影响登记及指令生成／专属验证；落地与真实宿主 finish 事件仍由调用方交付。

本宿主源码登记 automatic_cleanup/v2 与新的 .chrono-harness/state/automatic-cleanup-v2/，不重解释旧 v1 状态。短 check 和 bootstrap 显式委托 worktree 所有者，Python pass_fds 与嵌套 runner 复用原进程 engine。最后内核租约关闭后，普通入口可回收未 finish 的缓存，但不设 terminal 或宣称落地。源码与宿主政策同时修改；本地真实 Cargo test／原生子进程回归验证包装器终止后的租约保留，候选仍需已提交组合与支持平台验收后才能完整采用。兼容协调者二进制部署、旧生产者排除、已提交组合、支持平台与主宿主实际切换归调用方。

普通入口的清理拒绝按登记项隔离：无关项的原失败回执与 cleanup_failures 保留，合法当前目标仍执行；目标身份／政策／所有权及协调状态写入失败继续阻断。显式 maintain 对清理失败仍返回失败。check participation 转发原 stdout、stderr 与退出码；非空 stderr 不等于生命周期失败。

切换顺序归调用方：先在独立、干净且固定提交的候选 Git main clone 运行同一 bootstrap/check；当前 scoped 本地生产者 full_context=None，无需伪造 linked origin receipt。真实协调宿主先加入／排除不兼容生产者、保留 v1 状态、部署兼容 binary 与匹配且已提交的 v2 policy/config/participation，再通过普通 start/reconstruct 自动登记新 linked checkout，执行原 bootstrap/check。只升级 binary 或只升级 linked policy 均应保留并拒绝，不能放宽字节一致性。fixture 与本地结果不替代真实切换、原生及已提交 canonical 验收。产品源码与独立宿主政策同时受影响，成本含新行为回归、完整 worktree/CI、runner/instructions、格式与登记生成。
