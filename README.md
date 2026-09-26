# chrono-harness

用 Rust 构建的、以显式登记和 DELTA 判官为核心的 harness，附独立的宿主指令生成器。

**已实现原子规则与多语言组合，生成根指南、Markdown 和 skills；通用 check 与判官尚未实现，五份通用登记表仍为 proposed。**
完整中文合同、数据结构、协议与验收条件见 [SPEC.md](SPEC.md)。
宿主约束草案仅放在 [.chrono-harness](.chrono-harness/config.json)；产品代码在 `runner/` 与
`instructions/`，分别有专属的 `runner-tests/` 与 `instructions-tests/`。
每个项目都有独立 manifest、lockfile、target，没有根 workspace，也没有 generator → runner 依赖。

```sh
# 在本项目 checkout 中安装；确保 Cargo 的 bin 目录在 PATH 中
cargo install --locked --path instructions
# 安装后从任意工作目录初始化已有宿主，只需这一条命令
chrono-instructions init --host-root "/path/to/existing-host"
```

可执行文件内嵌[通用规则 catalog](assets/instructions/catalog.json) 与 root-only 默认 manifest，复制二进制后也无需 checkout。17 条通用规则具有稳定 ID、中文/英文 variant 和显式依赖；宿主 `.chrono-harness/instructions/` 采用独立 catalog、manifest 与空上下文。

默认产生含完整中文核心的普通 `CLAUDE.md`，`AGENTS.md -> CLAUDE.md` 是字面相对链接。新宿主可用 `--locale en` 绑定英文；`--methodology M` 保留自定义 UTF-8 方法为 opaque file atom，未声明语言时为 und；`--host-context C` 独立指定上下文。

日常编辑宿主 catalog 与输出计划，再运行 `chrono-instructions generate --host-root H`。计划可引用共享原子，选择语言并生成任意登记 Markdown 或聚焦 skill。依赖先于使用者、共享原子每输出仅一次；缺失选中翻译、循环或无效引用均明确失败，无自动翻译或 fallback。可复制 schema 与组合配方见[生成合同](docs/instructions.md)。

当前身份为 schema 2 / `atomic-rules/relative-alias/v3`。已知 read-both/v1 与 literal-core/relative-alias/v2 自动前向迁移，保留旧方法/上下文精确字节、路径与权限，不拆 prose 或改为默认。重复 init 省略选项保留采用数据，显式不同输入或 locale 拒绝；原子组合不能被 raw method 覆盖。

根块外原文属于宿主；两普通根只在渲染后整份字节相同才转换，有差异明确报错供授权 AI 保全并整合。额外输出为有身份标记的整文件投影，拒绝覆盖未声明所有权的现有文件。删除或改名输出条目保留旧文件；旧 skill 须明确退休，否则仍可能被发现。无变化时无写入，已有有效根链接不重建。

支持 Unix，行为验证限定当前 macOS。普通 IO 失败尝试回滚并报告未恢复路径/备份；无崩溃原子性或并发写者保证。退出码 0 完整、2 用法错误、1 生成/IO 失败。
[来源说明](docs/methodology-extraction.md) 定义通用规则与译文来源边界。本仓通过同一工具生成根中文、[英文方法](docs/generated/general-methods.en.md) 与[重复故障诊断 skill](skills/diagnose-recurring-failures/SKILL.md)；默认新宿主只生成根指南。生成不证明 AI 遵守、翻译语义等价或判官已执行。

```sh
cargo build --locked --manifest-path runner/Cargo.toml
cargo test --locked --manifest-path runner-tests/Cargo.toml
./runner/target/debug/chrono-harness --help
./runner/target/debug/chrono-harness spec status
```

runner 目前只实现帮助、版本和规格状态，以及公开的 `chrono_harness::dispatch` 调用边界。
`check` 返回退出码 **3**，不执行判官、不输出通过报告；未知命令或多余参数返回 **2**。
Cargo 测试通过只证明这个 CLI 基础的行为，不能表示宿主约束已经通过。

```sh
cargo fmt --check --manifest-path runner/Cargo.toml
cargo fmt --check --manifest-path runner-tests/Cargo.toml
cargo check --locked --manifest-path runner/Cargo.toml
cargo build --tests --locked --manifest-path runner-tests/Cargo.toml
cargo check --tests --locked --manifest-path runner-tests/Cargo.toml
```

计划中的本地/CI 唯一检查指令与启用条件见 SPEC §4、§12；当前没有声称生效的 CI gate。
DELTA 充分选测依赖实际输入完整登记和判定局部性；本地/CI 裁决相同还需完整有效输入相同及确定性求值。
当前工具链与环境输入闭包未填齐，登记表保持 proposed；七个默认判官计划各自拥有独立生产/测试项目。
v1 仅接受干净的不可变提交快照；暂存、未暂存或未跟踪非生成物变化将返回 E_SNAPSHOT_DIRTY。
无前序提交的根提交是显式 bootstrap，不能伪造 DELTA 成功。
远端仓库、提交、独立评审和 `dev` 默认分支由仓库创建流程完成。
