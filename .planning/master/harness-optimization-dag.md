# Harness Optimization Dependency DAG — PR-377..PR-414

**Status:** complementary planning extension. It does not alter the normalized historical DAG PR-001..PR-270, the Harness DAG PR-271..PR-345, or the Test Platform DAG PR-346..PR-376.

## Entry

```text
PR-270 exact-SHA release baseline
  → PR-271 capability baseline
  → PR-329 Harness V1 evaluation gate
  → PR-345 Harness V2 integration gate
  → PR-376 Test & Verification gate
  → PR-377 Harness Optimization baseline
```

## Critical path

```text
377 → 378 → 379 → 380 → 381 → 382/383 → 384 → 385 → 386 →
387 → 388 → 389 → 390 → 391 → 392 →
393 → 394 → 395 → 396 → 397 → 398 → 399 → 400 →
401 → 402 → 403 → 404 → 405 → 406 → 407 → 408 → 409 →
410 → 411 → 412 → 413 → 414
```

## Milestones

| Range | Milestone | Outcome |
|---:|---|---|
| 377–386 | M20 — HARNESS SKILLS V1 | contract, registry, official profiles, router, basic eval wiring |
| 387–392 | M21 — HARNESS PLANNING V1 | bounded adversarial planning, reviewers and reconciliation |
| 393–400 | M22 — HARNESS EVALUATION V1 | native evals, baselines, benchmarks and optional external adapters |
| 401–409 | M23 — HARNESS OPTIMIZATION V2 | evidence-driven candidates, shadow, promotion and rollback |
| 410–414 | M24 — META-HARNESS V2+ | isolated experiments, holdouts and controlled promotion only |

## Parallelism after evidence gates

- PR-382 and PR-383 may run in parallel after PR-381 because their official skill-profile files are disjoint.
- PR-394 (core benchmark fixtures) and PR-395 (evidence-fabrication corpus) may run in parallel after PR-393.
- PR-398 (external adapter contract) may run in parallel with PR-397 after PR-396, but never blocks the native evaluator when unavailable.
- PR-401 (friction signal contract) and PR-404 (shadow authority contract) may run in parallel after PR-400.
- No shadow/promotion/meta-experiment card may start before its predecessor evaluation/holdout gate has exact-SHA evidence.

## Cycle prevention

1. Skills depend on Policy and Tool contracts; Policy and Tool contracts never depend on skill text.
2. Planner/reviewers consume registered skill pins and evidence refs; they cannot mutate registry or active pointers.
3. Evaluation consumes a frozen candidate; a candidate never decides its own score or threshold.
4. External evaluators import findings into an adapter; they never become the native judge or required core runtime dependency.
5. Shadow compares a primary run but has no effect authority; promotion consumes comparison records only.
6. Meta-Harness creates Git/PR candidates through the normal execution path only after isolated experiment acceptance; it never mutates production directly.
7. Test Platform contracts remain test/dev dependencies; production core never depends on test-support.
