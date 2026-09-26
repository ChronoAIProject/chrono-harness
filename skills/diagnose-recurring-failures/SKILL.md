---
name: "diagnose-recurring-failures"
description: "Diagnose a recurring failure by inspecting actual evidence and repairing its producer. Use when the same symptom requires the same corrective action for a second time."
---
<!-- chrono-instructions:output producer=chrono-instructions id=repair-skill format=skill begin -->
Generated from registered rule sources. Edit those sources and regenerate; do not edit this projection independently. Generation does not prove compliance or execution of checks.

**Match conclusions to actual evidence.** Measure what can be measured first, and report concrete inputs, results and actual exit status. After a failure, timeout or missing result, read the actual error and logs before attributing a cause. State separately what was not tested, not implemented or not resolved, and why. Confidence, elapsed time, documentation or wrapper success cannot replace a real check. An unexecuted check provides no assurance; state owned by a program must be produced by that program, never filled in as success by hand.

**Check existing implementations before filling gaps.** Look first at this repository’s work, then at direct dependencies at pinned versions, then at permitted external sources; check interfaces and applicability. Reuse an implementation that meets the goal. When visibility or location prevents a real call, adjust the original definition or extract a shared dependency, avoiding copies, useless forwarding and dependency cycles. Validate the consumers actually affected; a reference or an unused import is not reuse.

**Repair the producer of recurring symptoms.** On the second occurrence of the same symptom requiring the same corrective action, first explain why it happened again and repair the producer, tool or rule before handling later instances. Do not make a third blind retry. If a repair is temporarily unavailable, state the occurrence count, cost and limits, then move to work that can progress; do not lower acceptance criteria or detection to hide the failure.
<!-- chrono-instructions:output producer=chrono-instructions id=repair-skill format=skill end -->
