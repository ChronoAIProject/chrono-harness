# chrono-harness 宿主上下文

本仓维护显式登记的 Rust harness 与独立宿主指令生成器。产品合同在 SPEC.md，当前行为与调用在 README.md、docs/instructions.md。

生产/专属测试配对：runner ↔ runner-tests，instructions ↔ instructions-tests。各自有 Cargo.toml、Cargo.lock、target，无根 workspace；instructions 不依赖 runner。构建/检查使用显式 manifest 与 --locked；仅测试项目的 build/check 带 --tests。操作、所有权、依赖和未知成本分别登记在 .chrono-harness/projects.json、FILEMAP.json；变更文件需同步精确登记。

通用判官、DELTA、调度与 CI gate 尚未实现。五份通用登记仍 proposed；chrono-harness check 返回 3，不产生验证通过报告。chrono-instructions 的专用 manifest 校验和生成已实现，两者不可混报。

产品资产位于 assets/，宿主 canonical 方法与上下文位于本目录。根 CLAUDE.md 的受管块直接呈现完整通用核心，AGENTS.md 是字面相对链接 CLAUDE.md；编辑本目录方法源后运行 chrono-instructions generate --host-root <本仓根>。不独立编辑根受管块，也不必重复读取方法源。采用新版产品方法须先明确比较并更新本目录方法，不能用 init 覆盖定制。

从 checkout 用 cargo install --locked --path instructions 安装生成器。新宿主只需 chrono-instructions init --host-root <已有宿主根>，默认写入编译时内嵌的精选通用核心 assets/methodology.md 和空上下文，运行时不读源码资产或推断项目事实。--methodology 与 --host-context 可独立显式覆盖；已登记宿主省略选项保留 canonical 字节，显式不同输入拒绝。本仓登记的 init 只传 --host-root .，在定制后仍可使用。专用登记现为 literal-core/relative-alias/v2，旧 read-both/v1 只允许前向升级布局，不自动替换已采用的方法；本仓明确采用新版产品核心后由同一工具生成正文与链接。方法与根框架资产的生产编译边以及两份资产的测试输入边在 FILEMAP 登记。

按任务目标自主实施、验证与修复，保持独立项目/专属测试和真实退出状态。分支约定与计划中的 integration 策略见 SPEC.md；Git 生命周期遵循当前任务授权，不从本文推断已启用机器门。不要改全局配置或参考仓库，不将过程转录写入本仓。
