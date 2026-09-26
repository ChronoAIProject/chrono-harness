# chrono-harness 规格 v0.1

状态：**DRAFT / SPEC-FIRST**。本文是待实现合同，不是已运行的验收报告。
当前实现含 Rust CLI 信息命令，以及独立的宿主指令生成器与各自配对测试；§14、§16 给出实现边界。
本文中的“必须”“错误”“判官”描述目标行为，除明确标注已实现者外均未执行。

## 1. 目标、权限与边界

Harness 的本质：对选定操作，只提供一种已登记的方法，并用可执行判官检查其结果。
AI 先把路径、归属、依赖、成本、操作及判定方式登记为白名单，再通过该方式处理任务。
缺少登记、重复入口、执行结果与登记不符必须有明确错误，不能靠“智能推断”补全。
系统信任 AI 非恶意，处理的是遗漏、漂移、错误和演进成本；AI 自主登记、修复和推进。
AI 拥有构建、测试、更新约束和分支重建所需权限，没有人工审批、签字或确认步骤。

可执行的项目准入约束必须实现为登记判官，不能隐藏在 runner、CI YAML、hook 或根指南中。
根指南直接呈现一般工作方法；它不是可执行判官，也不因严格措辞变成已运行的机器门。
runner 的固定职责仅为命令/协议解析、输入运输、启动指定程序、限时限量与结果聚合。
Git 差异是输入事实；是否允许脏目录、怎样选测试、分支是否过期，均由登记判官裁决。
插件、脚本和内置发行的判官遵守同一协议；“随产品发行”不等于“自动生效”。
宿主可替换判官，不能把某个项目名、固定目录规模或测试名称写死在通用运行时。

约束的可观察范围是提交快照、登记数据、harness 启动的操作及其报告。
不声称监控 AI 所有 shell 命令，不禁止调试命令，不用 OS 沙箱证明“只有一种方法”。
受治理操作的合格证据只能由其唯一登记入口产生；绕路、缺报告或不匹配报告会报错。
未受治理的编辑行为不需要虚构执行证据；新的治理范围由 AI 显式登记。

## 2. 仓库与自举边界

```text
SPEC.md                         产品合同，不承载生效宿主配置
README.md                       使用和实现状态
runner/Cargo.toml + Cargo.lock  runner 生产项目；无第三方依赖
runner/src/{lib,main}.rs         产品实现，公开 CLI dispatch 边界
runner-tests/Cargo.toml + Cargo.lock
runner-tests/tests/cli_contract.rs  runner 专属测试项目
instructions/Cargo.toml + Cargo.lock  独立的指令生成生产项目
instructions/src/{lib,catalog,main,transaction}.rs  组合解析、CLI 与文件发布
instructions-tests/Cargo.toml + Cargo.lock  生成器专属测试项目
instructions-tests/tests/{generation,atomic}.rs  临时宿主行为测试
assets/instructions/catalog.json           双语原子规则/框架，编译输入
assets/instructions/default-manifest.json  默认仅根输出计划，编译输入
docs/instructions.md            已实现生成合同
docs/methodology-extraction.md  来源与方法提取判断，不是运行依赖
AGENTS.md / CLAUDE.md           CLAUDE 普通正文、AGENTS 相对链接；仅指定块为投影
.chrono-harness/instructions/  宿主原子 catalog、上下文与输出计划（见 §16）
.chrono-harness/config.json     宿主入口、协议、显式字段分类
.chrono-harness/judges.json     宿主判官白名单
.chrono-harness/projects.json   项目、脚本和唯一操作登记
.chrono-harness/FILEMAP.json    逐文件归属、带类型的依赖及成本
.chrono-harness/workflow.json   分支、稳定性、迁移与混合修改策略
.chrono-harness/bin/            将来安装的指定二进制（不跟踪）
.chrono-harness/state/          将来输入/报告/证据（不跟踪）
```

没有根 Cargo workspace；不需要一起编译的项目不放进同一个编译单元。
生产 crate 同时导出库与极薄二进制，因为二者共享同一 CLI 行为；不另拆无价值项目。
测试 crate 通过显式 path dev-dependency 引用生产库，有独立 lockfile 和 target 目录。
产品 crate 不定义单元测试或 doctest；所有当前行为测试归唯一配对测试项目。
以后新增独立生产项目必须同时登记一个专属测试项目；不能以共享巨型测试项目替代。

宿主的所有生效 harness 约束、脚本配置、安装绑定和指令 canonical 材料都在宿主 `.chrono-harness/` 内。
根 CLAUDE.md 的受管块直接呈现完整方法，AGENTS.md 是指向 CLAUDE.md 的字面相对符号链接；
块外原有文字属于宿主；方法在 canonical 源编辑后生成，不独立维护副本。
宿主可以使用其他仓库构建的二进制，但须在此显式绑定路径、版本与摘要。
自举时 `runner/src/*` 仍然是 **product**，不能因产品是判官引擎便改称宿主 policy。
其稳定性需求由 workflow 的 `stability` 路径登记表达，不通过重新归类源码表达。
产品构建不会自动启用宿主规则、安装 hook、修改全局配置或访问参考仓库。

## 3. 登记格式与字段合同

本节的五个判官 JSON 文件均采用 `schema_version: 1` 和 `status: "proposed" | "active"`。
当前五份均为 proposed；`config.enforcement` 为 `not-implemented`。
启用前补齐实际输入闭包、二进制 SHA-256 和全部必需判官，再改为 active/enabled。
null 摘要只允许 draft；不能被解释为“任意二进制都已通过验证”。
这里的类型表是 v1 规范 schema，仓库中的 JSON 是完整的当前实例；尚无 schema 验证器。
对象未列出的字段、重复 JSON key、重复 ID、悬空引用由 registration 判官报错。
路径是仓库根相对 UTF-8 POSIX 路径，不含 `..` 或隐式 glob；environment.inputs.location 可显式指定外部绝对路径。
所有文件逐个登记；目录条目只用于明确生成物，不允许替代源文件登记。

| config 字段 | 类型与约束 |
| --- | --- |
| runner | `{path, version, sha256}`；path 为指定入口，sha256 为 64 位小写 hex 或 draft null |
| registries | judges/projects/filemap/workflow 四个唯一 JSON 路径 |
| canonical_check | `{operation, argv: string[]}`；唯一 `validate.delta` 路由 |
| tools | `{id, program, resolution, version_argv, expected_version}[]`；版本为字符串，draft 可为 null |
| environment | `{inherit: string[], values: object, inputs: {id, location, sha256}[]}`；变量及输入文件白名单 |
| input_closure | `{status: "incomplete"\|"declared-complete", unresolved: string[]}`；工程声明，不是完备性证明 |
| protocol | `{id, timeout_seconds, stdout_limit_bytes, encoding}`；正整数限制、UTF-8 |
| semantic_fields | `{path, pointers: string[], on}[]`；语义字段分类，见 §8 |
| artifacts | `{path, owner, kind, tracked: false}[]`；明确生成目录白名单 |
| enforcement | `not-implemented` 或 `enabled`；proposed 不产生通过报告 |

tools 的 `resolution` v1 仅有 `PATH-once`：只解析登记的 program 一次，报告绝对路径、
文件摘要和版本输出；不搜索其他同类工具作为替代，不自动纠正版本不匹配。
当前仅草拟 cargo，expected_version 为 null，未声明已测得工具链；换工具链由 AI 显式更新登记。
`argv` 是原始参数数组；只对注册的整参数占位符 `{base}`、`{candidate}` 做替换。
没有 shell 插值、模板语言或依据当前目录猜测 manifest 的行为。
只继承 environment.inherit 逐个列出的变量，缺值记录为 absent，values 显式覆盖。
inputs 为逐文件登记的有效输入，ID 唯一，摘要遵循 draft null 规则；报告绑定实际值，不自动读取 dotenv。
外部输入须保留两端快照，变化通过登记摘要进入提交 DELTA；磁盘值不符或快照缺失报输入不足，不隐式扩展 DELTA。
PATH 用于一次解析具名工具，HOME/CARGO_HOME/RUSTUP_HOME 用于已声明的工具链运行；
完整闭包包括实际 cargo/rustc、编译器后端、链接器、SDK、目标库和所读配置文件，
包括 HOME/CARGO_HOME 下配置、rust-toolchain、间接依赖、fixture、外部数据及环境变量。
会影响裁决的随机种子、时钟值、网络响应和超时/资源条件也须显式固定并登记为有效输入。
文件输入用 FILEMAP 或 environment.inputs 登记；后者以 input:<id> 显式连接消费者，不推断边。
当前 input_closure 为 incomplete，inputs 为空；补齐工具、输入、关系与证据后才可声明 declared-complete，unresolved 必须为空。
未知或已知缺失项返回 E_INPUT_UNDECLARED / E_EVIDENCE_UNRESOLVED；登记有效不证明真实输入完整。

| projects 字段 | 类型与约束 |
| --- | --- |
| owners | 唯一 owner ID 数组，包括无编译项目的 repository owner |
| projects | `{id, kind, manifest, lockfile, root, actions, test_project? , tests_for?}[]` |
| kind | `production` 或 `test`；production 必有 test_project，test 必有 tests_for |
| actions | build/check/format_check/execute 以及生成器的 init/generate 对象；每项 `{operation, tool, argv}` |
| scripts | `{id, path, test_script, actions}[]` 或 `{id, path, tests_for, actions}[]` |

每个 production 与一个 test 双向一一对应，test 不再要求递归配一个 test。
脚本以同样的一对一关系单独登记，不为它创建虚假的编译项目。
operation 在全部 actions 与 canonical_check 中唯一；同名不同参数也属于重复方法。
构建、测试依赖只由 FILEMAP 提供；manifest 可作为一致性检查输入，不能用来补登记。
脚本、fixture、环境文件、生成器输入都必须逐个登记依赖，不能按目录名猜测试集。
生成器自举 init/generate 通过已登记 cargo 工具的 `run --locked --manifest-path ... -- ...` 调用，
argv 显式指定本仓宿主根 `.`，init 不传材料路径以保留已有定制；首次采用内嵌精选核心与空上下文。
不新增工具发现或路径模板。任意外部宿主直接用 §16 的 CLI。

| FILEMAP 字段 | 类型与约束 |
| --- | --- |
| cost_models | 成本 ID → `{cpu_ms, wall_ms, peak_rss_bytes, io_bytes, basis}` |
| files | `{path, owner, surface, cost, edges: Edge[], symlink?, projection?}[]`；每路径恰好一次 |
| symlink | 可选字面相对目标字符串；登记实际链接身份，不能按解析后相等放宽字面目标 |
| projection | 可选 `{sources: string[], producer, scope}`；sources 为完整显式已登记源路径（manifest/catalog/file 源），producer 为已登记 project 节点；scope 为 `managed-block`（块外归宿主）或 `whole-file`。这是 proposed 形状扩展，不自动执行 |
| surface | `product / test / documentation / membership / instruction-policy / judge-policy / judge-implementation` |
| Edge | `{kind, to}`；from 隐含为 `file:<path>` |
| project_edges | `{from, kind, to}[]`；显式项目/脚本/外部输入关系 |
| test_costs | `{test, cost}[]`；每个可执行测试项目或脚本有成本引用 |

symlink/projection 是明确的文件表示及所有权说明，不自动增加编译或测试执行边。
本仓 AGENTS 的 symlink 恰为 CLAUDE.md，CLAUDE 的受管块与登记的英文 Markdown/skill 由 instructions 从宿主 manifest/catalog 产生；
框架资产的编译边、资产运输测试边和源/目标读取的 runtime-input 边另行显式登记。
这些 FILEMAP 扩展仍属 proposed；专用生成器 manifest 的实际链接/源校验不等于通用 FILEMAP 判官已实现。

图节点 ID 使用 `file:<path>`、`input:<id>`、`project:<id>`、`script:<id>`、`test:<id>`、`judge:<id>`。
input 节点引用对应状态的 environment.inputs；其边在 project_edges 显式列出，变化也成为种子。
环境变量及其他非文件输入以显式快照文件登记；两端值无法保留/比较时属于输入不足。
test 节点指向登记项目的 execute 操作或脚本的测试操作，不是猜出的测试函数。
每个 owner、cost、节点及操作引用必须存在；同一文件不能由两个项目隐式共享。
成本数字为非负估计，未知为 null，basis 必须解释来源；禁止把 null 当作 0。
所有项目都可通过登记演进，无固定项目数量上限；拆分项目时同时迁移归属与测试边。

| judges 字段 | 类型与约束 |
| --- | --- |
| migration_validator | candidate 中负责迁移证据的判官 ID；当前草案为 workflow |
| judges | `{id, executable, version, sha256, argv, selector, after, modes}[]` |
| selector | v1 使用 `every-delta`；仍然只检查该次 DELTA，不扫描裁决全部历史 |
| after | 必须存在的判官 ID 数组；有向无环，稳定 ID 顺序打破并列 |
| modes | v1 仅 `evaluate`；candidate 判官读取两端事实，见 §6 |

各默认判官分别指定 `.chrono-harness/bin/chrono-judge-<id>`，均为尚未实现的独立可执行文件。
每个 ID 实现时才创建并登记 `judge-<id>` 与专属 `judge-<id>-tests`，独立 manifest/lockfile/target。
七对项目分别编译测试，不由一个总判官二进制耦合；当前存在 runner、instructions 及各自测试，不创建空判官项目。
第三方程序可改成自己的 executable/argv，但宿主调用声明仍位于 `.chrono-harness/`。

| workflow 字段 | 类型与约束 |
| --- | --- |
| target_branch / feature_prefix / integration_prefix | 分支名与前缀，当前 dev / feature/ / integration/ |
| staleness | `{max_behind_commits, max_age_hours, combine: "any-exceeded"}` |
| stability | `{id, paths: string[], tests: string[], reason}[]`；精确路径 |
| semantic_changes_require_integration | bool；当前 true |
| mixed_change | `{level: "warning", code, requires_acknowledgement: false}` |
| integration | `{tests, bind, evidence}`；精确绑定字段与状态目录路径 |
| retirements | `{kind, id, replacement: string|null, reason}[]`；显式退出/迁移事实 |
| migrations | `{from_version, to_version, script, test, mappings, reason}[]`；mappings 为旧/新记录 ID 对 |

本节每份文件有共同 schema_version/status，表格列出其余字段；不另设未登记的规则文件。
指令生成专用 manifest 使用 §16 的独立合同，没有 active/proposed 状态，不参与判官启用。
登记的语法/引用约束也由 registration 判官实施；runner 只负责 JSON 运输与协议解析。
配置不可解析、进程无法启动等基础故障是运行时错误，不是一个未声明的业务判官。

## 4. 唯一检查指令、快照与 CI 对等

实现并安装指定二进制后，本地与 CI 必须从宿主根目录运行同一条指令：

```sh
./.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/config.json \
  --base "$CHRONO_BASE" \
  --candidate "$CHRONO_CANDIDATE" \
  --context .chrono-harness/state/context.json
```

两个变量必须是已存在的完整、不可变 commit OID（按仓库对象格式为 40 或 64 位 hex）。
不得直接传 `HEAD`、`dev`、短 SHA、工作目录别名或 CI 默认 merge ref。
`base` 是此轮已获取的 dev tip；`candidate` 是拟向 dev 交付的精确提交。
DELTA 比较二者的 tree，不自动改用 merge-base，base 不必是 candidate 的祖先。
分支过期和合并关系由 workflow 判官处理；不允许 CI 悄悄改成全量检查。
本地/CI 必须运输相同 OID、context 与版本化配置；其报告绑定这些内容的摘要。
同一指令保证调用和输入解释语义一致；相同裁决还要求完整有效输入、政策、执行物、工具链、
配置和环境相同，并且判官/测试确定性求值。仅 Cargo 版本或环境摘要存在不足以证明这些前提。
前提未建立时报告 parity:unestablished；输入/证据不足仍报错，不能承诺裁决相同。

v1 明确选择干净、不可变 commit 快照这一工程边界；不另设 dirty-worktree 验证模式。
工作树必须检出 candidate，索引和已跟踪文件必须等于 candidate，子模块不得隐式展开。
未跟踪文件须登记并提交；只有 config.artifacts 中生成目录可忽略。
仅写进 `.gitignore` 不够：判官检查所有未跟踪路径，包括被 Git 忽略的路径。
生成目录里若出现跟踪文件，按普通 DELTA 文件要求登记；忽略不覆盖 Git 事实。
暂存、未暂存、未跟踪的非生成物变化或 candidate 不匹配返回 `E_SNAPSHOT_DIRTY`，不静默丢弃。
这些行为是 registration 判官的默认策略，runner 只传入 Git 状态事实。
仓库路径边界、特殊文件类型不支持时必须返回 `E_INPUT_UNSUPPORTED`，不能漏判。

`--config` 的磁盘内容必须等于 candidate 中同路径 blob；base 配置单独从 base tree 读取。
判官接收两个只用于读取的快照路径；测试在 candidate 的独立构建目录运行。
context 为 UTF-8 JSON：`schema_version, base, candidate, dev_tip, branch_ref, fork_point,`
`branch_started_at, observed_at, operation, integration_evidence`，时间均为 UTC RFC3339。
operation 固定为 validate.delta；integration_evidence 为证据摘要或 null。
context 的 base/candidate 必须与 CLI 相同，dev_tip 必须等于 base；CI 不接受悄悄漂移。
Git ancestry 来自完整对象图；浅克隆缺对象返回 `E_HISTORY_MISSING`，AI 补取后重跑。
工作开始记录 branch_started_at，observed_at 是本轮获取 dev 的时间，不用提交时间猜分支年龄。
同一轮本地/CI 使用相同 context；dev 再推进就建立新一轮输入，不能冒用旧报告。

计划中的报告写入 `.chrono-harness/state/report.json`，stdout 输出同一 JSON 对象。
CI 只负责准备工具、不可变输入并调用指令、保存报告和原样传播退出码。
不得另写 CI 专属判官、skip 参数或本地宽松模式；当前仓库不设置虚假的绿色 workflow。

## 5. DELTA 与依赖图算法

DELTA 选择充分的前提是：两状态所有实际语义输入及其直接/间接依赖均完整登记，且判官/测试
谓词具有局部性：仅由已登记的语义输入决定，未受变更输入及其闭包影响的谓词保持不变；外部输入也属于状态。
图上闭包仅是相对于给定登记图的机械事实，真实依赖完整性与局部性是工程义务，不能由合法登记推出。
未知或已知缺失的 fixture、环境、外部输入或非局部影响须报输入/证据不足，AI 显式补登记或调整检查单元；
不推断补边，不声称能自动发现所有遗漏，也不声称已检查每个历史对象或证明未来 Rust 实现。

DELTA 是 base tree 到 candidate tree 的集合差，包含路径、blob OID 和 mode。
按字节稳定排序路径，用 Git `--no-renames` 的含 NUL 输出或等价库接口读取事实。
增加 A：base 不存在、candidate 存在；修改 M：内容或 mode 不同；删除 D：仅 base 存在。
重命名规范化为 D(old)+A(new)，不依赖 Git 相似度阈值；展示可附 rename 提示，不改变裁决。
同内容改名仍须检查旧归属、旧消费者、旧测试和新路径登记。
二进制文件也按 blob 比较；没有文本 diff 不等于没有 DELTA。

第 1 步：读取 B/C 两份登记，解析变更文件及变更的记录 ID、字段和边。
第 2 步：建立 `G = G_base ∪ G_candidate`，保留每条边来源 base/candidate/both。
第 3 步：变更文件、已登记有效输入的两端变化为种子；登记变化另把变更记录目标加入种子。
例如只删除 `runner → runner-tests` 测试边，也把 runner 与旧测试作为影响种子。
第 4 步：沿已登记的有类型正向边计算影响闭包，去重但保留来源和因果路径。
第 5 步：选出候选测试、判官、成本项；验证删除/迁移事实后执行候选中仍存在的测试。
第 6 步：每个 finding 必须关联一个 DELTA 路径或登记变更，以及明确的依赖因果链。
没有因果链的旧缺陷不应成为本次错误；可作为未裁决 context 列出。

| 边 kind | 方向与含义 | 不允许的推断 |
| --- | --- | --- |
| compile | 输入文件→项目；依赖项目→消费者项目 | 不解析 Rust import 来补关系 |
| build-input | manifest/lock/生成器/外部输入→项目 | 不把所有根目录文件当公共构建输入 |
| runtime-input | 已明确选择的资产/宿主 canonical 材料及为保留原文、校验标记而读取的已有输出字节→消费项目 | 不将运行数据伪装成编译输入，也不由运行时扫描补登记 |
| test-execution | 受影响项目/文件/脚本/输入→测试 ID | 不按文件名、测试名、Cargo 元数据猜测试 |
| judge-trigger | 文件/项目→判官 ID | 不凭扩展名选判官 |

compile/build-input/runtime-input 可沿显式项目 compile 边继续传播，但只有 test-execution 边能选测试。
配对关系表达所有权，不隐式产生测试边；缺配对执行边由 projects 判官报错。
every-delta 判官始终被选择，judge-trigger 提供额外影响解释；二者取并集去重。
依赖闭包需要读取未变文件和历史登记，这是读取上下文，不是重判历史文件。
若某个声明的测试单元会验证完整项目，允许执行它；测试由 DELTA 选出且故障须说明因果。
与该 DELTA 无因果关系的既存测试失败标为 `E_EVIDENCE_UNRESOLVED`，不能捏造通过或旧违规。

删除文件从 base FILEMAP 取得 owner/依赖/成本；不能因为 candidate 不再登记而丢失影响。
新增文件必须在 candidate 注册；修改文件必须在两端有可解释的注册或显式补登记记录。
完整登记表可读作上下文；初始完整性检查只要求本轮新增、变更、受影响引用闭合。
未改动、无影响的历史缺登记留给后续明确 migration DELTA；不假造全库已经合规。
schema 不可解析影响本轮输入时是错误；不能以“历史问题”忽略无法构图。

## 6. 登记变化、移除与演进

登记自身也是代码数据，其 A/M/D 进入 DELTA；不使用 candidate-only 图或“最后配置赢”。
FILEMAP 新增文件行、调整 owner、登记实际 build 输入/测试边是 membership 变化。
引用目标删除后仍存在的消费者报 `E_DANGLING_EDGE`；AI 修改消费者或明确登记迁移。
删除测试项目必须同步迁移生产项目的专属测试及执行边，不能留下一个无测试项目。
合法同时删除生产项目与其测试时，workflow.retirements 明确列出二者及原因。
旧测试计入 `retired_tests` 和成本影响；不存在的候选测试不被误标为已执行。
旧测试若无合法 retirement 或 replacement，返回 `E_REQUIRED_TEST_REMOVED`。
替代测试必须在 candidate 真正执行，旧成本依然可见；同一方法不永久冻结项目结构。

删除/替换判官要登记 `kind: "judge"` 的 retirement、replacement 或 null 和原因。
只执行 candidate 指定的当前判官，以 evaluate 模式按新政策判本轮 DELTA。
candidate 的 migration_validator 消费 registration/filemap/cost 的结果，核对 base 登记、
旧边、成本、retirement/replacement、迁移映射与 integration 证据；旧谓词不永久生效。
base 只提供历史事实，不启动旧判官，也不要求取得、重建或用适配器包裹失效旧执行物。
当前验证器可自主替换；其自身替换同样保留两端证据并经过新候选的 integration 验证。
不自动下载猜测的插件、不把未知删除当“关闭成功”；有意规则调整是正常自主工程动作。

schema 版本变更要提供登记迁移：旧/新版本、记录映射、删除原因和配对兼容测试。
workflow.migrations 中 script/test 必须引用已登记脚本，mappings 元素为 `{from, to}`，
from/to 可为 null 表示新增/删除，但不能同时为 null；删除原因放同条 reason。
仅实际 schema 转换需要迁移脚本及配对测试；同 schema 替换判官无需创建适配器。
candidate registration 按登记脚本处理旧 schema，保留原始记录、映射和摘要；证据不足报错，不落回默认配置。
新判官 ID、项目、语言和适配器都经白名单扩展；runner 无需知道当前项目全集。

## 7. 判官、脚本与插件协议

每个判官是显式 executable + argv，cwd 是 candidate 根目录，进程环境来自已登记工具。
启动直接传 argv，不经过 shell；需要 shell 的脚本必须显式指定解释器、脚本及其依赖。
stdin 恰好一个 UTF-8 JSON 文档，读到 EOF；stdout 恰好一个 JSON 文档，日志仅走 stderr。
脚本可用 Python、shell、Node 等；插件后端必须由登记适配进程转换为同一协议。
不要求 Rust dylib ABI、常驻 daemon 或自动插件发现；超时和输出上限来自 config.protocol。

请求的规范结构如下。尖括号为类型占位符；本例假设 base 已登记 old.rs 且无消费者，不是当前仓库快照：

```json
{
  "protocol": "chrono-judge/v1",
  "request_id": "<sha256 of canonical request without request_id>",
  "judge_id": "filemap",
  "mode": "evaluate",
  "base": {"commit": "<full oid>", "tree": "<tree oid>", "root": "<snapshot path>"},
  "candidate": {"commit": "<full oid>", "tree": "<tree oid>", "root": "<snapshot path>"},
  "delta": [{"kind": "D", "path": "old.rs", "old_blob": "<oid>", "new_blob": null,
             "old_mode": "100644", "new_mode": null}],
  "registries": {"base": "<snapshot .chrono-harness path>",
                 "candidate": "<snapshot .chrono-harness path>", "digest": "<sha256>"},
  "context": {"path": "<context path>", "sha256": "<sha256>"},
  "impact": {"seeds": ["file:old.rs"], "edges": [], "tests": [], "retired_tests": []},
  "prior_results": [{"protocol": "chrono-judge/v1", "request_id": "<registration request_id>",
    "judge_id": "registration", "status": "pass", "findings": [], "evidence": [], "outputs": {}}]
}
```

impact.edges 为 `{from, kind, to, origin}`；origin 是 base/candidate/both。
prior_results 仅含 after 列出的前置判官完整响应；不得从运行目录猜测旧输出复用。
registration 先消费原始快照与 context；filemap 据登记生成 impact，其他判官显式消费结果。
routes 先验证执行入口，projects 再执行测试；judges.after 中明确登记这个依赖顺序。
共享上下文的推导属于登记判官逻辑；runner 只传递具名输出，不在内部另写项目政策。
所有摘要使用 SHA-256；JSON 摘要使用 RFC 8785 JCS，文件 blob 按原始字节计算。
registries.digest 覆盖两端五份 JSON 的路径与内容；不存在的初始端须显式写 null。

```json
{
  "protocol": "chrono-judge/v1",
  "request_id": "<same request_id>",
  "judge_id": "filemap",
  "status": "pass",
  "findings": [],
  "evidence": [],
  "outputs": {"impact": {"seeds": ["file:old.rs"], "edges": [], "tests": [], "retired_tests": []}}
}
```

status 是 pass/warn/fail/error；每个 finding 含 `code, level, message, delta_refs, causes`。
delta_refs 是本请求 DELTA 路径或登记记录 JSON pointer；causes 是注册边路径数组。
evidence 条目为 `{path, sha256, kind}`，路径须位于 state；不能用不存在的输出作证明。
outputs 是登记的具名结果，filemap 的 impact 按上述结构；无结果时为 `{}`。
请求/响应 ID、协议、状态、必需字段或实际退出码不一致时返回 `E_PROTOCOL`。

| 子进程/总检查退出码 | 含义 |
| --- | --- |
| 0 | pass 或仅 warn；报告区分它们，warn 不要求确认 |
| 1 | 至少一个 fail：已完成判定，存在约束错误 |
| 2 | error：输入/协议/基础设施失败或必要证据不足 |
| 3 | 仅当前 CLI 使用：功能尚未实现，不是 pass/warn |

子进程输出 fail 却 exit 0、空 stdout、额外日志、截断 JSON、未知状态都属于协议错误。
本轮选中的当前进程崩溃、超时、不可执行、摘要不匹配、必需测试未执行都不能降级为通过。
总结果优先级 error > fail > warn > pass；基础错误阻止依赖任务，独立任务可以继续收集证据。
被阻止的任务列为 blocked，不能省略；完整通过必须有全部已选判官/测试结果。
测试操作本身不必输出判官 JSON；projects 判官调用登记操作并包装退出码、工具和结果摘要。
程序 stdout/stderr 是证据内容，不能靠查找字符串 PASS 代替进程状态。

## 8. 默认判官、规则表面与警告

当前 judges.json 登记以下计划判官；全部尚未实现，不存在默认隐形 gate。

| ID | 本轮职责与典型错误 |
| --- | --- |
| registration | schema/引用/白名单/快照/输入闭合；E_UNREGISTERED、E_SNAPSHOT_DIRTY |
| filemap | 两端图、类型边、归属、影响；E_DANGLING_EDGE |
| projects | 一一配对、项目隔离、所选构建/测试执行；E_TEST_PAIR、E_TEST_FAILED |
| routes | 唯一操作与 argv/证据一致；E_ROUTE_MISSING、E_ROUTE_AMBIGUOUS |
| cost | 显式成本与旧/新影响报告；未知值 W_COST_UNKNOWN |
| mixed | 规则与产品同一 DELTA 的可见成本警告；W_MIXED_JUDGE_PRODUCT |
| workflow | dev/integration、新鲜度、稳定性及登记迁移证据；E_BRANCH_STALE、E_INTEGRATION_REQUIRED |

FILEMAP.surface 显式声明整个文件表面，不通过文件内容、目录名或语言模型猜语义。
membership 文件中的部分字段通过 config.semantic_fields 声明为语义表面：
projects 的 actions 新增/修改/删除涉及治理方法；FILEMAP 已有行的 surface 改动/删除涉及分类。
pointer 的 `*` 仅匹配一个对象 key 或数组元素，不是文件路径通配符；行用 path/id 定位比较，
不能因 JSON 数组换序误判规则变化。schema_version 改动按登记 schema 迁移处理。
对同一文件取 base/candidate 表面分类并集，不能通过本轮重分类遗失旧语义影响。
config/judges/workflow 全文件为 judge-policy；独立判官脚本可登记为 judge-implementation。

代码加对应 FILEMAP 普通成员行或真实依赖边，是通常的代码+登记协同修改，不触发混合警告。
已有判定字段、动作、规则配置、判官实现或绑定二进制摘要变化属于规则语义变化。
此类变化与 product/test 文件修改同一 DELTA 时，mixed 输出 warning 及两组路径、旧/新成本。
warning 不升级成 blanket block、不索要批准、不要求人为确认或另起提交才能继续。
integration 是这类规则变更本身的稳定性要求，并非混合提交惩罚。
AI 先尝试在现有规则下解决任务；确需改判官时自主变更并验证，报告为什么和影响多大。
源码自举的 engine 修改只按显式 stability 登记要求 integration，本身不被当作宿主 policy。

## 9. 成本与执行报告

报告必须包含 `schema_version, status, base, candidate, candidate_tree, context_digest,`
`registry_digest, executables, tools, environment, effective_inputs, parity, delta, impact, judges, tests, findings, costs`。
executables 为 `{path, sha256, version}` 列表；tools 另记录实际解析路径和工具版本。
effective_inputs 记录两端完整有效输入的 ID、实际值摘要及来源证据；摘要存在本身不证明相等或完整。
parity 为 `{status: "established"|"unestablished", compared_report: string|null}`；仅在与具名报告比较、
确认 §4 全部条件及确定性证据时可 established，否则 unestablished；与性能测量差异分别报告。
judges/tests 记录每项的 selected/executed/blocked/retired 状态、退出码和证据摘要。
这里 selected 是执行计划状态，完整结果不能停留在 selected 就返回 pass。

costs 报告每个变更文件、影响项目、选择测试及旧删除节点的成本引用和来源。
四维向量分别为 CPU 时间、墙钟时间、峰值内存、IO 字节；不折算成虚构统一分数或罚税。
展示 declared_before、declared_after、affected_tests、retired_tests 和 unknown 列表。
不同资源的峰值不能直接相加；并行 wall 时间也不能当串行总和。
可按显式串行计划估算总 CPU/IO，任何输入未知则总估计注明不完整，不填 0。
实际测量放在 measured 下，附工具、样本与时间；不能把构建耗时冒充测试耗时。
当前所有成本 unknown 是诚实的初始登记，未来 cost 判官应 warning，不能捏造基准数据。

## 10. dev、integration 与过期分支

所有交付最终进入 dev；feature 与 integration 均从当时最新 dev 创建。
workflow 当前草案阈值：落后 dev **超过 3 个提交**，或本分支年龄 **超过 24 小时**，
任一成立即 E_BRANCH_STALE；等于阈值仍允许。二者在 workflow.json 可配置。
behind 是 fork_point 到 context.dev_tip 的提交数；年龄为 observed_at − branch_started_at。
fork_point 必须是 branch/candidate 与 dev 的共同祖先；负年龄或不完整记录是输入错误。
分支名匹配受登记前缀，dev 上的合并结果以其已登记来源分支 context 判定。

过期后 AI 自主保留旧分支为恢复点，将尚未保存工作先落在旧分支或外部补丁，
获取 dev，创建新名字 `feature/<task>-r2` 或 `integration/<task>-r2`，从最新 dev 开始。
逐项重新应用仍有必要的变化，解决冲突和需求漂移；不能把旧分支 merge 进新分支假装新鲜。
重新登记输入、构建、测试并生成新报告；旧报告与旧 integration 证据失效。
旧分支仅作可回退材料，完成前不删除工作；该流程不要求人介入。

判官语义/绑定改动或 stability 路径变动：先创建 fresh integration，组合拟交付完整变化。
integration 上执行同一 check 指令与注册稳定性测试；成功报告作为 integration 证据。
workflow 判官在 integration 上标记 `integration_run`，执行所需测试后生成证据，
不会递归要求本轮先提供自身成功证据；feature/dev 交付才消费已完成的 integration 证据。
纯普通变化在 feature 验证后可进入 dev；AI 可显式新增 stability 登记扩大测试需求。

integration.json 为 `{schema_version, base, candidate_tree, registry_digest, executables,`
`tools, environment, effective_inputs, required_tests, results_digest, branch_ref, fork_point, branch_started_at, observed_at, status}`。
绑定 base、候选 tree、两端登记摘要、执行二进制/完整工具链、有效输入与环境、所需测试及结果摘要。
证据放 state，不纳入 tree，从而不发生“报告包含自身提交摘要”的循环。
进入 dev 前最终 candidate tree 必须完全等于测试 tree，base 必须仍是最新获取的 dev。
允许 commit OID 因合并元数据不同而变，只要 tree/base/其他绑定字段相同；报告写明映射。
若合并冲突处理改变 tree 或 dev 推进，重新建立 integration 候选并测试，不能复用旧绿灯。
缺失、失败、过期或绑定不匹配证据分别为 E_INTEGRATION_REQUIRED / E_INTEGRATION_MISMATCH。
报告来自可信但可能出错的 AI/runner；摘要用于识别误用，不设计对抗 AI 的授权系统。

## 11. 独立脚本和插件扩展示例

以下是未来新增脚本的登记形状，不是当前 files/scripts 里已经存在的文件：

```json
{
  "id": "format-report",
  "path": ".chrono-harness/scripts/format-report.py",
  "test_script": "format-report-tests",
  "actions": {"execute": {"operation": "script.format-report",
    "tool": "python3", "argv": [".chrono-harness/scripts/format-report.py"]}}
}
```

同时登记 python3 工具、测试脚本、两者 FILEMAP 行、compile/build-input/test-execution 边及成本。
测试脚本以 tests_for 指回 format-report；协议适配脚本登记为 judge-implementation。
任意插件后端也由这样的具名适配程序接收 JSON，输出统一 findings；backend 的版本、
配置、数据文件、环境输入都作为明确依赖登记，不能在启动时扫描目录扩充能力。
对于无法可靠列举输入的插件，返回 E_INPUT_UNDECLARED，AI 补登记或选可描述的适配方法。

## 12. 初始根提交与自取自举

空仓库没有 base，允许一次明确标记的根提交存储 spec/scaffold/草案登记。
初始根提交不是一次通过的 DELTA，也不生成“所有历史已合规”的报告；当前正处此阶段。
随后修改都有真实 base；初次启用仍按 §6 由候选判官检查 activation DELTA 和迁移证据，并通过 integration。
base.status 为 proposed 时记录 previous_enforcement:none，不能回填之前的成功；根提交之外不得伪造空 base。
草案与 active 的迁移都不执行旧判官；启用必须填完 input_closure，声明完整仍不等于数学证明。

未来自举按登记方式从指定产品提交构建，将结果放宿主 `.chrono-harness/bin/`，
写入具体版本、产品源提交、二进制摘要和工具链证据后，再调用唯一 check 指令。
源提交记录在 state 构建证据中，由绑定摘要连接；源码与宿主 policy 始终是两个表面。
更新已生效二进制绑定是规则变化，须 integration；AI 可自行完成，无人审批。
构建失败、缺组件或 draft 配置不得生成成功报告；从本机取包也不得默默解析 latest。

## 13. 可执行验收矩阵（目标行为）

除最后四行外均为未来运行时验收，不能以当前 cargo test 通过来宣称完成。
以下充分选测结论均以 §5 的实际输入完整性与局部性为前提；图闭合本身不提供这些保证。

| 场景 | 预期裁决或可观察结果 |
| --- | --- |
| 已登记源码 M，有显式测试边 | 仅闭包所选项目/测试执行，记录 DELTA 因果 |
| 新文件 A 未登记，包括未跟踪/被 ignore 文件 | E_UNREGISTERED 或 E_SNAPSHOT_DIRTY；非零 |
| 新源码与普通 FILEMAP 行同改 | 正常执行测试，无 W_MIXED_JUDGE_PRODUCT |
| D 文件且 candidate 删除其 FILEMAP 行 | base 边仍保留；选择旧消费者测试和旧成本 |
| 同内容 rename old→new | D+A；检查两端登记，不依赖相似度 |
| 只改测试依赖边或删除边 | 变更记录成为种子，保留登记图的旧/新影响并集 |
| 删除生产项目与专属测试且有合法 retirement | retired 可见，剩余引用闭合；不假称旧测试已执行 |
| 删除仍需测试、缺 replacement | E_REQUIRED_TEST_REMOVED |
| 两个项目共用一个专属测试或缺配对 | E_TEST_PAIR |
| 修改 manifest 且发现依赖未登记 | E_DANGLING_EDGE / E_INPUT_UNDECLARED；不自动补边 |
| fixture/间接/环境/外部输入未知或已知缺登记 | 输入/证据不足；不因图闭合假称完整，不推断边 |
| 只改 README，未登记测试执行边 | 不启动所有项目测试；登记判官仍只判该 DELTA |
| 增加脚本和独立测试完整登记 | 只执行显式测试边选择的操作；无自动发现 |
| 同 operation 注册两种入口 | E_ROUTE_AMBIGUOUS |
| 绕开受治理入口、缺少绑定报告 | E_ROUTE_MISSING / E_EVIDENCE_UNRESOLVED |
| 规则语义与产品同改，已有有效 integration | W_MIXED_JUDGE_PRODUCT；仅此警告时 exit 0 |
| 产品自举 engine 源码变更、无 policy 修改 | stability 触发 integration，不自动归为混合规则修改 |
| policy 或 stability 变更缺 integration | E_INTEGRATION_REQUIRED；AI 自主建立 integration |
| integration 已通过但最终 tree 或 base 改变 | E_INTEGRATION_MISMATCH；重测 |
| feature/integration 落后 4 提交或年龄 25 小时 | E_BRANCH_STALE；从最新 dev 重建 |
| 恰落后 3 提交、年龄 24 小时 | 不因阈值本身拒绝，其他要求仍适用 |
| 本地与 CI 同一 check 指令 | 同调用/输入解释语义，不单凭命令保证裁决相等 |
| 完整有效输入/政策/执行物/工具链/环境相同且确定性求值 | 相同选择和裁决；不要求非裁决性能测量值相同 |
| 仅 OID/context/Cargo 相同，其他输入或确定性未证实 | parity:unestablished；输入/证据不足仍报错 |
| 暂存/未暂存/未跟踪非生成物变化 | E_SNAPSHOT_DIRTY，不偷偷只判 committed 子集 |
| 浅历史缺 base 或 fork_point | E_HISTORY_MISSING；补取后重跑 |
| 判官退出成功但 stdout 为空/非法 JSON | E_PROTOCOL，exit 2 |
| 插件崩溃/超时/摘要不匹配 | error，exit 2；不得 fail-open |
| 全部成本未知 | W_COST_UNKNOWN，unknown 明示，不编造测量 |
| 替换/退休判官，旧二进制缺失、崩溃或不支持新 schema | 当前 migration_validator 核对保留的旧事实与迁移证据；candidate evaluate + integration，无旧执行前提 |
| 无 DELTA | 报告 delta=[]，不裁决历史；输入/分支条件仍检查 |
| 首个根提交 | bootstrap/no prior base，无绿色 DELTA 报告 |
| 当前 CLI：check，含貌似正确参数 | **已实现** E_NOT_IMPLEMENTED，exit 3，stdout 空 |
| 当前 CLI：未知命令/信息命令多余参数 | **已实现** E_USAGE，exit 2 |
| 当前 CLI：spec status | **已实现** draft/not-implemented/proposed，exit 0 |
| 当前 CLI：help/version | **已实现** 信息输出，exit 0；help 提示 check 未实现 |

## 14. 实现阶段与实际验证范围

runner 已实现：无外部依赖的 Rust library + binary；公开 `dispatch(&[&str]) -> CliOutput`；
帮助、版本、规格状态；check 恒非零；未知命令/多余参数非零；输出失败不报成功。
配对 crate 验证公开边界的四类合同；binary 仅负责参数、stdout/stderr 和退出码运输。
每个项目的独立 Cargo.lock 必须提交；构建和测试使用各自 --manifest-path 与 --locked。
测试 crate 只有显式 test target，build/check 需带 --tests，避免只检查空默认目标。

instructions 已实现：内嵌默认数据的一条 init，独立输入/初始 locale 绑定，显式原子/翻译/依赖与输出计划，
确定性组合为根正文、登记 Markdown 与有效 skill，旧 v1/v2 opaque file atom 前向迁移、宿主字节保留、
严格引用/翻译/所有权/路径预检、相对链接、无写入重跑和多输出普通失败回滚；完整合同见 §16。
serde/serde_json/tempfile 锁定依赖仅属于生成器，不进入 runner 编译依赖。

尚未实现：通用判官登记解析与 schema 验证器、Git DELTA、测试影响图算法、判官二进制/脚本协议、
测试调度、成本报告、分支/integration 检查、自举安装、正式 check 与 CI gate。
JSON 草案和验收矩阵是这些实现的输入，不是假执行报告；没有覆盖率或 CI 通过徽章。
先实现 registration/filemap/protocol 并测试失败路径，再实现 projects/routes/cost，
最后实现 workflow/mixed、自举与同一指令的 CI 集成；每一步仍须明确未实现能力。
不在这个 spec-first 版本启动 daemon、远端服务、插件市场或全量约束运行时。

## 15. 实际参考经验

只读参考 trureturing 提交 `fbac8071cc862c8a43a247db2d8101dc61f655ea`：

- [Meta/FILEMAP.toml](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/FILEMAP.toml)：已有 resources/materials/require 登记；本设计保留显式影响，进一步逐文件化、区分边类型。
- [Meta/engineering-projects.json](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/Meta/engineering-projects.json)：明确 production、专属测试、references/build_inputs/execution_inputs；本设计避免从源码推断测试。
- [harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/.github/scripts/harness-gate.sh) 与 [local-harness-gate.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/local-harness-gate.sh)：实际入口传入 base、候选和报告；本设计进一步固定同一 argv，去除本地/CI 入口差异。
- [judge-content-address.sh](https://github.com/the-omega-institute/trureturing/blob/fbac8071cc862c8a43a247db2d8101dc61f655ea/tools/scripts/workflow/judge-content-address.sh)：判官地址包含 source/runtime/architecture/sdk；本设计同样绑定执行物与工具证据，避免复用错报告。

这些是经验来源而非兼容承诺，不复制已有复杂体系，不读取其运行时配置，不把它当依赖。
本项目不会修改 trureturing，也没有宿主全局 hook、权限门禁或人工流程。

## 16. 宿主指令生成（已实现）

`chrono-instructions init --host-root H` 从内嵌产品 catalog/default manifest 采用独立宿主数据，默认只生成中文根正文与 AGENTS 字面相对链接。17 项通用规则有稳定 ID、中文/英文 variant 与显式稀疏内容依赖；普通空文本 aggregate 组合全套，聚焦 skill 可只选择修复产生处及其证据/复用前提。无需运行时 checkout、自动语言推断、网络翻译、包解析或新平台。

当前 schema 2 / atomic-rules/relative-alias/v3 的 output plan 显式声明输出身份、路径、格式、locale、根引用及必要元数据。确定性 DFS 依赖先行、共享 atom 每输出仅一次；引用/循环/重复身份与选中闭包缺翻译报具体错误。source variant 是 inline 或宿主 .chrono-harness 下的 file，原 UTF-8 字节保留。程序不认证翻译语义等价或组合的语义完整性。

根受管块保留宿主块外原文、sole donor 和预期整份比较语义。Markdown/skill 为带身份 envelope 的整文件投影，未拥有/畸形现有文件预写入拒绝。skill frontmatter 从首字节开始，元数据显式验证。当前 manifest 是唯一管理计划，删条目/改名保留旧输出，由授权 AI 显式退休；不建立历史 ledger 或扫描删除器。

已知完整旧 read-both/v1 和 literal-core/relative-alias/v2 自动迁为 und file atom，保留旧方法/上下文的精确字节、路径、普通权限与根块外原文，不自动拆分、猜语言或采用新默认。init --locale 只绑定新根；省略选项保留采用数据，显式冲突报错。全计划预检、路径/链接/实际别名检查、父先子后建目录及多输出回滚复用现有 publisher；manifest 最后发布。当前 macOS 普通 IO 为已测边界，无跨平台、崩溃/并发保证。

完整 schema、组合配方、CLI、迁移、所有权与恢复合同由 [docs/instructions.md](docs/instructions.md) 单一维护。
[来源说明](docs/methodology-extraction.md) 定义实际通用核心与双语原子的保真边界。自举由同一工具生成中文根、英文 Markdown 与聚焦 skill；产品资产/宿主采用数据/生成投影均显式登记，不是第二份手工政策。
五份通用登记仍 proposed、input_closure 仍 incomplete；专用 manifest 校验与内容图解析不代表通用判官、DELTA、CI gate 或 AI 遵守已经实现。
