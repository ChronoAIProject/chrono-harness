# 发布与安装

`distribution` 与 `distribution-tests` 是独立 Rust 项目。版本化原生二进制在 [chrono-harness Releases](https://github.com/ChronoAIProject/chrono-harness/releases) 集中公开分发；[Go、TS、mix 示例](examples.md) 是实际消费宿主。宿主只保留安装登记与生成入口，二进制、SDK 和缓存不提交。

## 发布者

本库 `.chrono-harness/release/build.json` 显式列独立构建 manifest、Rust 工具链；`plan.json` 显式列版本、二进制名称与已构建路径。`build.py ROOT OUTPUT` 构建这些成员、运行专属安装测试，调用唯一 `pack` 实现并保留实际平台/工具版本。它是本产品的宿主配方，不是其它语言仓库的要求。`.github/workflows/chrono-release.yml` 在登记的 macOS、Ubuntu runner 上实际执行此配方；上传构建 artifacts 不等于已发布。

```sh
chrono-distribution pack --root PRODUCT --plan PRODUCT/.chrono-harness/release/plan.json --output ABSENT_OUTPUT
chrono-distribution assemble --input MAC_PACKAGE --input LINUX_PACKAGE --output ABSENT_RELEASE
```

`pack` 拒绝脏源码，固定实际 Git commit/tree；只复制明确列出的可执行文件，不扫描 crates。`assemble` 显式接收平台包，验证每个成员，拒绝源提交、树、版本或成员集不一致及重复平台。最终 `release.json` 使用 `chrono-release/v1`，包含 `version/source_commit/source_tree/platforms`。平台映射中每个具名工具有 `file/sha256/size`；文件名为 `NAME-OS-ARCH`，每个平台须提供 `chrono-distribution`。没有 archive extraction。

原生平台目前登记 `macos-aarch64`、`linux-x86_64`；实际产物以构建结果为准。其它平台需要显式增加构建与运行验证。构建证据、声明的源码身份及摘要不构成完整工具链/SDK 来源认证，也不保证不同平台结果相同。

## 宿主采用

取得固定发布清单和对应平台的 `chrono-distribution` 后，显式选择要安装的工具：

```sh
chrono-distribution adopt --host-root HOST --manifest release.json \
  --base-url https://github.com/ChronoAIProject/chrono-harness/releases/download/VERSION \
  --curl /usr/bin/curl --tool chrono-harness --tool chrono-judge-ci \
  --tool chrono-ci --tool chrono-instructions
python3 HOST/.chrono-harness/install.py HOST
```

采用工具写出 `.chrono-harness/distribution.json` (`chrono-install/v1`) 及 `.chrono-harness/install.py`。登记含版本、HTTPS 源、清单摘要/长度、各平台注册 installer 摘要/长度、工具名称到 `.chrono-harness/bin/` 文件的显式映射。发布清单是采用时的受信任输入，调用者应从所选固定发布获得并核对；没有浮动 latest。首次采用者也须核验执行的安装器身份。

本地和 CI 共用同一 `python3 .chrono-harness/install.py .`。生成入口验证已锁定 installer，执行该 Rust installer；Rust 验证锁定清单、版本、平台和所选资产。缺失平台/成员、摘要/长度不符、目的冲突、未知 schema 均非零。安装不读取宿主源码、语言 manifest、目录布局或测试配置。语言 SDK 继续由宿主自己的显式登记负责。

离线分发可以 `--source-dir ABSOLUTE_PACKAGE` 替代 HTTPS 参数，使用同一个校验/安装路径；本地绝对路径不适合提交给公共消费者。HTTPS transport 是明确登记的 curl，禁读 curlrc，只允许 HTTPS（含重定向），检查实际下载退出码。Python 3 仅承担初始 installer 下载与调用；常规安装语义由 Rust 维护。

## 更新及故障边界

新版本通过重新 `adopt ... --replace` 显式修改锁定配置；不自动选择版本或改预期摘要。默认拒绝覆盖不同的采用配置/入口，重复相同采用允许。AI 自主更新登记并完成适用的 integration 和宿主检查。

安装先验证全部选中成员，再发布文件；已安装文件只有摘要、长度、执行权限合格才可复用。只替换登记的文件，其它 SDK/工具保留；本次不再选择的旧工具不会自动删除，退休由宿主显式处理。安装结果写 `.chrono-harness/state/distribution.json`，含实际发布身份、平台和逐工具摘要。

同一宿主的协作安装由 `.chrono-harness/.distribution-lock` 排他。普通文件替换失败尝试逆序恢复，失败报告恢复目录。没有崩溃原子性或与不合作并发写者的隔离保证；进程被强杀后的锁/恢复目录须核实后恢复。源/目的 symlink、越界路径和非文件覆盖被拒绝。支持 Unix；不声称 Windows 支持。

专属测试覆盖安装、升级、损坏后修复、重复安装、保留宿主 SDK、坏清单/晚到坏资产不替换、普通替换失败回滚、平台/成员/安装器错配、目的冲突、路径与 symlink、配置定制保护、bootstrap 真退出及实际安装器在含空格路径/不同 cwd 下运行。发布与实际宿主 check 各保留自身成功/失败；安装成功不是治理通过。
