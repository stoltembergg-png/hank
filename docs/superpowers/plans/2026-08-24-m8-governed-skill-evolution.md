# M8 Governed Skill Evolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the next three unlocked M8 increments as isolated, auditable PRs: governed skill creation, non-activating candidate evaluation, and provenance-bound candidate generation.

**Architecture:** Keep skill domain behavior in the existing Rust `agent-runtime` boundary because this repository has no separate `skill-core` crate. Expose creation through a `tool-core::Tool` adapter that receives only bounded JSON and delegates to the runtime service; the adapter never parses or executes skill artifacts itself. Evaluation and generation remain pure/data-only boundaries, and every lifecycle mutation continues through `SqliteSkillRepository` and the validation evidence gate.

**Tech Stack:** Rust 2021, Tokio, SQLx SQLite, serde/serde_json, `agent-core` skill parser/domain types, `agent-runtime` validation/testing/repository services, `tool-core` tool schema/registry, ONP SDD verification, GitHub Actions.

**Spec:** `.planning/queue/queue-096-172.md` sections PR-148, PR-149 and PR-150; the feature SDD files created with each increment under `.spec/features/skill-creation`, `.spec/features/skill-evaluation` and `.spec/features/skill-candidate`.

## Global Constraints

- Creation output is project-scoped `draft` only; it never activates, publishes globally, executes scripts, changes runtime code, or installs dependencies.
- Evaluation consumes an immutable baseline and untrusted candidate data; pass/fail/timeout/inconclusive/quarantine never activates a skill.
- Candidate generation emits a proposal only; it cannot alter system/security instruction layers, grant capabilities, persist raw prompts, or mutate an active version.
- Every request carries project/agent identity, capability, policy, budget and non-nil trace identity; missing or mismatched evidence fails closed.
- Artifacts, references, dependencies, tests, output, compute and report reasons remain bounded; reports contain hashes and metadata, never raw skill content or secrets.
- Duplicate operations are deterministic and idempotent; rollback/discard leaves immutable history and does not silently delete active state.
- Use TDD for behavior changes: each production behavior has a new test that failed before implementation.

---

### Task 1: PR-148 — Governed skill creation tool

**Files:**
- Create: `.spec/features/skill-creation/spec.md`
- Create: `.spec/features/skill-creation/tasks.md`
- Create: `crates/agent-runtime/src/skill_creation.rs`
- Create: `crates/agent-runtime/tests/skill_creation_contract.rs`
- Modify: `crates/agent-runtime/src/lib.rs`
- Modify: `crates/agent-runtime/src/skill_repo.rs` only if initial-draft discard requires a repository-level atomic operation
- Create: `crates/agent-runtime/tests/skill_creation_tool_contract.rs`
- Modify: `.github/workflows/onp-sdd-evidence.yml`
- Create: `docs/skill-creation.md`
- Modify: `docs/superpowers/plans/2026-08-24-m8-governed-skill-evolution.md` only to record verified implementation decisions

**Interfaces:**
- Consumes: `SkillParser`, `SkillValidationService`, `DeterministicSkillTestRunner`, `SqliteSkillRepository`, `ToolRequest` and `ToolSchema`.
- Produces: `SkillCreationPolicy`, `SkillCreationRequest`, `SkillCreationResult`, `SkillDiscardRequest`, `SkillCreationService`, `SkillCreateTool`, and redacted output containing project/skill/version/status/revision/content hash/validation report hash.

- [ ] **Step 1: Write the failing service and tool contract tests.** Add tests proving that a valid fixture creates a draft, a duplicate returns the same draft without a second version, a script/network/injection fixture fails before persistence, wrong project/capability/budget/trace is rejected, discard archives an initial draft without changing an active version, and the tool registry can resolve/execute only with the exact project-scoped creation context.

- [ ] **Step 2: Run the new contracts and verify RED.** Run `cargo test -p agent-runtime --test skill_creation_contract --test skill_creation_tool_contract --locked`. Expected: compile/test failure because the creation service, input adapter and module exports do not exist.

- [ ] **Step 3: Implement the minimal creation service.** Parse only the supplied document/files, require `SkillScope::Project`, derive `Skill::new(parsed.manifest.clone(), Some(project_id))`, run the deterministic fixture runner, build a project-scoped `SkillValidationRequest`, reject any non-passed validation report, then call `SqliteSkillRepository::create`. Before persistence, return the existing draft only when its immutable parsed content and manifest identity are equal; otherwise return a bounded duplicate error. Use a bounded policy validator that requires `Capability::new(Resource::Skill, Action::Create).with_scope(project_id.to_string())` and a non-nil trace. Never return parsed instructions in the result.

- [ ] **Step 4: Implement the minimal `SkillCreateTool` adapter.** Define a stable schema for `skill.create` version `1.0.0`, require `skill:create` plus an agent identity and `PolicyDecision::Allow`, validate bounded JSON through `ToolSchema::validate_input`, map wire files/dependencies into parser/runtime types, delegate to `SkillCreationService`, and return only redacted metadata. Map all service failures to bounded `ToolError`/`ToolOutcome` values. Registering the tool must not invoke it.

- [ ] **Step 5: Implement explicit draft discard.** Add a service operation that validates project/actor/trace/confirmation and either archives the initial current draft atomically or delegates to the existing draft discard path for a non-head version. The operation must never archive or mutate an active version and must be idempotent for an already archived draft.

- [ ] **Step 6: Add SDD/docs/evidence.** Record US-647 and AC-803..AC-809 for identity/policy, parser/validation, dedupe, isolation, no execution, redaction and discard. Add the ONP verify step and document the draft boundary, input limits, report hashes and rollback/discard semantics.

- [ ] **Step 7: Run PR-148 gates and commit.** Run `cargo fmt --all -- --check`, the new contracts, all affected skill contracts, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo check --workspace --locked`, `cargo build --workspace --locked`, ONP verification for `skill-creation`, and the applicable frontend/Tauri aggregate. Commit as `feat(skills): add governed skill creation`.

### Task 2: PR-149 — Non-activating learning evaluator

**Files:**
- Create: `.spec/features/skill-evaluation/spec.md`
- Create: `.spec/features/skill-evaluation/tasks.md`
- Create: `crates/agent-runtime/src/skill_evaluation.rs`
- Create: `crates/agent-runtime/tests/skill_evaluation_contract.rs`
- Modify: `crates/agent-runtime/src/lib.rs`
- Create: `docs/skill-evaluation.md`
- Modify: `.github/workflows/onp-sdd-evidence.yml`

**Interfaces:**
- Consumes: `ParsedSkill`, immutable `SkillRecord` baseline, `SkillFixture`/`SkillTestReport`, `SkillValidationReport`, `BudgetLimits`, policy/capability/trace identity.
- Produces: `SkillEvaluationRequest`, `SkillEvaluationStatus::{Passed, Failed, TimedOut, Inconclusive, Quarantined}`, `SkillEvaluationReport`, and `SkillEvaluationService::evaluate` with deterministic report hashes.

- [ ] **Step 1: Write failing evaluator tests** for safe pass, baseline immutability, regression, injection/quarantine, budget exhaustion, timeout, inconclusive/flaky evidence, rerun dedupe and tampered report rejection.
- [ ] **Step 2: Run `cargo test -p agent-runtime --test skill_evaluation_contract --locked` and observe RED.**
- [ ] **Step 3: Implement a pure bounded state machine.** Validate identity/policy/budget/trace, compare candidate against the immutable baseline, run only the deterministic fixture harness, classify every non-pass terminal state as non-active, and never call repository promotion/activation APIs.
- [ ] **Step 4: Add deterministic redacted report and idempotency.** Hash candidate/baseline/test/policy/budget inputs, bound test cases/reasons, and return the same report for the same request identity; reject raw content and activation requests.
- [ ] **Step 5: Add SDD/docs/ONP evidence and run affected plus workspace Quality Gates.** Commit as `feat(skills): add non-activating learning evaluator` and open the isolated PR only after local gates pass.

### Task 3: PR-150 — Provenance-bound skill candidate generation

**Files:**
- Create: `.spec/features/skill-candidate/spec.md`
- Create: `.spec/features/skill-candidate/tasks.md`
- Create: `crates/agent-runtime/src/skill_candidate.rs`
- Create: `crates/agent-runtime/tests/skill_candidate_contract.rs`
- Modify: `crates/agent-runtime/src/lib.rs`
- Create: `docs/skill-candidate.md`
- Modify: `.github/workflows/onp-sdd-evidence.yml`

**Interfaces:**
- Consumes: bounded observation references, exact project/agent/policy/budget/trace identity, the governed creation parser/validator and the non-activating evaluator handoff.
- Produces: `SkillCandidateSource`, `SkillCandidateRequest`, `SkillCandidate`, `SkillCandidateStatus::{Draft, Quarantined, Discarded}`, `SkillCandidateGenerationService::generate`, and a redacted evaluator handoff containing hashes only.

- [ ] **Step 1: Write failing candidate tests** for valid proposal, missing provenance/scope/budget/policy, injection/capability escalation, secret/path/script poisoning, duplicate observations, malformed output and evaluator handoff.
- [ ] **Step 2: Run `cargo test -p agent-runtime --test skill_candidate_contract --locked` and observe RED.**
- [ ] **Step 3: Implement data-only bounded generation.** Normalize and dedupe observation IDs, preserve source references without raw prompt text, require project scope and exact capability/policy/budget/trace bindings, create only draft proposal metadata, and quarantine any attempt to alter system/security layers or capabilities.
- [ ] **Step 4: Connect the handoff without activation.** Produce a deterministic candidate digest and evaluator request; do not call repository create/update/promote or runtime execution. Add discard/rollback metadata with idempotent state transitions.
- [ ] **Step 5: Add SDD/docs/ONP evidence and run all applicable Quality Gates.** Commit as `feat(skills): add provenance-bound skill candidates`, open the PR, wait for official CI, and integrate only when every required check is green.

## Verification and Integration Rules

- Each PR is reviewed and integrated before starting its dependent PR; no branch may contain later-stage behavior prematurely.
- The official GitHub checks are authoritative for merge readiness, including Windows Rust, Tauri, frontend E2E, CodeQL, quality integrity, W0 and ONP SDD evidence.
- After PR-150 is integrated, run `tools/run-all-tests.mjs` with the bundled Node runtime and verify a clean worktree and matching `origin/main` tree.
