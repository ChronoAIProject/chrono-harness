# chrono-harness

用 Rust 构建的、以显式登记和 DELTA 判官为核心的 harness。先确定合同，再实现运行时。

**当前阶段：规格与可编译基础。判官尚未实现，草案登记表尚未生效。**
完整中文合同、数据结构、协议与验收条件见 [SPEC.md](SPEC.md)。
宿主约束草案仅放在 [.chrono-harness](.chrono-harness/config.json)；产品代码在 `runner/`，
唯一配对测试项目在 `runner-tests/`。两个项目各有 manifest、lockfile 和独立构建目录，没有根 workspace。

```sh
cargo build --locked --manifest-path runner/Cargo.toml
cargo test --locked --manifest-path runner-tests/Cargo.toml
./runner/target/debug/chrono-harness --help
./runner/target/debug/chrono-harness spec status
```

目前只实现帮助、版本和规格状态，以及公开的 `chrono_harness::dispatch` 调用边界。
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
