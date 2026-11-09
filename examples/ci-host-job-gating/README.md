# 使用已安装工具、按需运行 CI job 的宿主示例

此示例按变更范围检查，登记两个独立单元（`example`、`harness`）、一个 Git 变更检测 job 和一个汇总 job。分支保护只要求 **Example / aggregate**。先验证并合入合并树校验工作流，再关闭 strict；独立 PR 并行检查，只按实际依赖或冲突决定合并先后。选中的单元各自拥有 runner、bootstrap、超时、上传和 job 重跑；未选中的 job 在分配 runner 前跳过。汇总通过 `always()` 运行，用 `check --collect` 判断选中单元的原报告。

将本目录复制到新仓库，把兼容公开版本的 `chrono-harness`、`chrono-judge-ci`、`chrono-ci`、`chrono-worktree` 安装到明确的工具目录。示例自托管 runner 从 `/opt/chrono-tools` 读取工具。按宿主实际情况修改 `.chrono-harness/ci/units.json` 中所有选中 bootstrap 的参数和 runner。

变更检测 job 只通过薄 Shell 拷贝这些已安装的 harness 工具，读取明确登记的稀疏源码路径。harness 单元直接验证 Rust 生成器输出；这两处都不需要 Python。Python 只用于本例的 `message.py` 业务和 `check.py` 测试，由选中的业务单元使用。

示例明确选择 macOS Git `/usr/bin/git`、系统 Shell，以及业务 Python `/usr/bin/python3`，复制时按实际宿主选择这些工具路径。Git、Python 和 Shell 的版本、字节由本次运行观察，后续调用使用已绑定的执行物；仓库不维护主机摘要或版本观察表。harness 工具版本继续约束本例实际使用的接口。无需 Rust 源码检出；安装的版本须包含条件父工作流和原 `production` 输出合同，旧版本不兼容。公开版本发布和真实原生 Actions 验证仍由调用方负责。

完成上述配置后，初始化并验证生成结果：

```sh
/bin/sh .chrono-harness/ci/install.sh . /absolute/path/to/installed-tools
.chrono-harness/bin/chrono-ci init --host-root . --config .chrono-harness/ci/units.json
.chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
```

提交完整登记清单，建立远端 dev 基线，然后从宿主根目录使用固定的日常命令：

```sh
.chrono-harness/bin/chrono-harness check
.chrono-harness/bin/chrono-harness check --unit example
.chrono-harness/bin/chrono-harness check --unit harness
.chrono-harness/bin/chrono-harness check --collect
```

本地汇总须取得所选 DELTA 对应的成功原报告，不会补跑缺失单元。自动 CI 在内部为同样的短命令准备输入。首次 dev push 没有父提交时，使用登记的首次范围清单；若建立基线的提交已有父提交，则报出具体端点错误。

PR 的所有 job 检出固定原生合并提交 M，以第一父 B 为基线，校验第二父为事件 head H；GitHub 运行和工件归属按 H 核对。普通 push 使用完整 before／after，检查实际落地变化；检查时与落地时可能是不同树。本示例另明确规定：integration push 与检测 job 观察到的 `origin/dev` 比较，并为依赖 job 固定这次观察结果。

修改 `message.txt` 会选择 `example`；只改 README 不选择业务单元。问候语错误会让真实 Python 断言失败。修改 provider／工作流会选择 `harness`，其真实检查验证生成器输出。依赖、单元归属、删除、改名均按两个端点的登记判断。未登记输入或证据缺失会失败；不按目录或语言推断，也不回退到全选。

检测 job 读取完整 Git DELTA，包括超过 3,000 个路径的变更；harness 不设文件数量门槛。

此示例只展示按范围检查，不启用完整治理，不提供完整创建／输入凭据，也不证明本地与原生 CI 同判。
