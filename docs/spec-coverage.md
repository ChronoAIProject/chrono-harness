# Current SPEC coverage

This is a maintained coverage map, not a run log or a second policy. Full SPEC remains the objective. `implemented` means the named contract has a real producer and behavior test; it does not mean this host is activated or delivered. `partial` retains the uncovered obligation. All full host registries remain proposed, with incomplete input closure and missing executable bindings. Registration/filemap success on a bounded host is not seven-judge success. Local checks do not certify native CI or landing.

Test sources: **R** = `crates/runner-tests/tests/protocol_v1.rs`; **G** = `crates/judge-registration-tests/tests/registration_contract.rs`; **L** = `crates/runner-tests/tests/cli_contract.rs`; **C** = `crates/judge-ci-tests/tests/behavior.rs`; **F** = `crates/judge-filemap-tests/tests/impact.rs`; **P** = `crates/judge-filemap-tests/tests/consumer.rs`; **W** = `crates/judge-workflow-tests/tests/{branch,consumer}.rs`; instruction and CI generator tests live in their dedicated test projects. Tests in G invoke the built runner and separately built registration executable against real synthetic Git commits, including real host registry material. Production build prerequisites are explicit in the CI binding. P runs actual runner → registration → filemap subprocesses on committed bounded hosts with absent historical binaries and exact report source pointers. F checks witnesses against independently specified endpoint edges. No peer/independent review evidence is claimed.

| SPEC section | State | Producer and direct evidence | Remaining obligation |
| --- | --- | --- | --- |
| 1 authority/boundary | partial | runner `full.rs` declared external DAG; R `dag_*` | remaining judge policies and lifecycle |
| 2 independent projects/adoption | partial | independent manifests, locks, bootstrap registration build; SPEC §2.1 assigns shared mathematics to trureturing and operational/model correspondence to chrono-harness | generic full bootstrap provenance/activation; source ownership does not prove implementation refinement |
| 3 five registries | partial | registration `schema.rs`, G `strict_five_schemas_*`, `real_host_pseudo_script_*` | complete Cargo/SDK closure; bounded pairing/routes/migration/snapshots implemented |
| 4 fixed snapshots/context/CI | partial | runner `facts.rs`, registration `lib.rs`; G `context_and_config_identity_mismatch`, `ignored_untracked_staged_and_wrong_checkout_fail` | config v3 binds full/initial Git facts through all full consumers, explicit scoped-v2 acquisition and provider-v3 event reads/fetches (docs/git-facts.md); full native CI/parity and Git/delegated/Cargo/SDK input closure remain |
| 5 DELTA graph | partial | filemap typed union/records/closure and CI adapter; F all tests, P `actual_runner_registration_filemap_*`; SPEC §5.1 pins two-state locality and the compiled seeded-edge closure theorem (trureturing PR #10868, draft), with the old execution-edge counterexample | complete host input closure and native activation; new theorem review/freeze; Rust refinement is unproved and reached-node equality does not preserve typed test selection |
| 6 evolution/removal | partial | W actual replacement, joint retirement, alias repair and candidate migration certification; registration workflow v3 selects explicit endpoint versions, inputs consumer certifies a host config v1→v2 decoder with original snapshots | further host converters and full host activation |
| 7 protocol | implemented bounded: ordinary two-commit v1 and explicit initial profile | runner `wire.rs`, `full.rs`, `initial.rs`, shared process engine; R transport tests; G actual root consumers | bootstrap provenance and activation; initial inventory never claims a green DELTA; platform evidence limited to tested Unix |
| 8 seven judges/mixed | partial | all seven bounded producers; W full-chain consumers | full host input closure and activation |
| 9 report/cost | partial | runner `full.rs`; G `full_report_*`: all required fields, executable list with observed/configured distinction, findings and named-output sources, null/unresolved reasons, pass/warn/fail/error exits and stored/stdout agreement | complete toolchain/input coverage and optional measurements; declared cost reporting is implemented; bounded routes tools and projects test evidence are supplied; unknown or conflicting results do not establish complete governance |
| 10 lifecycle | partial | W freshness and integration producer/consumer with completed reports; independent worktree producer and dedicated real Git tests cover fetched target creation, adopted policy, explicit reconstruction, conflict/failure preservation and registered recover/cleanup/cleanup-fetch, explicit leased remote retirement, retained-intent interrupted checkout and fetch consumers, and explicit metadata rebind without old receipts | AI reconciliation, interrupted/partial-rebind continuation, PR/merge/landing orchestration |
| 11 script/plugin | partial | direct argv external transport; R real Python child fixtures | plugin/full external closure; real script pairing and routes have bounded full-chain evidence |
| 12 bootstrap/initial | partial | registered bootstrap; explicit root profile and candidate registration inventory with no fabricated base; public macOS arm64 host verifies generated v2 first push and ordinary DELTA | full source/toolchain provenance, cross-platform initial profiles and activation migration |
| 13 acceptance | partial | row map below | all pending row obligations remain |
| 14 boundaries | implemented documentation | README/SPEC/this map | update as later increments land |
| 15 generic reference experience | partial | existing fixed-source references; no reference repo modifications | apply remaining generic workflow lessons |
| 16 instructions | implemented existing scope | instructions and dedicated tests, current docs/instructions.md | existing documented platform/crash/concurrency limitations remain |

Rows below follow §13 in order; scoped evidence is explicitly limited to that profile.

| §13 row/scenario | State and direct evidence | Remaining full contract |
| --- | --- | --- |
| 1 registered source M selects tests | implemented bounded: F `typed_content_mode_binary_*`, P impact, projects consumer real execution | full native activation |
| 2 unregistered A / ignored dirt | implemented: G `changed_unregistered_fails_and_explicit_registration_repairs`, `ignored_untracked_staged_and_wrong_checkout_fail` | downstream test execution separate |
| 3 source plus membership | implemented bounded: registration repair, FILEMAP selection, mixed ordinary-member/edge controls | full native activation |
| 4 deleted file uses base edges/costs | implemented bounded: F/P facts, W joint retirement and retained old costs | full native activation |
| 5 rename D+A | implemented bounded: G/F endpoint facts, W replacement and retained old costs | full native activation |
| 6 changed/deleted edge seeds | implemented bounded: F `file_record_field_and_edge_*`, `file_edge_only_*`, shared scoped execution | full native activation |
| 7 legal retirement | implemented bounded: W joint producer/test removal, explicit alias repair | full native activation |
| 8 required test removed | implemented bounded: W explicit successful replacement or validated joint retirement | full native activation |
| 9 nonunique/missing test pair | implemented bounded: projects and real consumer negatives | full native activation |
| 10 manifest undeclared dependency | implemented bounded: optional Cargo adapter: explicit policy, TOML path dependencies and missing-edge rejection; optional v4 binds Cargo/compiler file inputs and selection with actual direct/full/scoped consumers | compiler-library/backend/linker/SDK closure; guarded registry/Git resolution implemented |
| 11 unknown input closure | partial: G `proposed_and_incomplete_never_receive_governance_success` | complete Cargo/SDK closure; retained bounded external/tool inputs implemented, no discovery guarantee |
| 12 README no all-tests | implemented bounded: F selection, C and projects consumer real nonexecution with unrelated historical defects | full native activation |
| 13 independent script tests | implemented bounded: real full-chain script pair | full native activation |
| 14 duplicate operation | implemented: routes preflight and full consumer | none in tested boundary |
| 15 bypass/missing operation evidence | implemented bounded: actual invocation and receipt comparison | does not monitor arbitrary shell actions |
| 16 mixed warning with integration | implemented bounded: W certified mixed delivery retains warning and exit 0 | full native activation |
| 17 engine stability | implemented bounded: explicit FILEMAP selection, W completed integration | full native activation |
| 18 policy requires integration | implemented bounded: W missing certificate failure and completed evidence acceptance | full native activation |
| 19 changed integration binding | implemented bounded: W base/tree/receipt/completed-report rejection, same-tree commit acceptance | full native activation and complete effective inputs |
| 20 stale branch | partial: W explicit dev/fork/time freshness rejects either excess; worktree reconstruct replays complete explicit carry/retire choices onto a fetched target and preserves conflicts | AI semantic reconciliation and autonomous delivery orchestration |
| 21 threshold equality | implemented: W count/age equality, nanosecond excess and configured limits | — |
| 22 same local/CI command | partial: scoped CI generator with explicit push-baseline configuration; CI behavior tests cover repeated pushes, remote advancement, missing refs and real CLI persistence | full dispatch transport is implemented (docs/full-ci.md); native generated dispatch adoption and complete inputs remain |
| 23 same complete deterministic inputs | pending | deterministic evidence/parity comparison |
| 24 parity unestablished | implemented field: full runner report; G external host | unresolved closure remains nonzero |
| 25 staged/unstaged/untracked | implemented: G dirt cases | no hidden dirty mode |
| 26 missing history | implemented bounded: G object checks, W common ancestry and shallow boundaries; worktree producer fetches its explicitly registered target | general missing-history recovery/reconstruction remains external |
| 27 empty/invalid stdout | implemented: R `malformed_stdout_crash_and_bounds`, R/L embedded-invalid-UTF-8 subprocess/CLI cases and valid U+FFFD controls | — |
| 28 crash/timeout/digest | implemented: R bounds and prelaunch digest tests | resource guard is infrastructure, not functional verdict |
| 29 unknown costs | implemented: cost consumer warns with zero exit, preserves null and known coordinates; no-Delta retained-input case | full host activation |
| 30 candidate migration validator | implemented finite contract: W original schema versions, real conversion/compatibility test, old method nonexecution and judge retirement; inputs consumer covers v3 version selection and config v1→v2 certification | further host converters and native activation |
| 31 no DELTA | partial: G no-delta/dirt behavior | full input/workflow checks |
| 32 initial root | implemented bounded: G root inventory, real/shallow parent rejection, checkout/reference failures and no activation | full bootstrap provenance/activation |
| 33 check CLI | implemented bounded transport/registration plus existing slice | whole configured host remains nonzero until obligations implemented |
| 34 unknown/malformed CLI | implemented: L `unknown_and_malformed_commands_are_errors` | — |
| 35 spec status | implemented: L `status_help_and_version_are_information_only` | — |
| 36 help/version | implemented: L same test | — |

The registered execution increment adds routes/projects, FILEMAP v2, retained endpoint inputs and the finite chrono-ci-check/v1 / FILEMAP v1 decoder. See [the execution contract and exact boundary](execution.md). Dedicated routes tests check ordered plans, actual argv/environment/tool bindings, receipt tampering, PATH shadow and byte replacement. Dedicated projects tests run the actual runner/registration/filemap/routes/projects chain on committed project/script hosts, cover exclusive pairs, manifest-free/custom-action hosts, explicit output isolation, retained input failures, real exits/effects, blocked dependents, docs nonexecution and mapped replacements. Its migration consumer uses real old repository registrations and actual ci.verify, including workflow drift and restoration. Existing test identities remain; the historical pseudo-script rejection reads the fixed old tree. Maintained regressions also cover retained inherited-environment changes through the full chain (including absent/empty, overridden and disconnected controls), registered intermediate workspace rejection before operations, both protocols' embedded invalid UTF-8, arbitrary operation bytes, and migration version failure diagnostics. The host interpreter binding is explicit macOS data; the failed original native run and the verified repaired native results are recorded in [CI documentation](ci.md).

Config v3 adds an explicit candidate Git binding before acquisition and through all seven full judges, initial inventory and input capture. Dedicated regressions preserve version 1/2 semantics and original process failures; the seven-judge integration/delivery consumer rejects ambient Git. This is bounded executable/environment binding, not complete Git/OS/delegated input closure.

CI provider v3 binds event acquisition through an explicit full-v3 `facts_config`. Dedicated `ci-tests/tests/git_facts.rs` regressions use real Git objects/remotes and the actual CLI to check missing-history acquisition, exact probe interpretation, failed probe/fetch evidence, candidate policy/checkout binding, declared environment, generated initial-profile selection and legacy controls. Native generated-v3 host adoption, network/credential/configuration closure and parity are still outstanding.

Scoped profile v2 independently opts into Git binding with policy.facts_config. Dedicated judge-ci tests exercise actual CLI selection with ambient Git shadowed, documentation nonexecution, both endpoints, v1 migration and old-only deletion, initial root/child/shallow behavior, fixed-policy/digest/version errors, dirty/index/checkout/post-execution guards and retained binary output/exit/timeout/overflow. The actual projects consumer checks optional full-registration interpretation and ordered execution through the same reader, with process-count/byte-hash checks and rejected canonical argv. V1 controls remain; the host has not adopted scoped v2.

Full activation and all other remaining obligations stay active. Generic project fixtures validate declared method execution and ownership; independent Cargo fixtures validate explicitly adopted TOML consistency. Neither proves Cargo/SDK completeness. Single-worker checks do not certify independent review, native CI or landing.

Public [beta.10](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.10) ships explicit Git binding, release/full CI generation, remote branch retirement and native release recipe v3/report v4. Its [fixed-source native build](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36472602944) passed 371 tests across ten dedicated projects on each of macOS arm64 and Linux x86_64, including full/scoped Git-binding consumers. Both original reports retain 31 actual processes, 28 staged consumer destinations and 24 matching identity observations. This validates those native consumers; generated provider-v3 host adoption, full input closure and deterministic parity remain outstanding. The current host release workflow is generated from an explicit chrono-github-release/v1 source; see [release CI](release-ci.md). That generator extension is distributed in beta.10; the linked native release uses its generated workflow.

This increment adds mixed classification and adopted registrations together. **Mixed policy/product change warning:** the project pair, semantic comparisons, bootstrap binding and execution plan expand the validation surface. Costs remain unmeasured. Dedicated tests validate the mixed report and shared cost consumer; host activation still requires the outstanding governance and input obligations.

The cost judge and its dedicated tests implement the [declared cost contract](costs.md): distinct endpoint values and source pointers, old-only costs, known/unknown coordinates, identity deduplication, affected-project members, declaration-only changes and docs locality. Real subprocess consumers verify pass/warn exits, missing impact failure, source fidelity and retained-environment impact with empty Git DELTA. Measurements are explicitly absent; no additive resource total or full-governance claim is made.

The mixed judge implements [explicit rule/product classification](mixed.md). Dedicated behavior tests cover normal member/edge additions, real action changes, endpoint surfaces, array identity and ordering, historical patterns, named JSON and malformed inputs. Real subprocess tests check warnings, pass/error exits, exact cost and source fidelity, rule-only outputs, missing predecessors, stale binding rejection and strict UTF-8. Workflow remains responsible for stability and integration certification; this increment supplies its rule-change input.

FILEMAP impact v2 now lowers explicit workflow stability and conditional integration test requirements before route execution. Dedicated tests retain base requirements, old costs and causal source pointers; real consumers prove additional tests run and shared operations run once. Named semantic-input errors now stop the FILEMAP stage. The mixed classifier remains the single implementation, with downstream FILEMAP/cost dependencies removed from its compilation graph. The independent [workflow judge](workflow.md) now certifies bounded freshness, completed integration, legal joint retirement, alias repair and finite migration. See [workflow selection](workflow-selection.md).

Workflow verdicts use context v2 with an explicit integration/delivery role. W exercises the real seven-judge chain, changed and stale evidence, missing final reports, actual receipts, skipped judges, failed tests, docs locality, custom branch prefixes, invalid names and shallow history. Runner transports actual prior process identities and retains unique completed reports. Cost consumes historical definitions as well as current declarations. These are bounded engineering checks, with no independent-review or complete-input claim.

The [retained input producer](inputs.md) supplies actual capture/pair snapshots, preserves absent/empty environment values and transports content-addressed state blobs between explicit roots. Registration streams retained and candidate file identities; reports remain representation-independent. This removes inline-JSON size as a prerequisite blocker for large toolchain files. It does not yet supply the host's complete Cargo/SDK dependency declarations or activate full native governance.

The host-neutral increment removes mandatory manifest/lock/root fields and the closed action vocabulary. `registered_intermediate_workspace_rejected_before_operations_and_standalone_passes` now belongs to judge-cargo-tests; the old mixed pair/manifest/route regression is split between generic pair/ownership/routes and the actual Cargo missing-edge consumer. The independent Cargo adapter supports chrono-judge/v1 or a registered standalone prerequisite; its structural-only contract still rejects external dependencies. The separate registered run entry validates retained registry/Git package inputs, actual locked/offline metadata, exact features/aliases/kinds, configuration ordering and before/after identities. Tests inputs and input_consumer exercise actual Cargo through direct/full/scoped consumers, including rejection and documentation-only DELTAs. The guarded configuration inventory validates explicit lookup presence/absence, ordered arguments, recursive includes and before/after mutations. Its v3 contract adds an explicit choice between ancestor inventories and an all-ancestors-absent assertion, plus host-relative Cargo home; v2 interpretation remains supported. Configuration tests use identical policy/input bytes at different checkout depths, check both configuration names, symlinks, persistent mutations and relative-home behavior. Dedicated configuration tests and full/scoped consumers check rejection, real Cargo precedence and docs nonexecution. Compiler/backend/linker/SDK, build-script and other external inputs, activation and native full parity remain outstanding.

Explicit config/snapshot v2 retains external file absence separately from empty bytes.
Capture/pair and real seven-judge consumers exercise both state transitions, old
edges, mismatch and legacy-version rejection, post-operation mutation, symlink
boundaries and disconnected/documentation nonselection. Cargo direct/full/scoped
consumers check connected absences before/after execution and reject absent IDs
where package bytes are required. Config v1 null remains unbound. Workflow v3 now
accepts host-registered config v1→v2 conversion/compatibility checks selected by
explicit before/after registry versions. Real consumers preserve original v1
snapshots, reject historical config rewrites, ambiguous/incomplete selectors and
missing or failed compatibility evidence, and retain documentation-only locality
after adoption. This is a decoder contract, not an automatic host config writer.
Complete input closure and full host activation remain outstanding.

Optional Cargo input policy v4 binds the delegated Cargo and compiler executable
to explicitly connected input IDs, paths, hashes and observed versions. Tests
exercise real compiler selection, rejection before metadata, configuration/include
and wrapper conflicts, persistent post-consumer mutation and full/scoped docs
locality. V2/v3 meanings remain supported. This does not certify compiler libraries,
delegated processes, backend/linker/SDK, build scripts or complete native parity.

The explicit initial profile uses `chrono-initial-judge/v1` for a single candidate,
without a synthetic base or DELTA. The default registration consumer retains
proposed/incomplete state and checks all initial registration references and
checkout facts; ordinary DELTA reference selection is preserved. Root inventory
completion is a separate report status, not governance success. Root, shallow
history, Git replacement overlays, dirty/index, malformed/dangling, actual process and protocol cases live
in the dedicated runner/registration/CI tests. Initial parent checks share the
original-header reader across the new profile, scoped judge and event preparer.
The CI v2 source explicitly declares and generates the initial profile alongside
the workflow; its dedicated tests exercise actual root registration and rejection,
exact event argv, preserved v1 behavior, drift and adoption collisions. The public
[initial-host example](examples.md) verifies beta.5 on a native parentless first
push and subsequent ordinary integration/PR/dev DELTA checks, explicitly scoped
to macOS arm64. Source/toolchain bootstrap provenance, complete input closure,
cross-platform initial profiles, activation and deterministic parity remain
outstanding; inventory completion does not certify them.

The registered [worktree producer](worktree.md) adds actual fresh target fetch,
isolated branch creation and source/process identity reports. Its dedicated tests
exercise public library/CLI behavior with real remotes and checkout hooks, and
reuse the existing strict registration loader and bounded process engine. Creation
does not certify full governance, stale recovery, PR delivery or landing. The
adopted policy and native asset build are explicitly registered; public beta.8
ships this binary on macOS arm64 and Linux x86_64. Its native release recipe
passed all 49 dedicated worktree tests on each platform.

Explicit reconstruction reuses the worktree producer and its original process/identity helpers. Real CLI tests cover an advanced target, complete carry/retire choices, original source preservation, binary/mode/link/add/delete changes, literal paths, failed apply conflicts, source mutation, patch output bounds and empty/reused destinations. A successful result is a staged index tree, without a candidate commit or governance/integration verdict. AI reconciliation and provider delivery remain unfinished; registered maintenance is described below.

Worktree cleanliness observes tracked changes separately and excludes explicitly
registered artifact directories from ignored and nonignored Git inventories
before bounded capture. Dedicated regressions reproduce the former artifact-output
limit failure and preserve detection of unknown neighbors, literal/case lookalikes,
tracked changes, and files or symlinks at artifact directory names. Git pathspec
environment normalization is confined to those queries and recorded per process;
this does not certify complete Git configuration or external-input closure.

The product native release recipe consumes an explicit ordered set of registered
project actions, with preflight rejection and actual process bytes in build v2
reports. Dedicated consumers test literal arguments, action ambiguity, failure
exits and packaging suppression. Public beta.8 includes worktree; the completed
native release run passed the 15 distribution and 49 worktree tests on each platform, and anonymous downloads
verified the manifest plus both platforms’ worktree and installer assets.
This recipe does not certify full input closure or deterministic parity.

Registered worktree maintenance reuses the existing Git/process, strict registry
loader and artifact classifier. Real consumers cover failed-hook recovery,
reconciled reconstruction, original-report preservation, identity/lock/index
refusals, explicit artifact disposal, ancestor/squash preservation, retries and
actual removal/ref races and recovery of cleanup locks after retained-ref races. These operations do not certify governance or remote
landing. Interrupted/partial-rebind continuation and autonomous provider orchestration
remain pending; missing old receipts and damaged metadata use the explicit
rebind contract below.

Receipt-bound temporary-ref cleanup reuses maintenance receipt and saved-commit
validation. Real Git tests cover preservation, explicit absence retries, malformed
identity/retention, symbolic refs, concurrent ref updates, and removal followed
by a failed Git exit. It preserves the original failure and reports unverified
effects separately from verified absence. It requires a complete original report;
interrupted or missing receipts remain outside this recovery contract.

Released interrupted checkout recovery publishes immutable identity before
creation or cleanup locking and consumes an explicitly selected intent with
absent/partial result identity. Dedicated real CLI interruption tests cover
creation, reconstruction and cleanup, original-byte preservation, terminal-result
refusal, identity drift and publication collisions. It reuses the reconciled
checkout release checks and leaves the original outcome unknown. This intent-based entry does not handle missing intent or damaged metadata; caller
confirmation that the original process stopped is a premise, not a fact proven
by the intent. This entry is included in public beta.8.


Released interrupted fetch cleanup publishes a separate immutable identity before
fetch, without an invented base or outcome. `cleanup-fetch-interrupted` reuses
ordinary direct-ref/expected-OID cleanup, explicit commit retention and verified
absence; an already absent ref requires an explicit retry plan. Real CLI tests
terminate start/reconstruct after fetching and start after temporary-ref deletion,
check absent/partial original results, identity/retention drift, terminal-result
refusal, publication collisions and post-deletion evidence changes. Original
outcomes stay unknown; missing identity is not reconstructed by this fetch entry. This entry is included in public beta.8. The four landed public examples verify both interrupted checkout and fetch consumers on macOS arm64; see [the example index](examples.md).


Remote branch retirement now has an explicit `cleanup-remote` consumer, reusing
registered Git/process and saved-work checks. Real CLI tests exercise a unique
expected push endpoint, local/remote target binding, exact branch leases, squash
retention, dirty local-work preservation, failed post-deletion exits, concurrent
branch updates, changed targets/plans and explicit absence retries. This is distributed
in public beta.10; the actual public macOS binary retired the release and four
example adoption branches after verified landing. It does not certify remote
transactions, Git/delegated input closure, PR/merge/landing orchestration, full governance or deterministic parity.

Full CI context transport is an explicit `chrono-github-full-ci/v1` projection.
The CI tests reuse the actual bound-Git fixture for exact bytes, candidate/config/
provider/workflow bindings, missing history, failed processes, output collisions
and literal generated Bash. The workflow tests reuse the seven-judge fixture
for local and prepared integration/delivery contexts, including rejected stale
context. Preparation remains `not-evaluated` governance and `unestablished` parity.
See [the adoption and retry boundary](full-ci.md). No product full-host activation
or native generated dispatch execution is claimed by these consumer tests.

Public beta.10 CI consumption verifies release/full generation, drift handling and
exact context transport using the anonymously downloaded macOS binary. All four
[example hosts](examples.md) adopted the pinned public release through actual
local/integration/PR/dev checks, then verified public-origin start, local cleanup
and leased remote retirement. These are bounded product and scoped-host results;
full generated dispatch adoption, complete input closure, activation, parity,
AI reconciliation/PR/merge orchestration and formal refinement remain pending.


Explicit metadata rebind now has separate inspect/execute plans with declared
branch, HEAD, index tree, original metadata member, backup and donor. Real CLI
regressions cover absent gitfile/metadata/parent and old receipts, corrupt index,
relative or corrupt pointers with owned backlinks, byte/mode/symlink preservation,
input and reference drift, wrong ownership, collisions, original failed process
bytes and partial donor effects. Reconciled attached failures reuse existing
`recover`; the original failed report is retained. Backup is rechecked before
success. This source addition is not in beta.10 and does not recover a lost
historical index. Interrupted rebind continuation, concurrent/crash atomicity and
full governance remain outside the contract. See [worktree](worktree.md#explicit-metadata-rebind).
