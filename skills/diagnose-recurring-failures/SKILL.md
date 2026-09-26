---
name: "diagnose-recurring-failures"
description: "Diagnose a recurring failure by inspecting actual evidence and repairing its producer. Use when the same symptom requires the same corrective action for a second time."
---
<!-- chrono-instructions:output producer=chrono-instructions id=repair-skill format=skill begin -->
Generated from registered rules. Edit sources and regenerate, not this file alone. Generation proves neither compliance nor executed checks.

<!-- Instructions are modified, generalized and translated from Copyright 2026 The Omega Institute, Apache-2.0. Accompanying license/provenance are optional legal reading, not policy. -->

Measure first. Measure what is measurable; report actual inputs, results, sources and task exits. Specify what is untested, unimplemented or unresolved, and why. Confidence, guesses, time, docs or wrapper success replace no checks.

Inspect failures first. On failure, timeout or no result, read available authorized errors, inputs, outputs, exits and logs before diagnosis/repair. Verify input matches dispatch intent. Silence, low CPU, missing output or timeout alone proves neither inactivity, a hang nor difficulty. Without evidence, leave causes unverified.

Programs produce their state. Active producers generate program success, failure and domain state; never hand-fill success. Unrun checks give no assurance. Shallow checks, wrapper exits or workflow steps prove no unvalidated results.

Search existing work first. Search repo, pinned direct dependencies, then allowed external sources, including private definitions and valid failure findings. Verify versions, interfaces, assumptions, scope and evidence status; reuse suitable work. Invisible does not mean absent.

Reuse at the source. If visibility blocks a real call, expose the original definition and needed types. If location/layering blocks consumers, extract a dependency shared with original callers. Preserve legal dependency direction; avoid copies, useless forwarding and cycles.

Verify real reuse. Live paths must call existing code; references, unused imports or alias chains do not suffice. Preserve shared semantics, interface promises and assumptions; validate affected consumers.

Repair recurring causes. Recurrence means the same symptom AND remedy. Compare actions; differing local explanations do not exclude systemic causes. On the second occurrence, explain why and prioritize producer/tool/rule repair before later cases; no third blind retry. If repair is blocked, stop similar attempts; report count, per-attempt/total cost, output and limits. Continue independent work. Do not weaken goals, acceptance or detection. Check known issues and valid remedies before investing.
<!-- chrono-instructions:output producer=chrono-instructions id=repair-skill format=skill end -->
