# chrono-harness 宿主上下文

本仓维护显式登记的 Rust harness 与独立宿主指令生成器。产品合同在 SPEC.md，当前行为与调用在 README.md、docs/instructions.md、docs/ci.md。

Production/dedicated-test pairs now include runner, inputs, judge-registration, judge-filemap, judge-routes, judge-projects, judge-cost, judge-mixed, judge-workflow, judge-ci, ci and instructions (each paired with its own -tests project). Thirty independent Cargo manifests/locks/targets; no root workspace. FILEMAP registry v2 is the sole current execution-plan declaration. FILEMAP impact v2 also lowers explicit workflow stability and conditional integration test requirements before execution; shared mixed classification remains the sole semantic classifier (docs/workflow-selection.md). Routes merges selected ordered operations, validates observed canonical invocation and binds tools once; projects checks affected generic pairs, explicit ownership/output isolation and consumes actual runner receipts. Explicit edges alone select full tests. Scoped CI reuses this implementation with labelled legacy selection/environment limitations.

Full host registries remain proposed/incomplete. Workflow v2 declares the finite chrono-ci-check/v1 plus FILEMAP v1 historical decoder, original bytes, mappings and exact legacy definitions. ci.verify belongs to ci.actions.execute and the real ci-tests plan; the pseudo-script/owner/test is retired with preserved old obligations and replacement execution. The historical profile explicitly binds its original method to the current units provider through method_replacements; legacy definitions remain exact. Public beta.14 includes this extension; see docs/workflow.md. Context can supply retained external bytes and absent/empty environment snapshots at both endpoints; FILEMAP compares their registration-validated effective environments through declared environment-node edges, retaining old-only edges and configured override semantics. Optional chrono-judge-cargo checks explicitly registered Cargo manifests, ancestors, output paths and path dependencies; generic projects never parses TOML. Both judge transports parse original stdout bytes, preserving arbitrary operation bytes separately. The macOS host explicitly binds /usr/bin/python3 with expected Python 3.9.6; failed migration version binding retains observed output/exit. Initial native run 36292159644 failed before governed operations on version mismatch; its actual interpreter version was not retained. Repaired integration/PR/dev reports were verified: the dev run 36295299904 used the exact system Python 3.9.6 and passed 34 operations/227 Rust tests. Positive bounded execution does not certify complete Cargo/SDK or undisclosed input closure. Workflow has bounded context-v2 freshness, retained completed integration evidence, explicit retirement/alias repair and finite schema-migration certification (docs/workflow.md). Full native CI/parity and autonomous delivery remain unfinished. Use .chrono-harness/ci/bootstrap.py to install registered candidate binaries including routes/projects, then the unchanged canonical scoped check command on the caller's clean committed candidate. See docs/execution.md and docs/spec-coverage.md for tested boundaries. Cost now publishes explicit before/after four-coordinate references, unknown warnings and source pointers (docs/costs.md); actual measurements remain unavailable and host estimates remain unknown. Mixed consumes explicit endpoint surfaces, named semantic fields and bound costs; warning stays exit 0 without acknowledgement. Its rule-change output is available to the workflow consumer (docs/mixed.md). Fresh creation is available through the separate registered chrono-worktree producer; explicit reconstruction stages chosen old changes on a fetched target; AI semantic reconciliation and PR/merge/landing remain caller-owned. Registered recover/cleanup now validate reconciled owned checkouts and explicit saved-work disposal; cleanup-fetch removes an explicitly receipt-bound temporary ref only when a fixed local branch preserves its expected commit. Released recover-interrupted consumes a retained pre-effect intent plus explicit absent/partial-result observation and reuses reconciled checkout/lock checks; the original outcome stays unknown. Released cleanup-fetch-interrupted consumes a separate immutable pre-fetch intent and explicit result observation, reusing ordinary temporary-ref retention/deletion checks. Both preserve original bytes and keep the original outcome unknown; both are in public beta.8. Explicit metadata rebind and source continuation are described below; PR/merge lifecycle remains unfinished.

产品默认位于 assets/instructions/catalog.json 与 default-manifest.json；宿主独立采用 catalog.json、manifest.json 和本上下文。根 CLAUDE.md 受管块直接呈现中文核心，AGENTS.md 是字面相对链接 CLAUDE.md；编辑 catalog 或其显式 file 源/输出计划，再运行登记的 generate。不要独立编辑投影，也不必重复读取已呈现的规则源。

当前专用 schema=2、producer=chrono-instructions、render=atomic-rules/relative-alias/v3。默认新宿主一条 init 从内嵌双语数据采用仅根输出，--locale en 可显式绑定初始英文；无运行时 checkout 或语言推断。已知旧登记自动保留 opaque 方法为 und file atom，升级不覆盖定制。本仓已明确采用产品逐条迁移的 98 个双语内容叶子与 16 个普通聚合；原有 17 个稳定入口及 core.general 保留，旧精选段落由单一内容叶子替代，已退休 monolith 不恢复。固定源及逐条处置在可选 docs/methodology-clause-map.md，许可在 docs/licenses/；这些不是必读政策或生成输入。旧 core.ownership 显式依赖 projection.criteria／projection.consumption，core.behavior 依赖 ci.actual-events；独立消费仍有原义务。默认新宿主双语根的实际字节预算及有限余量见可选 docs/methodology-extraction.md，不是运行时新增判官。catalog 的可选 layouts 与 output.layout 只组织阅读，不改 requires、权威或执行顺序；未选布局保持旧 v3 平铺字节。产品与本仓采用同一 general 双语布局（三部分、12 主题、98 正文各一次），宿主数据仍独立拥有。英文 docs/generated/general-methods.en.md 和 skills/diagnose-recurring-failures/SKILL.md 由同一 catalog/manifest 生成；skill 仅选择 evidence/reuse/repair-producer 的 7 个内容叶子，不做全局安装。删除输出条目保留旧文件，需授权 AI 明确退休；marker 不代表仍受管理。

产品双资产是编译/测试输入，宿主 catalog/manifest/context 和已存在投影是运行输入，具体 FILEMAP 边保持显式。FILEMAP projection.sources 是 proposed 的多源元数据扩展，不代表通用判官已执行。源翻译的语义完整性需内容核验；生成和格式验证不认证 AI 遵守或翻译等价。

按任务目标自主实施、验证与修复，保持独立项目/专属测试和真实退出状态。分支约定与计划中的 integration 策略见 SPEC.md；Git 生命周期遵循当前任务授权，不从本文推断已启用机器门。不要改全局配置或参考仓库，不将过程转录写入本仓。

`chrono-inputs` captures only fixed-config file/variable declarations and pairs explicit endpoint snapshots, optionally transporting retained blobs between roots. Registration accepts inline bytes or stream-verified state blob references and checks config/endpoint bindings. Snapshot publication is not a governance verdict and never updates expected hashes or infers dependencies. See docs/inputs.md; full Cargo/SDK closure and activation remain outstanding.

`distribution` / `distribution-tests` is the independent release/install pair (now 30 manifests including the optional judge-cargo and worktree pairs). `chrono-distribution` packages only explicitly declared native assets and generates the pinned host installer. Product release recipes live in `.chrono-harness/release/`; generated host adoption stays under that host’s `.chrono-harness/`. Public product and Go/TS/mix consumer links are indexed in `docs/examples.md`. See `docs/distribution.md` for integrity, ordinary-error rollback and platform limits.

The full generic projects schema permits omitted manifest/lockfile/root and arbitrary nonempty action names. `execute` retains test-entry semantics. `judge-cargo` / `judge-cargo-tests` owns former Cargo-only regressions; policy is explicitly supplied via `--policy`, never inferred from host names. The adapter also offers an ordinary registered run entry that validates retained registry/Git package inputs and actual locked/offline Cargo metadata before executing the declared consumer, then rechecks input identities. Full/scoped runners use the same action without a language-specific callback. This Rust host has not adopted the guarded policy; the guard preserves v2 literal inventories and supports v3 explicit ancestor-absence assertions and host-relative Cargo home; compiler/backend/linker/SDK, build-script and other external inputs remain incomplete and full governance remains proposed. See docs/cargo-projects.md.

This host explicitly adopts CI push_baselines for refs/heads/integration/ against refs/heads/dev on origin. Creation and later repair pushes bind the entire branch range to that observed dev tip; unmatched push events retain before/after behavior. This event-input preparation does not replace the full workflow judge or certify input completeness. See docs/ci.md.

`worktree` / `worktree-tests` owns fresh creation and explicit DELTA reconstruction from a fetched target. Reconstruction plans enumerate every old changed path as carry/retire; conflicts preserve original failures and work. Successful reconstruction is a staged tree with no candidate/check/integration success. Host policy is `.chrono-harness/worktree.json`; target and branch prefixes come from the registered workflow. The report retains actual Git process evidence and never claims governance activation. See `docs/worktree.md`. Public beta.11 contains chrono-worktree for macOS arm64 and Linux x86_64, with 67 dedicated worktree tests among the 391 tests executed by the native release recipe on each platform. Go now adopts public beta.19 with explicit native-platform Git facts selection; TS, mixed and initial examples still pin beta.14. The beta.14 local/integration/PR/dev checks and installed macOS arm64 public-origin start/cleanup consumers retain their original scope; current Go adoption and release evidence are in docs/native-ci-adoption.md and docs/distribution.md. Exact adoption and consumer evidence are indexed in docs/examples.md. The recorded public-origin recover-interrupted/cleanup-fetch-interrupted consumers, retained staged work and cleanup used beta.8; later binary upgrades do not relabel that evidence.

Full config v3 explicitly binds candidate Git via facts_git tool/input references before object acquisition and through full consumers, initial and input capture. Config v1/v2 retain their previous semantics. Scoped profile v2 explicitly selects full-v3 policy.facts_config before acquisition; both endpoints, optional full-registration interpretation and post-execution checks share that Reader, retaining structured success/failure observations including failed version binding. Scoped v1 and old CI provider versions retain their previous behavior. Git binding does not change scoped selection/operation environments or require full registry adoption; this host uses the independent-unit scoped v3 profile without opting into Git binding. Provider v3 explicitly selects a full-v3 facts_config for event reads/fetches, retains process observations and fetches only on an exact missing-object response. The host has not adopted that provider version; see docs/ci.md. Original process bytes, failures and per-process bounds are retained; missing or mismatched request bindings fail. Git configuration/OS/delegated closure, host activation and parity remain incomplete. See docs/git-facts.md.

The product native release recipe v3 explicitly stages 14 release assets into declared untracked consumer destinations and selects ten dedicated test projects, including full/scoped Git-binding consumers. Native report v4 retains per-operation source/destination identities and original child failures; recipe v2/report v3 remain supported. Public beta.10 ships this recipe contract, the Git-binding extensions, CI generators and remote retirement; binary installation does not adopt new profiles. Its fixed-source native release passed 371 tests per platform and retained 31 processes, 28 staged destinations and 24 matching identity observations. The release workflow is now generated by chrono-ci from explicit host .chrono-harness/ci/release.json, with ci.release.verify in the ci-tests plan. This generation extension is distributed in beta.10; the current host commands and boundaries are in docs/release-ci.md. This recipe does not activate full governance or certify input closure. See docs/distribution.md.

The adopted macOS interpreter pin is checked by the registered scoped-v1 host script tests. Portable CI/migration consumer fixtures explicitly adopt their executing interpreter without changing historical bytes or retiring the wrong-version regression. Native product tests do not redefine this host policy.


Public beta.10 `chrono-worktree cleanup-remote` adds an explicit remote disposal plan
under registered state: one expected push URL, registered work branch/current
OID and configured target/retained commit, checked locally and remotely. It reuses
saved-work checks, performs an exact leased deletion, and observes absence while
preserving original failures and explicit retry states. It does not certify an
atomic remote transaction, PR/merge, input closure or governance. The anonymous
public macOS binary retired the release branch; each example’s installed public
binary retired its own adoption branch; see docs/worktree.md#remote-branch-retirement.

Public beta.10 full CI transport uses `chrono-github-full-ci/v1` with explicit
workflow_dispatch context bytes. Candidate-bound Git v3 checks policy, provider,
checkout and supplied workflow projection; preparation stores original context
separately from process observations and invokes the same full check argv.
It does not infer branch roles/times, supply missing evidence or activate this
host. Seven-judge consumers live in judge-workflow-tests and explicitly require
build.ci. See docs/full-ci.md for collision/retry and provenance boundaries.

The beta.10 anonymous macOS CI binary verified release/full generation, drift
rejection/regeneration and exact context/argv preparation. This is a local real-Git
transport consumer, not native generated full-dispatch adoption. The four examples
retain their explicit scoped profiles; docs/examples.md binds adoption and dev runs.

Public beta.11 `chrono-worktree inspect-rebind`/`rebind` consumes explicit branch,
HEAD/index tree, metadata member, backup/donor and observed preservation identities.
It preserves visible work and remaining old metadata, including absent old
receipts, without inferring the historical index or original outcome. Attached
failed results can reuse receipt-bound reconciled `recover`. Source `resume-rebind`
also consumes retained intent/plan and absent/partial/failed results, reuses the
original donor/backup and preserves visible work and original outcomes. It rejects
conflicts for explicit AI reconciliation; beta.14 does not include this entry. Actual public macOS mixed-host consumption verified preserved work, explicit index and old metadata; original outcome remains unknown.
See docs/worktree.md#explicit-metadata-rebind for filesystem and failure boundaries.

Public beta.11 literal checkout identity compares fixed tree entries, index stages
and raw physical bytes/types/owner-executable bits with streaming Git blob hashes.
It does not invoke working-tree diff or filters. Full/initial/scoped consumers and
worktree creation/reconstruction/recovery/cleanup share this checker while keeping
their existing Git runner, flags, artifact and process contracts. Hosts must
materialize exact registered snapshot bytes; CRLF/filter/symlink emulation is not
silently normalized. Full Git/delegated/SDK closure remains unfinished. See
docs/git-facts.md#literal-checkout-identity; the public macOS mixed-host consumer verified hidden-mode check/cleanup rejection and preservation.

Source cleanup now preflights every explicitly selected artifact directory, rejects tracked contents, and disposes registered untracked outputs under the owned lock before bounded Git checkout removal. Per-path effects and later Git failures are preserved; filesystem deletion stays in the host job lifecycle without a Git subprocess deadline. Existing recovery and explicit retry handle artifact-only partial failures. This addition is distributed from beta.12; older partial source deletion, concurrent/power-loss transactions and arbitrary source-volume deadlines are not covered. See docs/worktree.md#declared-artifact-disposal-before-checkout-removal.

Public beta.13 includes explicit independent scoped CI units/collection and
`chrono-ci migrate` for customization-preserving owned workflow transitions.
Go, TS and mixed examples use one workflow per explicit unit with separate SDK
profiles; the initial example keeps its inventory contract. The three business
hosts’ beta.14 binary upgrades preserve CI/SDK source and generated workflow bytes. Native failures,
cancellation with selective retry and empty-DELTA collection were verified on
beta.12; anonymous beta.13 migration consumption uses the real three host configs.
This product registers sixteen independent unit workflows and a collector from
`.chrono-harness/ci/units.json`; the check profile explicitly owns complete plans
and repeated shared operations. Each workflow selects `bootstrap-core.json` for
runner, judge-ci and ci; remaining builds belong to the selected complete plans.
The bootstrap default still selects its full configuration. Local and native
unit checks use identical `check ... --unit ID` arguments; collection uses
`check ... --collect MANIFEST` without business reexecution. Input closure,
full governance, deterministic parity and delivery orchestration remain unfinished.
The source/previous-input migration commands and limits are in docs/ci-units.md;
current public host links and validation boundaries are in docs/examples.md.
