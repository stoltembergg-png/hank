# Architecture Invariants

**Status:** contrato verificável de planejamento; os checks abaixo são obrigatórios quando o card abrir a fronteira correspondente e não foram executados por esta entrega.  
**Resultado permitido:** `PASS`, `FAIL`, `BLOCKED` ou `NO_PROOF`; apenas `PASS` com SHA/tree/policy corretos libera dependente.

| ID | Invariante normativa | Verificação/evidência mínima | Falha bloqueia |
|---|---|---|---|
| AI-001 | Tauri é shell/adaptador; não é Agent Core. | Architecture graph + forbidden-import fixture; `agent-core` compila sem Tauri. | Toda UI/runtime PR |
| AI-002 | Frontend não acessa SQLite, filesystem, providers, tools ou secrets diretamente. | Static import scan + bridge contract test + negative UI fixture. | Toda UI/API PR |
| AI-003 | Domain/Core não importa Tauri, SQLx/SQLite, Tokio, rede, Python, providers/tools concretos ou env secrets. | Cargo dependency graph e forbidden-edge test. | Novo crate/adapter |
| AI-004 | Provider adapters dependem de `provider-core`; core não conhece adapter concreto. | Cargo graph + MockProvider contract + architecture lint. | Provider/chat PR |
| AI-005 | Application API é owner único dos use cases e traduz Command/Result/Event envelopes. | Contract tests por comando e bridge/CLI fixture. | UI, CLI, remote |
| AI-006 | Toda entrada externa é validada em schema → tamanho → identity/ownership → capability → lifecycle → quota/deadline → policy → efeito. | Malformed/oversized/stale/unknown-field negative matrix. | Qualquer boundary |
| AI-007 | Cada command tem `request_id`/idempotency key; retry não duplica efeito desconhecido. | Duplicate/retry/timeout reconciliation tests. | Workflow/tool/provider |
| AI-008 | `project_id` participa de keys, queries, events, artifacts, memory, skills, secrets e traces conforme escopo. | Cross-project negative queries e artifact/event fixtures. | Todo estado persistente |
| AI-009 | Compartilhamento entre projetos é grant explícito, mínimo, expirável, auditável e revogável. | Grant scope/expiry/revoke/race tests. | Memory/skill/remote |
| AI-010 | Tool calls passam Permission Engine; default deny e policy failure nunca vira allow. | Deny/timeout/revoke/no-UI/parallel/negative policy tests. | Tool/workflow/scheduler |
| AI-011 | Aprovação é para fingerprint exato de capability/target/args/schema/project/actor/expiry. | Arg mutation, replay, double-submit, self-approval e expiry tests. | High-risk effects |
| AI-012 | Shell/process/filesystem/network externos executam apenas via Sandbox/Execution Broker com bounds e cancellation. | Path/symlink/escape/network/process/resource adversarial suite por OS. | Tools/Python/plugins |
| AI-013 | Shell irrestrito nunca é default nem fallback silencioso. | Unavailable-sandbox fixture exige deny ou decisão explícita. | Release/tool enablement |
| AI-014 | Secrets nunca são plaintext em SQLite, `.env`, frontend, localStorage, log, trace, artifact ou backup não criptografado. | Secret scanning + golden redaction + crash/backup/clipboard tests. | Auth/provider/release |
| AI-015 | OAuth/deep link usa state/nonce/PKCE, redirect allowlist, expiry, anti-replay e callback bound ao principal/projeto. | Invalid/duplicate/expired/mismatched callback tests. | Provider/remote |
| AI-016 | Python é opcional; core boot, tests e chat funcionam sem runtime Python. | No-Python boot fixture + worker protocol lifecycle tests. | Python-dependent card |
| AI-017 | Python/MCP/plugin/remote não recebem capability por confiança transitiva de processo/host. | Manifest/transport/capability narrowing and quarantine tests. | Extensions/remote |
| AI-018 | Skill não altera runtime silenciosamente; versões são imutáveis, pinadas por run e rollback preserva histórico. | Manifest/lifecycle/pin/activation/rollback fixture. | Skills/evolution |
| AI-019 | Modelo apenas propõe memory; candidate exige provenance, dedupe, policy, edição e isolamento antes de persistir. | Candidate approval/clear/dedupe/isolation tests. | Memory/autolearning |
| AI-020 | Agent instruction hierarchy é determinística e texto de menor confiança não substitui security/project policy. | Precedence conflict and prompt-injection persistence tests. | Agent/context |
| AI-021 | InvocationGraph valida principal, access, capability narrowing, budget, depth, fanout e cycles antes de delegation/parallel. | Cycle/depth/fanout/cross-project/loop tests. | Multi-agent |
| AI-022 | Workflow é DAG persistente, versionado e contém contracts de node/input/output/capability. | Graph validator rejects cycle/unknown node/oversized payload. | Workflow/scheduler |
| AI-023 | Workflow state transitions, checkpoints e events são duráveis/idempotentes; crash não duplica side effect não reconciliado. | Crash injection antes/depois de boundary + restart/recovery tests. | Scheduler/recovery |
| AI-024 | Scheduler usa lease/generation/fencing, missed-run policy bounded e concorrência explícita. | Clock/DST/restart/duplicate/concurrent-run tests. | Automation |
| AI-025 | Toda execução autônoma possui run/trace/correlation IDs, spans e redaction; sink failure não é sucesso. | Trace schema, sensitive golden, retention e sink-failure tests. | Autonomous/release |
| AI-026 | Estado persistente tem owner, scope, schema version, constraints, UTC, migration checksum e retention. | Clean/upgrade/failed/torn migration + repository contract tests. | Schema/data |
| AI-027 | Migration faz preflight, lock, backup/last-known-good e transaction; downgrade sem migration validada é bloqueado. | Power-loss, full-disk, restore e compatibility matrix. | Distribution/data |
| AI-028 | Update verifica signature/hash/channel/expiry/min-version/compatibility antes de substituir e mantém rollback last-known-good. | Install/upgrade/rollback/compromise drill por OS. | Release/updater |
| AI-029 | Artefato de release tem SBOM, licenses, provenance, digest e binding ao commit/tree/policy. | Artifact manifest verifier e wrong-SHA/missing-artifact negative test. | Release gate |
| AI-030 | Actions/dependencies novas têm justificativa de necessidade, manutenção, licença, segurança, custo e substituição. | Dependency decision record + lock/SBOM/advisory/license check. | Dependency/CI |
| AI-031 | Um agente executa uma PR em worktree/branch exclusivo, nunca altera `main` ou arquivos fora do card. | Preflight/status/scope manifest + path allowlist fixture. | Toda PR de agente |
| AI-032 | Nenhum agente aprova seu próprio trabalho; review independente deve avaliar diff, testes, security e scope. | Distinct reviewer identity and required-check evidence. | Merge/release |
| AI-033 | Rebase, SHA novo, CI tardio ou resultado assíncrono invalida evidência anterior até revalidação. | Stale-result fixture e artifact identity check. | Todo gate |
| AI-034 | Scope drift, blocker, secret, migration insegura, CI/security failure ou API não comprovada faz o agente parar. | Policy decision log e negative acceptance fixture. | Implementação |
| AI-035 | O core é reutilizável por adaptadores não-Tauri sem trocar regras de domínio. | Contract test exercido por adapter fake/CLI-like. | Surface/API |
| AI-036 | Extension/plugin/provider/remote revocation encerra handles/leases dependentes e impede novas calls. | Revoke/quarantine/credential-rotation tests. | Extensions/auth |
| AI-037 | Claims de sandbox, site isolation, secure renderer ou production só existem com evidência por OS/versão. | Release claim matrix linked to adversarial tests. | Release marketing |
| AI-038 | Todos os cards possuem os 19 campos canônicos, dependências existentes/acíclicas, testes, security, docs, DoD e condição de desbloqueio. | Queue parser/schema validator; current 35 label mismatches remain `NO_PROOF`. | Queue execution |

## Protocolo de verificação

1. O agente identifica invariantes afetados no card e carrega ADR/contrato predecessor.
2. O check registra comando, versão de ferramenta, SHA/tree, policy/schema revision, artifact digests e status terminal.
3. Negative/failure cases são obrigatórios para invariantes de segurança; um happy path isolado não prova a boundary.
4. Qualquer mudança de ownership, schema, event/trace, threat boundary, migration, workflow, dependency ou release atualiza o invariante, teste e/ou ADR antes do merge.
5. Um resultado `NO_PROOF` é apresentado como ausência de evidência, nunca como aprovação implícita.

