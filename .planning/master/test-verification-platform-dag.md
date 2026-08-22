# Test & Verification Platform DAG — PR-346..PR-376

**Entry:** PR-345 exact-SHA Harness V2 gate PASS and PR-346 actual coverage baseline. The queue is a separate extension and does not alter PR-001..PR-345.

```text
345 → 346 → 347 → 348 → 349 → 350 →
351 → 352 → 353 → 354/355 →
356 → 357 → 358 → 359 → 360 →
361 → 362 → 363 → 364 → 365 → 366 →
367 → 368/369 → 370 → 371 → 372 →
373 → 374 → 375 → 376
```

## Parallel lanes after explicit gates

- PR-354 property pilot and PR-355 fuzz pilot may proceed after the unified negative corpus, subject to the risk/benefit decision in PR-355.
- PR-368 protected adapter tests and PR-369 provider smoke/evals may proceed after PR-367 policy, but neither can block deterministic core PR gates when credentials are unavailable; they report `NO_PROOF`.
- Cross-platform native desktop work remains sequential after Linux driver proof: PR-358 → PR-359 → PR-360.

## Cycle-prevention rules

1. Test catalog/fixtures/clock/MockProvider are foundation dependencies; application code does not depend on test-support in production.
2. Harness PRs consume test platform tools only through dev/test dependencies; Test Platform PRs must not require unfinished Harness runtime behavior before PR-345.
3. Required check registry observes actual workflow contexts; it does not mutate branch rulesets.
4. Change-aware selection is additive and cannot replace full release/recovery/security matrix.
5. External test credentials and real-provider smoke never determine a deterministic PASS; missing protected environment is `NO_PROOF`.

## Execution condition

`python3 .planning/scripts/validate_test_verification_platform_queue.py` verifies frozen queue integrity PR-001..PR-345, card fields PR-346..PR-376, dependency identity and acyclicity. Implementation remains blocked until PR-345 evidence exists.
