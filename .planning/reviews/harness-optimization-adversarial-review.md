# Harness Optimization — Adversarial Planning Review

**Scope:** review of the complementary PR-377..PR-414 planning extension against existing post-270 Harness/Test Platform contracts.  
**Evidence boundary:** planning artifacts and current source seeds only. This review does not prove runtime behavior, external evaluator availability, PR-270/345/376 readiness, or production safety.

## Independent adversarial evidence pass

Cinco revisões independentes foram executadas sobre os artefatos existentes e o pedido, sem autoridade de escrita:

- **Simplicidade:** `SkillManifest` já contém seed de versão/escopo/digest e PR-315..317 já planejam router/pin/E2E. PR-377 deve classificar cada novo campo/card como extensão necessária, reutilização ou deferimento; a extensão não autoriza um segundo registry/router.
- **Integração:** PR-270 → PR-345 → PR-376 continua gate obrigatório; validadores de queue existentes passam apenas para planejamento, não provam merges/artefatos de entrada. E2Es novos dependem da fundação determinística PR-346..376.
- **Evidência:** `SkillRuntime` permanece stub; ADRs e mapas são planejamento. Há inconsistência potencial entre numeração/títulos de ADR-HAR no master e os ADRs propostos presentes; PR-377 deve produzir mapa canônico ADR→card→contract→gate antes de implementar qualquer adapter.
- **Arquitetura:** isolamento deve ser aplicado por capability/policy/infrastructure, não convenção textual. Policy, Skill e Tool não compartilham autoridade; planner/reviewer/reconciler/evidence runner/promotion judge são papéis distintos.
- **Ameaças/alternativas:** baseline/candidate/approver devem ser separados; training/holdout é disjunto; Better Harness/HyperPlan são opcionais; shadow deve ser zero-write por capability; meta-experimentos só criam propostas normais de Git/PR.

Essas disposições foram incorporadas como: PR-377 baseline de compatibilidade, PR-378 boundary, PR-388 rounds/dedupe, PR-391 Evidence Engine, PR-397/412 benchmark/holdout, PR-398/399 adapter externo opcional, PR-404..406 shadow e PR-407..413 promotion/meta-harness controlados.

## Findings and reconciliations

| Attack | Finding | Resolution in plan | Residual gate |
|---|---|---|---|
| Skill is a prompt in disguise | A Markdown instruction alone could silently add authority, tools or effects. | PR-379 requires immutable structured contract, schemas, allowed tools, budgets, policy/eval refs and rollback; text is only referenced data. | PR-386 malicious-profile E2E. |
| Policy/Skill/Tool duplication | A skill capability list could become a second permission engine. | PR-378 fixes the split: Policy authorizes, Skill chooses procedure, Tool produces an effect. Skill allowlist is a subset of policy authority. | PR-385/386 evaluator and dispatch negatives. |
| Self approval | Candidate, benchmark runner, shadow or reviewer could promote itself. | PR-381, PR-397, PR-407 and PR-408 require independent approval/judge, baseline+holdout and exact digests. | PR-408 self-approval negative; PR-409 atomic activation test. |
| Overengineering | A marketplace, dozens of profiles and arbitrary reviewer swarm would exceed V1 value. | V1 has ten named official skills, no marketplace, no skill code loading and at most five reviewer roles. | PR-377 baseline may defer profiles not justified by evidence. |
| Excess reviewers and loops | Planner/reviewer conversations can grow without bound. | PR-388/389 cap five roles, two rounds, per-role/global budget, deadline and cancellation; required omissions yield HUMAN_REQUIRED. | PR-392 loop/budget/cancel E2E. |
| Non-deterministic eval | Model-varying fixtures could fabricate improvement. | PR-393..396 require deterministic fixtures, virtual effects, exact environment identity and declared terminal classes. | PR-396 rejects incomparable/nondeterministic runs. |
| Candidate-favoring benchmark | A candidate can choose its own cases/model/tools. | PR-397 freezes same fixture/policy/model/tool authority and requires independent comparison. | PR-397 comparator negative matrix. |
| Overfitting / no holdout | Training success can hide regressions. | PR-397 and PR-412 require versioned disjoint training/holdout sets and regression thresholds. | PR-412 leakage and reused-fixture negatives. |
| External dependency lock-in | Better Harness/HyperPlan could become a required runtime/toolchain dependency. | PR-398 is an optional adapter; absence returns NO_PROOF only for that external eval and core continues. | PR-398 unavailable adapter contract. |
| Imported report as authority | External text can claim PASS without current SHA evidence. | PR-399 imports as UNTRUSTED_EXTERNAL and maps only through Claim/Evidence resolution. | PR-399 foreign/stale/fabricated PASS negatives. |
| Shadow with authority | A shadow could write project state, send messages, create PRs or resolve secrets. | PR-404..406 declare zero-effect authority, synthetic-only artifacts and termination on effect attempt. | PR-406 same-task/no-side-effect E2E. |
| Meta-Harness modifies production | An experiment could mutate the running runtime directly. | PR-410..413 require isolated worktree/sandbox, training+holdout comparison and normal Git PR handoff only after independent approval. | PR-413 direct mutation/auto-merge negatives. |
| Missing rollback | Version promotion without a last-known-good pointer can strand runs. | PR-381/407/409 define version-pinned runs, atomic activation pointer, retained evidence and rollback pointer. | PR-409 crash/partial activation rollback test. |
| Observability leakage | Metrics could store chain-of-thought, prompts, secrets or raw evaluator reports. | Master plan limits projections to metadata, classifications and digests; raw private reasoning is prohibited. | PR-393/402/414 redaction and schema negatives. |

## Simplicity decisions retained

1. Reuse planned PR-315, PR-318/320, PR-323/326, PR-330..334 and PR-344 contracts rather than create parallel router/eval/shadow/candidate systems.
2. Keep external evaluation optional and adapter-scoped.
3. Do not add an official skill marketplace, dynamic code loader, autonomous promotion or general-purpose multi-agent chat bus in V1.
4. Defer Meta-Harness until native eval, benchmark, shadow, promotion and rollback have exact-SHA evidence.

## Open implementation conditions

- PR-270, PR-345 and PR-376 exact-SHA gates do not currently provide an implementation entry point; PR-377 remains blocked until they do.
- External evaluator adapter semantics beyond the defined import boundary remain intentionally `NO_PROOF` until a protected optional adapter run exists.
- Human approval identity/authorization implementation remains a downstream contract; this plan only reserves `HUMAN_REQUIRED` terminal behavior.

## Review conclusion

The extension safely captures the requested destination and is structurally validated as a **reserved conditional queue**. It is not formally ready for implementation or approval because the existing post-270 ADR/card/contract authority has documented ID/title/milestone conflicts. PR-377 is the required authority-reconciliation audit; PR-378..PR-414 must remain deferred until its exact-SHA disposition map is reviewed.
