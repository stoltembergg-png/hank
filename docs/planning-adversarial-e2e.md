# Adversarial planning E2E

`planning_adversarial_e2e.rs` is a deterministic contract-level fixture for
the planning path. It creates bounded virtual reviewer findings, sends them
through `PlanningReconciliation`, and, for the verified path, sends the same
finding through `PlanningEvidenceAdapter`.

The fixture proves that duplicate findings retain provenance and conflicting
reviewer dispositions produce `HUMAN_REQUIRED`. Hostile reviewer text remains
data; it never grants execution, approval or merge authority. Critical policy
conflicts, self-review, round overflow and the sixth virtual reviewer call fail
closed without a write effect.

The evidence scenarios require exact project/run/trace identity, resolver
status and evidence record. Missing, stale, foreign or fabricated records do
not promote a claim. Replaying the same request produces the same final-plan
fingerprint, while cancellation returns no final plan.

This is intentionally a contract fixture, not a production coordinator. It has
no model/provider/tool, persistence, UI or external side effect. The rollback
behavior is to disable the pipeline and retain only the bounded data artifact.
