# chrono-harness

用 Rust 构建的、以显式登记和 DELTA 判官为核心的 harness，附独立的宿主指令生成器。

**已实现 AGENTS.md / CLAUDE.md 生成；通用 check 与判官尚未实现，五份判官登记表仍为 proposed。**
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

可执行文件编译时内嵌精选的[通用软件/AI 工作核心](assets/methodology.md)，运行时不需要 checkout。
默认产生普通 `CLAUDE.md`，其受管块直接包含完整核心；`AGENTS.md -> CLAUDE.md` 是真实的字面相对符号链接。
新宿主同时获得 `.chrono-harness/instructions/` 下的方法编辑源、空上下文和专用 manifest，无需手工注入。
可独立选择 `--methodology M`、`--host-context C` 覆盖默认输入，精确保留选定 UTF-8 字节。
根块外文字属于宿主。两个现有普通根文件仅在渲染后的完整字节相同时自动转换；不同原文报无写入冲突，
授权 AI 保全并明确整合后重试。仅一个普通根文件时保留其文字，转成同一正文+链接布局。

日常编辑宿主方法源，再运行 `chrono-instructions generate --host-root "/path/to/existing-host"`。
init 和 generate 都支持从已知 `read-both/v1` 登记前向升级布局；保留已注册方法和上下文，不自动采用新默认。
重复 init 省略选项保留定制，显式不同输入拒绝；采用新版核心须明确比较、编辑宿主源再生成。
当前身份为 `literal-core/relative-alias/v2`，旧二进制不支持它。重复无变化时无写入，已有正确链接不重建。

支持 Unix；已验证当前 macOS，其他平台未验证，非 Unix 明确失败。退出码 0 完整、2 用法错误、1 生成/IO 失败。
普通 IO 失败尝试恢复原文件/模式或移除新链接，恢复失败报告路径和可用备份；无崩溃原子性或并发写者保证。
完整所有权、版本、冲突、字节和恢复边界见[生成合同](docs/instructions.md)。
[可选来源说明](docs/methodology-extraction.md) 说明实际通用规则的选择与研究材料的排除，不宣称全条款迁移。
本仓通过相同工具生成根正文和链接；自举不证明 AI 遵守，也不启用判官。

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
