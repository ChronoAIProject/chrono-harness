# Native generated-unit adoption evidence

This document records live GitHub Actions evidence for the generated
`chrono-github-units/v1` projection. It closes the bounded adoption question
for the public Go, TypeScript and mixed example hosts: each host ran its
generated collection workflow and every declared unit on both a
`pull_request` event and a `push` event. All runs below completed with
`success` on 2026-09-29 UTC and used the listed candidate commit.

The evidence is about generated unit workflows and their collection. It does
not establish complete effective-input closure, `chrono-github-ci/v3` host
adoption, universal local/CI parity, or full seven-judge host activation.

## Recorded runs

| Host | Candidate | Declared units | Pull request | Push |
| --- | --- | --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `ebeb022df573cb907ff1659c29844722a9f569f5` | `harness`, `rates` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618475), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618481), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618430) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589716), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589723), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589731) |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `8b55a765dfdd8d3ff8b4ed0f82df69e1ba36e6bb` | `harness`, `labels` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896292), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896319), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896450) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854511), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854173), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854228) |
| [Mixed](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `5137df028de4b1bf21d21825fb67483fcd7b78cf` | `harness`, `labels`, `normalize`, `rates` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900081), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899959), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900164), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900085), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899957) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853951), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853836), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853948), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853834), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853905) |

The final mixed `rates` link above is the push run at
`36624853905`.

## Command identity

The unit logs show `chrono-ci prepare` selecting the registered unit. The
prepared workflow then records and invokes the same canonical check used by a
local run:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --unit UNIT
```

Each collection workflow gathers the pinned unit attempts and passes the same
entry point with the explicit collection selector:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --collect .chrono-harness/state/collection/manifest.json
```

The run IDs and candidate hashes are retained so later updates can replace
this evidence with newer observations without treating an old successful run
as a current workflow guarantee.
