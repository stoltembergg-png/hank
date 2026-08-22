# Post-270 Harness Queue Index

## Scope

- **Historical queue:** PR-001..PR-270 remains authoritative and untouched.
- **Extension queue:** [`queue-271-345.md`](../queue/queue-271-345.md), 75 sequential cards.
- **Execution status:** `PLANNED / BLOCKED` until formal PR-270 merge and baseline evidence.
- **Validator:** `python3 .planning/scripts/validate_post270_harness_queue.py` against `.planning/contracts/post-270-queue-extension-contract.json`; it proves the three legacy queue files still match their frozen SHA-256 digests.

## Milestone map

| Range | Milestone | Focus |
|---:|---|---|
| 271–275 | M17-A | capability baseline, ADRs and common envelopes |
| 276–284 | M17-B | Context Compiler and ContextBudget |
| 285–292 | M17-C | typed memory, failure/decision memory and E2Es |
| 293–302 | M17-D | structured tool contracts, normalization and Evidence Engine |
| 303–312 | M17-E | Harness state, checkpoints/recovery and model routing/hot swap |
| 313–322 | M17-F | adaptive prompting, Skill Router, directed verification and independent review |
| 323–329 | M17-G | evaluation suite, baseline comparison, run recording and safe replay; V1 gate |
| 330–333 | M18-A | no-authority shadow and isolated experiments |
| 334–336 | M18-B | controlled learning and improvement candidates |
| 337–341 | M18-C | blackboard, concurrency, leases and HandoffPacket |
| 342–344 | M18-D | cost-aware control, adaptive-context candidates and observability |
| 345 | M18-E | full V2 E2E/release gate |

## Baseline reuse

The extension reuses rather than replaces existing provider-neutral capabilities, context selection, budget identities, tool permission/schema contracts, execution snapshots, migrations, trace IDs and project isolation. PR-271 is the formal conformance audit that turns this into an exact-SHA decision.

## Execution policy

PR-271 is the first possible post-270 implementation card. It starts only after the entry gate. Every successor requires exact-SHA evidence for normalized predecessors and applicable architecture/security/observability/rollback checks. V2 has an additional hard gate at PR-329.
