# Harness Optimization Queue Index — Complement PR-377..PR-414

## Scope

- **Frozen normative queue:** PR-001..PR-270; hashes are bound by the complementary queue contract.
- **Existing Harness extension:** PR-271..PR-345; unchanged.
- **Existing Test Platform extension:** PR-346..PR-376; unchanged.
- **New complementary optimization queue:** `queue-377-414.md`, 38 cards.
- **Status:** `RESERVED / CONDITIONAL`; only PR-377 may become executable after PR-270/345/376 exact-SHA evidence and the authority-reconciliation map. PR-378..PR-414 require PR-377 disposition `COMPATIBLE_EXTEND` or explicit approval.

## Entry condition

PR-377 can begin only after PR-270, PR-345 and PR-376 have exact-SHA required-check/artifact evidence and the optimization baseline re-audits reuse boundaries.

## Milestone map

| Range | Milestone | Focus |
|---:|---|---|
| 377–386 | M20 — HARNESS SKILLS V1 | structured skills, first-party profiles, router and E2E |
| 387–392 | M21 — HARNESS PLANNING V1 | bounded adversarial planning and reconciliation |
| 393–400 | M22 — HARNESS EVALUATION V1 | native evals, benchmarks and optional external evaluator boundary |
| 401–409 | M23 — HARNESS OPTIMIZATION V2 | friction candidates, zero-authority shadow, promotion/rollback |
| 410–414 | M24 — META-HARNESS V2+ | isolated experimentation and holdout-only promotion |

## Validator contract

`harness-optimization-queue-extension.json` freezes the five existing queue files, requires exact PR-377..PR-414 sequence, all canonical card fields, only declared predecessor references and no dependency cycle. The planning validation transcript is recorded in the session; an implementation card may introduce the repository-owned validator script only after PR-377 baseline confirms the extension contract remains appropriate.

## Safety boundary

No PR in this queue permits automatic production mutation. External evaluators are optional; shadow and experiment runs are zero-authority/isolated; activation needs benchmark + holdout + independent approval + atomic rollback.
