# 规则和产品一起修改时的警告

`chrono-judge-mixed` 读取 registration 解释后的两端登记、FILEMAP 绑定的 v2 变更及影响结果，以及成本检查的 `chrono-costs/v1` 结果。这三个程序都必须登记为直接前置。影响结果、成本或提交／文件树／登记／上下文绑定缺失或冲突时会报错。mixed 不重建依赖图、不查找依赖、不运行测试，也不重新计算成本。

## 怎么判断修改类型

只分类实际发生 Git 变化的路径，同时看基线和候选 FILEMAP 中的 `surface`：

| `surface` | 分类 |
| --- | --- |
| `product`、`test` | 产品修改 |
| `judge-policy`、`judge-implementation` | 可执行检查规则的修改 |
| 其他值 | 不能仅凭此值认定为可执行规则修改 |

候选重新分类或删除文件，不会抹掉基线的分类。产品引擎源码也可以通过工作流中明确登记的稳定性要求触发集成测试，无需归成宿主已采用的政策。

两端的 `config.semantic_fields` 还可以明确选出要比较含义的字段。每项声明指定一个已登记 JSON 文件、指针模式和修改模式。只读取被点名且已改变的文件。登记文件使用 registration 解释后的值；其他 JSON 从固定 Git 对象读取，严格按 UTF-8 解码。必需内容缺失、JSON 无效或指针转义无效都会报错。

指针采用 RFC 6901 的 `~0`、`~1` 转义；`*` 只匹配一级对象键或数组元素。以路径或 ID 标识的记录数组优先按 `id` 比较，没有 `id` 时按 `path`。工作流的 `retirements` 按 `(kind, id)` 比较，不同种类可以使用相同 ID。身份重复、无效，或同一数组混用有身份与无身份的记录，都会报错。

这些记录及其选中子树不受行顺序影响。`argv` 和操作序列等其他数组仍按顺序比较。报告同时保留原文件中的指针位置，不能拿稳定身份替代实际位置。字段不存在与字段存在且值为 JSON `null` 不同。`modify-delete-existing` 只比较基线已有的选中项；`add-modify-delete` 还比较新增项。格式迁移仍由 registration 和 workflow 负责。

按本仓采用的字段声明，普通源码修改加上新 FILEMAP 成员或依赖边，不算规则修改。修改已有 action、选中的 `surface`、执行计划或整份可执行规则文件，才会产生规则修改。程序不靠文件名、文档或 AI 指南文字猜测政策。

## 输出和结果

每次成功响应都输出 `outputs.rule_changes`，格式为 `chrono-rule-changes/v1`，包含固定 `binding`，排序后的 `rule_paths`、`product_paths`、`changes`、`product_changes`，以及 `mixed`、`requires_acknowledgement: false` 和原始成本结果。

字段含义变化的记录包含模式、修改方式、稳定身份，以及前后两端的实际指针和值。成本保留两端映射、引用、已知和未知维度、受影响和移除的测试，以及测量限制，不另算汇总值。runner 在 mixed 响应中保留结果；workflow 可把 mixed 登记为直接前置后读取它。

规则修改和产品修改都非空时，警告代码取自候选的 `workflow.mixed_change.code`，本仓默认是 `W_MIXED_JUDGE_PRODUCT`。状态为 `warn`，退出码为 `0`，同时展示两组变化及成本引用。只有规则修改时仍输出变化，状态为 `pass`。

警告不增加审批或确认步骤，也不豁免其他失败。工作流稳定性、迁移、移除和集成验证仍各有要求，尚未完成的部分不会因此算完成。本仓尚未全面启用完整检查。

分类和固定对象读取逻辑与 FILEMAP v2 共用，工作流要求的测试会在执行前进入已有计划。mixed 在编译时不依赖 FILEMAP 或 cost 程序，运行时读取它们带版本和绑定信息的结果。被点名的 JSON 无效时，FILEMAP 可能先于 mixed 报错；分类逻辑仍只有一份实现。
