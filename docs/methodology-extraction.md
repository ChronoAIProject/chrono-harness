# 通用方法从哪里来，怎样采用

**派生规则已经修改、泛化和翻译，不是源指南的原样复制。** 固定来源为 [trureturing 的 CLAUDE.md](https://github.com/the-omega-institute/trureturing/blob/55f7968bb03b1422298d80fc65ac5291db9ad682/CLAUDE.md)。版权、修改声明与分发义务归[许可资料](licenses/methodology-attribution.md)。来源资料不是宿主必读指令或生成器输入；历史逐条迁移、文本审读和统计直接查 Git／PR，不在当前树维护副本。

## 当前内容与所有者

[产品 catalog](../assets/instructions/catalog.json) 定义双语规则、组合入口及布局，[产品 default manifest](../assets/instructions/default-manifest.json) 选择新宿主的默认输出。本仓明确采用独立的[宿主 catalog](../.chrono-harness/instructions/catalog.json) 和 [manifest](../.chrono-harness/instructions/manifest.json)。两份 catalog 即使字节相同，所有者也不同；现有宿主的定制不会因安装新二进制或重复 init 自动更新。

用户要求与当前 SPEC 决定目标规则，来源仓的数学准入、工具、Git、worker、权限和研究制度不在本仓执行。规则触发条件、动作和限制归 catalog 的唯一正文，入口组合与顺序归其显式依赖和布局；本文不维护第二份规则清单或来源到规则的运行映射。

中文和英文由作者分别编写。编辑时核对触发、“必须／可以”“同时／任选”、作用域和证据限制。生成器校验显式图、引用、locale 及内容传递，不翻译、不证明含义等价，也不执行治理判官。生成成功不能证明 AI 阅读、遵守或完整治理已启用。

## 实际消费者边界

本仓 manifest 明确选择根指南、完整中文 Markdown 和中文诊断 skill；各输出读取所选规则。根指南另要求读取 host-context，块外内容由宿主维护；AGENTS.md 是指向 CLAUDE.md 的字面相对链接。完整 general 库不是默认根，新宿主的选择仍归产品 default manifest。

`job.wait` 由默认根消费，要求优先使用真实完成通道，禁止用短周期轮询代替等待；`test.same-language` 同时由默认根和 `core.behavior` 消费。需求→审计→重构→开发由既有规则与 `workflow` 布局承载。具体部署、清理和操作入口归现役 host-context 及相应所有者文档；生成指南不替代实际执行结果。

实际注入上限以使用方设置为准。host-context、块外附文、其他项目指南和个人配置也须计入消费范围。内容编辑按受验修订核对完整载荷，字节按包含框架的实际 UTF-8 内容计量，结果归本轮 state／artifact；不维护长期读数表，不截断规则或修改全局配置。

## 更新与验证

采用新默认时，按[现有源更新路径](instructions.md#新默认的宿主采用边界)比较所选产品版本、保留定制，编辑唯一权威源后生成。宿主 manifest、catalog 与块外内容由宿主维护；产品默认和宿主选择须分别验证。

专属测试及生成／初始化消费者验证登记图、语言选择、内容传递、所有权、路径与 IO 合同，具体边界归[指令说明](instructions.md)。它们不证明翻译等价、实时注入、其他平台、输入完整或完整 SPEC 交付。源码合入、原生行为、落地和公开采用须分别取得真实结果。

分发派生指令资产、生成指南或内含这些内容的二进制时，须随附许可与来源说明。许可资料不为整个仓库更换许可证，也不取得运行政策权威。
