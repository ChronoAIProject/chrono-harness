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

可执行文件在编译时内嵌完整 `assets/methodology.md`，运行时不需要源码 checkout。
新宿主默认获得完整指南和空上下文，无需准备输入文件或手工注入 Markdown。
可独立选择 `--methodology M`、`--host-context C` 覆盖任一默认输入；外部输入须是可读 UTF-8 普通文件。
init 精确保留选定字节到宿主 `.chrono-harness/instructions/`，
两个根入口都要求全文读取同一方法与宿主上下文；只替换自己拥有的块，保留块外内容。
日常更新直接编辑保留源，再运行 `chrono-instructions generate --host-root "/path/to/existing-host"`。
已登记宿主再次 init 时，省略的选项保留现有源，不与新二进制默认内容比较；显式不同的输入仍报错。
升级二进制不自动更新宿主指南；采用新版须有意识地比较、编辑 canonical 方法，再 generate。
成功重复调用无写入。退出码为 0 完整、2 用法错误、1 生成/IO 失败。
普通 IO 失败尝试回滚；不提供多文件崩溃原子性，不支持并发写者。
完整格式、所有权、别名、恢复边界与验证命令见 [生成合同](docs/instructions.md)。
实际 CLAUDE.md 条款逐节迁移的完整中文正文见 [产品资产](assets/methodology.md)，
102 个来源子节的保留、适配和排除决定见 [来源映射](docs/methodology-extraction.md)。
本仓也由该工具生成根入口；这是能力示例，不证明 AI 实际阅读/遵守，也不启用判官。

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
