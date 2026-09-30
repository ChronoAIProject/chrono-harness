# chrono-harness 宿主上下文

本仓维护 Rust harness 产品与独立宿主采用数据。SPEC.md 是尚未全部实现的产品合同；当前生产者、调用与限制见 README.md、docs/spec-coverage.md 及对应专用文档。30 个独立 Cargo manifest/lock/target 构成生产／专属测试配对，没有根 workspace。

产品默认归 assets/instructions/catalog.json、default-manifest.json；本宿主独立拥有 .chrono-harness/instructions/catalog.json、manifest.json 和本上下文。产品资产是编译／测试输入，宿主数据及已存在投影是运行输入；FILEMAP 的 owner、输入、消费者与边仍显式登记。升级二进制或重复 init 不覆盖既有宿主选择；采用新默认须明确编辑宿主数据。

当前指令 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。114 个 atom（98 个双语内容叶子、16 个聚合）及原 requires 保留。产品与本宿主根明确选择 20 个叶子及 workflow 五节短流程；完整 core.general/general 仍可用于自定义输出。本宿主 general-en 输出仍含 98 个叶子，repair-skill 仍为原 7 叶子闭包。CLAUDE.md 是生成的中文根，AGENTS.md 是字面相对链接 CLAUDE.md；修改宿主源后使用登记入口，不手改投影：

```sh
cargo run --locked --manifest-path crates/instructions/Cargo.toml --bin chrono-instructions -- generate --host-root .
```

现役 FILEMAP v2 是唯一执行计划源。routes 合并显式所选操作、核规范调用并绑定工具；projects 核独立配对、输出隔离与实际回执。scoped CI 复用该执行链，仍保留专用选择／环境边界。ci.verify 属于真实 ci/ci-tests 配对；有限历史转换及方法替换由登记的 candidate decoder 消费原字节，不执行旧判官。

本宿主采用 chrono-ci-check/v3 的 16 个显式单元与对应 workflow／汇总入口。启动核心只构建 runner、judge-ci、ci；其余构建归所选完整计划。当前启动与 scoped 检查入口如下，固定完整 OID 且候选须已提交、检出干净；本地与 CI 使用相同命令，各单元仅按登记追加同一 --unit ID：

```sh
/usr/bin/python3 .chrono-harness/ci/bootstrap.py . .chrono-harness/ci/bootstrap-core.json
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json --base FULL_BASE_OID --candidate FULL_CANDIDATE_OID
```

单元、共享操作及报告位置归 .chrono-harness/ci/check.json，workflow 与启动参数归 units.json；汇总沿同一 check 入口消费显式 --collect MANIFEST，不重执行业务。integration push 对照所观察的 origin dev tip。macOS 宿主显式绑定 /usr/bin/python3、Python 3.9.6；工具链版本及操作来自登记，不能以本地默认值替代。具体合同与既有原生证据可选择读取 docs/ci.md、docs/ci-units.md。

五份 full 治理登记仍 proposed，enforcement=not-implemented，input_closure=incomplete；Cargo/编译器/SDK、配置及其它适用输入、执行物绑定仍有未解决项。本宿主尚未采用 Cargo guarded policy 或 scoped/provider 的显式 Git 绑定。chrono-inputs 只捕获声明文件／变量的两端快照，不补依赖、不改预期摘要、不启用治理。普通 full check 在登记要求及所选义务通过后表示 DELTA 满足现役登记合同；declared-complete 是 AI 对宿主范围负责的工程声明，现役 registration 仍报告 completeness_proven:false。已知缺输入、必需依赖未解决、缺绑定／快照和漂移仍须失败，不要求普遍隐藏输入证明或 VM。详见 docs/inputs.md、docs/cargo-projects.md、docs/git-facts.md。

本地/CI 同命令已经采用；普遍同判仍以完整相同有效输入和确定性求值为条件。独立 parity 比较器要求更强 completeness_proven:true 和完整观察，当前未建立；它不另加普通 full check 门，也不从成对读数证明普遍确定性。完整宿主启用、full 原生 CI、端到端交付及完整 SPEC 验收仍未完成。worktree 生产者支持显式创建／重建／恢复／清理，冲突协调及 PR／合并／落地生命周期归调用方，不能把阶段成功报成完成交付。

详细历史与限定仍在可选 docs/ 下，来源／许可资料不是必读政策。指令生成不证明 AI 遵守或翻译等价；成本仍 unmeasured。Git 生命周期按当前任务授权，不从本文推断提交、推送或启用门已获授权；不改全局配置、参考仓库或其它工作树，不把过程转录写入产品源。
