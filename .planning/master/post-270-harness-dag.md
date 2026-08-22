# Post-270 Harness Dependency DAG — PR-271..PR-345

**Status:** planned extension; it does not modify `.planning/master/dependency-dag.md`, which remains the normalized historical DAG for PR-001..PR-270.

## Global entry edge

```text
PR-270 (formal merge + exact-SHA baseline PASS)
  → PR-271 Capability baseline
  → PR-272..275 Harness contracts/ADRs
```

## V1 critical path

```text
271 → 272/273/274 → 275 →
276 → 277 → 278 → 279 → 280 → 281 → 282 → 283 → 284 →
285 → 286 → 287/288/289 → 290 → 291 → 292 →
293 → 294/295 → 296/297 → 298 → 299 → 300 → 301 → 302 →
303 → 304 → 305 → 306 → 307 → 308 → 309 → 310 → 311 → 312 →
313 → 314 → 315 → 316 → 317 → 318 → 319 → 320 → 321 → 322 →
323 → 324/325 → 326 → 327 → 328 → 329
```

`PR-329` is the **Harness V1 gate**. V2 cards are not eligible unless it has exact-SHA full-gate evidence.

## V2 critical path

```text
329 → 330 → 331 →
332 → 333 →
334 → 335 → 336 →
337 → 338 → 339 → 340 → 341 →
342 → 343 → 344 → 345
```

## Parallelism policy

- Cards sharing a persistent aggregate/schema, authority policy, common envelope, or evidence identity must remain sequential.
- Potentially parallel lanes begin only after predecessor evidence: decision/failure memory (`288/289`), specialized tool descriptor/normalizer (`294/295`), evaluation fixtures (`324/325`), and V2 shadow/experiment planning (`330/332`).
- Parallel execution still requires separate worktrees, exact scope manifests, independent review, and revalidation after any shared contract merge.

## Cycle prevention rules

1. Context consumes memory candidates through ports; memory never imports context compiler internals.
2. Evidence resolves claims from tools/external sources; it does not depend on model/router decisions.
3. Checkpoint references evidence/context/memory by immutable IDs; none may require checkpoint creation to validate themselves.
4. Router selects a model attempt; provider adapters never select or persist Harness Run state.
5. Learning/experiments consume evaluation records and can propose changes only; they never write back to active policy without PR/evaluation/approval.
6. Blackboard/leases/handoff depend on V1 state/evidence contracts, not conversely.

## Execution condition

The validator in `.planning/scripts/validate_post270_harness_queue.py` is the extension planning gate. It requires exact sequential IDs PR-271..PR-345, all canonical fields, dependencies only on PR-270 or extension cards, and a cycle-free graph. It is intentionally separate from the historical 270-card validator to preserve prior artifacts unchanged.
