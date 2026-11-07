# 输入来源的保管与当前采用情况

这里保留的固定来源是 PR163 的 `65d577f0c570451b674d271642d385b316c7085b`、独有源码 `87a77182c5da33627deb3b837e7515748eb6fd58`（文件树 `adf79eba9137f7c3822c11b138534c953229e94c`），以及 PR205 的 `8dc3203e9d73a622326e72249ee72ed21bdeee32`。它们只作原始源码和数据，不能当作候选检查程序执行。

调用方负责维持 Git 引用可达，并交接工作者的原始源码归档和校验值。归档要保留到源码协调、评审、历史原件义务和原 PR 处理都由实际消费者确认完成。它们不会因此成为永久运行依赖，本文也不授权自动删除。

| 保留内容和用途 | 负责人 | 当前处理情况 |
| --- | --- | --- |
| 观察、采集、准备、运行时源码及原同语言测试 | inputs 和宿主 bootstrap | 已采用可兼容的源码；当前绑定、安装后受保护的使用和真实 CI 验收仍待完成 |
| Cargo v6、directory v3、通用保护测试及目录迁移 | judge-cargo | 已采用可兼容的源码；固定 51 原生测试仍只证明其有限范围，更广的当前候选验收待完成 |
| PR163 全部并列保护政策和操作，包括 runner 构建 | 宿主 Cargo 政策 | 保留原 33 政策／95 操作映射；当前方法使用 95 条直接 Cargo 配方；额外 diagnostics 保护链已退役，不强制恢复旧的并行保护别名 |
| 原始 package／resolver 参数、33 个并列 package-file 绑定和七个完整检查程序绑定 | 宿主输入政策及各具名消费者 | 保留原件；不声称 PR205 更窄的声明与其含义等价 |
| rust／python／sdk／developer 四份原本地清单 | 输入政策 | 保留源码和测试样本；历史个人路径及预期摘要不是当前宿主的依据 |
| bootstrap 发布选择、回执／载荷核验、共享发布／导入／验证和专属 Python 测试 | 宿主 bootstrap 和 distribution | 有限的真实 CI core／detector 源码供应和原载荷已核验；默认 14 工具安装、便携／发布采用和整分支退出尚未核验 |
| 主宿主 Cargo 获取／保护实验和外层超时下原件发布测试 | inputs 和 judge-cargo 测试 | 原源码保留；历史执行不能证明当前候选合格 |
| 历史 e21／3083 两端快照及外部保留 blob | 输入传输与调用方保管者 | 已在允许来源完成恢复查找，未找到合格原件；别处是否存在仍未知，现在重新采集不能替代历史原件 |

SPEC §3 规定 Cargo 保护是可选的。当前方法、原断言、原样参数和输入统计义务仍保留。固定源码 `51d85d833e4fdf9f7a607af901bfa52d3e64939a` 的真实 CI 运行 `41021444210/1` 核验了对应 95 个当前操作中的 37 个：36 个直接操作和一个受保护构建。其余 58 个仍有登记，但该组没有合格执行证据；不能据此认定它们已过时，也不能据此要求全部执行。这些固定来源回执不能证明后续候选通过。

以下是 PR163 的 33 个原政策路径，统一以 `.chrono-harness/cargo/` 为前缀。当时的 diagnostics 政策是单独选择采用的，不能由此推导每个历史政策都必需。

- `cache.json`
- `cache-tests.json`
- `ci.json`
- `ci-tests.json`
- `diagnostics.json`
- `diagnostics-tests.json`
- `distribution.json`
- `distribution-tests.json`
- `inputs.json`
- `inputs-tests.json`
- `instructions.json`
- `instructions-tests.json`
- `judge-cargo.json`
- `judge-cargo-tests.json`
- `judge-ci.json`
- `judge-ci-tests.json`
- `judge-cost.json`
- `judge-cost-tests.json`
- `judge-filemap.json`
- `judge-filemap-tests.json`
- `judge-mixed.json`
- `judge-mixed-tests.json`
- `judge-projects.json`
- `judge-projects-tests.json`
- `judge-registration.json`
- `judge-registration-tests.json`
- `judge-routes.json`
- `judge-routes-tests.json`
- `judge-workflow.json`
- `judge-workflow-tests.json`
- `runner-tests.json`
- `worktree.json`
- `worktree-tests.json`

PR163 固定 config blob 为 `4690d372cc8ecf89bf1256eeee8b119024449dd3`，runner 政策 blob 为 `7fced91018b447989ccec39eff25f727c9a16b62`。PR205 保留的 native／macos 测试样本分别为 `14a5b9f51c216861055e6b4067c2b3e36cc7d6c4`、`38c73f003b417c6291c02d935fe2449a606a0245`。这些内容不同的原件仍是固定 Git 数据。

本仓不再把历史 native 样本及其源码形状比较作为当前测试输入，当前采集测试读取观察声明。没有历史 native 样本被当作已采用的完整政策。四份本地清单在 PR163 和 PR205 中字节相同，但这不证明已删除政策相同，也不证明实际输入已齐全。

PR163 的三份固定 bootstrap 配置明确使用了 distribution-release.json：bootstrap-core.json、bootstrap-detector.json 和 bootstrap.json。bootstrap.py 负责安装和验证，bootstrap_shared.py 传递并重新验证安装载荷与原始回执。因此，先前“这是可选分支且没有消费者”的结论已纠正。当前 distribution 产品已提供启动程序，原 install.py 与 assets/distribution/bootstrap.py 的 blob 同为 `1508d9f71c7f72a4945ff126bcd271abe1bfbeb1`。保留来源不要求重复造安装器，也不强制采用 beta.21。

<a id="remaining-consumer-prerequisites"></a>

## 各消费者还缺什么

每行只约束明确的消费者或范围，不阻断无关的已授权工作。详细方法和身份仍在原映射与生产报告中。

| 范围 | 负责人和待验收内容 |
| --- | --- |
| P1 普通完整输入 | 输入负责人：绑定源码的所选根元数据、features、packages、配置／home 和编译器／SDK／运行时原件；普通按范围 Cargo 回执不能补出这种完整对应 |
| P2 独立 runner | 额外 guard 提案已按当前必要性合同退役；日常 runner 检查继续使用锁定 Cargo 操作。旧实验原件和失败结论保留给原保管者，不列为待激活入口，也不认定历史验收通过 |
| P3 本地 diagnostics | 额外保护采用缺少必要验收终点，已退役；普通锁定 Cargo 构建和真实行为测试保留。历史原件继续由原保管者维护，不再列为待激活入口 |
| P4 七个完整检查程序绑定 | 完整宿主／bootstrap 负责人：匹配默认配置的安装字节、生产者／消费者原件、两端输入和声明的完整范围；proposed 的空绑定仍未合格 |
| P5 默认／发布安装 | bootstrap／distribution 负责人：默认 14 工具的安装后使用，以及所选便携／发布替代；产物 `12961832338` 已核验全部 18 个原成员和五个可执行载荷，包括核心工具 1–4，但只用于精确的原生 51 core／detector 供应 |
| P6 历史原件保管 | 原采集／归档负责人：真实 e21／3083 快照及引用 blob；先前允许范围的恢复未取得它们 |

先前受保护所有者修正 `d04bf3004d17f438bfcbadde3bd4d85023eb08c3` 保留了 3 项 Rust／68 项 Python 专项测试、受保护构建，以及对原未受管理消费者的拒绝。后续候选 `b09f4506cba077b9ecb8b15e4aa8823467965c76` 完成受保护 bootstrap 发布和实际所有者采用，并核对安装载荷摘要及原 Rust 采用回执。这仍不证明完整宿主范围通过。

PR163 已于 2026-11-01 按历史保管目的关闭，剩余义务转入 PR214。PR172、170、205、208、209、211 也已处理并保留源分支。关闭 PR 不代表功能交付。各自变更仍须核对当前候选检查、本地／关联工作树／真实 CI 采用、实际落地 SHA／文件树，以及安装后消费者的状态。

真实 CI 负责人保留原 Developer 别名越界等失败或缺失范围、作业／进程输出，以及 run／attempt／workflow 身份。生命周期 L1／L2 和未知原生结果仍各有待办。调用方负责源码／引用保管与生命周期交接；本文不触发部署、finish、maintain 或 cleanup。
