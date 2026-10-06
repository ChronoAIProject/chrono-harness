# Current SPEC coverage

This is a maintained coverage map, not a run log or a second policy. Full SPEC remains the objective. `implemented` means the named contract has a real producer and behavior test; it does not mean this host is activated or delivered. `partial` retains the uncovered obligation. All full host registries remain proposed, with incomplete input closure and missing executable bindings. Registration/filemap success on a bounded host is not seven-judge success. Local checks do not certify native CI or landing.

Ordinary full-check acceptance means the DELTA satisfies the active registered contract. `declared-complete` is the trusted AI's accountable declaration of the host-defined governance/input scope; judges verify declared references, identities, snapshots and observed operations. Current registration emits `completeness_proven:false`, which does not prevent ordinary acceptance after the required registration and selected checks pass. Known missing inputs, unresolved required dependencies, missing bindings/snapshots and drift still fail; real toolchain/SDK/configuration obligations remain. A universal hidden-input-absence proof or VM is not an ordinary acceptance requirement.

Local/CI must use the same registered command. Universal equal verdicts remain conditional on equal complete effective inputs and deterministic evaluation. The separate parity comparator requires `completeness_proven:true` and complete verdict-bearing observations; absent these stronger premises, parity remains unestablished without gating ordinary full check. Pairwise observations alone do not prove universal determinism. This correction changes contract wording and default content, without changing runtime, schema or test behavior or completing full host adoption.

Test sources: **R** = `crates/runner-tests/tests/protocol_v1.rs`; **G** = `crates/judge-registration-tests/tests/registration_contract.rs`; **L** = `crates/runner-tests/tests/cli_contract.rs`; **C** = `crates/judge-ci-tests/tests/behavior.rs`; **F** = `crates/judge-filemap-tests/tests/impact.rs`; **P** = `crates/judge-filemap-tests/tests/consumer.rs`; **W** = `crates/judge-workflow-tests/tests/{branch,consumer}.rs`; instruction and CI generator tests live in their dedicated test projects. Tests in G invoke the built runner and separately built registration executable against real synthetic Git commits, including real host registry material. Production build prerequisites are explicit in the CI binding. P runs actual runner → registration → filemap subprocesses on committed bounded hosts with absent historical binaries and exact report source pointers. F checks witnesses against independently specified endpoint edges. No peer/independent review evidence is claimed.

| SPEC section | State | Producer and direct evidence | Remaining obligation |
| --- | --- | --- | --- |
| 1 authority/boundary | partial | runner `full.rs` declared external DAG; R `dag_*` | remaining judge policies and lifecycle |
| 2 independent projects/adoption | partial | independent manifests, locks, bootstrap registration build; SPEC §2.1 assigns shared mathematics to trureturing and operational/model correspondence to chrono-harness | generic full bootstrap provenance/activation; source ownership does not prove implementation refinement |
| 2.2 implementation/test language | partial: instruction guidance, shared full/scoped projects v2 pair checking and native test helpers | bilingual `test.same-language` atom is selected by fresh-host defaults and this host’s generated guides; explicit project/script language fields; affected pairs require equal IDs or shell→python; dedicated Rust schema/pair/metadata-DELTA cases; registered Rust CI transport, context judge, lifecycle, Cargo and Git fault helpers; main host v2 declarations, independent Python workflow-inventory unit, explicit worktree core/host-adoption groups with exhaustive inventory coverage, and a manifest-free Python release recipe/test pair with independent unit and real-binary integration groups | complete candidate acceptance and example adoption; migrate remaining embedded cross-language test logic and helpers; separately register genuine external-interface fixtures; declarations do not establish actual source language |
| 3 five registries | partial | registration `schema.rs`, G `strict_five_schemas_*`, `real_host_pseudo_script_*` | complete Cargo/SDK closure; bounded pairing/routes/migration/snapshots and v5 toolchain inventory implemented |
| 4 fixed snapshots/context/CI | partial | runner `facts.rs`, registration `lib.rs`; G `context_and_config_identity_mismatch`, `ignored_untracked_staged_and_wrong_checkout_fail` | config v3 binds full/initial Git facts through all full consumers, explicit scoped-v2 acquisition and provider-v3 event reads/fetches (docs/git-facts.md); full native CI/parity and Git/delegated/Cargo/SDK input closure remain |
| 4.2 CI caches and incremental builds | not implemented | independent registered target/bin outputs and artifact transport exist; current generated check/release workflows do not restore/save persistent build caches | explicit host cache registration and generated restore/save; dependency, current judge/bootstrap binary and per-project compilation caches including enabled Rust incremental data; compatibility keys, executable verification, cold/warm/changed-source/failure/concurrent native acceptance |
| 5 DELTA graph | partial | filemap typed union/records/closure and CI adapter; F all tests, P `actual_runner_registration_filemap_*`; SPEC §5.1 pins two-state locality and Theorem 6's conditional node-set consequence (merged trureturing PR #10868), reusing frozen ConsequenceClosure declarations, with prior exact-application compilation and the old execution-edge counterexample | complete host input closure and native activation; Rust refinement is unproved and reached-node equality does not preserve typed test selection |
| 6 evolution/removal | partial | W actual replacement, joint retirement, alias repair and candidate migration certification; registration workflow v3 selects explicit endpoint versions, inputs consumer certifies a host config v1→v2 decoder with original snapshots | further host converters and full host activation |
| 7 protocol | implemented bounded: ordinary two-commit v1 and explicit initial profile | runner `wire.rs`, `full.rs`, `initial.rs`, shared process engine; R transport tests; G actual root consumers | bootstrap provenance and activation; initial inventory never claims a green DELTA; platform evidence limited to tested Unix |
| 7.1 standard logging | partial: independent Rust logging library and one runner entry | `diagnostics` JSON Lines/tracing layer; dedicated sink/context tests; runner invalid UTF-8 argument path; original protocol/process channels retained | all-producer adoption, non-Rust adapters, registered formatting/sinks, subprocess context and common emergency/evidence publication |
| 7.2 exception chains and general handling | partial: explicit error records and native source ownership API | `diagnostics::error` wraps/imports `chrono-error/v1`; dedicated source/related/recovery/failed-sink cases; [adoption boundary](diagnostics.md) | native-error adapters and typed propagation throughout products, protocol adoption, persisted references and secondary-failure publication |
| 8 seven judges/mixed | partial | all seven bounded producers; W full-chain consumers | full host input closure and activation |
| 9 report/cost | partial | runner `full.rs`; G `full_report_*`: all required fields, executable list with observed/configured distinction, findings and named-output sources, null/unresolved reasons, pass/warn/fail/error exits and legacy explicit stored/stdout agreement | complete toolchain/input coverage and optional measurements; declared cost reporting is implemented; bounded routes tools and projects test evidence are supplied; unknown or conflicting results do not establish complete governance |
| 10 lifecycle | partial | W freshness and integration producer/consumer with completed reports; independent worktree producer and dedicated real Git tests cover fetched target creation, adopted policy, explicit reconstruction, conflict/failure preservation and registered recover/cleanup/cleanup-fetch, explicit leased remote retirement, retained-intent interrupted checkout and fetch consumers, and explicit metadata rebind without old receipts plus retained-intent continuation | AI reconciliation of conflicting/ambiguous recovery state and PR/merge/landing orchestration |
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
| 10 manifest undeclared dependency | implemented bounded: optional Cargo adapter: explicit policy, TOML path dependencies and missing-edge rejection; optional v4 binds Cargo/compiler file inputs and v5 adds explicit sysroot/backend/linker/SDK/build-script inventories with actual direct/full/scoped consumers | compiler-library/backend/linker/SDK semantic closure; guarded registry/Git resolution and full host activation remain |
| 11 known missing inputs / unresolved required dependencies | implemented bounded: explicit external nodes, required project edges, declared-complete v3 regressions, v5 directory/toolchain inventories, optional versioned input-domain coverage and `proposed_and_incomplete_never_receive_governance_success` | this host's declared Cargo/SDK/configuration scope remains incomplete; no dependency-discovery or universal completeness guarantee |
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
| 22 same local/CI command | implemented bounded: scoped CI generator with explicit push-baseline configuration; CI behavior tests cover repeated pushes, remote advancement, missing refs and real CLI persistence; C `units::generated_unit_workflow_cli_adoption_uses_the_local_canonical_command` executes the generated unit prepare CLI against a committed event, checks the candidate-bound workflow/context, and runs the recorded check argv; [native adoption evidence](native-ci-adoption.md) records successful live push and pull-request collection plus every declared unit for the public Go, TypeScript and mixed hosts | complete effective-input closure, provider-v3 host adoption, deterministic parity and full host activation remain |
| 23 same complete deterministic inputs | implemented bounded: separate `chrono-harness parity` requires `completeness_proven:true`, compares verdict-bearing projections, preserves unresolved evidence and updates canonical/retained reports; runner parity tests cover equality, incompleteness, verdict drift and publication | host compiler/SDK/external-input evidence and native local/CI adoption; pairwise evidence proves neither undisclosed input completeness nor universal determinism |
| 24 parity unestablished | implemented field: ordinary full runner report; G external host | stronger parity premises remain unestablished, without gating ordinary full check; known missing required inputs still fail |
| 25 staged/unstaged/untracked | implemented: G dirt cases plus shared raw tree/index/physical identity checks, clean-filter/CRLF/mode/symlink counterexamples, full/initial/scoped and worktree consumers | complete configuration/OS input closure and concurrency isolation remain separate |
| 26 missing history | implemented bounded: G object checks, W common ancestry and shallow boundaries; worktree producer fetches its explicitly registered target | general missing-history recovery/reconstruction remains external |
| 27 empty/invalid stdout | implemented: R `malformed_stdout_crash_and_bounds`, R/L embedded-invalid-UTF-8 subprocess/CLI cases and valid U+FFFD controls | — |
| 28 crash/timeout/digest | implemented: R bounds and prelaunch digest tests | resource guard is infrastructure, not functional verdict |
| 29 unknown costs | implemented: cost consumer warns with zero exit, preserves null and known coordinates; no-Delta retained-input case | full host activation |
| 30 candidate migration validator | implemented finite contract: W original schema versions, real conversion/compatibility test, old method nonexecution and judge retirement; inputs consumer covers v3 version selection and config v1→v2 certification | further host converters and native activation |
| 31 no DELTA | implemented bounded: G no-delta/dirt behavior; W `no_delta_still_runs_the_full_workflow_gate_without_business_tests` executes the complete judge chain with an empty DELTA and no business test | full native activation and complete host input closure |
| 32 initial root | implemented bounded: G root inventory, real/shallow parent rejection, checkout/reference failures and no activation | full bootstrap provenance/activation |
| 33 check CLI | implemented bounded transport/registration plus existing slice | whole configured host remains nonzero until obligations implemented |
| 34 unknown/malformed CLI | implemented: L `unknown_and_malformed_commands_are_errors` | — |
| 35 spec status | implemented: L `status_help_and_version_are_information_only` | — |
| 36 help/version | implemented: L same test | — |
| 37 standard program diagnostics | partial: Rust library and runner invalid-argument entry | complete producer adoption, registered sinks and context propagation across process boundaries |
| 38 multilayer/cross-process error chain | partial: explicit typed source/related API and versioned record import | actual native adapters, judge transport adoption and immutable evidence references |
| 39 handled exception and eventual success | partial: recovery events retain the original error record | adopt every handling/retry/recovery boundary in products and adapters |
| 40 secondary handling/publication failure | partial: source/related API and sink failures retain attempted records | publish combined failures through the declared emergency/evidence boundary without fabricated references |
| 41 open/infinite input classes | pending: SPEC §7.2 contract only | general semantic rules and unknown-member behavior; representative/boundary/counterexample checks do not prove exhaustive coverage |
| 42 cold and exact-hit CI cache | pending: SPEC §4.2 contract only | actual persistent restore/save and current executable verification; selected judgments/tests run in both states |
| 43 changed-source incremental cache | pending: SPEC §4.2 contract only | reuse compatible per-project fingerprints/dependencies/incremental data with the same build entry; bind candidate judge outputs |
| 44 incompatible compilation inputs | pending: SPEC §4.2 contract only | partition incompatible toolchain/target/profile/features/input domains and rebuild before use |
| 45 missing/corrupt/unavailable cache | pending: SPEC §4.2 contract only | retain original error and same-entry recovery; real build/binding failure remains nonzero |
| 46 concurrent jobs/reruns and later test failure | pending: SPEC §4.2 contract only | isolated live outputs, joined cache producers and truthful build/test/evidence states |
| 47 same-language production/test pair | partial: v2 declarations and affected reciprocal-pair checks; registered Rust helpers for CI and lifecycle fixtures | complete host migration and real registered execution; declarations alone do not establish source language |
| 48 declared or observed mismatched/embedded cross-language test logic | partial: projects rejects declared language mismatch; Rust regressions cover both projects and scripts | remaining foreign-language test logic migration and observed-entry correspondence; outer-language wrappers do not establish compliance; no claim to detect arbitrary hidden logic |
| 49 Shell production / Python tests | partial: the declared shell→python exception is directional and covered by dedicated pair tests | actual host adoption with independent ownership, dependencies, methods and real test evidence |
| 50 mixed-language host isolation | partial: pair tests accept arbitrary literal language IDs and paths; language-only DELTA is covered | mixed-host adoption and complete per-unit execution without inferred language, shared giant tests or a Rust host requirement |

The registered execution increment adds routes/projects, FILEMAP v2, retained endpoint inputs and the finite chrono-ci-check/v1 / FILEMAP v1 decoder. See [the execution contract and exact boundary](execution.md). Dedicated routes tests check ordered plans, actual argv/environment/tool bindings, receipt tampering, PATH shadow and byte replacement. Dedicated projects tests run the actual runner/registration/filemap/routes/projects chain on committed project/script hosts, cover exclusive pairs, manifest-free/custom-action hosts, explicit output isolation, retained input failures, real exits/effects, blocked dependents, docs nonexecution and mapped replacements. Its migration consumer uses real old repository registrations and actual ci.verify, including workflow drift and restoration. Existing test identities remain; the historical pseudo-script rejection reads the fixed old tree. Maintained regressions also cover retained inherited-environment changes through the full chain (including absent/empty, overridden and disconnected controls), registered intermediate workspace rejection before operations, both protocols' embedded invalid UTF-8, arbitrary operation bytes, and migration version failure diagnostics. The host interpreter binding is explicit macOS data; the failed original native run and the verified repaired native results are recorded in [CI documentation](ci.md).

Config v3 adds an explicit candidate Git binding before acquisition and through all seven full judges, initial inventory and input capture. Its optional `chrono-git-inputs/v1` guard checks explicitly referenced file identities or absence before the version probe and around every Git process, preserving the original child result on post-call drift. Real full/initial consumers and runner regressions cover adoption, no-launch failures, per-call mutation, request binding and legacy controls. Dedicated regressions preserve version 1/2 semantics and original process failures; the seven-judge integration/delivery consumer rejects ambient Git. This is bounded executable/environment and declared-file binding, not complete Git/OS/delegated input closure.

The full-entry native selector increment reuses the same strict `chrono-git-configs/v1`
parser and OS-ARCH mapping for full, initial, input capture, scoped optional
registration, full-CI check configuration and worktree registry acquisition. `runner-tests::selector_snapshot_keeps_entry_and_independent_endpoint_targets`
checks fixed base/candidate targets, independent target paths, missing-target rejection
and real-path byte retention. Registration tests execute selected initial inventory;
workflow consumers execute selected seven-judge integration/delivery. Input consumers pass actual captured/paired
selected v3 snapshots through all seven judges, including direct/map transitions,
independent target paths, historical target deletion, old-only dependency/test edges
and docs nonexecution. They execute a selected workflow schema migration with a v3
decoder and compatibility test, and reject missing/substituted bindings and rewritten
historical bytes/config. Projects tests exercise scoped optional selected registration
against a direct v3 control, including effective tools/environment, actual operations
and canonical invocation rejection. Workflow tests prepare and consume a selected
full-CI command. Full/initial/
registration/mixed/workflow consumers now carry entry/effective identities and selector
metadata through their existing request and view contracts; selected historical views
use versioned view/decoder bindings and selected input captures use snapshot v3. This resolves the full
configuration entry obstacle. Worktree's shared callback snapshot acquisition now
resolves the source workflow before fetch and independently resolves the fetched
endpoint, retaining complete selector/effective registry digests. Actual CLI tests
cover different effective configuration/workflow paths with unchanged worktree
policy, dirty source preservation, missing/unsupported endpoint rejection and a
changed selected workflow target without checkout creation, selected reconstruction,
and selected recovery/cleanup with original failed-process evidence. Direct controls
remain. The worktree producer and bound Reader share the bounded explicit-registry
batch acquisition. Tests bind each endpoint to original metadata/content bytes and
retain actual failed or malformed batch output before fetch or checkout creation;
reader tests retain output partitioning, guard, identity and legacy controls.
The next full-SPEC gaps remain full host activation, complete effective
inputs, native full dispatch and lifecycle delivery; parity and formal Rust
refinement are also unfinished.

CI provider v3 binds event acquisition through an explicit full-v3 `facts_config`. Dedicated `ci-tests/tests/git_facts.rs` regressions use real Git objects/remotes and the actual CLI to check missing-history acquisition, exact probe interpretation, failed probe/fetch evidence, candidate policy/checkout binding, declared environment, generated initial-profile selection and legacy controls. Native generated-v3 host adoption, network/credential/configuration closure and parity are still outstanding.

The public beta.17 `chrono-git-configs/v1` selector lets scoped/provider facts entries
choose an explicitly registered native-platform policy while preserving one
canonical entry path. Reader regressions cover real Git selection, both fixed
candidate blobs, invalid maps/targets, request substitution, pre/post-call drift
and original failed-process evidence. Actual scoped and provider CLI consumers
exercise source/docs locality and push/PR acquisition through the selected policy.
Direct policies retain their contract. This is a prerequisite for native host
adoption, not activation, complete input closure or cross-platform parity; the
full root/initial host configuration is not selected by this extension. It is
distributed in beta.17; see [Git facts](git-facts.md#explicit-platform-configurations).

Scoped profile v2 independently opts into Git binding with policy.facts_config. Dedicated judge-ci tests exercise actual CLI selection with ambient Git shadowed, documentation nonexecution, both endpoints, v1 migration and old-only deletion, initial root/child/shallow behavior, fixed-policy/digest/version errors, dirty/index/checkout/post-execution guards and retained binary output/exit/timeout/overflow. The actual projects consumer checks optional full-registration interpretation and ordered execution through the same reader, with process-count/byte-hash checks and rejected canonical argv. V1 controls remain; the host has not adopted scoped v2.

Scoped-v3 collection has explicit optional manifest/report byte limits, retaining
64 MiB defaults and all original evidence verification. Boundary consumers cover
exact limits, overflow, invalid policies and original reports larger than 64 MiB.
Scoped-v3 report serialization removes indentation amplification while retaining
original process bytes and identical stored/stdout reports. Public beta.18 distributes
these changes. Its fixed-source native release passes 601 macOS and 596 Linux
tests across all fifteen dedicated projects; all 35 public assets match their
original hashes/sizes. Anonymous manifest and both platform installers were
verified. Native Go adoption evidence is scoped separately below; this does not
certify full effective-input closure or deterministic parity.

The bounded parity increment adds `chrono-harness parity --host-root H --report P --compared-report Q`. It compares two independently published full reports only after both producers state `effective_inputs.completeness_proven: true`, requires all verdict-bearing fields and executed judge responses, normalizes absolute executable paths by observed digest/version, and preserves a failed comparison as an unresolved nonzero result. It updates both the named report and its retained `report_path`. This establishes pairwise deterministic evidence for a declared input boundary; it does not certify undisclosed host inputs, native adoption or universal determinism.

Full activation and all other remaining obligations stay active. Generic project fixtures validate declared method execution and ownership; independent Cargo fixtures validate explicitly adopted TOML consistency and the bounded v5 toolchain inventory. Neither proves complete Cargo/SDK input closure. Single-worker checks do not certify independent review, native CI or landing.

Source snapshot acquisition now omits explicit artifact directories from Git's
untracked output across full/initial/scoped and registration/projects
consumers, sharing literal exclusions with worktree checks. Raw inventory APIs
remain available. Scoped and actual full/initial CLI regressions cover artifact
volume above the Git output bound; ignored neighbors, literal lookalikes,
noncanonical declarations and bound-input drift retain rejection. A real
migration consumer allows declared decoder outputs while rejecting undeclared
files and tracked input mutations. These checks concern inventory boundaries,
not effective-input completeness or native full activation. Public beta.19
distributes the change; its fixed-source native recipe passes 606 macOS and 601
Linux tests across all fifteen dedicated projects. All 35 public asset digests
and sizes match the original packages; anonymous manifest and both installer
downloads were also verified. See [artifact inventory](git-facts.md#registered-artifact-inventory)
and [release evidence](distribution.md).

Public [beta.11](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.11) adds explicit metadata rebind and literal checkout identity, alongside the previously released Git binding, CI generation, remote retirement and recipe v3/report v4. Its [fixed-source native build](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36566708631) passed 391 tests across ten dedicated projects on each of macOS arm64 and Linux x86_64, including full/scoped Git-binding consumers. Both original reports retain 31 actual processes, 28 staged consumer destinations and 24 matching identity observations. This validates those native consumers; generated provider-v3 host adoption, full input closure and deterministic parity remain outstanding. The current host release workflow is generated from an explicit chrono-github-release/v1 source; see [release CI](release-ci.md). That generator extension is distributed in beta.10; the linked native release uses its generated workflow.

Public [beta.16](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.16)
distributes report comparison, explicit input bindings/coverage, Cargo v5 inventories
and declared Git file guards. Its [fixed-source native release](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36655069168)
passed 588 tests on macOS arm64 and 583 on Linux x86_64 across all fifteen dedicated
test projects; five existing macOS filesystem-alias tests explain the difference.
Both reports retain 36 successful processes, 30 consumer destinations and 34 matching
identity observations. Anonymous macOS installation into a local Go-host clone
verified both existing units, config/absence drift rejection, restoration and
original historical bytes. The observed Git report required an explicitly increased
host transport bound; see [public consumption](git-facts.md#public-binary-consumption).
This does not adopt the local facts configuration in remote/native CI, establish
complete input closure, activate full governance or certify full local/CI parity.

Public [beta.17](https://github.com/ChronoAIProject/chrono-harness/releases/tag/v0.1.0-beta.17)
distributes explicit native-platform Git facts selection. Its
[fixed-source release](https://github.com/ChronoAIProject/chrono-harness/actions/runs/36661720249)
passed 596 macOS arm64 and 591 Linux x86_64 tests across all fifteen dedicated
projects, with 36 successful processes, 30 consumer destinations and 34 matching
identity observations per platform. All 35 public assets match their original
hashes/sizes. Go's original beta.17 candidate verified native Linux policy selection
but failed collection on oversized reports. Public beta.18 repairs collection;
Go PR #18 adopts that release with matching local and native push/PR evidence.
See the [precise adoption boundary](native-ci-adoption.md#native-git-policy-adoption).
Full host activation, effective-input closure and parity are still outstanding.

This increment adds mixed classification and adopted registrations together. **Mixed policy/product change warning:** the project pair, semantic comparisons, bootstrap binding and execution plan expand the validation surface. Costs remain unmeasured. Dedicated tests validate the mixed report and shared cost consumer; host activation still requires the outstanding governance and input obligations.

The cost judge and its dedicated tests implement the [declared cost contract](costs.md): distinct endpoint values and source pointers, old-only costs, known/unknown coordinates, identity deduplication, affected-project members, declaration-only changes and docs locality. Real subprocess consumers verify pass/warn exits, missing impact failure, source fidelity and retained-environment impact with empty Git DELTA. Measurements are explicitly absent; no additive resource total or full-governance claim is made.

The mixed judge implements [explicit rule/product classification](mixed.md). Dedicated behavior tests cover normal member/edge additions, real action changes, endpoint surfaces, array identity and ordering, historical patterns, named JSON and malformed inputs. Real subprocess tests check warnings, pass/error exits, exact cost and source fidelity, rule-only outputs, missing predecessors, stale binding rejection and strict UTF-8. Workflow remains responsible for stability and integration certification; this increment supplies its rule-change input.

FILEMAP impact v2 now lowers explicit workflow stability and conditional integration test requirements before route execution. Dedicated tests retain base requirements, old costs and causal source pointers; real consumers prove additional tests run and shared operations run once. Named semantic-input errors now stop the FILEMAP stage. The mixed classifier remains the single implementation, with downstream FILEMAP/cost dependencies removed from its compilation graph. The independent [workflow judge](workflow.md) now certifies bounded freshness, completed integration, legal joint retirement, alias repair and finite migration. See [workflow selection](workflow-selection.md).

Workflow verdicts use context v2 with an explicit integration/delivery role. W exercises the real seven-judge chain, changed and stale evidence, missing final reports, actual receipts, skipped judges, failed tests, docs locality, custom branch prefixes, invalid names and shallow history. Runner transports actual prior process identities and retains unique completed reports. Cost consumes historical definitions as well as current declarations. These are bounded engineering checks, with no independent-review or complete-input claim.

The [retained input producer](inputs.md) supplies actual capture/pair snapshots, preserves absent/empty environment values and transports content-addressed state blobs between explicit roots. Registration streams retained and candidate file identities; reports remain representation-independent. This removes inline-JSON size as a prerequisite blocker for large toolchain files. V5 now supplies an explicit bounded Cargo toolchain inventory, but does not yet supply complete Cargo/SDK dependency declarations or activate full native governance.

The host-neutral increment removes mandatory manifest/lock/root fields and the closed action vocabulary. `registered_intermediate_workspace_rejected_before_operations_and_standalone_passes` now belongs to judge-cargo-tests; the old mixed pair/manifest/route regression is split between generic pair/ownership/routes and the actual Cargo missing-edge consumer. The independent Cargo adapter supports chrono-judge/v1 or a registered standalone prerequisite; its structural-only contract still rejects external dependencies. The separate registered run entry validates retained registry/Git package inputs, actual locked/offline metadata, exact features/aliases/kinds, configuration ordering and before/after identities. Tests inputs and input_consumer exercise actual Cargo through direct/full/scoped consumers, including rejection and documentation-only DELTAs. The guarded configuration inventory validates explicit lookup presence/absence, ordered recursive includes and before/after mutations. Its v3 contract adds an explicit choice between ancestor inventories and an all-ancestors-absent assertion, plus host-relative Cargo home; v2 interpretation remains supported. V4 binds Cargo/compiler selection, and v5 adds real sysroot, backend, linker, SDK and build-script input inventories with selection-conflict and identity-drift rejection. Dedicated configuration, inputs and full/scoped consumers check these paths. Complete delegated/OS/network input closure, activation and native full parity remain outstanding.

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
to explicitly connected input IDs, paths, hashes and observed versions. V5 extends
this with explicit directory manifests and selected linker, sysroot, backend, SDK
and build-script inputs, preserving before/after identity checks and report
evidence. Tests exercise real compiler/toolchain selection, rejection before
metadata, configuration/include and wrapper conflicts, persistent post-consumer
mutation and full/scoped docs locality. V2/v3/v4 meanings remain supported. This
does not certify compiler libraries, delegated processes, SDK metadata, undiscovered
build-script reads or complete native parity.

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
landing. Retained-intent rebind continuation is described below; autonomous provider
orchestration remains pending; missing old receipts and damaged metadata use the explicit
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
success. This addition is distributed in beta.11 and does not recover a lost
historical index. Retained-intent continuation is described below; concurrent/crash
atomicity and full governance remain outside the contract. See [worktree](worktree.md#explicit-metadata-rebind).


Literal checkout comparison closes demonstrated configuration-hidden snapshot
mismatches. Runner tests check raw blob hashes against real SHA-1/SHA-256 Git
objects, executable class, CRLF normalization, clean filters, symlink emulation,
physical parents and staged cancellation. Full/initial and scoped consumers reject
hidden mode changes, including after an operation. Worktree consumers preserve
unsaved work on creation/recovery/cleanup; they share the same tree/index comparer
through the existing registered Git runner. This addition is distributed in
beta.11. It establishes neither full input closure nor deterministic parity;
see [Git facts](git-facts.md#literal-checkout-identity).

Source cleanup disposes only explicitly selected untracked artifacts under its
owned lock before the bounded Git removal. Real consumers verify effect ordering,
tracked-content refusal before disposal, preserved external symlink targets, and
original Git failure plus explicit retry. This is distributed from beta.12; recovery of
older partial source deletion and atomic concurrent/power-loss disposal remain
outside this increment. See [artifact disposal](worktree.md#declared-artifact-disposal-before-checkout-removal).

The beta.11 release and [all four example upgrades](examples.md) have exact
landing and dev evidence. A public macOS mixed-host consumer additionally
verified hidden-mode rejection/preservation and explicit metadata rebind with
a damaged index and absent prior receipts, preserving saved/unsaved work and
old metadata. That public beta.11 consumer did not exercise interrupted rebind.
Complete input closure, full activation, deterministic parity and Rust formal
refinement remain unfinished.

Public beta.12/13 scoped CI provides explicit complete-plan unit ownership,
shared-operation declarations, independent workflows and offline collection.
Global DELTA obligations are checked before partitioning; gathering binds original
workflow sources, endpoints and attempts. The [public examples](examples.md)
verify separate SDK profiles, actual failure/cancellation isolation, selective
retry and zero-DELTA collection. The three business hosts’ binary upgrades preserve
CI/SDK sources and generated workflows byte for byte. The product host also
registers sixteen independent complete-plan workflows and a collector, with an
explicit three-tool bootstrap configuration. The bootstrap regression exercises
configured operation/install selection, unchanged default behavior and failure
propagation. Historical repair and initial-profile consumers use the adopted
provider without reinterpreting historical definitions. Public beta.15 historical
method replacement support binds exact original/current methods and mapped
owners, rejects missing/stale/duplicate/unbound declarations, retains the omitted
field's strict contract and exercises actual drift detection and repair.
Ambiguous provider runs
still fail explicitly; this does not certify full input closure or governance.

Public beta.13 adds owned projection migration from explicit scoped v1/units
sources to units. Tests preserve custom initialization, source bytes and unrelated
workflows; old edits and new destination collisions reject before writes. Actual
public macOS consumers migrate the three delivered host configurations and verify
the resulting workflow bytes. Initial/full/release migrations, multi-file atomicity,
complete input closure, full activation, deterministic parity and formal refinement
are not certified by this mechanism. See [CI customization](ci-units.md#host-customization-and-updates).

The full-v3 unit core reuses the runner's pure assignment/selection helpers in
both scoped CI and the seven-judge full entry. It validates global obligations
and the operation DAG before partitioning. Integration and delivery slices
validate local actual evidence, produce contributions, and defer completion.
One sealed original request template reconstructs each judge stdin; selected
reports retain exact process bytes in a lossless compact encoding and addressed
context/input/evidence closure within the registered bound. Collection launches
no business executable (including version commands), checks actual retained
success/warnings and declaration-derived contracts, and produces source-linked
coverage without invented aggregate execution rows. The ordinary full workflow
policy and delivery certificate consumer validate the finalized collector report
and all original source evidence. Ordinary unscoped full check preserves local
streaming blob validation without the portable unit/report bound. Provider
generation and automatic transport use the existing CI owner and explicit
full-context/upload mappings. Actual full native host activation, Go adoption and
public release remain pending; parity and runtime speedup are unestablished.


Source `resume-rebind` consumes an original v1 rebind intent, its exact plan and
explicit missing/partial/failed result identity. Initial and resumed execution
share the same preservation and attachment steps. The dedicated worktree tests
terminate real producer processes around every governed Git phase, preserve
visible work, original result/intent/plan and backup bytes, and retry interruptions
of continuation itself. Conflicting inputs, foreign ownership, index drift,
completed results and ambiguous partial allocations reject without discarding
work. Caller reconciliation of unknown temporary files/Git locks, concurrent
writers and power-loss recovery remain outside this bounded contract. This source
extension is included in beta.15; see [continuation](worktree.md#continue-an-interrupted-or-failed-rebind).

Optional `chrono-input-coverage/v1` records explicit consumer/domain scopes and
requires exactly one disposition per registered pair. Bound and absent rows
reference the same consumer's nonempty bindings; absent rows require actual
registered file absence. Declared-complete configurations cannot omit bindings
or retain unresolved domains. Dedicated real-runner regressions cover historical
v3 adoption without rewriting base bytes, missing/empty/misbound/duplicate rows,
unknown consumers, declared absence contradicted by real bytes, and draft/legacy
controls. Omission preserves the old v3 contract. This supplies a checked
accounting surface for host-chosen input domains; undisclosed reads, complete
input closure, native activation and deterministic parity remain outstanding.

The simple-fixed increment supports schema4 short global/scoped check and
value-less scoped collection through existing runner/worktree/CI owners. Real CLI
tests cover remote/HEAD advancement, dirty and missing inputs, true short entry,
independent units, zero-business collection, pins/stale/failure rejection and
local execution of generated native steps. Full unscoped context preserves the
original worktree birth/fork and exact native context bytes. Local immutable
round reuse now checks a fresh observation through the workflow age predicate,
renews expired contexts and retains original bytes. Unit file capture/validation
and per-unit collection comparison share registration/input's explicit projection;
shared and governance obligations remain required. Focused source fixtures do not
establish native Go/TS execution. Native hosted execution, release/example adoption
and complete SPEC acceptance remain pending; full transport fixture success does
not activate proposed host governance.

Full core/provider/short source integration supports declared full-v3/v4 independent
units, global DELTA bare check, original-evidence full collection and the existing
automatic units provider via explicit v2 full-context/upload mappings. Source
fixtures do not activate this host or publish new public binaries. Clean-candidate
admission, native full workflows and complete SPEC acceptance remain outstanding.

The native provisioning source increment adds versioned streamed retained blobs,
offline composition of genuine endpoint pairs using the existing historical unit
projection, and the optional units/v2 native-adoption extension. The generated
adapter delegates existing event endpoints, publishes one original schema2 seed
in detection, acquires its fixed original producer binding without polling and forwards authentic
acquisition fields through the real short producer. Detection captures registered governance inputs; collection composes those
originals and selected unit inputs, and performs no business/SDK operations.
A separate actual current observation reaches the workflow age judge without
changing the common original context or earlier successful reports. Delivery preparation carries
nullable evidence; workflow alone selects whether the finalized certificate is
required, retaining all nine bindings. See [input composition](inputs.md),
[external closure](execution.md) and [host declarations](ci-units.md).

**Mixed policy/product change warning:** reusable code/templates and host FILEMAP
and verification-operation registrations change together. Costs remain unmeasured.
The existing host selections remain scoped; release, main/example activation,
actual native push/PR/dev evidence and parity are caller-owned outstanding work.

## User-directed conditional-job generator correction

Implemented source and scoped host projection: opt-in `chrono-job-gating/v1` on
units v1/v2 produces one detector, explicit job-level conditional unit jobs and
an `always()` aggregate needing all jobs. The sole required check is the aggregate;
actual checks still use the fixed short entries and final original-report judge.
The product host now generates one parent with 18 units and retires the exact
16 owned obsolete projections. Git supplies complete trees and registered blobs without a file-count gate. Legacy
providers and previously published native evidence retain their meanings.

Dedicated real-Git/actual-command coverage is in
`crates/ci-tests/tests/gating.rs`: isolated/multiple/empty DELTA, two-endpoint
edge/assignment changes, deletion/rename, complete >3000 paths,
multi-commit push, PR base/head, explicit frozen integration baseline, shallow
missing endpoint acquisition without business source checkout, creation/deletion
and structural failures; actual selected-unit reports and zero-operation collection;
failed/cancelled/skipped/missing jobs and detection; artifact/source/run/attempt
mismatches; independent job retries with original prerequisite attempts;
customization/owned retirement/init/migration conflicts; and the copied
installed-tools example's actual passing/failing short commands. Full declaration
scheduling uses FILEMAP/routes without context/SDK snapshots; final effective-input
admission and original seven-judge evidence remain required. Both scoped and full
scheduling reject decoder-dependent history before interpreter acquisition.

No native Actions/rerun/branch-protection activation or public release is claimed.
Full native context, original birth/input provisioning, large streamed artifact
composition and decoder-required historical scheduling remain separate boundaries.
This correction changes product behavior and host policy/projections together;
`W_MIXED_JUDGE_PRODUCT` applies, with no approval or consensus claim.


Explicit same-project test groups extend the shared registration binding owner,
full/scoped routes, DELTA record targets, pair validation and existing cost nodes.
The product host assigns its workflow suite to three explicitly registered groups.
An inventory guard checks the current groups for nonempty, disjoint and complete
coverage, while the unfiltered release action remains intact. Local owner
and group results belong to their recorded source; native group checks, release
platform checks/publication and full main/examples activation remain caller work.
This changes product source and host policy together; the added validation cost is
three inventory checks plus directly affected owner tests and exhaustive group runs.

## Registered automatic lifecycle cleanup

The candidate worktree owner implements opt-in exact artifact policy, surviving
coordinator state, successful birth enrollment, explicit import, registered-command
managed use, terminal finish and shared finish/start/reconstruct/maintain drain.
Legacy Cleanup remains the whole-checkout mechanical owner. Local real-Git
fixtures cover full deletion and protected/cache-only work, use admission, identity
and retained-ref drift, symlink containment, partial filesystem/Git effects, original
report preservation and retry. Main host adopts a cache-only policy retaining
state/bin and generated lifecycle instructions. Actual deployed binary adoption,
main-host finish/landing and existing-host reclamation are caller-owned acceptance;
local fixtures do not prove those events or periodic idle cleanup. Interrupted
unknown jobs/state need explicit reconciliation; no general archive, scheduler,
process scanner or concurrent/power-loss transaction is claimed. See
[worktree contract](worktree.md#opt-in-automatic-lifecycle-cleanup).

The v2 candidate adds stable kernel admission/enrollment leases, no-finish cache
recovery, immutable cache retry intents, short-check/bootstrap participation and
explicit Python/nested-runner forwarding. The local real Cargo test regression
checks lease retention by a distinct native child after both wrappers die.
Committed composition, supported native platforms and complete host adoption
remain acceptance obligations.
Legacy v1 records remain protected; no retrospective ownership is fabricated.

Automatic retry evidence uses versioned retained-input references, preserving
immutable receipts in place and storing opaque partial bytes once at the surviving
coordinator. The dedicated worktree regressions
`automatic_retries_keep_original_evidence_without_recursive_growth` and
`cache_retries_reference_failed_and_partial_originals_without_growth` exercise
repeated real terminal/cache failures, bounded report growth, original-byte
preservation, transitive drift/missing/symlink refusals and partial-result
retention without claiming success. Explicit maintenance keeps its prior shape.
This does not dispose historical evidence, deploy the coordinator or establish
complete native acceptance.

The runner retains monotonic kernel-exit observations across its anonymous launch
tree. A resumed monitor can join a child observed terminal before its deadline;
late or missing evidence never adds execution time. Protocol real-process tests
suspend only the nested monitor, independently observe child exit, and distinguish
on-time exit from late exit and output overflow. Cancellation and ownership cleanup
keep their separate failure paths. The shared layout rejects incompatible owners;
all participating binaries must be upgraded together. This behavior does not
establish the cause of other host timeouts or full candidate acceptance.

The registered scheduler accepts optional explicit launch priority while retaining
dependency readiness, cap, failure propagation, exclusive resources/outputs and
canonical report order. Real-process rendezvous tests cover reordered launches,
unlisted work, blocked prerequisites and conflicts; schema tests reject malformed
IDs and FILEMAP tests cover both-endpoint changes, reordering and removal. Empty
priority preserves legacy plan identity; nonempty priority is bound by the plan
and retained comparison. Main explicitly prioritizes its workflow prerequisite
and long-test chain with cap 2. Timing sufficiency, committed canonical/native
acceptance, deployment and complete SPEC delivery remain separate obligations.

The release recipe v5 consumes explicit build subsets and a mandatory per-verifier boolean Rust toolchain choice. Python verification can execute without available Rust tool bindings; actual Rust asset consumers retain declared build dependencies without requiring the compiler. The local scheduler uses registered dependencies and bounded concurrency; the collector checks each selected producer, tool evidence, raw process and complete unit membership. Rust behavior tests remain Cargo actions, Python behavior/driver/assertion code and external fixtures have their own registered owner. This source increment does not establish native candidate acceptance, public distribution, remaining helper migration or full host activation.

The host bootstrap has an explicit Python production/test script pair and separate Python SDK and operation fixtures. Its independent CI and release units execute the registered Python test directly, including source identity, selected configuration, installed digests, toolchain installation and failure propagation. Rust lifecycle integration tests retain their real bootstrap boundary. Complete candidate/native acceptance, remaining language ownership and public adoption are separate obligations.
