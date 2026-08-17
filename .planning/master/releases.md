# Release plan e demos verificáveis

**Status:** metas e gates de planejamento; nenhuma demo foi executada e nenhum release está aprovado.  
**Regra:** a demo deve usar um artefato instalado, dataset/fixture versionado, SHA/tree/policy revision identificados e relatório reproduzível. `NO_PROOF` impede promoção.

## Contrato comum de release

Cada candidato precisa de: matriz OS/arch suportada; build e install/upgrade smoke; checks required verdes no mesmo SHA; SBOM e licenças; provenance/attestation; assinatura por canal; checksums; migration/backup status; security/threat regression; observabilidade/redaction; rollback testado; reviewer de arquitetura, segurança, QA e distribuição. Falha, timeout, resultado stale, ausência de artifact ou check não obrigatório mantém `NO_GO`.

| Release | PRs/milestones | Demo verificável | Gate mínimo e não-goals |
|---|---|---|---|
| **v0.1 Foundation** | M0–M2, PR-001–PR-055 | Instalar o artefato local em uma máquina suportada; criar um Project e um Agent, editar personalidade/instruções/policies, fechar/reabrir e observar isolamento, erros e eventos redigidos. | Workspace/CI/architecture fixtures, Project/Agent persistence e Application API; Tauri não é core, UI não toca SQLite, sem tools/providers/remote/autoevolução. `NO_PROOF` até smoke instalado e testes de isolamento. |
| **v0.2 Single-Agent Chat** | M3–M4, PR-056–PR-095 | Em fixture offline com MockProvider, selecionar modelo, enviar uma mensagem, receber streaming, cancelar, retry permitido, reabrir sessão e conferir tokens/custo/trace sem prompt/secrets em logs. | Provider-core/normalized stream, credential handles, OAuth negative tests, Session/Message/context/state machine e Tauri events; sem grupo, workflow unattended ou provider não contratual. |
| **v0.3 Agent Tools** | M5–M6, PR-096–PR-121 | Em Project temporário, pedir leitura/listagem e uma escrita aprovada; negar path fora do root, shell não autorizado e aprovação expirada; executar uma tool Python opcional com worker isolado e reproduzir sem Python. | Schema/registry/permission/approval, sandbox profile por OS, timeout/cancel/output bound, audit/trace e Python JSON-RPC lifecycle; sem shell irrestrito, plugin/MCP ou secrets em output. |
| **v0.4 Memory + Skills** | M7–M8, PR-122–PR-154 | Criar uma memória candidate, aprovar/editar/remover e recuperar somente no Project correto; instalar uma skill com versão pinada, executar teste, ativar e fazer rollback mantendo histórico. | Candidate/provenance/dedupe/retrieval budget, global skill import explícito, manifest/lifecycle/evaluator/activation/rollback; sem skill alterar runtime ou auto-publicar sem policy. |
| **v0.5 Multi-Agent** | M9, PR-155–PR-172 | Criar Group com dois agentes MockProvider, delegar uma tarefa, mostrar mensagens/grafo/custo, limitar rounds/depth/budget e rejeitar ciclo/projeto inacessível. | InvocationGraph, principal/capability narrowing, cycle/depth/fanout/parallel budgets, moderator/round/synthesis e UI projetada; sem aprovação por agente solicitante. |
| **v0.6 Workflows + Scheduler** | M10–M11, PR-173–PR-203 | Criar DAG Agent→Condition→Approval→Tool, reiniciar durante um node, recuperar checkpoint sem duplicar efeito, agendar intervalo/cron em sandbox, aplicar missed-run e impedir concorrência duplicada. | Workflow persistence, idempotency, recovery, leases/fencing, bounded queues, clock/DST, history e notification; automations só após scheduler e approvals. |
| **v0.7 Development Agents** | M12, PR-204–PR-217 | A partir de um repository fixture, criar worktree/branch por task, executar um card, gerar PR e obter review/CI status; tentativa de alterar `main` ou arquivo fora do scope falha. | Repository/worktree/branch/task policy, profiles separados coding/reviewer/QA/security/architecture, independent review, CI evidence e release-agent sem chaves; sem autoaprovação. |
| **v0.8 Controlled Self-Evolution** | M13, PR-218–PR-231 | Gerar candidate de skill/workflow/config, avaliar contra baseline/holdout, executar regression suite, exigir approval, fazer canary/rollout e acionar rollback em regressão; criar issue futura para trabalho fora do card. | Proposal/evaluation/regression/scoring, capability L0–L4, pin por run, provenance, approval, rollback e Git/PR para runtime Rust; sem self-rollout silencioso. |
| **v0.9 Plugins + MCP + Remote** | M14–M15, PR-232–PR-251 | Registrar MCP/plugin opt-in com manifest e capabilities, negar transporte/permissão inválida, conectar daemon remoto autenticado, executar tool dentro do project scope, revogar credencial e observar stream/trace. | Transport/auth/permissions antes de discovery, lifecycle/quarantine antes de plugin, remote protocol/node identity/credential isolation antes de efeitos; sem confiança transitiva ou install automático. |
| **v1.0 Production** | M16, PR-252–PR-270 + estabilização | Instalar artefato assinado em Windows/Linux/macOS suportados; fazer backup, restore e migration interrompida; simular crash/rate/resource/security/fuzz/load/provider/recovery; verificar updater, rollback e distribution gates com artefato last-known-good. | Backup/restore/migration/secrets, audit/limits, adversarial tests, compatibility, signing, SBOM/provenance, installer, updater, rollback e compromise drill. Sem claim de “production-ready” antes de evidência por OS. |

## Evidência mínima por demo

1. `demo_id`, release, fixture/data version, OS/arch e ferramenta de execução.
2. artifact digest, repository commit/tree, policy/schema/ADR revisions e lista de PRs.
3. passos reproduzíveis com expected success/failure, logs/trace redigidos e resultado terminal.
4. checks required, security scan, migration/rollback e reviewer identities; comentários de IA não contam como aprovação.
5. artefatos anexos, retenção e vínculo de cada afirmação ao teste ou fato observado.

## Estado atual

Todas as versões estão `PLANNED / NO_PROOF`. O planejamento não afirma que qualquer demo, check, assinatura, backup, migration, sandbox, provider ou updater foi executado. Uma release somente muda para `READY FOR HUMAN REVIEW` após os gates e para `APPROVED` somente após aprovação independente conforme `agent-development-policy.md`.

