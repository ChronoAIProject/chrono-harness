# 宿主原子指令生成合同：atomic-rules/relative-alias/v3

`chrono-instructions` 是独立 Rust 库与薄 CLI，生产/专属测试为 instructions ↔ instructions-tests；无根 workspace，不依赖 runner。它校验显式数据、解析内容依赖并使用同一个普通 IO publisher 发布投影，不执行判官，不推断语义依赖、项目、语言或政策，不翻译、不联网解析包。

## 调用与唯一编辑源

```sh
cargo install --locked --path crates/instructions
chrono-instructions init --host-root "/path/to/existing host"
# 仅新宿主可显式绑定默认英文；不是运行时翻译
chrono-instructions init --host-root "/path/to/another host" --locale en
# 修改该宿主的 catalog / manifest / 显式 file 源后
chrono-instructions generate --host-root "/path/to/existing host"
```

默认 init 采用编译时内嵌的 `assets/instructions/catalog.json` 与 root-only `default-manifest.json`，创建独立宿主 `.chrono-harness/instructions/catalog.json`、`manifest.json`、空 `host-context.md`。复制后的二进制无需 checkout。产品资产与宿主采用数据不是同一所有者；升级二进制不会覆盖已采用数据。采用新默认须明确比较并编辑宿主数据，然后 generate。

默认根显式选择 20 个现有双语内容叶子，按 `workflow` 的目标与入口、登记与隔离、实施、检查与修复、演进与交付五节同级短流程呈现。每项受治理操作只暴露当前登记路径；定制先登记，方法演进更新登记并完成适用验证，保留一个正式入口。完整 114-atom 库、`core.general` 与 `general` 布局仍可选择用于自定义 Markdown／skills，不自动覆盖既有宿主的 manifest。

CLI：`init --host-root H [--methodology M] [--host-context C] [--locale L]`；`generate --host-root H`。H 已存在，各选项仅一次，参数按 OS 路径运输。新宿主省略 locale 选择 zh-CN；显式 methodology 以一个 opaque file atom 保留，未指定 locale 则绑定 und（未指定语言），指定 locale 则明确绑定该语言而不翻译。初始 locale 必须已在嵌入 catalog 声明。默认 root title 仅属于默认 zh-CN 绑定；改用其他 locale 时省略此可选 title，作者可在 manifest 显式添加本地化 title。context 可独立选择。显式外部 M/C 必须可读 UTF-8 普通文件，可由调用者选择链接。

已登记 init 的省略选项保留原数据；显式不同 context、raw method 或 root locale 拒绝，提示编辑源/输出计划再 generate。atomized root 不允许用 raw methodology 替换。源缺失、登记损坏不补默认。没有登记却已有保留源/控制文件时拒绝碰撞。public `init`、`init_with_defaults`、`generate`、`dispatch` 继续可用。

退出 0 完成/无写入/帮助/版本；2 CLI 用法错误；1 输入/生成/IO/平台错误。成功报告实际改动数和 `no judges executed`。独立 `chrono-harness check` 已有 [CI slice](ci.md) 和有界 full 判官链，本宿主完整启用仍未完成；指令生成不证明 AI 已阅读、遵守、执行判官或通过检查。

## Schema 与可复制组合配方

专用 manifest schema_version=2，producer 固定 `chrono-instructions`（独立于包版本），render 固定 `atomic-rules/relative-alias/v3`。以下是完整示例：默认宿主仅有第一项，增加后两项再 generate 即可得到英文文档与聚焦 skill。本仓以相同输入生成这些实例。

```json
{
  "schema_version": 2,
  "producer": "chrono-instructions",
  "render": "atomic-rules/relative-alias/v3",
  "catalog": ".chrono-harness/instructions/catalog.json",
  "host_context": ".chrono-harness/instructions/host-context.md",
  "outputs": [
    {"id":"root","path":"CLAUDE.md","format":"root-guide","locale":"zh-CN","roots":[
      "goal.deliverable", "action.autonomy", "method.entry", "method.host-tools",
      "registry.explicit", "judge.register", "core.small-projects", "candidate.isolation",
      "reuse.search", "owner.canonical",
      "delta.selection", "delta.candidate-judge", "delta.local-ci", "test.behavior", "evidence.program-state", "evidence.failure",
      "policy.evolution", "policy.mixed-warning", "candidate.landing", "artifact.results"
    ],"title":"通用工作方法","layout":"workflow"},
    {"id":"general-en","path":"docs/generated/general-methods.en.md","format":"markdown","locale":"en","roots":["core.general"],"title":"General working methods","layout":"general"},
    {"id":"repair-skill","path":"skills/diagnose-recurring-failures/SKILL.md","format":"skill","locale":"en","roots":["core.repair-producer"],"skill":{"name":"diagnose-recurring-failures","description":"Diagnose a recurring failure by inspecting actual evidence and repairing its producer. Use when the same symptom requires the same corrective action for a second time."}}
  ]
}
```

恰有一个 root-guide，path 必须为 `CLAUDE.md`；`AGENTS.md` 保留给字面相对链接 `CLAUDE.md`。发布时固定根 CLAUDE/alias 对先于所有额外输出，CLAUDE 在所需 alias 前；额外输出之间按 outputs 声明顺序发布，与 root-guide 条目在数组中的位置无关。ID/路径唯一。每项必填 id/path/format/locale/roots；title 可选，单行非空无控制字符；只有 skill 必须且允许 skill metadata。

Catalog schema_version=1，必填 locales 与 atoms 数组。结构例子（合成内容，替换宿主 catalog 时须相应修改 manifest roots 与 locale）：

```json
{
  "schema_version": 1,
  "locales": [{"id":"en","root_frame":"Edit the registered sources and regenerate; read the registered host context before work. Generation does not prove compliance.","projection_notice":"Generated from registered sources. Edit those sources and regenerate."}],
  "atoms": [
    {"id":"example.evidence","requires":[],"variants":[{"locale":"en","source":{"type":"inline","text":"Inspect the actual error before attributing a failure."}}]},
    {"id":"example.repair","requires":["example.evidence"],"variants":[{"locale":"en","source":{"type":"file","path":".chrono-harness/instructions/repair.en.md"}}]},
    {"id":"example.bundle","requires":["example.repair"],"variants":[{"locale":"en","source":{"type":"inline","text":""}}]}
  ]
}
```

locale id 显式且唯一，允许追加本地语言；无语言标签猜测/自动 fallback。atom id 是稳定引用身份（ASCII 字母/数字及 `._-`，非空）；requires 有序，variants 以 locale 唯一；source 是严格 tagged inline{text} / file{path}。JSON 重复、未知字段拒绝。根引用和依赖引用必须存在，所有已登记依赖均检查 cycle，并报告链。所有 file variants 都是完整显式输入，即使未被当前输出选择，也校验存在、UTF-8、普通类型和路径。只有选中闭包需要对应 locale variant；未选中的 atom 可缺翻译。

无 layout 的输出按 roots / requires 声明顺序 DFS，先依赖后使用者，共享 atom 每个输出只渲染一次；不按文字推断边。`core.general` 是具有空 zh-CN/en 文本的普通 aggregate，显式选择原有 17 个稳定入口及新增便携义务。当前共有 98 个内容叶子、16 个普通聚合；原有主题 ID 只组合原职责，不扩成章节全集。`core.repair-producer` 是内容叶子，仅依赖 `core.evidence`（3 个叶子）和 `core.reuse`（3 个叶子），skill 共呈现 7 个叶子；不引入容量、缓存、CI 或研究路线。空 inline 可表达纯组合，非空段按顺序以两个 LF 分隔，段内字节不 trim、不正规化；file UTF-8 字节与 JSON 解码后的 inline UTF-8 精确保留。BOM、CRLF、空内容、无末尾换行均可运输。title、框架、输入引用、分隔 LF 与所有权标记是投影格式。选中内容及框架禁止保留前缀 `<!-- chrono-instructions`，context 不嵌入所以不受此限制。

locale 拥有 root_frame 与 projection_notice；框架和正文必须使用所选 locale 的登记数据。root 的 manifest/catalog/host_context 路径随实际绑定列出；正文直接在根受管块内，不要求重复读取源。翻译是作者提供的数据，不声称语义等价或完整性已由程序证明。

## 可选文档布局

本扩展只新增 catalog 的可选 `layouts`（缺省空数组）和 output 的可选 `layout`（具名引用）；catalog schema 1、manifest schema 2 / render v3 的原有字段语义不变。未选择 layout 时保留原 DFS 与逐字节输出行为，注册但未选择的布局不自动生效。旧二进制会拒绝这些未知字段，不会静默忽略；升级生成器即可读取，已有宿主不自动采用产品新布局。raw-method init 和 v1/v2 迁移明确不带布局及选择器。

布局由有序 `sections` 组成，每节必填相对 `depth`、`titles` 与显式内容 `atoms`。以下片段可加入上面的合成 catalog，并在输出选择 `"layout":"example"`：

```json
"layouts": [{"id":"example","sections":[
  {"depth":1,"titles":[{"locale":"en","text":"Methods"}],"atoms":[]},
  {"depth":2,"titles":[{"locale":"en","text":"Evidence and repair"}],"atoms":["example.evidence","example.repair"]}
]}]
```

布局 ID 唯一；引用必须是已登记 atom，布局内不可重复放置。depth 是 1–6 整数，从 1 起，相邻只可增深一级，可任意回浅；有 output title 时渲染深度加 1，结果不得超过 Markdown 的 6 级。标题按显式 locale 唯一，文本非空单行、无控制字符或保留 marker；未知字段与重复 JSON 字段拒绝。所有布局检查结构与引用；选择布局的输出还必须有每节的所选语言标题，无 fallback。

先按原 roots/requires 求完整闭包、读取所选 variant 并验证，再要求布局恰好覆盖其所有非空正文一次。空按 UTF-8 字节长度判定，不 trim；不放置空 aggregate。缺少依赖正文、重复、未知或闭包外放置均报错，不能借布局扩大选择。各节先输出标题再输出显式列出的正文，间隔仍是两个 LF，正文原字节不变。空 atoms 可用作上层标题。所有输出完成预检与渲染后才进入原 publisher，失败不写源、输出、alias 或目录。

**布局只组织阅读，不产生权威优先级或执行顺序承诺；requires 仍仅拥有内容依赖。** 产品 `workflow` 显式放置默认所选 20 个叶子，五节 depth 均为 1；保留中文 root title，因此中文根节标题为二级，初始英文无 title 时为一级，locale 行为不变。`general` 仍显式列出基本原则、工作方法、执行合同三部分及 12 个主题，覆盖完整 98 个内容叶子；本仓英文指南继续选择 `core.general/general`。主题成员不是扫描、ID 前缀或正文推断，也不是 header atom。宿主采用数据独立拥有；focused skill 不选择通用布局，仍按原 7 叶子闭包平铺。

## 所有权、路径与退休

根仅替换 `<!-- chrono-instructions:begin -->` 至 `<!-- chrono-instructions:end -->` 一个受管块；标记须唯一、有序、独占行，其他保留前缀拒绝。块外原字节（可非 UTF-8）保留；无块则追加，必要时补 LF。只有 AGENTS 普通文件时作为 donor，继承原文与普通权限；两普通根分别渲染后整份预期字节相同才转换 AGENTS。不同原文明确冲突，授权 AI 保全并整合后重试。已有有效 alias inode 不触碰。

额外 Markdown / skill 整文件归投影。文件不存在可创建；已有文件必须有恰当 producer、output ID、format 的完整 begin/end envelope。未标记、错误身份、畸形 envelope 一律预写入失败，不静默覆盖宿主文件。skill 的 `---` 必须是最初字节，name/description 用 JSON 字符串形式的合法 YAML 双引号标量，所有权标记在 frontmatter 关闭之后。name 1–63 位小写 ASCII 字母/数字/连字符，无首尾或连续连字符，且父目录同名、文件名恰为 SKILL.md；description 非空、单行无控制字符，最多 1024 个字符且无尖括号，供当前外部 validator 消费。title/metadata 不推断使用范围，作者负责聚焦且有用的语义。

仅当前 manifest 的输出受管理。删除条目或改名均保留旧文件原样；旧 marker 仅记录来源，不代表当前管理。旧 skill 可能继续被发现，授权 AI 须显式退休旧文件。没有 previous-manifest/history ledger、glob、扫描卸载或自动删除。

所有登记路径必须为规范的相对 `/` 路径，无空段、`.`、`..`、反斜杠、控制字符或绝对路径；catalog/context/file 源在宿主 `.chrono-harness/` 下。控制/源/输出/alias 不得相互覆盖或构成文件祖先冲突。重复引用同一个 file variant 输入允许；多个输出路径不允许。所有已有祖先必须是真实目录，保留文件/控制文件/输出拒绝链接、硬链接、特殊类型（AGENTS 固定别名是唯一例外）。检查已存在普通文件的设备/inode 别名；当前 macOS 还保守拒绝各路径段经 Unicode case folding 和文件系统表示转换后相同的不同拼写（如 `straße` / `STRASSE`，即使所在卷区分大小写）。比较键先由现有 CoreFoundation 的 `CFStringFold` 以 `kCFCompareCaseInsensitive` 和规范系统 locale 生成，再经文件系统表示 API 转换；不改登记原字节、不增加 Cargo 依赖、不写入探测文件，避免预期新文件/目录别名。先验证完整计划并渲染所有输出，再写任何内容；目录按父先子后去重创建。

## 严格前向迁移

只接受完整已知 schema=1 / producer=`chrono-instructions/0.1.0` 身份：sources 按顺序为 methodology:`.chrono-harness/instructions/methodology.md`、host-context:`.chrono-harness/instructions/host-context.md`；旧 read-both/v1 outputs 按顺序为 agents-entrypoint:AGENTS.md、claude-entrypoint:CLAUDE.md；旧 literal-core/relative-alias/v2 outputs 为 claude-guide:CLAUDE.md、agents-relative-alias:AGENTS.md。混用角色/版本/路径或未知字段拒绝。

init/generate 自动迁移为 current manifest + catalog，`legacy.method` 的 und file variant 显式引用旧方法。und 表示语言未指定，不冒称中文或英文；框架是显式未指定语言的数据。保留精确旧 method/context 字节、路径、普通权限及根块外原文，不拆旧 prose、不更新为默认、不删除旧文件；只刷新框架、组合绑定与根布局。迁移最后发布 manifest。新 raw-method init 使用同一模式；显式 --locale 则由作者声明语言。current manifest 的排版在未改绑定时保留。

本仓明确采用本次逐条迁移的产品双语 catalog 到独立宿主 catalog，再使用实际重建二进制生成三个现有消费者。先前 001e501 的精选 17 段被细分与补充，稳定入口保留且没有第二份旧段落权威；旧 monolith 继续退休，历史由版本库保留。这不是其他宿主的自动更新：既有宿主继续保留自己采用的 catalog 或 opaque method，新默认采用须明确比较、保留定制并编辑宿主数据。可选 [逐条处置表](methodology-clause-map.md) 与 [许可说明](licenses/methodology-attribution.md) 不进入生成图。

## IO、边界与验证

复用同一 publisher：同目录暂存普通新文件与原文件备份，flush/sync，新文件为 tempfile 默认私有权限，原普通权限保留；source 在 output 前，CLAUDE 在 alias 前，新建/迁移 manifest 最后。当前用户已编辑 manifest 不是程序保存的历史。全部相等时不创建临时文件，不改 inode/mtime。

普通发布失败逆序恢复字节/普通类型/权限、删除本次新文件/链接、子先父后清理空目录。错误列 published、unrestored、recovery、cleanup；published 不等于仍改变。回滚失败保留原文件备份并报告路径；无原文件则报告未移除目标。恢复实际异常状态后重试。清理失败也非零，完成发布但清理失败明确说明。

只限定 Unix / 当前 macOS 普通 IO；不保证崩溃原子性、并发写者、跨平台、ACL、扩展属性、原 inode/时间戳/所有者恢复。无恶意 AI 门或全局安装/provider 假设。

```sh
cargo fmt --check --manifest-path crates/instructions/Cargo.toml
cargo build --locked --manifest-path crates/instructions/Cargo.toml
cargo check --locked --manifest-path crates/instructions/Cargo.toml
cargo fmt --check --manifest-path crates/instructions-tests/Cargo.toml
cargo build --tests --locked --manifest-path crates/instructions-tests/Cargo.toml
cargo check --tests --locked --manifest-path crates/instructions-tests/Cargo.toml
cargo test --locked --manifest-path crates/instructions-tests/Cargo.toml
```

产品默认内容同时按完整根字节数核消费边界，包括框架而非只数字符或 token；当前空上下文新宿主 zh-CN 根为 6554 字节／20 叶子，en 根为 6995 字节／20 叶子。修正前分别为 29916／32755 字节、98 叶子；原 title／locale 行为保持。本仓另读的 host-context 从 18653 缩为 4575 字节，完整英文 Markdown 从 32552 变为 32806 字节／98 叶子，聚焦 skill 仍为 2821 字节／7 叶子。完整 [实测读数与适用边界](methodology-extraction.md#实际消费者边界) 另列；不把完整库当作默认根预算结论。内容编辑须复核实际受影响消费者，不截尾、不借全局配置扩限。独立选择 `core.ownership` 或 `core.behavior` 的 Markdown 消费者分别包括投影与真实 CI 义务；此合同不新增文字匹配测试框架。

专属行为测试覆盖图顺序/去重/错误、显式 locale、精确字节、布局重排/双语复用/深度/完整覆盖/预写入拒绝、ownership/path 预检、迁移、原文/alias/no-op、普通与注入失败恢复。test-support 无生产开关。真实复制二进制验证默认 init，再显式添加多输出配方；实际消费者读数及外部 skill 格式验证的适用边界见 [迁移说明](methodology-extraction.md#实际消费者边界)，不引入生产依赖，不证明语义。

FILEMAP 的 proposed projection 元数据从单一 `source` 扩展为显式 `sources` 数组（catalog、manifest 及被引用 file 输入），producer/scope 不变；生成 root-frame 的路径引用也由 manifest 决定。产品资产是 build-input/test-execution，宿主数据和既有投影是 runtime-input。五份完整治理登记仍 proposed、成本 unmeasured；指令专用 schema 校验不冒称治理判官执行。独立 CI slice 的实际 DELTA、协议与生成范围见 [CI 合同](ci.md)。
