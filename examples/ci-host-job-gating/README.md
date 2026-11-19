# 使用已安装工具、按需运行 CI job 的宿主示例

## 给已有测试加一条断言

在已接入并安装工具的示例仓库中，给 `check.py` 末尾加一行：

```python
assert greeting("  hello  ") == "hello"
```

只提交这个文件，保持检出干净，在示例仓库根目录运行 `.chrono-harness/bin/chrono-harness check`。现有登记已将 `check.py` 连到 `test:example`，因此会执行 `example.check`。把新增断言右侧改成 `"wrong"` 再提交，检查应以退出码 1 指出 `example.check` 失败。若断言开始读取新的数据文件，再为该输入登记到测试的依赖。[文件与测试的关系](.chrono-harness/FILEMAP.json)

## 接入示例仓库

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

## 用三个独立改动核对选测

修改选测工具时，用候选工具运行本示例。从同一已登记的 `dev` 基线分别提交下列改动，每次只改表中一个文件，运行上面的日常 `check`。预期来自文件的登记边和 `check.py` 中的业务断言。

| 单次改动 | 应选测试 | 应观察到的执行 |
| --- | --- | --- |
| 在 `message.txt` 的 `hello` 两侧加空格 | `test:example` | `greeting` 去除空格，`example.check` 退出 0 |
| 只给示例的 `README.md` 添加文字 | 空 | 业务操作和执行回执均为空，检查退出 0 |
| 将 `message.txt` 改为 `goodbye` | `test:example` | `example.check` 运行原断言并退出 1，检查失败 |

沿终端给出的原报告查看检查响应。`evidence.changed_paths` 标明本次输入，`selected` 给出选中的测试，`executed` 保留实际操作的进程回执。这三项都在 `evidence` 中，可据此区分漏选、误选与断言失败。修改 provider 或工作流则会选中 `harness`，验证生成器输出。两端登记共同决定删除与改名的影响。

检测 job 读取完整 Git DELTA，包括超过 3,000 个路径的变更；harness 不设文件数量门槛。

此示例只展示按范围检查，不启用完整治理，不提供完整创建／输入凭据，也不证明本地与原生 CI 同判。
