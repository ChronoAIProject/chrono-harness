# 使用已安装工具的宿主示例（旧版 provider）

此示例保留 v1 的原有行为和历史首次清单测试。新宿主如需日常短命令，可采用[按需运行 CI job 的示例](../ci-host-job-gating/README.md)。迁移须明确修改登记，不能重新解释旧报告。

将本目录复制到新 Git 仓库，把 `chrono-harness`、`chrono-judge-ci`、`chrono-ci` 一起安装或复制到明确的工具目录。运行示例无需 harness 源码或 Rust。

示例 GitHub 配置使用 `self-hosted` runner，工具目录为 `/opt/chrono-tools`。初始化前，把 `.chrono-harness/ci/github.json` 的 `bootstrap` 改成实际工具目录，并按需配置 runner 标签。托管 runner 须另行明确配置固定版本的下载／安装步骤；示例不假定已有公开二进制服务。

```sh
python3 .chrono-harness/ci/install.py . /absolute/path/to/installed-tools
.chrono-harness/bin/chrono-ci init --host-root . --config .chrono-harness/ci/github.json
```

提交初始化后的完整登记清单。首次提交没有父提交时，执行：

```sh
.chrono-harness/bin/chrono-harness check --config .chrono-harness/ci/check.json --candidate FULL_INITIAL_OID --initial
```

后续提交沿用此命令，去掉 `--initial`，加上 `--base FULL_BASE_OID --candidate FULL_CANDIDATE_OID`。首次提交的默认分支为 `dev`，普通 push／PR 路由均明确配置。第一次将无父提交推到 `dev` 时，普通分支创建事件会自动执行同一条 `check --candidate FULL_INITIAL_OID --initial` 命令，运行 `example.check` 和工作流验证。`message.txt` 必须恰好是 `hello` 加一个换行，否则本地检查和首次 push 检查都会失败。

示例在 `push_baselines` 明确登记 `refs/heads/integration/`，基线为 `origin` 上的 `refs/heads/dev`。每次匹配的 push，包括后续修复 push，都把完整候选与当次观察到的 dev 顶端比较。在已有 dev 顶端创建 integration 分支会得到真实的空 DELTA，报告不选择操作。其他已有分支仍使用事件载荷中的 before／after。

如果创建 dev 时使用的提交已有父提交，就没有先前基线，准备阶段会失败。须通过 `workflow_dispatch` 或本地标准命令明确提供完整 base／candidate 范围；有父提交的历史不能使用 `--initial`。已有宿主首次采用时，须把 `policy.adoption_base` 设为前一个完整 OID，并保留真实基线 FILEMAP／projects 登记；不能把未登记的历史当作空树。

扩展检查时，在 projects.json 增加脚本和 action，在 check.json 增加 `test:...` 绑定，并明确登记 FILEMAP 文件和边。修改 provider／bootstrap 数据后，运行 `chrono-ci generate --host-root . --config .chrono-harness/ci/github.json`，不要手改工作流 YAML。修改 `message.txt` 会执行真实 Python 断言；修改 README 不会选择该断言。未知输入会报错。
