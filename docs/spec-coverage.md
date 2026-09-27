# Current SPEC coverage

This is a maintained coverage map, not a run log or a second policy. Full SPEC remains the objective. `implemented` means the named contract has a real producer and behavior test; it does not mean this host is activated or delivered. `partial` retains the uncovered obligation. All full host registries remain proposed, with incomplete input closure and missing executable bindings. Registration/filemap success on a bounded host is not seven-judge success. Local checks do not certify native CI or landing.

Test sources: **R** = `crates/runner-tests/tests/protocol_v1.rs`; **G** = `crates/judge-registration-tests/tests/registration_contract.rs`; **L** = `crates/runner-tests/tests/cli_contract.rs`; **C** = `crates/judge-ci-tests/tests/behavior.rs`; **F** = `crates/judge-filemap-tests/tests/impact.rs`; **P** = `crates/judge-filemap-tests/tests/consumer.rs`; instruction and CI generator tests live in their dedicated test projects. Tests in G invoke the built runner and separately built registration executable against real synthetic Git commits, including real host registry material. Production build prerequisites are explicit in the CI binding. P runs actual runner → registration → filemap subprocesses on committed bounded hosts with absent historical binaries and exact report source pointers. F checks witnesses against independently specified endpoint edges. No peer/independent review evidence is claimed.

| SPEC section | State | Producer and direct evidence | Remaining obligation |
| --- | --- | --- | --- |
| 1 authority/boundary | partial | runner `full.rs` declared external DAG; R `dag_*` | remaining judge policies and lifecycle |
| 2 independent projects/adoption | partial | independent manifests, locks, bootstrap registration build | generic full bootstrap provenance/activation |
| 3 five registries | partial | registration `schema.rs`, G `strict_five_schemas_*`, `real_host_pseudo_script_*` | semantic pairing/routes/cost policy; full host ci-verify migration; external snapshots |
| 4 fixed snapshots/context/CI | partial | runner `facts.rs`, registration `lib.rs`; G `context_and_config_identity_mismatch`, `ignored_untracked_staged_and_wrong_checkout_fail` | full native CI/parity evidence, complete external/tool input transport |
| 5 DELTA graph | partial | filemap typed union/records/closure and CI adapter; F all tests, P `actual_runner_registration_filemap_*` | full execution, retained effective inputs and locality evidence |
| 6 evolution/removal | partial | affected reference checks; G `changed_references_*` | schema migration execution, retirement/replacement obligations and tests |
| 7 protocol | implemented within ordinary two-commit v1 | runner `wire.rs`, `full.rs`, shared process engine; R all tests | no full root/initial success mode; platform evidence limited to local Unix |
| 8 seven judges/mixed | partial | registration and filemap; G/F/P external pass/negative findings | projects, routes, cost, mixed, workflow |
| 9 report/cost | partial | runner `full.rs`; G `full_report_*`: all required fields, executable list with observed/configured distinction, findings and named-output sources, null/unresolved reasons, pass/warn/fail/error exits and stored/stdout agreement | full tools/effective inputs/test/cost producers; impact now supplied by filemap; unknown or conflicting results do not establish complete governance |
| 10 lifecycle | pending | caller owns current delivery; no runtime producer | fresh dev/integration, stale rebuild, provenance and landing orchestration |
| 11 script/plugin | partial | direct argv external transport; R real Python child fixtures | real script pairing/routes and plugin input closure |
| 12 bootstrap/initial | partial | registered bootstrap builds and installs registration/filemap | full source/toolchain provenance, activation migration and explicit initial transport |
| 13 acceptance | partial | row map below | all pending row obligations remain |
| 14 boundaries | implemented documentation | README/SPEC/this map | update as later increments land |
| 15 generic reference experience | partial | existing fixed-source references; no reference repo modifications | apply remaining generic workflow lessons |
| 16 instructions | implemented existing scope | instructions and dedicated tests, current docs/instructions.md | existing documented platform/crash/concurrency limitations remain |

Rows below follow §13 in order; scoped evidence is explicitly limited to that profile.

| §13 row/scenario | State and direct evidence | Remaining full contract |
| --- | --- | --- |
| 1 registered source M selects tests | partial: F `typed_content_mode_binary_*`, P actual impact, scoped C execution | full projects execution |
| 2 unregistered A / ignored dirt | implemented: G `changed_unregistered_fails_and_explicit_registration_repairs`, `ignored_untracked_staged_and_wrong_checkout_fail` | downstream test execution separate |
| 3 source plus membership | partial: G explicit repair | full selection and mixed warning classification |
| 4 deleted file uses base edges/costs | partial: F `base_only_test_edge_*`, P removed-test facts, scoped C | cost/retirement verdict and full execution |
| 5 rename D+A | partial: G D+A facts; F `typed_content_mode_binary_add_modify_delete_and_rename` | cost/retirement verdict and full execution |
| 6 changed/deleted edge seeds | implemented declaration impact: F `file_record_field_and_edge_*`, `file_edge_only_*`, scoped C execution | full projects execution separate |
| 7 legal retirement | pending | workflow/projects |
| 8 required test removed | partial: F base-only requirement, P `full_report_removed_test_*` | workflow/projects rejection or approved retirement/replacement |
| 9 nonunique/missing test pair | partial: strict required shape only | semantic one-to-one projects judge |
| 10 manifest undeclared dependency | partial: affected target references | semantic consumer comparison |
| 11 unknown input closure | partial: G `proposed_and_incomplete_never_receive_governance_success` | complete retained external/tool inputs, no discovery guarantee |
| 12 README no all-tests | implemented selection: F `no_delta_and_disconnected_docs_*`, C real nonexecution | full projects execution separate |
| 13 independent script tests | pending | real script owner/routes/projects |
| 14 duplicate operation | pending | routes |
| 15 bypass/missing operation evidence | pending | routes/projects |
| 16 mixed warning with integration | pending | mixed/workflow/cost |
| 17 engine stability | pending | workflow |
| 18 policy requires integration | pending | workflow |
| 19 changed integration binding | pending | workflow |
| 20 stale branch | pending | workflow/lifecycle |
| 21 threshold equality | pending | workflow with injected context time |
| 22 same local/CI command | partial: existing scoped CI generator | full native events and exact inputs |
| 23 same complete deterministic inputs | pending | deterministic evidence/parity comparison |
| 24 parity unestablished | implemented field: full runner report; G external host | unresolved closure remains nonzero |
| 25 staged/unstaged/untracked | implemented: G dirt cases | no hidden dirty mode |
| 26 missing history | partial: G `missing_oid_and_symbolic_endpoint_are_errors`; registration fork object check | complete workflow ancestry |
| 27 empty/invalid stdout | implemented: R `malformed_stdout_crash_and_bounds` | — |
| 28 crash/timeout/digest | implemented: R bounds and prelaunch digest tests | resource guard is infrastructure, not functional verdict |
| 29 unknown costs | pending | cost warning producer |
| 30 candidate migration validator | partial: G `base_executable_binding_is_data_only`, R predecessor forwarding | full migration/retirement evidence |
| 31 no DELTA | partial: G no-delta/dirt behavior | full input/workflow checks |
| 32 initial root | partial: full CLI rejects absent base, no green | explicit initial report mode |
| 33 check CLI | implemented bounded transport/registration plus existing slice | whole configured host remains nonzero until obligations implemented |
| 34 unknown/malformed CLI | implemented: L `unknown_and_malformed_commands_are_errors` | — |
| 35 spec status | implemented: L `status_help_and_version_are_information_only` | — |
| 36 help/version | implemented: L same test | — |

The host `ci-verify` entry is a scoped pseudo-script without full `path` and pairing fields, and its `chrono-ci` tool is absent from full config. G diagnoses the real shape, then repairs only fixture structure to expose the independent tool defect. No fake script project or schema exception is introduced. Registration rejects untransported external-input snapshots and unsupported schema changes with evidence/input errors. Full impact is a real versioned producer (see [contract](filemap-impact.md)); costs, routes, full test execution and integration are not filled with placeholder green results. Direct input declaration tests are not retained-external-input success: P `full_external_input_declaration_remains_unresolved_before_filemap` checks the existing nonzero registration boundary. Product transport and adopted host bindings change together; mixed-change cost estimates remain unknown, and native policy warning production is still pending.


This increment changes shared selection code and adopted policy together. **Mixed policy/product change warning:** new projects, explicit compile/execution edges, bootstrap/install bindings, artifacts and CI operations expand the affected validation surface. Costs remain unmeasured. The warning does not imply that the future mixed/cost judges have run. The applicable existing registration, CI executor, CI generator and instructions suites remain required; mutation evidence is bounded to named detection assertions, not proof of all graph semantics. Full current-host canonical check requires a clean committed candidate and stays with caller-owned lifecycle.
