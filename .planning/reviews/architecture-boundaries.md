# Revisão de arquitetura — fronteiras consolidadas

> **Status:** proposta normativa de planejamento; não é implementação nem prova de comportamento.
>
> **Fonte primária:** `.planning/source/sdd-input.md` (Draft v0.1, 2026-08-17).
>
> **Escopo:** consolidar a arquitetura da plataforma desktop multiagente, explicitar trust boundaries e registrar contratos que devem existir antes das PRs de produto. Este documento não cria código, não escolhe um provedor específico e não abre a capability de extensões.

## 1. Decisão arquitetural consolidada

A plataforma é um core Rust modular, reutilizável por Desktop, CLI, TUI, Web, Mobile, API e workers remotos. Tauri 2 é apenas shell desktop privilegiado: cria janelas, hospeda a UI local e expõe uma bridge tipada. A UI não é o core e nunca acessa SQLite, filesystem, providers, tools ou secrets diretamente.

O sistema é dividido em quatro planos, com contratos explícitos entre eles:

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Presentation plane                                                    │
│ frontend local ── typed commands/events ── Tauri/Desktop              │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ Application API
┌──────────────────────────────▼───────────────────────────────────────┐
│ Policy and domain plane                                               │
│ Domain/Core · Application Layer · Agents · Multi-agent Orchestration   │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ ports, commands, events
┌──────────────────────────────▼───────────────────────────────────────┐
│ Execution and durable plane                                           │
│ Workflows · Scheduler · Tool Runtime · Python · Providers · Memory    │
│ Skills · Infrastructure/Persistence                                   │
└──────────────────────────────┬───────────────────────────────────────┘
                               │ capability broker / isolated adapters
┌──────────────────────────────▼───────────────────────────────────────┐
│ Trust and extension plane                                             │
│ Sandbox · Secrets/Auth · Observability · Plugin/MCP · OS/external net │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.1 Regras de composição

1. O Domain/Core é a autoridade para identidade, invariantes de domínio, políticas puras e máquinas de estado; ele não conhece Tauri, SQLite, Python, providers concretos ou rede.
2. A Application Layer coordena casos de uso, transações, autorização e lifecycle por meio de ports. Ela não contém implementação de provider, tool, storage ou shell.
3. Infrastructure implementa ports e adapta recursos externos. Um adapter não pode promover seus detalhes ao contrato de domínio.
4. Toda capacidade que pode causar efeito externo passa pelo Permission Engine e pelo Sandbox/Execution Broker correspondente. O modelo nunca chama shell, processo ou filesystem diretamente.
5. Projeto é a unidade padrão de isolamento. Nenhum objeto, memória, skill, artifact, workflow, secret ou evento de um projeto fica acessível a outro sem uma concessão explícita, auditável e revogável.
6. Autonomia é uma política, não uma autorização implícita. Scheduler, workflow e agentes só usam capabilities previamente concedidas; ações de alto risco continuam sujeitas a aprovação humana.
7. A atividade autônoma gera trace redigido e eventos correlacionados. Falta de trace, identity mismatch, resultado stale, timeout, skip ou evidência ausente é falha, não sucesso neutro.
8. O produto começa como processo local modular. Sidecars, plugins, MCP, Python, workers remotos e futuro multi-processo são boundaries adicionais; não herdam confiança apenas por estarem no mesmo computador.

### 1.2 Estado de confiança

| Nível | Ator/conteúdo | Pode fazer | Não pode fazer |
|---|---|---|---|
| T0 — não confiável | usuário fornecido a uma sessão, conteúdo de página, resposta de provider, saída de tool, pacote/manifesto externo, MCP remoto, plugin não ativado | ser analisado como dados dentro de schema/tamanho/contexto | alterar política, executar comando, obter secret ou atravessar projeto |
| T1 — restrito | arquivos e settings de um projeto, skills importadas, memória aprovada, workflow versionado | participar de execução dentro do project scope | acessar outro projeto ou ampliar capability |
| T2 — controlado | frontend local, Tauri shell, worker Python configurado, sandbox | solicitar operações pelos contratos autorizados | acessar storage/secrets ou decidir aprovação por conta própria |
| T3 — privilegiado | Application Layer, Permission Engine, Secrets Broker, Update Verifier | decidir política, emitir efeitos autorizados e registrar auditoria | confiar em payload sem validar identity, capability, lifecycle e versão |
| T4 — autoridade de distribuição | chave de assinatura, CI protegido, metadata de release, operador break-glass | assinar, revogar, publicar ou congelar canal | ser lida pelo app, provider, plugin, PR de fork ou agente |

Processo único não transforma T0/T1/T2 em boundary de segurança. Até existir isolamento de processo e evidência adversarial por OS, o produto não deve alegar sandbox, site isolation ou renderer seguro.

## 2. Contratos transversais

### 2.1 Envelope de entrada

Toda entrada externa ou entre módulos deve ser decodificada para um contrato tipado antes de qualquer efeito:

| Campo | Regra |
|---|---|
| `schema_version` | obrigatório; versões desconhecidas são rejeitadas |
| `request_id`/`idempotency_key` | obrigatório para comando; duplicata retorna resultado já registrado ou erro explícito |
| `actor_id` e `actor_kind` | identifica usuário, agente, workflow, scheduler, plugin, MCP ou sistema |
| `project_id`/`profile_id` | obrigatório quando a operação possui estado; ownership é verificado antes do lookup |
| `session_id`, `agent_id`, `workflow_run_id` | contexto mínimo e consistente com o projeto |
| `capability` | ação pretendida; não pode ser inferida de texto livre |
| `trace_id`/`parent_span_id` | correlação de execução; ausente é criado na primeira boundary confiável |
| `deadline`/`cancellation_token` | limite e cancelamento propagados; ausência não autoriza execução sem limite |
| `payload` | schema validado, tamanho limitado, sem campos desconhecidos quando a operação for sensível |
| `caller_context` | origem, window/frame, geração, perfil e tenant; necessário para bridge/approval |

Validação é fail-closed na ordem: schema → tamanho → identidade/ownership → capability → lifecycle → quota/deadline → política → efeito. Nenhuma camada downstream deve repetir parcialmente a validação e então considerar a entrada confiável.

### 2.2 Resultado e erro

Toda operação retorna um resultado correlacionado, com `request_id`, estado terminal (`succeeded`, `rejected`, `failed`, `cancelled`, `timed_out`, `not_supported` ou `blocked`), código estável, mensagem segura para UI e detalhe redigido para diagnóstico. Erros não devem carregar token, prompt completo, conteúdo de página, segredo, path absoluto ou saída não sanitizada de processo.

Retry só é permitido quando o contrato declara idempotência e o efeito não pode ser duplicado. Timeout não significa que o efeito não ocorreu; o caller deve consultar o estado pelo `request_id` antes de repetir.

### 2.3 Event Bus

Eventos internos e eventos projetados à UI usam este envelope mínimo:

```text
EventEnvelope {
  event_id, event_type, schema_version,
  occurred_at_monotonic, recorded_at_wall,
  trace_id, span_id, causation_id, correlation_id,
  producer_id, producer_revision,
  project_id?, profile_id?, session_id?, actor_id?,
  scope, sequence, sensitivity, redaction_class,
  payload
}
```

Regras:

- evento é fato observado; não é uma instrução imperativa para a camada receptora;
- `sequence` é monotônica no stream e `scope` impede mistura entre projetos/sessões;
- consumidor rejeita versão desconhecida, payload oversized, producer não permitido e evento de geração/epoch antigo;
- eventos duplicados devem ser seguros para reprocessamento por `event_id`;
- eventos de execução autônoma incluem `run_id` e nunca omitem `trace_id`;
- page content, headers, tokens, secrets e texto integral de prompts são sempre redigidos antes do sink;
- a UI recebe somente projeções autorizadas, nunca o Event Bus bruto nem resultados de tool não filtrados.

Tipos iniciais incluem `AgentCreated/Started/Finished`, `MessageReceived/Generated`, `ToolStarted/Finished`, `WorkflowStarted/Finished`, `SkillCreated/Updated`, `ProjectChanged`, `ProviderConnected`, `TaskCreated/Completed`, `ApprovalRequested/Resolved`, `SandboxStarted/Stopped`, `PythonWorkerStarted/Exited`, `PluginEnabled/Quarantined` e `RecoveryStarted/Finished`.

### 2.4 Trace de execução

Cada execução autônoma tem um trace raiz (`run_id`, `trace_id`) e spans filhos para assembly de contexto, seleção de modelo, provider request/response, tool call, sandbox, Python, memória, skill, delegação, aprovação, checkpoint e recovery. Cada span deve registrar apenas:

```text
TraceRecord {
  trace_id, span_id, parent_span_id?, run_id,
  project_id, session_id?, actor_id, actor_kind,
  operation, state, started_at, ended_at?, duration_ms?,
  input_digest?, output_digest?, provider_id?, model_id?,
  capability?, result_code?, retry_count, cost?, token_usage?,
  artifact_refs?, error_class?, redaction_class,
  repository_revision?, policy_revision?, schema_version
}
```

O conteúdo original fica fora do trace por padrão. Payload sensível exige opt-in explícito, retenção curta e redaction policy; não é necessário armazenar o prompt integral para comprovar a sequência de decisões. Trace e audit log são append-only no período definido; correções são eventos compensatórios, não edição silenciosa.

### 2.5 Ports e isolamento de estado

O Domain/Core define modelos e ports abstratos. A Application Layer possui os use cases e é a única autorizada a compor múltiplos ports numa transação. Infrastructure fornece adapters concretos. O estado de apresentação pertence ao frontend; estado de execução pertence ao runtime; estado durável pertence ao repositório dono. Um componente que apenas observa não pode se tornar owner por copiar o estado.

## 3. Fronteiras por camada

Cada entrada abaixo é contrato de planejamento. As dependências listadas como `port` significam dependência por interface estável, não por implementação concreta.

### 3.1 Domain/Core

| Aspecto | Contrato |
|---|---|
| Responsabilidades | IDs tipados; entidades `Project`, `Agent`, `Session`, `Message`, `Task`, `Group`; value objects; políticas puras; instruction hierarchy; budgets; capability vocabulary; invariantes de lifecycle; erros de domínio. |
| Entrada/saída | Recebe value objects e comandos de domínio já validados. Produz decisão pura, novo estado, domain events e erros tipados; não faz I/O. |
| Permitido | std; serialização/schema mínimo; crates de tipos base; ports definidos no próprio domínio quando necessários para modelagem. |
| Proibido | Tauri; frontend; SQLite/SQLx; filesystem/rede; Tokio/runtime; provider/tool/plugin/Python concreto; secrets; logging com dados externos; acesso a env vars. |
| Estado owner | Estado semântico de entidades e invariantes; não é owner de UI, conexão, arquivo, processo ou secret. |
| Testes | unitários; property/fuzz de invariantes, IDs, budgets e instruction ordering; mutation tests para decisões; testes devem rodar offline e sem runtime externo. |
| Falhas | comando inválido, transição impossível, budget excedido e capability ausente retornam erro determinístico; nunca panic, fallback permissivo ou mutação parcial. |

### 3.2 Application Layer

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Casos de uso; autenticação/autorização de caller; orchestration de domain, storage, runtime, memory, skills, permissions e events; transações; idempotência; cancellation; projeções para API/UI. |
| Entrada/saída | Aceita `CommandEnvelope` de UI/CLI/API/scheduler/workflow. Emite `ResultEnvelope`, `EventEnvelope`, approval requests e comandos para ports; nunca expõe adapter concreto. |
| Permitido | Domain/Core; ports de Infrastructure; Permission Engine; Event Bus; Runtime services; clock/ID abstraídos; policy/config versionados. |
| Proibido | Tauri API dentro de casos de uso; SQL/HTTP direto; provider SDK; shell; acesso direto a keychain; aceitar texto de agente como policy; bypass de approval. |
| Estado owner | Estado de execução de use cases, idempotency records, approvals pendentes e coordenação; não possui UI nem detalhes físicos de persistência. |
| Testes | unit de use case; integration com adapters fake; contract tests de command/event; concurrency, cancellation, stale caller, duplicate e permission negative tests; E2E por fluxo crítico. |
| Falhas | rejeita schema/identity/capability/lifecycle; rollback transacional; estado `blocked` para dependência ausente; timeout e cancellation são terminais e observáveis. |

### 3.3 Infrastructure

| Aspecto | Contrato |
|---|---|
| Responsabilidades | SQLite/SQLx e migrations; repositories; blob/artifact store; locks; filesystem paths; HTTP transport; clocks; process handles; OS integration; adapters de persistência e de rede. |
| Entrada/saída | Implementa ports com dados tipados e limites de tamanho; retorna metadados, referências, checksums e erros classificados; nunca retorna conexão ou path bruto ao frontend. |
| Permitido | Domain contracts; Application ports; SQLite/SQLx; filesystem canonicalizado; OS APIs isoladas; HTTP/TLS configurado; crates de serialização e hashing. |
| Proibido | regra de negócio de agente; decisão de permissionamento fora do Permission Engine; provider routing; tool execution sem broker; segredo em plaintext; dependência de Tauri/frontend. |
| Estado owner | Metadados persistentes, blobs, locks, schema version e last-known-good snapshots; não é owner de entidades em memória durante execução. |
| Testes | migrations clean/upgrade/failed/torn; lock concorrente; atomic write; corruption/backup restore; path traversal; adapter contract; offline HTTP fixtures; OS matrix. |
| Falhas | erro de disco/lock/schema não é convertido em vazio; transação aborta; corrupção preserva snapshot válido; conexão/worker é reciclado só com fencing e diagnóstico. |

### 3.4 Tauri/Desktop

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Compor aplicação; lifecycle de janela/perfil; capability manifest; CSP; deep link; native notifications; serializar/deserializar bridge; packaging hooks. |
| Entrada/saída | Recebe eventos de window e frontend em allowlist; valida origem, window/frame, geração e tamanho; chama Application API; publica somente eventos projetados e autorizados. |
| Permitido | Application API; platform ports; Tauri 2; frontend local empacotado; capability mínima explícita. |
| Proibido | carregar URL remota na UI privilegiada; `invoke` genérico; acesso do frontend a filesystem/process/network/SQL/secrets; lógica de domínio; provider/tool/SQLite direto. |
| Estado owner | Handles de app/window e lifecycle do shell; seleção visual não é fonte de verdade para sessão/projeto. |
| Testes | shell smoke por OS; CSP/capability fixture; malformed/oversized/duplicate IPC; caller origin/window/frame/generation negative; close/reopen/deep-link; packaging smoke. |
| Falhas | bridge indisponível gera erro visível e não executa efeito; window stale perde request; shutdown ordenado; capability desconhecida é rejeitada. |

### 3.5 Frontend

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Renderização e interação; Projects, Chats, Agents, Groups, Tasks, Workflows, Skills, Automations, Models, Usage e Settings; estado efêmero de apresentação; acessibilidade. |
| Entrada/saída | Envia commands tipados por Application API e consome eventos/projeções versionados; renderiza loading/error/approval/stale explicitamente. |
| Permitido | Schema gerado; client de IPC tipado; estado local de UI; assets locais; bibliotecas de apresentação. |
| Proibido | SQLite; browser-core direto; Tauri capability genérica; filesystem/process/network; provider SDK; secrets em localStorage; confiar em texto da página ou agente como instrução. |
| Estado owner | Seleção de aba/painel, filtros, drafts não enviados e estado visual; não possui projeto, sessão, memória, approval ou execução. |
| Testes | component/unit; schema contract; accessibility; visual por OS/tema; stale event/double submit; unauthorized IPC; E2E de aprovação e recovery. |
| Falhas | exibe estado de erro e permite retry seguro; não repete efeitos sem idempotency; descarta evento de projeto diferente; nunca mascara `blocked` como sucesso. |

### 3.6 Provider Adapters

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Traduzir `NormalizedModelRequest/Response`; stream/complete; capabilities; model discovery; usage/cost; health; erros e limites por provider. Implementações iniciais podem cobrir OpenAI, Anthropic, Gemini, OpenRouter, Ollama e OpenAI-compatible. |
| Entrada/saída | Recebe request normalizado, credential handle não extraível e deadline; retorna stream de eventos normalizados, resultado, usage e erro classificado. |
| Permitido | `provider-core`; transport port; Secrets Broker; clock/rate-limit; SDK do provider confinado ao adapter. |
| Proibido | acessar Domain/Core storage; decidir permissions; ler secrets diretamente; executar tools; escrever memory/skills; lógica específica no Orchestrator; enviar todo o banco de contexto por padrão. |
| Estado owner | Connection/session state do provider, capabilities cache e health; não possui sessão, prompt assembled, memory ou budget global. |
| Testes | provider contract; MockProvider determinístico; fixtures de stream/429/timeout/outage/quota/malformed; redaction; compatibility por revisão; nunca depender de internet em PR. |
| Falhas | 429/outage/timeout retornam erro e só ativam fallback se policy permitir; resposta parcial exige estado explícito; provider não pode confirmar tool effect que não ocorreu. |

### 3.7 Tool Runtime

| Aspecto | Contrato |
|---|---|
| Responsabilidades | registry, schema validation, capability lookup, permission check, approval coordination, execution dispatch, timeout, output limits, cancellation, audit e result normalization. Tools iniciais incluem filesystem, terminal, git, HTTP/web/browser/search, Python, process, clipboard e notification. |
| Entrada/saída | Recebe `ToolCall` com tool/version, schema args, actor/project/session, capability, budget, sandbox profile e trace. Retorna `ToolResult` com status, bounded output, artifact refs, digest e side effects declarados. |
| Permitido | Application permission port; Sandbox; Infrastructure filesystem/process/network ports; Observability; registry versionado. |
| Proibido | shell irrestrito por default; chamada direta do LLM ao OS; aceitar path/provider output sem schema; auto-approve destrutivo; acesso cross-project; secrets no output. |
| Estado owner | Tool definitions, call records, approval decisions e execution handles; arquivos/branches/processos pertencem ao adapter/sandbox respectivo. |
| Testes | schema/property/fuzz; permission matrix; traversal/injection; timeout/cancel; output truncation; idempotency; crash/retry; adversarial tool result; E2E com sandbox real quando aplicável. |
| Falhas | deny por default; chamada entra em `rejected`, `blocked`, `timed_out` ou `failed`; não há retry automático de efeitos destrutivos; processo é terminado por broker e resultado fica auditado. |

### 3.8 Python Runtime

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Sidecar/worker opcional; JSON-RPC versionado; SDK; lifecycle; dependency environment; Python tool registration; logs redigidos; quotas e permissions. |
| Entrada/saída | Recebe requests JSON-RPC bounded, com worker ID, tool ID, project scope, env profile, deadline e capability. Responde `result/error/progress` correlacionado; streams e artifacts têm limite e digest. |
| Permitido | Tool Runtime via broker; Sandbox process; stdio/IPC autenticado localmente; SDK versionado; filesystem temporário concedido. |
| Proibido | ser requisito de boot do core; acessar secrets/SQLite/other projects diretamente; rede ou shell fora da sandbox; instalar dependência sem approval/policy; tratar stdout como instrução. |
| Estado owner | Worker lifecycle, environment metadata, request correlation e temp artifacts; não possui sessão, budget global ou approval. |
| Testes | protocol/schema/version; worker start/stop/restart; malformed/oversized JSON-RPC; timeout/cancel; dependency isolation; permission; crash recovery; no-Python core path. |
| Falhas | worker morto produz erro terminal e pode ser reiniciado com novo generation; dependência não resolvida bloqueia a task; output truncado é explícito; nenhum retry com efeito desconhecido. |

### 3.9 Memory

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Working/Short-Term/Long-Term; candidate extraction; importance; dedupe; keyword/embedding retrieval; retention; manual editing; policy por projeto/agente. Tipos: fact, preference, decision, lesson, project_context, technical_context, failure e successful_pattern. |
| Entrada/saída | Recebe candidato contextualizado e aprovado por policy; produz candidate, retrieval result com score/proveniência ou decisão de não incluir. Não recebe todo banco para cada prompt. |
| Permitido | Domain memory types; Application policy; Infrastructure repositories/indexes; embedding port; Observability redigida. |
| Proibido | o modelo escrever diretamente memória durable; cross-project retrieval; guardar secrets/credenciais como fato; alterar instruções/skills; usar embedding provider sem redaction/consent policy. |
| Estado owner | Memórias e índices do projeto/agent scope, status pending/approved/rejected/archived e provenance; conversa continua owner das mensagens brutas. |
| Testes | isolation; candidate review; dedupe; retention/delete; retrieval budget; poisoning/adversarial prompt; corruption/migration; embedding unavailable; manual edit audit. |
| Falhas | candidato permanece não publicado; índice indisponível degrada para busca permitida ou sem memória; dado ambíguo não vira fato; clear data remove/revoga escopo definido. |

### 3.10 Skills

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Parser/validator de `SKILL.md`; manifest, scripts, templates, references, tests, metadata; registry/repository; bindings; lifecycle draft/testing/active/deprecated/archived/blocked; evaluator e rollback. |
| Entrada/saída | Recebe pacote/manifesto não confiável; produz versão validada, capability declaration, resolved references e candidate test report. Ativação retorna uma versão imutável e seu digest. |
| Permitido | Skill schema; project storage; Tool Runtime para scripts dentro de Sandbox; policy de agent/project; test harness. |
| Proibido | alterar Rust runtime, workflow ou config silenciosamente; executar scripts sem permission/sandbox; importar skill global automaticamente; sobrescrever versão ativa; esconder referências ou testes. |
| Estado owner | Conteúdo/versionamento/activation pointer e resultados de avaliação; runtime interpreta a skill, mas não a edita em execução. |
| Testes | parse/schema/path traversal; script isolation; fixtures; version compatibility; evaluator; activation policy; rollback; malicious prompt/reference; project/global scope. |
| Falhas | manifesto inválido, capability não permitida ou teste ausente bloqueia; versão ativa permanece last-known-good; candidate não é publicado por sugestão do modelo. |

### 3.11 Agents

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Identidade persistente; role/personality/instructions; model/provider policy; tool/memory/skill/context policy; autonomy; budget; project bindings; response profile. |
| Entrada/saída | Recebe comandos de criação/alteração versionados e contextos autorizados; produz `AgentConfigSnapshot`, execution intent e proposed actions, nunca efeitos OS diretos. |
| Permitido | Domain/Core; Application services; provider/tool/memory/skill ports; policy engine; project scope. |
| Proibido | alterar sua própria policy fora de workflow de evolução; bypass de budget/approval; acessar outro agent/project sem binding; tratar output de provider como autoridade. |
| Estado owner | Configuração versionada, bindings, budget counters e policy snapshot; session/run state pertence à Session/Runtime. |
| Testes | schema/instruction hierarchy; policy precedence; budget; autonomy levels L0–L4; project isolation; deterministic MockProvider loop; prompt injection/unsafe output. |
| Falhas | config inválida ou provider indisponível bloqueia execução; budget excedido termina; alteração proposta fica pending; nunca há fallback para permissões maiores. |

### 3.12 Multi-agent Orchestration

| Aspecto | Contrato |
|---|---|
| Responsabilidades | AgentGroup; members; moderator; routing/turn/round policy; shared context; delegation; InvocationGraph; depth/cycle detection; parallelism; synthesis; budgets e permissions. |
| Entrada/saída | Recebe `InvocationRequest` com caller/member/project/session, target, purpose, depth, graph hash, budget e capability. Produz invocation results/events, approvals e synthesis com provenance por membro. |
| Permitido | Agent service; Application event bus; provider/tool/memory ports sob policy; bounded task executor; group storage. |
| Proibido | delegação implícita; membro fora do project scope; ciclos; unlimited rounds/depth/parallelism; compartilhar secrets ou memória sem grant; moderator que aprova seu próprio high-risk effect. |
| Estado owner | InvocationGraph, group membership/policies, round counters, shared context snapshot e budgets; agent configs continuam owner dos agentes. |
| Testes | graph validation; cycle/depth/round/budget; deterministic routing; parallel cancellation; partial failure; provenance; prompt injection between agents; approval race. |
| Falhas | ciclo/depth/budget/permission falha fechado; membro indisponível produz partial/blocked explícito; synthesis não apaga erros nem atribui ação não executada. |

### 3.13 Workflows

| Aspecto | Contrato |
|---|---|
| Responsabilidades | DAG persistente; nodes Agent/Tool/Python/Condition/Parallel/Delay/Approval/SubWorkflow; validation; run state; checkpoints; logs; recovery e viewer. |
| Entrada/saída | Recebe workflow versionado e `WorkflowRunRequest` com trigger, project, actor, input schema, budget e policy. Emite node state transitions, events, artifacts, approval requests e terminal run result. |
| Permitido | Application services; Domain workflow model; Tool/Agent/Python ports; Scheduler trigger; storage checkpoints; Observability. |
| Proibido | prompt gigante como substituto de DAG; execução de node sem capability; estado somente em memória; loop sem limite; alterar definição ativa no meio da run; esconder output de node. |
| Estado owner | Definição/version, graph, run/node state, checkpoint, retry policy e provenance; scheduler possui trigger, não run state. |
| Testes | deterministic DAG; invalid/cyclic graph; node contracts; approval; crash/restart/resume; idempotency; parallel/timeout/cancel; migration; partial recovery; load. |
| Falhas | node entra em failed/blocked/cancelled; workflow pausa se approval/credential missing; recovery retoma do checkpoint seguro, não repete side effect desconhecido; definição last-known-good fica disponível. |

### 3.14 Scheduler

| Aspecto | Contrato |
|---|---|
| Responsabilidades | one-shot, interval, cron, event e dependency triggers; missed-run policy; concurrency; history; notifications; durable leases e wake-up. |
| Entrada/saída | Recebe `ScheduledJob` validado com trigger, timezone policy, project, target workflow/agent, capability set, budget e concurrency key. Emite `TriggerFired`, `RunSkipped/Missed/Started` e execution request idempotente. |
| Permitido | Application/workflow trigger port; clock; durable scheduler storage; notification port; cancellation. |
| Proibido | executar tool/agent diretamente; ignorar project/approval policy; depender de timer apenas em memória; criar runs duplicadas após restart; usar horário não versionado. |
| Estado owner | Job definition, next fire, lease, missed-run history, concurrency key e trigger provenance; workflow owns execution. |
| Testes | cron/timezone/DST; restart/missed policy; lease/concurrency; duplicate fire; clock skew; cancellation; event trigger auth; notification failure; persistence migration. |
| Falhas | job inválido é disabled/blocked; missed run segue policy explícita (skip, coalesce ou catch-up bounded); lease stale é recuperada com fencing; nenhuma execução sem trace. |

### 3.15 Sandbox

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Perfis `trusted`, `restricted`, `isolated`; filesystem/network/process quotas; environment; process lifecycle; kill; artifact boundary; futura evolução para Docker/Podman/SSH/remote/WASM. |
| Entrada/saída | Recebe execution spec, capability grants, canonical paths, resource quotas e deadline; retorna process/run handle, bounded stdout/stderr, artifacts e exit classification. |
| Permitido | Tool Runtime/Python broker; OS process APIs; temp storage; policy manifest; Observability redigida. |
| Proibido | conceder shell irrestrito por default; herdar env/secrets; root/home do usuário sem scope; rede sem allowlist; confiar no UID do processo como única policy; executar plugin/MCP sem perfil. |
| Estado owner | Sandbox instance, leases, quotas, process handles, mount/network policy e cleanup; conteúdo produzido pertence ao project artifact store após validação. |
| Testes | escape/traversal; env/secret leakage; network e process allowlist; quota/timeout/kill; crash cleanup; OS-specific adversarial tests; capability regression. |
| Falhas | criação falha fechado; kill e timeout são observáveis; cleanup é idempotente; isolamento não comprovado deve ser declarado como risco e bloquear claim correspondente. |

### 3.16 Secrets/Auth

| Aspecto | Contrato |
|---|---|
| Responsabilidades | OS keychain/Tauri Stronghold adapter; credential handles; OAuth browser/deep-link/callback/token exchange; account binding; expiry/revoke; auth session; secret rotation. |
| Entrada/saída | Recebe `CredentialRequest` com actor/provider/project scope, purpose, consent e expiry; entrega handle de uso limitado ao adapter ou erro. O segredo nunca entra em command, frontend, SQLite, artifact, prompt ou log. |
| Permitido | OS keychain/Stronghold; Auth provider ports; Application Permission Engine; provider adapters via opaque handle; redacted Observability. |
| Proibido | plaintext em SQLite, `.env`, localStorage, memory/skill, logs, prompts, crash bundles ou PR; provider/tool/plugin acessar keychain diretamente; OAuth callback sem state/PKCE/anti-CSRF policy. |
| Estado owner | Secrets store, account metadata não secreta, token lifecycle, consent e rotation; provider mantém apenas reference/connection state. |
| Testes | storage ACL; missing/expired/revoked/rotated; OAuth state/replay/callback; redaction; backup exclusion; migration; lock; no-secret fork/CI fixtures. |
| Falhas | auth failure bloqueia efeito; token expirado pede reauth; store indisponível não usa fallback plaintext; revocation invalida handles e sessões relacionadas. |

### 3.17 Observability

| Aspecto | Contrato |
|---|---|
| Responsabilidades | tracing, metrics, audit events, redaction, crash bundle local, health/diagnostics, retention e opt-in telemetry. |
| Entrada/saída | Recebe `TraceRecord`, `EventEnvelope` e error metadata; emite sinks redigidos, counters, bounded logs e bundles com version/digest. |
| Permitido | Cross-cutting instrumentation por port/macro; IDs derivados; local files com retention; sink remoto somente após opt-in e policy. |
| Proibido | raw URL com credentials/query secrets; page text, cookie, headers, tokens, prompt integral, path de usuário ou secret; telemetry silenciosa; alterar status de gate; IA como autoridade. |
| Estado owner | Trace/audit retention, redaction policy revision, diagnostic bundle e health snapshots; não possui estado de domínio. |
| Testes | golden redaction; schema/oversize; trace correlation; sensitive-value fuzz; crash/hang bundle; retention/clear; sink outage; stale identity; privacy review. |
| Falhas | sink indisponível não bloqueia lógica segura nem faz retry infinito; local fallback é bounded; redaction failure descarta evento sensível; bundle parcial declara lacuna. |

### 3.18 Plugin/MCP

| Aspecto | Contrato |
|---|---|
| Responsabilidades | Manifest/registry; discovery; lifecycle; permission/capability mapping; MCP stdio/HTTP client; futuro MCP server; provider/tool/memory/workflow connector hooks. |
| Entrada/saída | Recebe manifest e endpoint não confiáveis; valida ID/name/version/schema/signature/permissions; produz capability-scoped registration e envelopes MCP normalizados. |
| Permitido | Extension/Plugin Broker; Tool/Provider/Memory/Workflow ports; Sandbox; Secrets Broker por handle; allowlist explicitamente ativada. |
| Proibido | carregamento implícito; herdar capability do host ou frontend; acesso direto ao core DB; receber todos os secrets; registro de tool sem schema/permission; MCP HTTP sem TLS/auth/pinning policy; alterar runtime silenciosamente. |
| Estado owner | Manifest/version, activation state, grants, endpoint health, quarantine e compatibility result; plugin não possui estado de projeto fora das referências concedidas. |
| Testes | malicious manifest; unknown permission/version; transport framing; auth/TLS; replay; tool schema; isolation/escape; lifecycle/revoke/quarantine; provider compatibility; project boundary. |
| Falhas | plugin/MCP fica disabled/quarantined; timeout/outage não abre fallback permissivo; versão incompatível é rejeitada; revoke mata leases e impede novas calls. |

## 4. Matriz de dependências

Legenda: **D** = pode depender diretamente; **P** = somente por port/contrato; **C** = condicional após capability/ADR; **X** = proibido; **—** = não aplicável.

| Camada | Domain | Application | Infra | Tauri/Desktop | Frontend | Runtime/Agents | Security | External/SDK |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Domain/Core | D | — | X | X | X | X | P | X |
| Application | D | D | P | X | X | P | D/P | X |
| Infrastructure | D | P | D | X | X | X | P | C |
| Tauri/Desktop | X | D | P | D | D | X | P | C |
| Frontend | X | P | X | P | D | X | X | X |
| Provider adapters | X | P | P | X | X | P | P | D (SDK/transport) |
| Tool Runtime | X | P | P | X | X | P | D | C |
| Python Runtime | X | P | P | X | X | X | P | D (Python/JSON-RPC) |
| Memory | D | P | P | X | X | P | P | C (embedding) |
| Skills | D | P | P | X | X | P | D | C (scripts/templates) |
| Agents | D | P | P | X | X | D | D | X |
| Multi-agent | D | D | P | X | X | D | D | X |
| Workflows | D | D | P | X | X | D | D | X |
| Scheduler | D | P | P | X | X | P | D | X |
| Sandbox | X | P | P | X | X | X | D | C (OS/container) |
| Secrets/Auth | X | P | P | X | X | X | D | C (OS/OAuth) |
| Observability | P | P | P | P | X | P | D | C (sinks) |
| Plugin/MCP | X | P | P | X | X | P | D | C (transport/plugin) |

Regras da matriz:

- `P` não pode ser substituído por uma referência concreta apenas para reduzir trabalho.
- Uma nova aresta exige atualização desta matriz, do manifesto de arquitetura, teste negativo de aresta proibida, owner e ADR se introduzir estado, privilege ou novo trust boundary.
- Frontend → Application é IPC/API tipada; Frontend → Tauri é apenas o client de bridge, sem capability genérica.
- Application → provider/tool/storage/secrets é sempre via port e Permission Engine; o core nunca conhece SDK concreto.
- Plugin/MCP, Python e provider são adapters não confiáveis mesmo quando compilados/instalados localmente.
- Nenhuma camada pode depender ciclicamente de outra. Extração de crate exige segundo consumidor de produção ou boundary de segurança, contrato público, testes independentes e owner.

## 5. Invariantes arquiteturais e threat boundaries

### 5.1 Invariantes bloqueadores

1. **Tauri não é Agent Core.** O shell só compõe e transporta contratos.
2. **Frontend nunca acessa SQLite ou storage físico.** Toda leitura/escrita passa por Application API.
3. **Domain/Core não conhece providers concretos, tools concretas, Python, Tauri ou secrets.**
4. **Provider adapter depende de `provider-core` e recebe credencial por handle; provider logic não entra no Orchestrator.**
5. **Toda tool call passa por schema validation, Permission Engine, approval quando exigido, Sandbox profile, timeout, audit e trace.**
6. **Shell irrestrito não é default.** `terminal`, `process`, `git write`, rede, clipboard e instalação de pacote têm capabilities distintas.
7. **Projeto é isolamento default.** Compartilhamento exige import/grant explícito, scope e revogação.
8. **Skills são imutáveis por versão e não alteram runtime silenciosamente.** Autoevolução produz candidate → testes → avaliação → aprovação/activation → rollback.
9. **InvocationGraph não aceita ciclos, depth, rounds, fan-out ou orçamento ilimitados.**
10. **Workflow é DAG persistente e sobrevive a restart.** Estado somente em memória não autoriza execução.
11. **Scheduler não executa efeitos diretamente e não perde/duplica triggers silenciosamente.**
12. **Secrets nunca aparecem em plaintext em SQLite, `.env`, frontend storage, logs, memory, skills, prompts, artifacts ou traces.**
13. **Python é opcional ao core.** Ausência, crash ou upgrade incompatível do worker não impede boot seguro do produto.
14. **Toda atividade autônoma tem trace e event identity.** Falta de correlação é `NO_GO` para o fluxo.
15. **Atualizações são verificadas, assinadas e reversíveis para last-known-good.** Downgrade, canal errado, metadata expirada ou digest divergente falha fechado.
16. **Ausência de capacidade não é sucesso silencioso.** O resultado é `not_supported`, `blocked` ou degradação documentada e testada.
17. **Nenhum agente, provider, plugin ou reporter de IA é autoridade de aprovação, merge, release ou alteração de policy.**
18. **Mudança de contrato, threat boundary, persistência, workflow trust root ou capability requer ADR, teste negativo e plano de rollback.**

### 5.2 Threat boundaries

| Boundary | Principal abuso | Controle obrigatório | Gate |
|---|---|---|---|
| Frontend/Tauri → Application | IPC forjado, origem/frame/window/generation errados, replay | allowlist, schema/size, caller context, idempotency, capability manifest e CSP local-only | negative IPC; capability/CSP contract |
| Conteúdo/LLM/provider → Agent Core | prompt injection, output malformado, instrução que se passa por policy | separar dados de instruções; schema; policy precedence; nunca executar output diretamente | adversarial agent tests |
| Agent/Workflow → Tool | privilege escalation e efeitos fora do projeto | Permission Engine, approval, sandbox, canonical paths, quotas, trace | permission matrix + sandbox tests |
| Tool/Python → OS | shell injection, escape, secret/env leakage | broker, profile isolado, allowlists, kill, output bounds | fuzz/OS adversarial suite |
| Project A → Project B | cross-tenant memory/file/secret/artifact access | project_id em todas as chaves, repositories scoped, negative queries e grants explícitos | isolation tests |
| Provider/embedding → Data | exfiltração e retenção não autorizada | context builder mínimo, redaction, provider policy e consent; não enviar DB completo | provider/memory privacy tests |
| Plugin/MCP → Core | malicious manifest/tool/transport, replay, excessive capability | registry allowlist, manifest/version, auth/TLS, sandbox, lease/revoke/quarantine | plugin/MCP threat suite |
| Scheduler → Runtime | duplicate/missed unattended high-risk execution | durable lease/fencing, missed-run policy, pre-approved capabilities e approvals | restart/concurrency tests |
| Secrets/Auth → adapters | token theft, callback replay, plaintext fallback | keychain/Stronghold, opaque handle, state/PKCE/expiry/revoke | secret/OAuth tests |
| Update/CI → installed app | supply-chain compromise, rollback/downgrade | signed metadata, provenance, SBOM, channel/key separation, last-known-good | release/compromise drill |

## 6. Permission and approval flow

Uma operação sensível percorre o fluxo abaixo; nenhuma camada pode pular etapas porque o caller é um agente conhecido:

```text
intent
  → normalize + schema/size validation
  → caller/project/session/agent identity check
  → capability lookup and scope check
  → budget/quota/lifecycle check
  → risk classification
  → Permission Engine decision
       ├─ deny → terminal rejected + audit
       ├─ allow by bounded policy → execute in Sandbox + trace
       ├─ approval required → human prompt + signed decision + execute
       └─ missing/ambiguous → blocked (never implicit allow)
```

`PermissionRequest` deve conter `permission_type`, `requesting_origin` quando aplicável, `top_level_scope`, `opener_scope`, `profile_id`, `project_id`, `agent_id`, `workflow_run_id`, `tool_id`, `target_fingerprint`, `user_gesture` quando exigido, `requested_effect`, `expiration`, `trace_id` e `policy_revision`.

Políticas:

- default deny para filesystem write, process/shell, credentials, network e external side effects;
- estados de decisão: `always_allow`, `ask_once`, `ask_every_time`, `deny`; `always_allow` só pode existir para capability explicitamente allowlisted, escopo limitado, sem risco destrutivo e com revogação;
- destruição, force push, credenciais, pagamentos, instalação de pacotes, publicação, mudança de permission, alteração de skills/workflows/config e efeitos externos irreversíveis exigem aprovação humana ou são proibidos pela policy;
- aprovação vale para o fingerprint exato de target, capability, projeto, versão e período. Não pode ser reutilizada para outro path, branch, plugin, origem, geração ou payload;
- aprover não é o agente solicitante, provider, plugin, reporter ou saída de outro modelo; a UI mostra origin/target reais, não título/texto fornecido pela página;
- scheduler/workflow pode usar approvals previamente persistidas apenas dentro do escopo e validade. Se o risco mudou, pausa e solicita nova aprovação;
- revoke/clear de dados invalida grants, leases e handles derivados;
- approvals, denies, expirations e revocations entram no audit log redigido.

## 7. Modelo de estado e persistência

SQLite é o store inicial de metadados com migrations versionadas. Blobs grandes e potencialmente sensíveis ficam em `data/artifacts`, `sessions`, `skills`, `projects`, `cache` e `logs`, com referência, digest, tamanho, owner e retention no SQLite. As entidades mínimas são `projects`, `agents`, `groups/members`, `sessions/messages`, `memories/embeddings`, `skills/versions`, `workflows/nodes/edges/runs`, `tasks/runs`, `providers/accounts`, `scheduled_jobs`, `tool_calls`, `artifacts` e `usage_events`.

Ownership mínimo:

| Estado | Owner | Escopo mínimo |
|---|---|---|
| project/settings/files/repositories | Project/Infrastructure via Application | `project_id` |
| agent config/policy/budget | Agent service | `project_id`, `agent_id`, `config_version` |
| messages/sessions/traces | Session/Observability | `project_id`, `session_id` |
| memory/index | Memory | `project_id`, agent scope e provenance |
| skill packages/activation | Skills | global importado explicitamente ou project |
| workflow definition/run/checkpoint | Workflow | `project_id`, `workflow_id`, `run_id` |
| scheduler job/lease | Scheduler | `project_id`, `job_id`, `lease_generation` |
| tool calls/approvals | Application/Tool Runtime | caller, target, capability, trace |
| provider account metadata | Secrets/Auth + Provider registry | user/project/provider; secret por handle |
| artifacts/logs | Infrastructure/Observability | owner scope, digest, retention |

Nenhuma query aceita somente uma string de ID se o scope esperado também está disponível; a chave composta e a autorização são verificadas antes de devolver `not_found`/`forbidden`, evitando confusão entre ausência e vazamento de existência.

## 8. Migration and versioning rules

1. Cada contrato persistente/IPC/evento/provider/plugin/MCP/worker tem `schema_version` ou `api_version` explícita.
2. Versão desconhecida, enum desconhecido, campo obrigatório ausente, digest errado ou producer incompatível é rejeitado; não há parsing permissivo de segurança.
3. Mudança aditiva compatível deve manter readers antigos, ser coberta por contract tests e registrar política de default seguro. Remoção, semântica diferente, trust boundary ou mudança de ownership exige novo major/version bump e ADR de supersession.
4. Migrations são forward-only no binário publicado, transacionais, checksum-verificadas e com preflight. Antes de alterar dados: acquire profile lock, criar backup/last-valid snapshot e verificar espaço/compatibilidade.
5. Migração interrompida nunca marca versão nova como concluída. Reinício retoma atomicamente ou restaura snapshot; corrupção/rollback não apaga a última versão válida.
6. Downgrade de app não presume downgrade de schema. Se não houver migration reversa validada, o updater bloqueia downgrade ou exige restore explícito de backup compatível.
7. Dados externos não podem dirigir migration. Provider, plugin, skill ou Python só podem fornecer dados após schema validation; migration é determinística e offline.
8. Skills e workflows usam versões imutáveis; ativação aponta para digest/version. Rollback troca o activation pointer, sem sobrescrever histórico.
9. Event consumers devem deduplicar por `event_id` e conservar provenance. Upcasters são versionados e nunca reinterpretam silenciosamente evento de outro projeto.
10. Mudança de schema, API, engine/provider capability, permission semantics, trace/event fields ou manifest de packaging bloqueia PR downstream até evidência de compatibilidade.

## 9. Recovery, cancellation and rollback

### 9.1 Execução local

- queues são bounded; admission, backpressure, timeout e cancellation produzem estados observáveis;
- `CancellationToken` é propagado por app/project/session/agent/workflow/node/tool/provider/Python;
- cancellation produz resultado terminal e libera leases; corrida cancel × completion resolve por `request_id`/generation, sem duplicar side effect;
- provider retry exige idempotency; tool/process retry é proibido quando o efeito pode ser desconhecido;
- engine/worker/plugin crash marca run afetada, preserva checkpoint e inicia recovery somente se policy e geração permitirem;
- nenhum formulário, commit, pagamento, publish, force push ou download parcial é reenviado automaticamente após crash;
- observabilidade registra recovery attempt, cause, previous generation, checkpoint digest e terminal outcome.

### 9.2 Workflows e scheduler

Workflow persiste cada transição relevante e checkpoint após nodes com efeito. Ao reiniciar, valida graph version, policy revision, checkpoint digest e leases; retoma somente node idempotente ou pede reconciliação/approval. Scheduler usa lease com generation/fencing; `missed-run-policy` é `skip`, `coalesce` ou `catch-up` bounded e versionado.

### 9.3 Dados e configuração

Rollback de skill/workflow/agent config troca o pointer para last-known-good e preserva a versão rejeitada para análise. Rollback de migration restaura backup/snapshot compatível ou aplica forward-fix; não executa SQL inverso não testado. Revogar provider/plugin/secret encerra sessões/leases dependentes e impede novas calls.

### 9.4 Aplicação e update

Shutdown: parar novos triggers e commands, resolver/recusar approvals, persistir sessão/workflow/scheduler, drenar filas bounded, fechar workers/sandboxes, liberar locks e gravar checkpoint. Se o timeout expirar, o app preserva last-known-good e informa estado incompleto.

Update: verificar assinatura e metadata antes de instalar, manter a versão anterior, fazer health/launch smoke pós-atualização e reverter para last-known-good em falha. Rollback não remove profile, secrets ou artifacts. Key/channel comprometidos exigem kill switch, revocation, freeze de publicação, release conhecida e invalidation de evidências antigas.

## 10. Distribuição, atualização e obrigações de segurança

Antes de qualquer canal alpha/beta/stable, cada artifact por OS/arch deve ter:

- build reprodutível o suficiente para o nível de risco, Cargo lock/dependency policy e origem/licença/advisory verificados;
- SBOM SPDX ou CycloneDX, checksums, provenance/attestation vinculada a repository, workflow, commit, tree e artifact digest;
- assinatura separada por canal/ambiente, chaves fora do repositório e nunca expostas a PR/fork, provider, plugin ou app;
- metadata assinada com versão, canal, OS/arch, hash, tamanho, min supported version, expiry, revocation e rollback/last-known-good;
- cliente verificando assinatura, hash, canal, downgrade, expiry e compatibility antes de substituir a instalação;
- instalador e sidecars com permissões mínimas, paths controlados, execução sem secrets herdados e clean-install/upgrade/uninstall smoke por OS;
- deep links/callbacks validados com state/nonce/PKCE quando aplicável; handler externo allowlisted e sem shell interpolation;
- auto-update fail-closed; metadata ausente/inválida não pode ser tratada como update disponível;
- security tests de secret scanning, dependency/advisory/license, action pinning, permissions mínimas, SAST, fuzz de manifests e compromise recovery;
- claims de sandbox/site isolation/secure renderer ligados a evidência por OS e versão. Sem engine host/process boundary comprovado, o produto permanece experimental/Beta.

Plugins, MCP servers, Python environments e provider adapters não entram automaticamente no instalador como trusted code. Cada distribuição registra versões/digests, capabilities, licença, owner e rollback/quarantine path.

## 11. Contratos que bloqueiam PRs posteriores

Uma PR posterior não pode ser mergeada se depender de contrato ausente, proposta sem evidência ou boundary não testado. A tabela é o gate de entrada do backlog:

| ID | Contrato obrigatório | Evidência mínima | Bloqueia |
|---|---|---|---|
| AB-001 | Manifesto de camadas e arestas | architecture graph validado contra metadata; fixture de edge proibida/ciclo | novo crate, dependência ou extração |
| AB-002 | Command/result/event envelope | schema, versioning, size/unknown rejection, identity/idempotency tests | UI/API/IPC, event bus, CLI |
| AB-003 | Project/profile isolation | negative cross-project queries, storage keys e artifact checks | memory, skills, agents, tools, remote |
| AB-004 | Permission/approval matrix | default-deny, scope/fingerprint/expiry/revoke e race tests | tool, workflow, scheduler, plugin, OAuth |
| AB-005 | Provider port | normalized request/stream/cost/capability/error contract + MockProvider | qualquer provider concreto/fallback |
| AB-006 | Tool/Sandbox contract | schema, capability, quota, timeout, cancellation, output limit e escape tests | filesystem, terminal, process, git, HTTP |
| AB-007 | Python worker protocol | JSON-RPC versionado, lifecycle/restart, no-Python boot, isolation tests | Python SDK/tool/dependency manager |
| AB-008 | Memory lifecycle | candidate approval, provenance, dedupe, retrieval budget e clear/isolation tests | long-term memory/embeddings/autolearning |
| AB-009 | Skill lifecycle | manifest, immutable versions, test/evaluator, activation e rollback | skill editor/creator/autoevolution |
| AB-010 | Agent policy | instruction precedence, autonomy, tool/memory/provider policy e budgets | agent builder/execution |
| AB-011 | Invocation graph | identity/access/depth/cycle/round/fanout/budget e provenance | group/delegation/parallel agents |
| AB-012 | Durable workflow | DAG schema, node contracts, checkpoint, crash recovery e idempotency | workflow editor/nodes/scheduler integration |
| AB-013 | Scheduler semantics | trigger/clock/DST/missed-run/lease/concurrency/notification tests | automation e background runs |
| AB-014 | Secrets/Auth | keychain/Stronghold, opaque handles, OAuth callback/revoke/rotation tests | provider connect, remote auth, plugins |
| AB-015 | Trace/event redaction | schema, correlation, sensitive golden tests, retention e sink failure | autonomous execution, diagnostics, telemetry |
| AB-016 | Migration/versioning | preflight, backup, transaction, interrupted migration/restore e compatibility | qualquer mudança de schema/contract |
| AB-017 | Plugin/MCP boundary | manifest/permission/transport/lifecycle/quarantine e no-transitive-capability tests | plugin registry, MCP client/server |
| AB-018 | Distribution/update | signed artifact, SBOM, provenance, verify, rollback e compromise drill | publishing, updater, stable claims |
| AB-019 | Threat model regression | cenários mapeados a controle/teste/evidence; status fail-closed | release gates e capability enablement |

Uma PR pode implementar somente documentação/fixture de um contrato; não pode marcar o contrato como aceito quando a implementação, teste real ou evidência de identity ainda está ausente. `planned`, `skipped`, `no-run`, `partial`, `stale` e `blocked` não equivalem a `pass`.

## 12. ADRs obrigatórios

Os ADRs abaixo devem ser criados ou atualizados antes das PRs que abrem cada boundary. Um ADR não é substituído por descrição em issue, prompt de agente, comentário ou teste de mock. Cada ADR deve conter contexto, decisão, alternativas, ownership, threat model, testes/evidence, migration impact, rollback, status (`proposed`/`accepted`/`superseded`) e condição de revisão.

| ADR | Tema e decisão mínima exigida | Gate sugerido |
|---|---|---|
| ADR-AB-001 | Layer map, crate/package graph, extraction policy e forbidden edges | Foundation/architecture validator |
| ADR-AB-002 | Application API, command/result/event envelopes, schema/versioning e idempotency | primeira API/IPC |
| ADR-AB-003 | Project/profile isolation, ownership de estado, tenancy e sharing grants | project/storage |
| ADR-AB-004 | SQLite/blob layout, transactions, locks, backup e migration/rollback | persistência |
| ADR-AB-005 | Provider-core, normalized model protocol, routing/fallback, cost e credential handles | provider system |
| ADR-AB-006 | Tool registry, capability taxonomy, approval policy, high-risk actions e audit | tools |
| ADR-AB-007 | Sandbox profiles, quotas, OS/container/remote boundary e claims permitidos | process execution |
| ADR-AB-008 | Python sidecar JSON-RPC, lifecycle, dependency environment e security posture | Python runtime |
| ADR-AB-009 | Memory taxonomy, candidate approval, retrieval/index, retention e deletion | memory |
| ADR-AB-010 | Skill manifest/lifecycle, global import, evaluator, autonomy e rollback | skills |
| ADR-AB-011 | Agent identity, instruction precedence, response profile, autonomy e budgets | agents |
| ADR-AB-012 | Multi-agent graph, delegation, moderator/round policy, synthesis e provenance | groups |
| ADR-AB-013 | Durable workflow DAG, node ABI, checkpoint, retry/idempotency e recovery | workflows |
| ADR-AB-014 | Scheduler clock/timezone, missed runs, leases, concurrency e notification | scheduler |
| ADR-AB-015 | Secrets backend, OAuth/deep-link, consent, rotation/revocation e backup exclusion | auth/providers |
| ADR-AB-016 | Trace/event schema, redaction, retention, crash bundles e telemetry opt-in | observability |
| ADR-AB-017 | Plugin/MCP manifest, transport/auth, permission mapping, lifecycle e quarantine | extensions |
| ADR-AB-018 | Tauri capability/CSP, frontend bridge e desktop packaging/deep links | desktop |
| ADR-AB-019 | Distribution, signing, SBOM, provenance, channels, updater e compromise recovery | release |
| ADR-AB-020 | Remote runtime/worker protocol, node identity, transport, credential isolation e revocation | remote phase |
| ADR-AB-021 | Consolidated threat model, trust levels, residual risks e release claims | every new trust boundary/release |

ADRs de browser/Servo/Tauri existentes no workspace hospedeiro devem ser reconciliados com estes contratos quando o produto desktop multiagente compartilhar infraestrutura. Um ADR proposto não pode ser usado como autoridade para liberar uma dependência downstream; a decisão permanece bloqueada até o gate de evidência correspondente.

## 13. Sequenciamento recomendado e critérios de aceitação

1. **Foundation:** AB-001/002/003/004/021, manifests de dependência, schemas, ownership e fixtures negativas.
2. **Single-agent:** AB-005/006/014/015/016, MockProvider, tool broker, secrets e trace redigido.
3. **Python/tools:** AB-007/008; core continua funcional sem Python.
4. **Skills/agents/multi-agent:** AB-009/010/011/012, com candidate/approval/rollback antes de qualquer autoevolução.
5. **Workflow/scheduler:** AB-013/014, persistência, recovery e leases antes de automations unattended.
6. **Plugin/MCP/remote:** AB-017/020, opt-in, sandbox/quarantine, auth e revocation; não fazem parte do trust baseline por default.
7. **Distribution:** AB-018/019; canais superiores ficam bloqueados até artifacts, signatures, SBOM, provenance e rollback drill.

Para cada etapa, o DoD mínimo é: contrato versionado; dependências permitidas/proibidas no manifest; owner; unit + integration + contract tests; negative/failure tests; trace/event evidence; impacto em migration/security/docs; rollback executável ou claramente bloqueado; resultado atrelado ao commit/tree/policy revision corretos.

## 14. Estado desta revisão

Este arquivo é o contrato consolidado de fronteiras para o planejamento derivado de `sdd-input.md`. Ele não afirma que qualquer camada, teste, migration, sandbox, assinatura, provider, plugin ou runtime já esteja implementado. Qualquer documento posterior que declarar “concluído”, “seguro”, “isolado”, “aprovado” ou “production-ready” deve apontar para a evidência específica e respeitar os bloqueadores acima.
