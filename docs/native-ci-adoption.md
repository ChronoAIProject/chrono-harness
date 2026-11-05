# 生成的 CI 单元在真实仓库中的验证

本文保留 `chrono-github-units/v1` 在真实 GitHub Actions 中的采用证据。公开 Go、TypeScript 和混合示例都在 `pull_request`、`push` 两类事件上运行了生成的汇总工作流及全部登记单元。下表运行都于 2026-09-29 UTC 成功完成，使用表列候选提交。

这批 beta.14 证据只证明生成单元工作流及汇总已经实际运行，不证明有效输入齐全、宿主采用 `chrono-github-ci/v3`、本地与 CI 普遍同判或完整七程序检查已启用。

## 固定运行记录

| 仓库 | 候选提交 | 登记单元 | PR 运行 | push 运行 |
| --- | --- | --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `ebeb022df573cb907ff1659c29844722a9f569f5` | `harness`, `rates` | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618475), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618481), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618430) | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589716), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589723), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589731) |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `8b55a765dfdd8d3ff8b4ed0f82df69e1ba36e6bb` | `harness`, `labels` | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896292), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896319), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896450) | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854511), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854173), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854228) |
| [混合语言](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `5137df028de4b1bf21d21825fb67483fcd7b78cf` | `harness`, `labels`, `normalize`, `rates` | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900081), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899959), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900164), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900085), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899957) | [汇总](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853951), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853836), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853948), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853834), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853905) |

表中混合示例最后一个 `rates` push 链接为 `36624853905`。

## 本地与 CI 的命令一致

单元日志显示 `chrono-ci prepare` 选中登记单元，准备后的工作流记录并调用与本地相同的检查命令：

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --unit UNIT
```

各汇总工作流取得固定的单元尝试，向同一入口传入明确汇总参数：

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --collect .chrono-harness/state/collection/manifest.json
```

运行 ID 和候选摘要保留，后续可加入新观察，但不能拿一次旧成功保证当前工作流。

<a id="native-git-policy-adoption"></a>

## 在真实 CI 中采用 Git 政策

上表原成功运行仍只证明 beta.14 范围。Go 的 [PR #18](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/18) 采用公开 beta.18，并为按范围检查和 provider v3 事件准备选择同一平台配置，逐项登记原生 Git 文件、预期版本、环境及受保护的 global／include 文件。

六个已选工具、SDK 配置和三份独立工作流字节保留。这是按范围检查；完整真实 CI 启用、实际输入齐全和本地／CI 确定同判仍未完成。

通过验收的候选为 `ada0b39e2adfe4adf6cd51440317f3944c9abb98`，基线为 `9b008540944976258d2ec61857cfb34b17474d8a`。合入 dev 的 `6cf641fbd1e7828d922262884b2d735de3235ed0` 与候选文件树相同。

本地 macOS 标准 rates／harness 检查执行四项操作，包括十项 bootstrap 测试及登记 Go 构建／测试。原报告分别为 50,296,479、50,284,966 字节，标准汇总通过。global 配置漂移、原声明缺失的 include 文件出现，都会在业务前失败；恢复后空变更通过，执行零操作。

| 真实 CI 事件 | harness 单元 | rates 单元 | 原报告汇总 |
| --- | --- | --- | --- |
| integration 分支 push | [36668775068](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775068) | [36668775175](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775175) | [36668775063](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775063) |
| PR | [36668778188](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668778188) | [36668777932](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668777932) | [36668778051](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668778051) |
| dev 分支 push | [36669165203](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165203) | [36669165232](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165232) | [36669165186](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165186) |

表中运行全部成功。已核验 push／PR 原报告摘要、固定端点／配置、生产输出流摘要和可执行身份。provider 与检查报告选择登记的 Linux 政策、Git 摘要及实际版本 2.55.0；对应 rates 报告为 50,197,605、50,196,779 字节。宿主明确限制检查 stdout 为 16 MiB、汇总清单为 1 MiB、每份原报告为 64 MiB。不同平台政策不证明有效输入或结果等价。

原 beta.17 候选 `e508cb05e5079155fb6a46e9dcf608534d75ba4a` 的两个真实单元通过，但 rates 报告达 149,387,940、149,386,115 字节，导致[push 汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36663954779)和[PR 汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36664001392)失败。

beta.18 的紧凑 scoped v3 序列化修复体积溢出，保留原进程字节和报告核验。候选／配置改变须生成新报告，不能改写旧失败或旧报告。此次采用没有增加 64 MiB 报告读取上限；当时其他示例 dev 仍锁定 beta.14，之后的版本更新见[示例索引](examples.md)。

<a id="public-artifact-inventory-adoption"></a>

## 采用公开版本的产物清单排除

Go 的 [PR #19](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/19) 通过公开 beta.19 清单和生成安装器升级六个工具，只改发布锁和 README；SDK、Git／检查政策及工作流字节与基线相同。

基线是 `6cf641fbd1e7828d922262884b2d735de3235ed0`，候选是 `65aa3342574cd827d8b1a43fd94f9b2f89623aae`。dev 落地为 `fb7fc9541ad5843285f58647bea7346aac6e079e`，文件树与候选相同，为 `c4da934b8a12d593ab0688856d449206962ee950`。

匿名下载的安装器／清单与发行一致，六个 macOS 工具的摘要和大小均匹配登记。本地 rates／harness 检查执行四操作，包括 Go 构建／测试和十项 bootstrap，汇总核对原报告。两份报告为 1,568,334、1,544,990 字节。在声明 SDK、状态输出已存在时，每份报告的两次 Git 未跟踪清单都返回零字节。

真实 Linux [push 汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36676291466)与[PR 汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36676344969)各验证两个单元、四操作。下载产物已用于核对原端点／配置、二进制身份和 184 次成功流摘要。rates 报告为 1,649,671、1,648,114 字节；两次清单调用都返回零，选择登记 Linux 政策。

合入后的 [dev 汇总](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36676560203)也通过，两份原报告、四操作和 184 次成功流摘要核验绑定到实际落地提交。包含 CodeQL 的四个 dev 工作流全部成功。

前述 beta.18 原报告保持不变。这些是分别取得的按范围宿主观察，不代表有效输入相同、完整检查启用或本地／CI 确定同判。
