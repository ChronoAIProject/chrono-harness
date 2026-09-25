# 宿主指令生成合同 v1（已实现）

`chrono-instructions` 是独立的 Rust 库与薄 CLI；不依赖 runner，不执行判官。
生产和专属测试项目分别为 `instructions/`、`instructions-tests/`，各自拥有 manifest、
lockfile、target。JSON 使用 serde/serde_json 的严格结构解析；同目录临时文件使用 tempfile。
默认生产构建没有故障注入入口，配对测试通过 `test-support` feature 启用有界注入。

## 调用与输入

```sh
cargo build --locked --manifest-path instructions/Cargo.toml
./instructions/target/debug/chrono-instructions init \
  --host-root "/path/to/host" \
  --methodology "/path/to/chrono-harness/assets/methodology.md" \
  --host-context "/path/to/selected-context.md"
./instructions/target/debug/chrono-instructions generate --host-root "/path/to/host"
```

三个路径均是显式参数，支持空格；相对参数只相对于调用时工作目录解释。
宿主必须是已有目录，不要求 Git 仓库。传入根先解析为实际目录，生成路径始终相对于该根。
不自动寻找仓库、不探测语言、工具、依赖或用户配置，不在生成时用 AI 推断项目事实。
两个外部输入必须是可读的 UTF-8 普通文件，可通过调用者明确选定的文件链接读取。
宿主上下文可显式为空文件；方法也不由程序判定语义质量。输入字节（包括换行和 BOM）原样保留。
宿主 AI 可以自主撰写和演进上下文与方法；内容判断不冒充生成器已做的检查。

选项顺序可交换，每个必需选项恰好一次；未知、重复、多余或缺失参数是 usage 错误。
路径作为 OS 参数运输，不经 shell 插值。没有隐式环境变量开关。

| 退出码 | 含义 |
| --- | --- |
| 0 | 完整生成或完全相同的无写入结果；帮助/版本也为 0 |
| 2 | CLI 用法错误，未开始生成 |
| 1 | 输入、类型、登记、标记、生成或 IO 错误；stdout/stderr 写入失败也为 1 |

生成成功只输出改写文件数及“no judges executed”。失败输出具体路径和错误，不能将其当部分成功。
已有 runner 的 `chrono-harness check` 仍为未实现，退出 **3**；两条命令的退出合同互不替代。

## 唯一来源、登记与更新

产品发行资产 `assets/methodology.md` 是可迁移方法的维护源；不是每个宿主的运行时依赖。
init 选定资产后留下宿主自己的 canonical 快照；以后两者可明确地独立演进，不自动同步。
`assets/entrypoint.md` 是编译时嵌入的固定路由文案，变化需要重新构建；没有模板语言。
资产均在 Rust 源目录之外。源码中的字段、角色、路径与标记属于封闭格式合同。

init 保留以下文件：

```text
.chrono-harness/instructions/methodology.md   选定方法的精确字节，之后由宿主维护
.chrono-harness/instructions/host-context.md  明确宿主事实与操作，允许为空
.chrono-harness/instructions/manifest.json   格式、生产者、顺序、路径和角色登记
AGENTS.md                                   agent 发现入口，块外是宿主原文
CLAUDE.md                                   agent 发现入口，块外是宿主原文
```

manifest 严格接受以下对象（格式空白不重要）；禁止额外/重复字段、重复/遗漏/重排记录、
未识别版本/生产者/渲染身份以及不同路径/角色。源路径相对宿主根。此登记立即由生成器消费，
独立于尚未启用的五份 harness 登记；不需要为任意宿主预装这些判官草案。

```json
{
  "schema_version": 1,
  "producer": "chrono-instructions/0.1.0",
  "render": "read-both/v1",
  "sources": [
    {"role": "methodology", "path": ".chrono-harness/instructions/methodology.md"},
    {"role": "host-context", "path": ".chrono-harness/instructions/host-context.md"}
  ],
  "outputs": [
    {"role": "agents-entrypoint", "path": "AGENTS.md"},
    {"role": "claude-entrypoint", "path": "CLAUDE.md"}
  ]
}
```

两入口都明确要求按顺序**全文阅读这两个文件**；没有第三份合并正文，也没有独立的方法副本。
程序验证两源存在、可读且为 UTF-8，然后刷新路由；正文变化不需要重写相同的路由块。
入口全文不被宣称为投影，只有指定块是生成器所有。入口相同不构成语义独立的核验。

正常更新：宿主 AI 有意识地编辑保留的两个 canonical 文件，再运行 generate。
若要采用新版产品方法，也应先比较并明确更新宿主方法源，再 generate；程序不会自动覆盖定制。
重复 init 仅在两个显式输入都与保留字节相同时允许刷新；不同则报错并提示正常更新路线。
重复 init/generate 且全部预期字节相同，不创建目录、临时文件或改写任何文件。
已登记 manifest 的排版原样保留；不因语义相同而重排 JSON。
首次 init 若无 manifest 却已有任一保留源，报 reserved collision，不领养或覆盖。
`.chrono-harness` 内其他文件和非保留文件不影响初始化，也不由生成器改写。

## 块与文件类型

只拥有从 `<!-- chrono-instructions:begin -->` 到 `<!-- chrono-instructions:end -->`
（含两个标记，不含 end 后的换行）的一个块。标记必须各一次、顺序正确且独占行。
前缀 `<!-- chrono-instructions` 保留给协议，畸形/重复/缺对标记均在写入前失败。
缺块时在现有字节末尾追加：必要时先补换行，随后是块及末尾换行。
有块时仅替换该字节区间，其余前后缀（包括非 UTF-8 字节）完全保留。
块内手工改写会被替换，不设输出哈希账本或漂移审批门。冲突指令的语义由宿主判断。

不存在的入口创建为普通文件。唯一支持的根文件别名是字面相对链接
`AGENTS.md -> CLAUDE.md`，且 CLAUDE.md 必须已经是普通文件；只更新其有效目标一次并保持链接。
反向链接、`./CLAUDE.md` 等其他写法、悬空链接、目录或其他特殊类型均报错。
保留源/manifest/存储目录不得是符号链接；Unix 下有多个硬链接的保留文件/入口也拒绝，
避免普通误操作悄悄改变别名关系。这些检查不是针对恶意调用者的安全隔离。

## IO 与恢复边界

先读取、校验所有所需输入与输出，再创建缺失的保留目录；为每个有变化的目标在其所在目录
暂存新内容和原内容备份，flush/sync 临时内容后顺序 rename 发布。已有文件权限保留；
新文件使用 tempfile 的私有默认权限。成功重复运行完全不写入。

普通发布失败时，逆序恢复已发布的原字节/权限，移除本次新文件和空目录。
错误含 `published`（曾发布的精确路径）、`unrestored`（未恢复目标）、`recovery` 和 `cleanup`。
无法恢复的原文件备份保留在错误给出的临时路径；未恢复新文件则直接报告目标路径。
清理失败也非零；若所有目标已完成但备份清理失败，错误明确说明 publication complete。
`published` 不等于当前仍被改变，只有与恢复结果一起才能解释。缺失/失败从不伪装成 no-op。

这不是跨文件崩溃原子事务。进程被杀、电源故障或系统崩溃可能留下部分发布与临时文件；
没有持久 journal、重启恢复服务或跨目录持久性保证。恢复字节和权限不保证原 inode、时间戳、
ACL、扩展属性或所有者；更新需要单写者，不支持并发生成/并发编辑或抵抗竞态攻击。
中断后的含糊状态须查看实际保留文件与临时备份并明确恢复，再运行；不能靠重复 init 静默领养。

验证范围为当前 macOS 上的普通文件系统与 Unix 链接/权限行为；Windows 和其他文件系统未验证。
API 保留 OS 路径字节；文件系统本身可拒绝无效编码路径。路径文字错误信息用于人工定位，
不是保证任意非 UTF-8 文件名无损回传的机器协议。

## 验证与自举

```sh
cargo fmt --check --manifest-path instructions/Cargo.toml
cargo fmt --check --manifest-path instructions-tests/Cargo.toml
cargo build --locked --manifest-path instructions/Cargo.toml
cargo check --locked --manifest-path instructions/Cargo.toml
cargo build --tests --locked --manifest-path instructions-tests/Cargo.toml
cargo check --tests --locked --manifest-path instructions-tests/Cargo.toml
cargo test --locked --manifest-path instructions-tests/Cargo.toml
```

配对测试使用真实临时宿主，验证路由、保留字节、二次无写入、源演进、登记和标记失败、类型和
链接边界、CLI 状态、普通权限失败及有界的晚发布/回滚失败。权限测试在特权用户可绕过 mode 位时
明确给出诊断，不宣称这种运行验证了拒读；故障注入仍测试相同回滚逻辑。
产品方法资产是测试的显式运行输入，路由资产是生产编译输入；两者分别登记。

本仓根入口和 `.chrono-harness/instructions/` 由此工具生成，宿主上下文只描述本项目。
这证明生成能力可用于本仓；不启用 proposed 登记、check 或 CI gate，也不证明任何 AI 已遵守方法。
