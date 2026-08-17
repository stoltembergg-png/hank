# SDD mestre — Plataforma Desktop Multiagente

**Status:** especificação normativa de planejamento; não é implementação, execução, aprovação ou prova de comportamento.  
**Fonte primária:** `../source/sdd-input.md` (Draft v0.1, 2026-08-17).  
**Fontes reconciliadas:** `../reviews/specification-review.md`, `../reviews/architecture-boundaries.md` e as três filas em `../queue/`.  
**Regra de evidência:** `planned`, `NO_PROOF`, `blocked`, `no-run`, `partial` ou `stale` nunca equivalem a `pass`.

## 1. Decisões fechadas e escopo

As decisões abaixo fecham conflitos dos insumos e passam a ser normativas para todos os cards:

| ID | Decisão fechada |
|---|---|
| D-001 | Tauri 2 é shell desktop/adaptador de bridge, janela, eventos nativos, deep links e packaging. Não é Agent Core. |
| D-002 | O produto é um core Rust modular reutilizável; a composição é Presentation → Application → Domain/Core → Execution/Durable → Trust/Extensions, com ports estáveis. |
| D-003 | `project_id` é a unidade de isolamento. Compartilhamento exige grant explícito, escopado, auditável, expirável e revogável. |
| D-004 | Toda entrada passa, em ordem, por schema, tamanho, identidade/ownership, capability, lifecycle, quota/deadline, policy e só então efeito; falha é fail-closed. |
| D-005 | Efeitos externos passam por Permission Engine e Sandbox/Execution Broker. Default é deny; aprovações são vinculadas ao fingerprint exato da operação. |
| D-006 | Python é sidecar opcional. O core deve inicializar, testar e operar sem Python disponível. |
| D-007 | Workflows são DAGs persistentes; checkpoint, idempotência, lease/fencing e recovery precedem scheduler e automações unattended. |
| D-008 | Skills, workflows e configurações evoluídas têm versões imutáveis, pin por run, avaliação, ativação atômica e rollback. Rust só muda por Git, testes, PR, review e policy. |
| D-009 | Todo release é promovido somente com artefato, SHA/tree, policy revision, assinatura, SBOM/proveniência e evidência de testes ligados. |
| D-010 | A fila é preservada como recebida. O índice e o DAG registram correções sem editar os arquivos de queue; inconsistências permanecem `BLOCKER`/`NO_PROOF` até remediação. |

### Escopo comprometido

O produto planejado é uma plataforma desktop para projetos isolados contendo agentes, grupos, sessões, memória, skills, tools, workflows, automações, tasks, arquivos, repositórios, histórico, permissões, orçamento e contexto compartilhado. O core Rust deve poder ser adaptado por Desktop, CLI, TUI, Web, Mobile, API e workers remotos, mas a paridade de todas as superfícies não é compromisso de cada release.

| Release | Compromisso observável | Fora do compromisso |
|---|---|---|
| v0.1 | Foundation, domínio de Project/Agent e políticas mínimas; demo local instalada, sem efeitos perigosos por default. | Tools, Python, remoto, autoevolução e claims de produção. |
| v0.2 | Chat single-agent com provider port, streaming, cancelamento, sessão persistente e trace redigido. | Multi-agent, workflows unattended, provider sem contrato ou segredo em texto. |
| v0.3 | Tools opt-in, permission/approval, sandbox profile comprovado e Python opcional com protocolo. | Shell irrestrito, fallback permissivo, plugins e MCP trusted por default. |
| v0.4 | Memória isolada/editável e skills versionadas, testáveis e reversíveis. | Aprendizado publicado sem avaliação/rollback. |
| v0.5 | Groups, delegation, graph, ciclos, profundidade, orçamento e UI observável. | Delegação sem principal ou capability herdada. |
| v0.6 | Workflow DAG persistente, recovery e scheduler com lease/missed-run/concurrency. | Automação sem limites ou aprovação. |
| v0.7 | Development agents com repository/worktree/branch policy, review e CI status. | Agente alterando `main`, secrets ou aprovando a si mesmo. |
| v0.8 | Evolução controlada por proposta, benchmark, regressão, aprovação e rollback. | Self-rollout sem baseline ou runtime Rust modificado diretamente. |
| v0.9 | MCP/plugins opt-in e remote runtime com auth, protocolo, isolamento e revogação. | Extensão sem provenance/quarantine; remote sem credential isolation. |
| v1.0 | Hardening, backup/restore, migrations/secrets, limiting, audit, testes adversariais, signing, installers, updater e rollback. | Qualquer claim de segurança/isolamento sem evidência por OS e artefato. |

## 2. Non-goals e condições de parada

- Não é objetivo desta fase criar código, inicializar Git, executar produto ou afirmar que qualquer check, migration, sandbox, assinatura, provider, plugin ou release está concluído.
- Microsserviços locais, engine Servo/browser, Docker/Podman/SSH/WASM como fallback automático, parity Web/Mobile e deployment cloud ficam fora até possuírem ADR e card explícito.
- Conteúdo de provider, página, tool, plugin, MCP, skill, Python ou usuário é dado não confiável; texto não cria policy, capability ou aprovação.
- Um card é bloqueado quando seu contrato predecessor não tem evidência, quando CI/security/architecture check falha, quando há migration insegura, secret, scope drift, base SHA stale, dependência inexistente/cíclica ou decisão arquitetural aberta.

## 3. Arquitetura normativa

### 3.1 Planos e fronteiras

| Plano/camada | Responsabilidade e ownership | Pode depender de | Não pode conhecer/usar |
|---|---|---|---|
| Presentation | Frontend local e Tauri; estado de tela, bridge tipada, CSP, capabilities mínimas, notificações e deep links. | Application API e projeções/eventos autorizados. | SQLite, filesystem, providers, tools, secrets ou Event Bus bruto. |
| Application | Use cases, autorização do caller, transações, idempotência, approvals, cancellation e projeções. | Domain, ports de infraestrutura/runtime, Permission Engine e Event Bus. | SDK de provider, SQL/HTTP/shell direto, keychain direto ou Tauri dentro da regra. |
| Domain/Core | IDs, entidades, invariantes, policies puras, state machines, budgets, errors e ports. | Tipos estáveis e serialização/schema mínima. | Tauri, SQLite/SQLx, Tokio, rede, Python e providers/tools concretos. |
| Execution/Durable | Agent Runtime, provider port/adapters, Tool Runtime, Sandbox Broker, Python worker, Memory, Skills, Workflow e Scheduler. | Contracts versionados, Application ports, Permission Engine e stores owners. | Bypass de approval, accesso cruzado, provider concreto no core ou efeito sem trace. |
| Infrastructure | SQLite/SQLx/migrations, blobs, locks, paths canônicos, HTTP/TLS, OS handles e repositórios. | Domain/Application ports e APIs OS isoladas. | Regras de domínio, decision de permission, Tauri/frontend ou secret plaintext. |
| Trust/Extensions | Secrets/Auth, observabilidade, Plugin/MCP, remote transport, Update Verifier e release authority. | Capability broker, manifests, auth, signing e schemas. | Confiança transitiva por estar no mesmo host; promoção silenciosa de capability. |

Tauri não é Agent Core; `agent-core` não depende de `browser-core`/Servo. Provider adapters dependem de `provider-core`, nunca o inverso. A UI chama Application API; nenhuma tela acessa SQLite.

### 3.2 Trust levels e entrada

- **T0 não confiável:** provider/page/tool output, usuário, pacote externo, MCP remoto, plugin não ativado. Só pode ser analisado dentro de schema/tamanho/contexto.
- **T1 restrito:** dados/skills/memória/workflow de um projeto. Não amplia capability nem atravessa projeto.
- **T2 controlado:** frontend, Tauri, Python configurado e sandbox. Só solicita operações por contrato.
- **T3 privilegiado:** Application, Permission Engine, Secrets Broker e Update Verifier. Decide apenas após validar identity, capability, lifecycle e versão.
- **T4 autoridade de distribuição:** chaves, CI protegido e metadata de release. Nunca é lido por app, provider, plugin, fork ou agente.

O envelope de comando/evento deve conter `schema_version`, `request_id`/`idempotency_key`, actor, project/profile, contexto de session/agent/run, capability, trace, deadline/cancellation e payload limitado. Resultado terminal é um de `succeeded`, `rejected`, `failed`, `cancelled`, `timed_out`, `not_supported` ou `blocked`, com erro estável e redigido.

## 4. Domínio, runtime e policies

- **Project:** owner de agentes, groups, sessions, memories, skills importadas, workflows, tasks, repositories, folders, artifacts, settings e arquivos de instrução. Todas as chaves, consultas e eventos incluem scope.
- **Agent:** identity, role, personality, instruction hierarchy (`system → security → project → agent → workflow → skill → conversation → user`), model/provider/tool/memory/skill/context policies, autonomy, budget e project binding.
- **Session/Message:** project, agent, participantes, mensagens, tool calls, artifacts, snapshots, tokens, custos e traces; streaming é event-driven e cancelável.
- **AgentGroup/InvocationGraph:** members, moderator, routing/turn, rounds, budgets, shared context e permissions; delegation verifica existência, access, project, capability, quota, profundidade e ciclos.
- **Provider:** `ModelProvider` normaliza complete/stream, model, capabilities, errors, usage e cost. Providers iniciais são OpenAI, Anthropic, Gemini, OpenRouter, Ollama e OpenAI-compatible; fallback só em policy/idempotência.
- **Context:** seleciona system/security/project/agent/skills/memories/conversation/task/tools/group com orçamento; nunca envia o banco inteiro. Compressão preserva checkpoint e fonte bruta.
- **Budget:** limite por project/agent/workflow/task e por tokens/custo; retry, parallel e delegated work consomem o mesmo budget traceável.
- **Tool:** name, schema, capability, environment, timeout, cancellation, output bound e handler. Filesystem, process, terminal, git, HTTP/browser/search, Python, clipboard e notifications passam pelo broker.
- **Memory:** working/short-term/long-term; candidate → importance → dedupe → storage → retrieval. Modelo sugere; usuário/policy aprova, edita e remove. Isolamento e provenance são obrigatórios.
- **Skill:** `SKILL.md`, manifest, scripts/templates/references/tests e metadata; estados draft/testing/active/deprecated/archived/blocked, versões imutáveis, pin e rollback.
- **Workflow/Scheduler:** DAG persistente com Agent/Tool/Python/Condition/Parallel/Delay/Approval/SubWorkflow; checkpoint e recovery antes de triggers. Scheduler suporta one-shot/interval/cron/event/dependency, leases, missed-run bounded, concorrência, histórico e notificações.

## 5. Segurança, extensões e remoto

Secrets ficam em OS keychain/Stronghold por opaque handle; nunca em SQLite, `.env`, localStorage, logs, traces, artifacts ou backups não criptografados. OAuth usa state/nonce/PKCE, redirect allowlist, expiry, callback anti-replay e troca no core. A aprovação humana é separada de sugestão e execução, ligada ao hash de target/capability/args/schema/project/actor/expiry; deny/timeout/revoke são terminais.

Sandbox profiles definem roots, symlink/path canonicalization, rede, processos, ambiente, CPU/memória/disco, usuário e comportamento quando o isolamento não está disponível. Nenhum perfil degrada silenciosamente para shell irrestrito.

Python usa worker JSON-RPC versionado, lifecycle/restart, output bounds, permissões e logs; boot sem Python é suportado. MCP e plugins são opt-in: manifest, provenance, hash/versão, capability allowlist, auth/transport, quarantine, lifecycle, revoke e rollback. Remote exige node identity, auth, protocolo versionado, TLS apropriado, event stream, capability narrowing, credential isolation e revogação.

## 6. Persistência, distribuição e observabilidade

SQLite é store inicial de metadados com migrations versionadas, constraints, ownership, UTC, locks, idempotência, outbox/inbox quando necessário e blobs com digest/tamanho/retention em stores controlados. Migrations são transacionais, checksum-verificadas, com preflight, lock, backup/last-known-good; downgrade sem migration validada é bloqueado. Workflows/runs possuem state durable, checkpoint, lease/fencing e recovery sem replay cego.

Cada execução autônoma produz `run_id`/`trace_id` e spans de contexto, provider, tool, sandbox, Python, memory, skill, delegation, approval, checkpoint e recovery. Trace/audit são append-only no período definido, com correlation, digest, policy/schema/revision, cost/usage, outcome e redaction de prompt, headers, tokens, secrets e conteúdo de página. Falha do sink, trace ausente ou identity mismatch é `NO_PROOF`, não sucesso.

Cada artefato por OS/arch precisa de checksums, SBOM SPDX/CycloneDX, provenance ligada a repo/workflow/commit/tree/digest, assinatura por canal, metadata de versão/expiry/revocation/min version e smoke de install/upgrade/rollback. Updater verifica assinatura, hash, canal, compatibilidade e downgrade antes de substituir; mantém last-known-good e não remove dados/secrets/artifacts no rollback.

## 7. Testes, releases e rastreabilidade

Cada crate tem unit/integration/contract tests; a matriz inclui MockProvider determinístico, provider compatibility, permission negatives, project isolation, migrations, crash/power-loss, workflow recovery, loop/depth/cycle, skill rollback, Python lifecycle, plugin/MCP, remote auth, security/fuzz/load e release drills. Required checks são aplicados ao tipo do card e ao release; rebase/SHA novo invalida evidência anterior.

Os releases v0.1–v1.0 são definidos em `releases.md`. Os invariantes testáveis estão em `architecture-invariants.md`, o fluxo de agentes em `agent-development-policy.md` e o prompt executável em `PR-EXECUTION-CONTRACT.md`. Cada PR deve apontar para requisito/invariante/ADR, teste/artifact e demo; nenhum comentário de IA substitui review ou aprovação.

## 8. Reconciliação adversarial e estado dos insumos

A validação mecânica encontrou 270 cards únicos e sequenciais (`001–270`), três ranges sem lacunas/duplicatas, nove categorias válidas, 30 cards com o rótulo alternativo `Arquivos prováveis` em vez de `Arquivos/crates prováveis`, e cinco cards de fim de milestone com `Condição para desbloquear a próxima milestone` em vez de `...próxima PR`. Isso é preservado como blocker de conformidade documental; não se reescreveu a fila.

As referências de dependência expandiram ranges e confirmaram 0 IDs inexistentes e 0 ciclos após normalização. Foram identificadas cinco referências numéricas para frente: PR-105→110 é gate de release, não predecessor; PR-150 menciona PR-218+ explicitamente como “não é dependência”; e a precedência semântica é PR-228→227, PR-236→235, PR-250→248/249/251. O DAG documenta a normalização e mantém `NO_PROOF` até a fila ser corrigida.

O review possui 78 achados únicos: 27 `BLOCKER`, 45 `MAJOR`, 5 `MINOR`, 1 `SUGGESTION`; veredito original `Not ready`. Os blockers de arquitetura, identidade/permission, sandbox/secrets, persistência/recovery, API/eventos, observabilidade/testes, distribuição/extensões, autonomia e governança permanecem bloqueadores de implementação até que os contratos e evidências correspondentes existam. Nada neste documento os declara resolvidos em runtime.

### Regras imutáveis preservadas

1. Tauri não é Agent Core.
2. Frontend nunca acessa SQLite diretamente.
3. Agent Core não depende de providers concretos.
4. Providers dependem de `provider-core`.
5. Tool calls sempre passam pelo Permission Engine.
6. Shell irrestrito não é default.
7. Skills não alteram runtime silenciosamente.
8. Autoevolução é versionada e reversível.
9. Projetos são isolados por padrão.
10. Invocações têm limite de profundidade e ciclos.
11. Workflows persistentes sobrevivem a reinicialização.
12. Segredos nunca ficam plaintext.
13. Python não é requisito do core.
14. Atividade autônoma tem trace redigido.
15. Mudança no app passa por Git, testes, review e policy.

**Conclusão:** o SDD mestre fecha a interpretação normativa e organiza a execução, mas o planejamento continua `PLANNED/NO_PROOF` até que cada blocker e cada gate de release tenha evidência vinculada ao SHA/tree/policy corretos.
