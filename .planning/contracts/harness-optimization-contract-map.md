# Harness Optimization Contract Map — Complement PR-377..PR-414

This contract map extends, but does not replace, the planned `SkillRouteDecision`, `VerifierResult`, `EvaluationCase`, `BaselineReport`, `ShadowRun` and `ImprovementCandidate` contracts in PR-271..PR-345.

| Contract | Owner | Core fields | Extension card | Negative conditions |
|---|---|---|---:|---|
| `FirstPartySkillContract` | core/application | ID/version/digest/triggers/schemas/tool allowlist/risk/budgets/rounds/policy/tests/evals/lifecycle/rollback | PR-379 | unknown field, tool expansion, missing policy/eval/rollback |
| `SkillRegistrySnapshot` | runtime/store | registry revision, immutable manifests, active pointers, scope, digest | PR-380 | duplicate ID/version, cross-project lookup, mutable active manifest |
| `SkillActivation` | application | baseline/candidate/holdout/approval/atomic pointer/rollback | PR-381/409 | self-approval, stale candidate, partial activation |
| `FirstPartySkillProfile` | core | named official profile, trigger class, declared requirements, eval refs | PR-382/383 | prompt-only profile, missing schema, undeclared tool |
| `SkillRouterEvent` | protocol/core | run/project/trace/event/state digest/policy/budget | PR-384 | missing run, forged event, ambiguous state |
| `SkillRouteDecision` extension | core/runtime | candidate set, selected pin, exclusions, reason codes, reservation, terminal | PR-385 | unpinned/stale version, unauthorized skill, fallback prompt |
| `PlanRequest` / `PlannerDraft` | core | goal/scope/constraints/evidence refs/budget/deadline/plan digest | PR-387 | raw secret/prompt, missing identity, planner policy override |
| `ReviewerFinding` | core | reviewer version, severity, category, evidence refs, canonical key, disposition | PR-388 | fabricated evidence, self-review, oversized finding |
| `PlanningReconciliation` | application | findings, disagreement state, decisions, `HUMAN_REQUIRED`, round count | PR-390/391 | loop overflow, critical finding erased, self-approval |
| `EvaluationCase` extension | test-support | scenario, fixture, authority, scorer, metric schema, holdout flag | PR-393/394 | nondeterministic fixture, unsafe side effect, missing expected terminal |
| `BenchmarkComparison` | eval runtime | baseline/candidate digests, same environment, deltas, thresholds, decision | PR-397 | incomparable run, missing holdout, benchmark self-selection |
| `ExternalEvaluationReport` | adapter | evaluator/version/config/run/SHA/tree/report digest/status | PR-398 | foreign SHA, unsupported schema, evaluator unavailable |
| `ImportedFinding` | application | external source, normalized evidence, trust state, mapped internal claim | PR-399 | direct promotion to PASS, raw report authority |
| `ImprovementFrictionSignal` | projection | friction type, frequency window, evidence refs, redacted context | PR-401/402 | raw prompt, fabricated frequency, cross-project aggregation |
| `ShadowExecutionSpec` | core/runtime | primary run ref, candidate config, no-effect authority, sandbox/budget | PR-404 | inherited write/credential approval, live tool fallback |
| `ShadowComparison` | eval runtime | primary/shadow metrics and divergence | PR-405/406 | side effect, incomparable fixture, hidden model change |
| `PromotionDecision` | application/judge | candidate/baseline/holdout/approval/active pointer/rollback | PR-407..409 | candidate self-approval, non-atomic pointer, stale evidence |
| `MetaExperimentSpec` | runtime/application | baseline/candidate/training/holdout/isolated environment/PR target | PR-410..413 | direct production mutation, holdout leakage, missing rollback |
| `HarnessOptimizationProjection` | application API | aggregate metrics, version pins, findings/candidates/rollbacks, digests | PR-414 | raw CoT, secrets, direct durable-store UI access |

## Invariants

1. Policy authorizes; skill does not. Skill guides; tool performs effects.
2. Every decision is scoped to project/run/trace and a policy/schema revision.
3. A skill/eval/candidate version is immutable after benchmarking; active routing is an atomic pointer only.
4. Primary, reviewer, benchmark, external evaluator and promotion judge are separated identities.
5. External reports and model text are untrusted data until resolver-bound evidence maps them to an internal state.
6. Shadow, replay and meta-experiment authority is no-write by default and cannot inherit credentials or approvals.
7. Evaluation training/holdout sets are versioned, disjoint and recorded in the comparison artifact.
