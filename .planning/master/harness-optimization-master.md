# Harness Optimization Extension — Skills, Planning and Evidence

**Status:** `PLANNED / FORMAL AUTHORITY RECONCILIATION BLOCKED / IMPLEMENTATION BLOCKED UNTIL PR-376 PASS+MERGED`  
**Planning status:** `BLOCKED` until PR-377 resolves the canonical ADR→card→contract→gate map against exact predecessor evidence.  
**Implementation status:** `NOT READY`.  
**Authority order:** approved SDD → PR-001..PR-270 immutable queue → existing Harness PR-271..PR-345 → Test Platform PR-346..PR-376 → architecture/ADRs → exact-SHA evidence → this complementary extension.

## Scope and non-interference

This is a complementary queue beginning at **PR-377**. It does not edit, reorder, reinterpret or invalidate PR-001..PR-270, the existing Harness queue PR-271..PR-345, or the Test & Verification queue PR-346..PR-376. Existing planned contracts are extended only through explicit adapters and compatible versions.

Better Harness, HyperPlan and similar tools are conceptual inputs and optional external evaluators. They are never required runtime dependencies, policy authorities, activation authorities, secret stores or sources of PASS.

## Authority-reconciliation gate

Before any PR-378+ implementation, PR-377 must publish a single exact-SHA map:

```text
existing ADR ID/title/status
→ existing queue card/milestone/dependency
→ existing contract owner
→ existing test/evidence gate
→ extension disposition: REUSE | COMPATIBLE_EXTEND | DEFER | HUMAN_REQUIRED
```

The current master plan, contract map and proposed ADR files have divergent ADR IDs/titles and overlapping card ranges for evidence, shadow, experiments, improvement candidates, leases and projections. Title similarity is not authority. Until this map is reviewed, PR-378..PR-414 are reserved conditional cards, not approved new runtime contracts.

## Entry gate

No PR-377+ implementation may start until all are true:

1. PR-270 is merged with exact-SHA release-baseline evidence;
2. PR-345 Harness V2 integration gate is merged with required exact-SHA evidence;
3. PR-376 Test & Verification platform gate is merged with required exact-SHA evidence;
4. PR-377 re-audits actual implementations at that merge SHA and classifies every reused contract `SUFFICIENT`, `PARTIAL`, `ABSENT`, `STALE` or `NO_PROOF`;
5. the complementary queue/DAG integrity check proves PR-001..PR-376 hashes unchanged, PR-377..PR-414 sequential and acyclic.

Missing, pending, stale, skipped, conflicting or unverifiable evidence is `BLOCKED` or `NO_PROOF`, never PASS.

## Gap analysis against the current Harness plan

| Area | Already planned / seeded | Assessment | Complementary extension |
|---|---|---|---|
| Skill model | `agent-core::SkillManifest` has ID/version/files/capabilities/tests/digest; `SkillRuntime` is a stub. Existing PR-315 plans `SkillRouteDecision`. | Partial seed; manifest lacks bounded execution/policy/eval contract. | Versioned First-Party Skill Contract, official registry, lifecycle/pin/activation evidence and router event mapping. |
| Policy and tools | Existing tool registry/evaluator and autonomy policies are separate components. | Strong basis. | Make the separation normative: **Policy decides what may happen; Skill decides how to solve; Tool performs an effect**. No skill grants capability. |
| Directed verification | Existing PR-318/320 plan verifier result, uncertainty, max rounds and independent review. | Partial planned overlap. | Generalize to adversarial planning findings/reconciliation; do not create a second reviewer authority. |
| Evaluation/replay | Existing PR-323..330 plan deterministic evals, baseline, recording and safe replay. | Partial planned overlap. | Native Harness Evaluation Suite, skill benchmark comparison, holdout protocol and external evaluator import boundary. |
| Shadow/experiments | ADR-HAR-004 and PR-330..333 plan isolated/no-write shadow and experiments. | Planned overlap. | Define shadow task equivalence, comparison metrics and explicit zero-side-effect enforcement. |
| Improvement candidates | Existing contract map assigns `ImprovementCandidate` to PR-334. | Planned overlap. | Add friction evidence/frequency model and required baseline/candidate/holdout evaluation, never direct activation. |
| Promotion/rollback | ADR-HAR-005 requires baseline comparison, approval, versioned activation and rollback. | Planned overlap. | Formalize skill lifecycle and atomic active pointer; pin every run to its starting version. |
| Observability | Existing plans retain digests/metadata and forbid chain-of-thought. | Partial. | Add skill selection, plan findings, eval/benchmark, shadow and intervention projections. |
| External harnesses | Better Harness review is documentary; HyperPlan review is historical planning input. | Optional/manual only. | Adapter boundary with imported structured findings bound to tool/version/run/SHA; unavailable external tool means `NO_PROOF` for that external eval only. |

## Architecture extension

```text
Policy Engine ───── decides authority/capabilities/budgets/approval
     │
Skill Registry ───── immutable structured skill versions + lifecycle pointer
     │                         │
Skill Router ─────── event/run state → pinned SkillRouteDecision
     │                         │
Plan Orchestrator ── bounded Planner → reviewers → reconciler → FinalPlan
     │                         │
Tool Runtime ─────── executes only evaluator-authorized typed effects
     │
Run/Evidence/Checkpoint lineage
     ├─ Native Eval Runner → baseline/candidate/holdout reports
     ├─ External Eval Adapter → imported untrusted findings
     ├─ Shadow Runner → no-authority comparison report
     └─ Improvement Candidate → isolated experiment → promotion gate
```

### Boundaries

- **Core/domain:** skill contracts, lifecycle state machines, route/finding/eval/benchmark/candidate/shadow/promotion DTOs and pure evaluators. No provider SDK, filesystem, network, secret, GitHub or external-harness imports.
- **Application:** orchestration commands, policy evaluation, run pinning, reconciliation, benchmark comparison and read-model projections.
- **Infrastructure adapters:** SQLite, artifact store, test fixture executor, external evaluator CLI/API adapter, isolated worktree/sandbox and telemetry sinks.
- **Trust boundary:** prompt, model, tool, external evaluator and candidate text are data. They cannot mint a policy decision, capability, approval, active version, PASS state or rollback authority.

## Skill Contract V1

A native skill is an immutable, structured capability recipe — **not a Markdown prompt**. Human-readable instructions may be an artifact reference, but executable authority is the parsed contract and its versioned digest.

Required fields:

```text
skill_id, version, description, triggers[], required_capabilities[],
input_schema_ref, output_schema_ref, allowed_tool_ids[], risk_level,
token_budget, cost_budget, max_rounds, policy_requirements[],
test_catalog_refs[], eval_catalog_refs[], lifecycle_state,
rollback_ref, schema_version, digest
```

Invariants:

1. `allowed_tool_ids` is an allowlist subset of Policy-granted tools; a skill cannot broaden it.
2. `required_capabilities` is a request, not an authorization grant.
3. input/output schemas are bounded, versioned and fail closed on unknown fields.
4. max rounds, token/cost budgets, deadline, cancellation and trace/run/project identity are mandatory for execution.
5. a skill version is immutable after `TESTING`; changes create a new version.
6. runs pin skill ID/version/digest at start; later promotion or rollback never rewrites historical runs.
7. first-party V1 catalog is intentionally limited to: `systematic-debugging`, `verification-before-completion`, `test-driven-development`, `code-review`, `architecture-review`, `security-review`, `dependency-upgrade`, `blocker-resolution`, `release-readiness`, and `incident-analysis`.

## Skill lifecycle and promotion

```text
DRAFT → TESTING → BENCHMARKED → APPROVED → ACTIVE → DEPRECATED
                          ↘                     ↘
                           ROLLED_BACK ←─────────┘
```

- `DRAFT`: syntactically valid only; never routed.
- `TESTING`: unit/contract/integration/negative tests pass under isolated fixtures.
- `BENCHMARKED`: baseline and candidate reports exist for declared evals; regressions remain explicit.
- `APPROVED`: independent approval references exact candidate digest, benchmark digest and policy revision.
- `ACTIVE`: one atomic activation pointer per skill ID/scope; new runs may resolve it.
- `DEPRECATED`: selectable only by an explicit pinned compatibility request.
- `ROLLED_BACK`: pointer atomically returns to an approved prior version; evidence and affected runs are retained.

No candidate, benchmark runner, reviewer, shadow run, external evaluator or executor can self-approve or change the active pointer.

## Skill Router contract

`SkillRouteDecision` extends the existing planned route decision; it must include:

```text
run_id, project_id, trace_id, schema_version, router_policy_revision,
event_class, state_snapshot_digest, candidate_skills[], selected_skill_id,
selected_version, selected_digest, exclusions[], reason_codes[],
budget_reservation, terminal_state
```

Canonical V1 event mapping:

| Event / state | Skill |
|---|---|
| `TEST_FAILURE` | `systematic-debugging` |
| `IMPLEMENTATION` | `test-driven-development` |
| `PR_READY` | `code-review` |
| `ARCHITECTURE_CHANGE` | `architecture-review` |
| `SECURITY_FINDING` | `security-review` |
| `DEPENDENCY_FAILURE` | `dependency-upgrade` |
| `BLOCKED` | `blocker-resolution` |
| `RELEASE_CANDIDATE` | `release-readiness` |
| `INCIDENT` | `incident-analysis` |
| `VERIFY_REQUIRED` | `verification-before-completion` |

Selection is deterministic over a version-pinned registry snapshot. Ambiguous, stale, policy-ineligible or over-budget selections return `NO_SKILL`/`HUMAN_REQUIRED`; they never guess or fall back to a free-form prompt.

## Adversarial Planning V1

The planner pipeline is bounded and produces structured artifacts, not a chat swarm:

```text
PlanRequest
  → PlannerDraft
  → Architecture / Security / Test / Simplicity / Failure reviewers
  → Finding normalizer + duplicate elimination
  → Reconciliation Judge
  → FinalPlan | HUMAN_REQUIRED | REJECTED
```

- Max reviewer roles: five named reviewer classes; each executes at most once per planning round.
- Max rounds: two review/reconciliation rounds; a third is prohibited unless an independently approved policy explicitly restarts from a new PlanRequest fingerprint.
- Budgets: per-reviewer token/cost/deadline plus global planning reservation; optional reviewers stop first under budget pressure, but Security and Test review omission yields `HUMAN_REQUIRED` rather than PASS.
- Finding: `finding_id`, reviewer kind/version, severity (`INFO|LOW|MEDIUM|HIGH|CRITICAL`), category, affected contract, evidence refs, claim digest, suggested mitigation, confidence and disposition.
- Duplicate elimination: canonical key over affected contract + normalized consequence + evidence digest; preserve distinct consequences and reviewer provenance.
- Disagreement: reconciler may accept, reject, split, defer, or `HUMAN_REQUIRED`; it cannot erase a high/critical finding without evidence-based rationale.
- Reviewer output is advisory; only the policy/judge controls plan state. No reviewer may approve its own planner output.

## Evaluation architecture

### Native Harness Evaluation Suite

Each `EvaluationCase` has fixture digest, task contract, allowed effects, policy/budget, deterministic scorer, expected terminal classes, holdout flag and artifact requirements. V1 benchmark corpus covers:

1. fix a Rust bug;
2. investigate broken CI;
3. detect architecture violation;
4. update a vulnerable dependency;
5. reject unsafe operation;
6. recover interrupted task;
7. retrieve relevant Failure Memory;
8. choose correct skill;
9. detect fabricated evidence;
10. conduct multi-agent delegation;
11. respect budget;
12. avoid tool misuse.

Required metrics:

```text
success, terminal_state, tests_passing, tool_calls, failed_tool_calls,
retries, tokens, cost, latency_ms, human_intervention, evidence_quality,
policy_violations, context_misses, memory_hits, evidence_conflicts,
skill_selection, external_side_effect_attempts
```

Metrics retain IDs/digests/categories only; no private chain-of-thought, raw prompt, secret, raw provider payload or raw evaluator transcript is a required telemetry field.

### Skill benchmarking

A benchmark compares exactly one `baseline` skill digest to one `candidate` digest on the same versioned evaluation suite and policy class. It requires:

- disjoint training and holdout cases;
- deterministic fixture seeds where applicable;
- identical model class, tool authority, budgets and timeouts, or explicit incomparable classification;
- regression thresholds for success, policy violations, evidence quality, cost, latency and tool failures;
- independent review of the comparison artifact.

A candidate cannot replace an active skill because it has a higher aggregate score. Promotion requires baseline + candidate + holdout + approval + rollback reference.

## External evaluator boundary

External evaluators, including Better Harness-compatible tools, are invoked only through an optional adapter contract:

```text
ExternalEvaluationRequest → ExternalEvaluationReport → ImportedFinding[]
```

Import binds evaluator name/version/digest, command configuration digest, environment class, target repository, base/head SHA, tree, run ID, policy revision, report digest and importer result. Imported findings are `UNTRUSTED_EXTERNAL` until mapped to internal evidence. Unavailability, timeout, unsupported format or missing binary yields `NO_PROOF` for that adapter without impairing core Harness execution.

## Harness Improvement Candidate

`ImprovementCandidate` extends the existing planned aggregate with:

```text
candidate_id, problem, evidence_refs[], frequency_window, frequency,
proposed_change_ref, expected_impact, risk_level, affected_contracts[],
baseline_ref, required_evals[], state, owner, policy_revision, rollback_ref
```

Only telemetry/evidence detectors can propose a candidate. Examples: repeated file search, tool failure clusters, context misses, repeated error signatures and excessive skill retries. A candidate is not a patch and cannot mutate runtime/skill/router/model/tool/memory configuration.

## Shadow execution model

A shadow run receives a normalized copy of task identity, allowed fixture/input references and pinned candidate configuration. It may read explicitly shared, redacted artifacts but has zero write authority.

Shadow prohibition matrix:

- no repository/project write;
- no destructive tool;
- no message/PR/external publication;
- no credential/secret resolution unless an explicitly synthetic fixture grants it;
- no active pointer mutation;
- no hidden fallback to live production tools.

Comparison output records decision/result/cost/latency/tool calls/evidence quality against the primary run. It never replaces the primary decision in real time.

## Meta-Harness V2+ guardrail

Meta-Harness remains a later phase after reliable native evaluation and shadow gates. The only allowed path is:

```text
current baseline → candidate change → isolated environment → training evals
→ holdout evals → comparison → accept/reject → approved Git branch/PR
```

Potential candidate dimensions: prompts, context strategy, Skill Router, model routing, tool descriptions, memory retrieval and skill versions. Production Rust/runtime changes remain Git branch + tests + PR + independent review; no experiment directly modifies production.

## Threat model and fail-closed matrix

| Threat | Mandatory terminal behavior |
|---|---|
| Skill calls unauthorized tool / escalates capability | `DENY`, no tool dispatch, evidence record |
| Malicious skill or prompt injection changes policy | `DENY`, policy digest unchanged |
| Candidate self-approves | `DENY`, activation pointer unchanged |
| Benchmark lacks baseline, holdout or required eval | `BLOCKED` |
| Regression exceeds threshold | `REJECTED` / `ROLLED_BACK`, never auto-promote |
| Eval missing/stale/fabricated evidence | `NO_PROOF` / `CONFLICTING` |
| Shadow attempts write or external send | `DENY`, terminate shadow |
| Skill version stale/unpinned | `BLOCKED` |
| Rollback activation fails | retain prior active pointer, `HUMAN_REQUIRED` |
| External evaluator missing | core continues; external result `NO_PROOF` |

## Observability model

Project/run-scoped metadata projections answer “did this change make the agent better?” using rate/trend comparisons:

- success rate and terminal distribution;
- retries, tool calls/failures and side-effect denials;
- context misses and memory hit/omission classes;
- evidence conflicts and fabrication detections;
- skill selection, exclusions and version pins;
- token/cost/latency/budget exhaustion;
- reviewer findings/severity/disagreement/reconciliation state;
- benchmark baseline/candidate/holdout deltas;
- shadow divergence and prohibited-effect attempts;
- human intervention and promotion/rollback transitions.

## Global Definition of Done

Every extension card requires exact-SHA evidence for named unit, contract, integration, negative, security and documentation checks; named E2E/performance checks; independent review; bounded rollback; project/run/trace/policy/schema/digest identity; no required `FAIL`, `BLOCKED`, `STALE`, `CONFLICTING` or `NO_PROOF`; and no chain-of-thought/private reasoning persistence.
