# 宿主指令生成合同：literal-core/relative-alias/v2（已实现）

`chrono-instructions` 是独立 Rust 库与薄 CLI，不依赖 runner、不执行判官。
生产/专属测试为 `instructions/` 与 `instructions-tests/`，各有 manifest、lockfile、target。
serde/serde_json 解析严格 JSON，tempfile 暂存普通文件和链接；没有新增运行时框架。

## 调用与内容边界

```sh
cargo install --locked --path instructions
# 安装后从任意 cwd 使用；H 必须是已有目录，可含空格，不要求 Git
chrono-instructions init --host-root "/path/to/host"
# 编辑宿主方法或上下文后刷新
chrono-instructions generate --host-root "/path/to/host"
```

默认 init 只需 host-root：二进制内嵌 `assets/methodology.md` 的精选通用核心，默认上下文为空。
运行时不读 checkout、资产目录或私人配置，不发现项目、语言、依赖，不推断政策。
这份核心选择通用软件/AI 工作方法，并非原始指南的完整迁移；取舍见可选的
[来源说明](methodology-extraction.md)。研究专属体制不作为默认核心或第二份必读附件。
文字是工作纪律，生成器不判断内容质量、不认证 AI 阅读或遵守、不实现通用 check。

`init --host-root H [--methodology M] [--host-context C]` 可独立覆盖任一输入。
显式输入必须可读、为 UTF-8 普通文件，可以由调用者明确选择外部文件链接。
选项顺序可交换；host-root 必填且各项最多一次。generate 只接受 host-root。
相对参数相对于调用 cwd，输出相对于解析后的 host-root。参数按 OS 路径运输，不经 shell 插值。
保留源和嵌入方法均保留精确字节，包括 BOM、CRLF、空内容及无末尾换行；生成框架另加分隔换行。
方法禁止包含保留前缀 `<!-- chrono-instructions`，无论是否独占行；在任何写入前报错。
该限制也适用于旧登记中的方法；先有意识地修改方法中的冲突文本再重试。上下文不嵌入，故不受此限制。

| 退出码 | 含义 |
| --- | --- |
| 0 | 完整生成或完全无写入；帮助/版本也返回 0 |
| 2 | CLI 用法错误，未开始生成 |
| 1 | 输入、类型、登记、标记、平台、生成或 IO 失败，包括 stdout/stderr 写入失败 |

成功输出改动路径数和 `no judges executed`。失败输出具体路径及错误，不能当部分成功。
`chrono-harness check` 仍未实现、返回 3；两个命令的退出语义不互相替代。

## 单一编辑源和根布局

```text
.chrono-harness/instructions/methodology.md   宿主方法的唯一编辑源
.chrono-harness/instructions/host-context.md  宿主明确上下文，可为空
.chrono-harness/instructions/manifest.json   专用格式/源/输出登记
CLAUDE.md                                   普通文件：完整方法受管块 + 宿主块外文字
AGENTS.md -> CLAUDE.md                       实际符号链接，字面相对目标恰为 CLAUDE.md
```

CLAUDE.md 的受管块直接包含完整方法，无需再读 methodology.md。
框架说明编辑源、生成命令和另读 host-context.md；只有这个块是生成投影，块外原文仍归宿主。
AGENTS.md 与 CLAUDE.md 解析到同一字节，没有第二份独立编辑的政策。
产品 `assets/methodology.md` 是发行默认，首次 init 后的宿主源是独立采用的快照。
升级二进制不覆盖宿主定制：采用新版核心须先比较并明确编辑宿主源，再生成。
`assets/entrypoint.md` 仅是固定框架文字，生产者在框架后原样附方法，无模板语言。
两份资产都是编译输入，修改产品资产后须重建二进制。

正常编辑 canonical 源后，init 或 generate 都刷新根方法；只改上下文通常无需改根字节。
已登记 init 省略选项时保留对应源，显式输入不同则拒绝并提示 edit + generate；不与新默认比较。
源缺失、不可读或登记损坏不补默认；无 manifest 却有保留源时拒绝 reserved collision。
`.chrono-harness/` 其他文件不受生成器改写。
全部预期状态相同时不创建临时文件、不改 inode/mtime；已有有效 AGENTS 链接即使更新方法也从不触碰。

## 版本和前向迁移

当前 manifest 严格接受下列值；格式空白任意，重复/未知字段、漏项、重排、路径或角色变化均拒绝：

```json
{
  "schema_version": 1,
  "producer": "chrono-instructions/0.1.0",
  "render": "literal-core/relative-alias/v2",
  "sources": [
    {"role": "methodology", "path": ".chrono-harness/instructions/methodology.md"},
    {"role": "host-context", "path": ".chrono-harness/instructions/host-context.md"}
  ],
  "outputs": [
    {"role": "claude-guide", "path": "CLAUDE.md"},
    {"role": "agents-relative-alias", "path": "AGENTS.md"}
  ]
}
```

schema 形状保持 1；新的 render 身份和输出角色明确规定完整布局，包括字面链接目标。
唯一可升级输入是旧 `producer=chrono-instructions/0.1.0`、`render=read-both/v1`、同一 schema/sources，
outputs 按顺序恰为 `agents-entrypoint:AGENTS.md`、`claude-entrypoint:CLAUDE.md` 的完整登记。
init 和 generate 共用预检与发布路径：保留旧方法/上下文，转换根文件，最后写当前 manifest。
这只升级布局，不把旧的自定义方法自动替换成精选核心；本仓另行明确采用新版资产后才运行迁移。
输出只使用当前身份，没有旧渲染模式或降级开关。当前 manifest 的原排版在重复运行时保留；旧登记升级时重新序列化。
旧二进制会拒绝新 render/角色；需继续使用新二进制，不手改 manifest 冒充降级。
专用 manifest 已由代码执行校验，独立于仍 proposed 的五份通用 harness 登记。

## 原文保留与转换合同

只替换从 `<!-- chrono-instructions:begin -->` 到 `<!-- chrono-instructions:end -->` 的一个块，
含两标记但不含 end 后换行。标记须各一次、顺序正确、独占行；保留前缀的其他出现也拒绝。
块外字节可非 UTF-8，原样保留。缺块时追加，必要时先补一条分隔换行。

| 预检发现的根文件 | 行为 |
| --- | --- |
| 两者都无 | 新建 CLAUDE.md 和相对链接 |
| 仅一个普通文件 | 保留其原文并渲染为 CLAUDE.md；仅 AGENTS 时也继承其普通权限 |
| 两个普通文件 | 分别渲染同一核心，只有预期完整字节相等才转换 AGENTS；CLAUDE 权限优先 |
| AGENTS 为字面 `CLAUDE.md` 链接且目标是普通文件 | 保留链接本身，只刷新目标 |
| 两普通文件的预期字节不同 | 无写入冲突；诊断同时指出两路径和明确整合方法 |
| 反向、非字面、绝对、悬空链接、特殊类型或硬链接普通文件 | 无写入拒绝 |

两个旧块不同不妨碍转换，相同核心渲染后块外差异仍冲突。
处理冲突的授权 AI 应先保全有效原文、明确整合到 CLAUDE.md，再对齐 AGENTS 普通文件或移除冗余文件，重试。
生成器不自动合并两份不同宿主政策、不静默丢字节，也不要求人类批准。
保留源、manifest 及 `.chrono-harness/` 两级目录不允许链接；目录必须真实目录。
Unix 普通文件的硬链接也拒绝，避免隐藏写入共享 inode 的所有权歧义。

## IO、恢复与平台边界

预检完成才创建缺失目录。在目标同目录暂存新普通字节、原普通文件备份和新符号链接。
普通文件暂存后 flush/sync；已有普通权限保留，新文件采用 tempfile 默认私有权限。
不打开或 chmod 暂存链接。发布顺序为新源、CLAUDE、AGENTS、新建或升级 manifest。
已有有效链接不加入事务；新别名和普通 AGENTS 的转换都由同一 publisher 负责。

普通发布失败时逆序恢复原字节/普通类型/权限，移除本次新文件或链接、空目录。
错误给出 `published`（曾发布路径）、`unrestored`、`recovery`、`cleanup`；published 不等于仍被改变。
回滚失败的原文件备份保留在诊断的临时路径，可据其恢复对应普通文件；没有原文件则直接报告未移除目标。
检查实际状态，恢复已报告目标后再运行，不能让悬空链接或缺失源被当作正常重跑。
清理失败也非零；目标全已完成但备份清理失败时明确报告 publication complete。

仅支持 Unix 链接发布；非 Unix 在写入前明确失败，不回退为两个普通副本。
已验证当前 macOS 的文件系统、权限与链接行为；其他 Unix、Windows 和文件系统未验证。
不提供多文件崩溃原子性、并发写者、竞态防护、重启恢复服务；进程被杀可能留下部分发布。
恢复不保证原 inode、时间戳、ACL、扩展属性或所有者。使用单写者，保留源后有意识地恢复异常状态。
文件系统可能拒绝非 UTF-8 路径；诊断路径是定位文字，不是任意 OS 路径字节的机器往返协议。

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

测试验证字节运输、实际链接身份、重复无写入、源编辑、定制保留、根转换/冲突、严格前向迁移、
输入/标记/类型错误、真实权限失败和注入的别名发布前后失败及恢复。test-support 仅配对测试启用，CLI 无注入开关。
特权进程可绕过权限位时测试明确报告未验证该拒绝，不称通过了权限边界。
资产字节测试是运输合同，不是散文关键词或语义完整性认证。
本仓使用同一个工具升级为根正文和链接，FILEMAP 明确登记字面别名、源/投影所有权、编译/测试/运行输入。
自举不启用通用判官、DELTA 或 CI gate。
