# Tasks: Foundation workspace

> feature: foundation-workspace

<!--
  Como ler este arquivo (o formato é verificado por `onp-spec audit`):
  - T-xxx = tarefa (código de rastreio, único no projeto inteiro).
  - Toda tarefa referencia em `Refs:` pelo menos uma história de usuário
    (US-xxx) ou critério de aceite (AC-xxx).
  - Toda tarefa lista os arquivos que cria/altera em `Arquivos:` — capriche:
    é o que decide o que `onp-spec plano` roda em PARALELO (arquivos
    disjuntos) e o que roda em sequência.
  - Campos opcionais por tarefa, usados pelo plano de execução:
    `- Modelo: claude-sonnet-5` e `- Esforço: alto` (baixo|medio|alto|xalto|max).
  - Uma tarefa só pode virar [concluida] quando os critérios de aceite dela
    tiverem prova PASS registrada por `onp-spec verify`.
  Status: pendente | em-andamento | concluida
    (atalho: `onp-spec tarefa <feature> <T-xxx> <status>`)
-->

## T-301 — Criar Cargo.toml raiz do workspace [concluida]
- Refs: US-301, AC-301, AC-302, AC-305
- Arquivos: Cargo.toml, rust-toolchain.toml
- Notas: Workspace com resolver="2", members apontando para crates/, workspace.dependencies e workspace.lints básicos

## T-302 — Criar crate agent-core (domínio puro) [concluida]
- Refs: US-301, AC-301, AC-303
- Arquivos: crates/agent-core/Cargo.toml, crates/agent-core/src/lib.rs, crates/agent-core/src/agent.rs, crates/agent-core/src/budget.rs, crates/agent-core/src/error.rs, crates/agent-core/src/memory.rs, crates/agent-core/src/project.rs, crates/agent-core/src/session.rs, crates/agent-core/src/skill.rs, crates/agent-core/src/workflow.rs, docs/project-aggregate.md
- Notas: Sem dependências de tauri, tao, wry, sqlx, tokio, providers concretos; apenas agent-protocol e std

## T-303 — Criar crate agent-runtime (execution/durable) [concluida]
- Refs: US-301, AC-301, AC-303
- Arquivos: crates/agent-runtime/Cargo.toml, crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/memory.rs, crates/agent-runtime/src/provider.rs, crates/agent-runtime/src/python.rs, crates/agent-runtime/src/sandbox.rs, crates/agent-runtime/src/scheduler.rs, crates/agent-runtime/src/skill_runtime.rs, crates/agent-runtime/src/tool.rs, crates/agent-runtime/src/workflow_runtime.rs
- Notas: Dependência permitida: agent-core, agent-protocol; pode usar tokio, sqlx aqui

## T-304 — Criar crate agent-protocol (tipos estáveis/serialização) [concluida]
- Refs: US-301, AC-301, AC-303
- Arquivos: crates/agent-protocol/Cargo.toml, crates/agent-protocol/src/lib.rs, crates/agent-protocol/src/capability.rs, crates/agent-protocol/src/envelope.rs, crates/agent-protocol/src/ids.rs, crates/agent-protocol/src/policy.rs, crates/agent-protocol/src/version.rs
- Notas: Crate raiz das fronteiras; apenas serde, thiserror, tipos estáveis; sem runtime

## T-305 — Criar crate test-support (dev-only, fixtures de arquitetura) [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/test-support/Cargo.toml, crates/test-support/src/lib.rs, crates/test-support/src/arch_fixtures_test.rs
- Notas: Dev-only; exporta forbidden-import test, cycle detection test; não entra em produção

## T-306 — Criar crate xtask (automação de build/CI) [concluida]
- Refs: US-301, AC-301, AC-305
- Arquivos: crates/xtask/Cargo.toml, crates/xtask/src/main.rs
- Notas: Binária; usa cargo-metadata, clap; roda fora do workspace de produto

## T-307 — Adicionar testes de arquitetura (forbidden-import + cycle detection) [concluida]
- Refs: US-301, AC-303, AC-304
- Arquivos: crates/test-support/src/arch_fixtures_test.rs
- Notas: Testes cobrindo AC-301..305: build, metadata, forbidden imports, cycle detection, resolver/toolchain

## T-308 — Verificar build limpo e metadata correta [concluida]
- Refs: US-301, AC-301, AC-302, AC-305
- Arquivos: Cargo.toml, rust-toolchain.toml, Cargo.lock
- Notas: Comando de validação; tarefa passa se ambos saírem com exit code 0

## T-309 — Adicionar framework determinístico de fixtures [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/test-support/src/lib.rs, crates/test-support/src/fixtures.rs, docs/fixtures.md
- Notas: Fixtures dev-only, sintéticas, offline, bounded, determinísticas por seed/hash e com cleanup obrigatório

## T-310 — Adicionar contrato de eventos de aplicação [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-protocol/src/events.rs, crates/agent-protocol/src/ids.rs, crates/agent-protocol/src/lib.rs, docs/application-events.md
- Notas: Envelope versionado, project-scoped, bounded e com rejeição de versão/sequence/payload inválidos

## T-329 — Validar Agent aggregate e project binding [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/agent.rs, docs/agent-aggregate.md
- Notas: Agent domain-only, project-bound, lifecycle explícito e limites de identidade/personality

## T-330 — Adicionar Agent repository SQLite [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/agent_repo.rs, crates/agent-runtime/src/lib.rs, docs/agent-repository.md
- Notas: CRUD project-scoped sobre schema migrado, queries parametrizadas, limites e mapping de erros

## T-341 — Adicionar Agent CRUD services [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/agent_service.rs, crates/agent-runtime/src/lib.rs, docs/agent-crud-services.md
- Notas: Application DTOs, validation, CRUD, Project binding, event publication, optimistic version e policy composition

## T-331 — Adicionar Agent configuration schema [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/config.rs, crates/agent-core/src/lib.rs, docs/agent-config.md
- Notas: Envelope versionado, IDs obrigatórios, defaults determinísticos, deny unknown fields e limites bounded

## T-334 — Adicionar tool permission schema [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/tool_permissions.rs, crates/agent-core/src/lib.rs, docs/tool-permissions.md
- Notas: allow/ask/deny, Project/Agent/Session scope, default deny, expiry, conflitos e privileged wildcard rejection

## T-335 — Adicionar model policy schema [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-protocol/src/policy.rs, docs/model-policy.md
- Notas: Identificadores provider-neutral, modalities explícitas, limites bounded, fallback depth e rejeição de secrets/endpoints

## T-336 — Adicionar autonomy policy L0-L4 [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/autonomy.rs, crates/agent-core/src/lib.rs, docs/autonomy-policy.md
- Notas: Níveis L0-L4, evaluation matrix, approval/expiry, downgrade permitido e autoelevação rejeitada

## T-337 — Adicionar budget policy multi-scope [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/budget.rs, crates/agent-core/src/lib.rs, docs/budget-policy.md
- Notas: Limites Project/Agent/Session/Workflow/Task, reservation lifecycle, reset e overflow fail-closed

## T-332 — Adicionar personality schema [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/agent.rs, docs/personality-schema.md
- Notas: Personality bounded, deny unknown fields, rejeição de secrets/instruction override e validação independente

## T-333 — Adicionar instruction hierarchy contract [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-protocol/src/policy.rs, docs/instruction-hierarchy.md
- Notas: Ordem determinística, sources únicas, security immutable, size budget e validação fail-closed

## T-311 — Implementar event bus bounded [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/event_bus.rs, crates/agent-runtime/src/lib.rs, docs/event-bus.md
- Notas: Bus tipado com FIFO, backpressure/lag explícito, fechamento determinístico e sem fila ilimitada

## T-312 — Adicionar armazenamento SQLite transacional [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/sqlite.rs, crates/agent-runtime/src/lib.rs, docs/sqlite-storage.md
- Notas: Conexão transacional SQLx/Tokio com WAL mode, foreign keys, validação de path traversal e sem acesso direto do frontend

## T-313 — Adicionar migrações SQL de schema inicial [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: migrations/0001_initial_schema.sql, crates/agent-runtime/src/migrations.rs, crates/agent-runtime/src/lib.rs, docs/migrations.md
- Notas: Execução transacional de migrações embutidas, criação de tabelas projects/agents/sessions/messages e integridade de foreign keys

## T-314 — Adicionar fixtures determinísticas de IDs tipados [concluida]
- Refs: US-301, AC-303
- Arquivos: crates/test-support/src/ids.rs, crates/test-support/src/lib.rs, docs/ids.md
- Notas: IDs tipados determinísticos por seed para testes reproduzíveis; catálogo de IDs e regra de não usar strings em contratos

## T-315 — Implementar repositório SQLite para Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/project_repo.rs, crates/agent-runtime/src/lib.rs, docs/project-repository.md
- Notas: Implementação transacional do port ProjectRepository usando queries parametrizadas, paginação e mapeamento de DomainError

## T-316 — Criar serviço de aplicação para criar Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/project_service.rs, crates/agent-runtime/src/lib.rs, docs/create-project-service.md
- Notas: Use case de criação de projetos com validação de entrada, persistência transacional e publicação do evento ProjectCreated

## T-317 — Criar serviço de aplicação para listar/buscar Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/project_query_service.rs, crates/agent-runtime/src/lib.rs, docs/list-project-service.md
- Notas: Query use case para listagem paginada (com limites restritos 1..100) e recuperação de projetos por ID

## T-318 — Criar serviço de aplicação para atualizar Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/project_update_service.rs, crates/agent-runtime/src/lib.rs, docs/update-project-service.md
- Notas: Use case de atualização de projeto com concorrência otimista, bloqueio em arquivados e emissão do evento ProjectUpdated

## T-319 — Criar serviço de aplicação para arquivar Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/project_archive_service.rs, crates/agent-runtime/src/lib.rs, docs/archive-project-service.md
- Notas: Use case de arquivamento seguro e idempotente com emissão do evento ProjectArchived

## T-320 — Vincular pastas locais a um Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: migrations/0002_project_folders.sql, crates/agent-core/src/project.rs, crates/agent-runtime/src/project_repo.rs, docs/project-folders.md
- Notas: Suporte a vínculos Project->Folder no banco SQLite e no aggregate Project com prevenção de path traversal e unicidade

## T-321 — Vincular repositórios Git a um Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: migrations/0003_project_repositories.sql, crates/agent-core/src/project.rs, crates/agent-runtime/src/project_repo.rs, docs/project-repositories.md
- Notas: Registro e persistência segura de repositórios Git vinculados ao Project com validação de URL e sem execução Git

## T-322 — Adicionar settings de Project [pendente]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/project.rs, crates/agent-runtime/src/project_repo.rs, docs/project-settings.md
- Notas: Modelagem, validação de limites de retenção/agentes e persistência dedicada de ProjectSettings

## T-349 — Definir trait ModelProvider provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-core/Cargo.toml, crates/provider-core/src/lib.rs, crates/provider-core/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/provider-core.md
- Notas: Trait object-safe para complete/stream/list/health, IDs opacos, credential ref redacted, cancellation/backpressure typed, MockProvider offline e nenhum SDK/concrete provider

## T-350 — Definir schema de capabilities de modelo [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/capabilities.rs, crates/provider-core/src/lib.rs, crates/provider-core/tests/capability_contract.rs, docs/model-capabilities.md
- Notas: supported/unsupported/unknown explícitos, limites bounded, comparação determinística e rejeição antes do adapter

## T-351 — Definir normalized request provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/request.rs, crates/provider-core/src/lib.rs, crates/provider-core/tests/request_contract.rs, docs/normalized-request.md
- Notas: envelope versionado com identity/project scope, roles/messages bounded, capabilities, tool metadata, budget, cancellation e redacted summary

## T-352 — Definir normalized response provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/response.rs, crates/provider-core/src/lib.rs, crates/provider-core/tests/response_contract.rs, docs/normalized-response.md
- Notas: status/finish forward-compatible, output parts bounded, usage/cost optional, error taxonomy redacted e summary observável sem payload bruto

## T-353 — Definir eventos de streaming provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/stream.rs, crates/provider-core/src/lib.rs, crates/provider-core/tests/stream_contract.rs, docs/streaming-events.md
- Notas: envelope start/delta/tool/usage/finish/error/cancel, sequence/generation, terminalidade única, buffer bounded, backpressure e rejeição fail-closed

## T-354 — Implementar adapter OpenAI-compatible isolado [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-adapters/openai-compatible/Cargo.toml, crates/provider-adapters/openai-compatible/src/lib.rs, crates/provider-adapters/openai-compatible/tests/adapter_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/openai-compatible-adapter.md
- Notas: transport injetável offline, endpoint HTTPS bounded, credential ref opaca, mapping complete/stream/error, timeout/cancel, sem retry implícito e sem execução de tools

## T-355 — Adicionar descriptor/provider OpenAI provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-adapters/openai/Cargo.toml, crates/provider-adapters/openai/src/lib.rs, crates/provider-adapters/openai/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/openai-provider.md
- Notas: provider ID/model mapping/capabilities determinísticos, validação pré-adapter, wrapper sobre compatível, credential/endpoint opacos e sem discovery/OAuth/UI

## T-356 — Adicionar adapter/descriptor Anthropic e transport boundary compartilhado [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-core/src/transport.rs, crates/provider-core/src/lib.rs, crates/provider-adapters/openai-compatible/src/lib.rs, crates/provider-adapters/anthropic/Cargo.toml, crates/provider-adapters/anthropic/src/lib.rs, crates/provider-adapters/anthropic/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/anthropic-provider.md
- Notas: transport HTTPS/credential/backpressure compartilhado em provider-core, Anthropic content/stop/stream mapping, capabilities explícitas, timeout/cancel, sem discovery/OAuth/UI e sem secrets

## T-357 — Adicionar adapter/descriptor Gemini provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-adapters/gemini/Cargo.toml, crates/provider-adapters/gemini/src/lib.rs, crates/provider-adapters/gemini/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/gemini-provider.md
- Notas: contents/parts/generationConfig/candidates mapping, streamGenerateContent, capabilities/limits explícitos, endpoint/credential opacos, timeout/cancel e sem SDK/discovery/UI

## T-358 — Adicionar adapter/descriptor OpenRouter sem fallback implícito [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-adapters/openrouter/Cargo.toml, crates/provider-adapters/openrouter/src/lib.rs, crates/provider-adapters/openrouter/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/openrouter-provider.md
- Notas: rota lógica/upstream determinística, identity preservation, capability validation, upstream error explícito, endpoint/credential bounded e sem fallback/policy bypass

## T-359 — Adicionar adapter/descriptor Ollama com validação de endpoint local [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/provider-adapters/ollama/Cargo.toml, crates/provider-adapters/ollama/src/lib.rs, crates/provider-adapters/ollama/tests/provider_contract.rs, crates/test-support/src/arch_fixtures_test.rs, docs/ollama-provider.md
- Notas: messages/options mapping, localhost endpoint allowlist, capabilities/limits explícitos, sem process launch/shell/install, timeout/cancel, endpoint HTTPS validated

## T-360 — Implementar provider registry provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/lib.rs, crates/provider-core/src/registry.rs, crates/provider-core/tests/provider_contract.rs, crates/provider-core/tests/registry_contract.rs, docs/provider-registry.md, onpspec.config.json, .spec/features/foundation-workspace/tasks.md
- Notas: registro determinístico, duplicate/lookup, enable/disable, capability filter, sealing fail-closed, thread-safety observável, canonical CapabilityReport e sem credential/fallback/plugin/UI

## T-361 — Adicionar credential service provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/lib.rs, crates/provider-core/src/credentials.rs, crates/provider-core/tests/credentials_contract.rs, docs/credential-service.md, .spec/features/foundation-workspace/tasks.md
- Notas: opaque CredentialRef, project/account authorization, connect/disconnect/revoke/resolve, unavailable/cancelled states, bounded metadata, redaction, in-memory fixture sem persistência ou plaintext

## T-362 — Adicionar encrypted secret storage boundary [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/secrets-core/Cargo.toml, crates/secrets-core/src/lib.rs, crates/secrets-core/tests/secret_store_contract.rs, docs/encrypted-secret-storage.md, .spec/features/foundation-workspace/tasks.md
- Notas: SecureSecretBackend injetável para OS keychain/Stronghold, SecretMaterial bounded/zeroized, put/get/delete/rotate, account/scope binding, unavailable fail-closed, mock backend somente em testes e sem plaintext fallback

## T-363 — Adicionar OAuth framework provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: Cargo.toml, Cargo.lock, crates/auth-core/Cargo.toml, crates/auth-core/src/lib.rs, crates/auth-core/tests/oauth_contract.rs, docs/oauth-framework.md, .spec/features/foundation-workspace/tasks.md
- Notas: state/PKCE S256/redirect exactness, one-shot flow, replay/expiry/cancel/capacity fail-closed, TokenExchangeBackend handoff de CredentialRef, sem client secrets/token storage/UI/provider endpoint

## T-364 — Adicionar tratamento de OAuth callback provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/auth-core/src/lib.rs, crates/auth-core/src/callback.rs, crates/auth-core/tests/callback_contract.rs, docs/oauth-callback.md, .spec/features/foundation-workspace/tasks.md
- Notas: deep-link allowlist hank://oauth/callback, parser bounded, provider/account/project binding, state/replay/timeout/cancel validation, opaque CredentialRef result e sem token logs/open redirect/UI command

## T-365 — Adicionar provider settings UI service-only [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: frontend/src/api/provider-settings.ts, frontend/src/providers/settings/ProviderSettingsPage.tsx, frontend/src/providers/settings/ProviderSettingsPage.css, frontend/tests/provider_settings_ac_tests.test.tsx, docs/provider-settings-ui.md, .spec/features/foundation-workspace/tasks.md
- Notas: typed bridge/service intents, status/pending/revoked/unavailable states, OAuth stale callback handling, accessible UI, no secret/code/token DOM or browser storage

## T-366 — Adicionar model discovery service [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/lib.rs, crates/provider-core/src/discovery.rs, crates/provider-core/tests/discovery_contract.rs, docs/model-discovery.md, .spec/features/foundation-workspace/tasks.md
- Notas: registry/credential integration, normalized CapabilityReport records, requirements fail-closed, pagination bounded, metadata-only cache/invalidation, unavailable/expired/cancelled handling e sem raw provider payload

## T-367 — Adicionar model selector provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: frontend/src/api/model-selector.ts, frontend/src/providers/model-selector/ModelSelectorPage.tsx, frontend/src/providers/model-selector/ModelSelectorPage.css, frontend/tests/model_selector_ac_tests.test.tsx, docs/model-selector.md, .spec/features/foundation-workspace/tasks.md
- Notas: discovery/policy service-only, capability filtering supported-only, disabled/expired/unknown reasons, stale conflict preservation, bounded identifiers, accessible radios, no secret/token/endpoint DOM ou fallback

## T-368 — Adicionar provider health check service [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/lib.rs, crates/provider-core/src/health.rs, crates/provider-core/tests/health_contract.rs, docs/provider-health.md, .spec/features/foundation-workspace/tasks.md
- Notas: project/account credential scope, registry enabled guard, DefaultHealthProbe via ModelProvider::health, stable status/reason taxonomy, timeout policy, async cancellation, rate/debounce metadata cache, latency/evidence and redaction

## T-369 — Definir política de fallback provider-neutral [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/provider-core/src/lib.rs, crates/provider-core/src/fallback.rs, crates/provider-core/tests/fallback_contract.rs, docs/fallback-policy.md, .spec/features/foundation-workspace/tasks.md
- Notas: pure decision engine, retryable matrix 429/timeout/outage/quota, non-retryable auth/invalid/policy termination, deterministic healthy/capability/scope filtering, bounded attempts/tokens/cost, cancellation and stream attempt identity

## T-370 — Adicionar provider application/invocation service [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/Cargo.toml, crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/provider_service.rs, crates/agent-runtime/tests/provider_application_contract.rs, docs/provider-application-service.md, .spec/features/foundation-workspace/tasks.md
- Notas: única fachada runtime provider-neutral, registry/credential/capability checks, complete/stream DTOs, fallback orchestration, cancellation, terminal stream, attempt identity, no adapter import/type leakage or secret payload

## T-371 — Adicionar Session entity [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/session.rs, crates/agent-core/tests/session_contract.rs, docs/session-entity.md, .spec/features/foundation-workspace/tasks.md
- Notas: project/agent binding, correlation/schema version, Created/Active/Closing/Closed/Failed lifecycle, terminal/idempotent close, project-scoped participants, bounded metadata/budget/trace refs, no prompt/secret/provider types

## T-372 — Adicionar Message entity [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-core/src/session.rs, crates/agent-core/tests/message_contract.rs, docs/message-entity.md, .spec/features/foundation-workspace/tasks.md
- Notas: role/provenance/untrusted parts, session binding, generation/sequence ordering, duplicate/stale/out-of-order rejection, bounded content, terminal message states and no tool execution/provider payload

## T-373 — Adicionar Session storage [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: migrations/0004_session_storage.sql, crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/session_repo.rs, crates/agent-runtime/tests/session_storage_contract.rs, docs/session-storage.md, .spec/features/foundation-workspace/tasks.md
- Notas: migration forward extension/version safety, SQLite repository create/get/list/update/close, project scope/FK, bounded pagination, optimistic updated_at conflict, atomic no-overwrite semantics and no Message/prompt/credential storage

## T-374 — Adicionar Message storage [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: migrations/0005_message_storage.sql, crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/message_repo.rs, crates/agent-runtime/tests/message_storage_contract.rs, docs/message-storage.md, .spec/features/foundation-workspace/tasks.md
- Notas: migration ordering/provenance/status/parts, explicit project/session append scope, unique idempotency/order index, duplicate/stale/out-of-order rejection, partial recovery, terminal update and bounded pagination

## T-375 — Adicionar context builder interface [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/context.rs, crates/agent-runtime/tests/context_contract.rs, docs/context-builder.md, .spec/features/foundation-workspace/tasks.md
- Notas: deterministic source precedence, bounded token/source/content budgets, missing/duplicate/sensitive omissions, untrusted provenance, tool metadata-only, cancellation and no retrieval/provider/tool execution

## T-376 — Adicionar basic context builder [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/context.rs, crates/agent-runtime/src/context/basic.rs, crates/agent-runtime/tests/basic_context_contract.rs, docs/basic-context-builder.md, .spec/features/foundation-workspace/tasks.md
- Notas: concrete deterministic layer assembly over bounded inputs, conversation window, budget/truncation omissions, layer-kind fail-closed validation, task/tool metadata boundaries and no direct storage/provider/UI access

## T-377 — Adicionar Agent execution state machine [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/execution/mod.rs, crates/agent-runtime/tests/execution_contract.rs, docs/execution-state-machine.md, .spec/features/foundation-workspace/tasks.md
- Notas: Preparing/Running/Streaming/Completed/Failed/Cancelled states, exactly-one terminal transition, provider application coordinator, generation/invocation fences, bounded budgets/concurrency, cancellation and snapshot recovery

## T-378 — Adicionar provider streaming [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/streaming.rs, crates/agent-runtime/tests/streaming_contract.rs, docs/provider-streaming.md, .spec/features/foundation-workspace/tasks.md
- Notas: normalized stream consumer over ProviderApplicationService, sequence/generation fencing, delta/message updates, terminal mapping, cancellation, backpressure-ready bounded events and redacted failures

## T-379 — Adicionar cancellation boundary [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/cancellation.rs, crates/agent-runtime/tests/cancellation_contract.rs, docs/cancellation-boundary.md, .spec/features/foundation-workspace/tasks.md
- Notas: bounded token registry, idempotent cancel/unregister, synchronized Execution/Message terminal cancellation, completion race semantics, concurrency safety and no process/provider-specific kill

## T-380 — Adicionar retry policy [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/retry.rs, crates/agent-runtime/tests/retry_contract.rs, docs/retry-policy.md, .spec/features/foundation-workspace/tasks.md
- Notas: pure transient-error matrix, bounded deterministic backoff/jitter contract, max attempts/token/cancellation budget, attempt identity and no retry for auth/invalid/cancel/tool/destructive operations

## T-381 — Adicionar Session application service [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/session_service.rs, crates/agent-runtime/tests/session_service_contract.rs, docs/session-service.md, .spec/features/foundation-workspace/tasks.md
- Notas: project/agent/session lifecycle authorization, user/assistant persistence, injected provider application invoker, Execution orchestration, success/failure/cancel terminal results, bounded concurrency and no UI/adapter direct access

## T-382 — Adicionar typed chat command [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-protocol/src/lib.rs, crates/agent-protocol/src/chat_command.rs, crates/agent-protocol/tests/chat_command_contract.rs, crates/agent-runtime/src/lib.rs, crates/agent-runtime/src/chat_command.rs, docs/chat-command.md, .spec/features/foundation-workspace/tasks.md
- Notas: versioned bounded command envelope, typed caller/project/agent/session identity, generation/cancellation, dedupe/stale registry and thin injected runtime dispatcher without Tauri generic invoke or provider/storage access

## T-383 — Adicionar Tauri streaming event bridge [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: crates/agent-protocol/src/lib.rs, crates/agent-protocol/src/chat_stream.rs, crates/agent-protocol/tests/chat_stream_contract.rs, apps/desktop/src-tauri/Cargo.toml, apps/desktop/src-tauri/Cargo.lock, apps/desktop/src-tauri/src/main.rs, apps/desktop/src-tauri/src/streaming.rs, frontend/src/contracts/chat-stream.ts, frontend/tests/chat-stream-contract.test.ts, docs/tauri-streaming.md, .spec/features/foundation-workspace/tasks.md
- Notas: authorized WebviewWindow event sink, typed stream identity/generation/sequence, atomic validation and queueing, bounded backpressure with terminal preservation, sink retry retention and frontend consumer contract; no generic invoke/provider/storage/UI implementation

## T-384 — Adicionar single-agent chat UI [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: frontend/src/chat/ChatPage.tsx, frontend/src/chat/ChatPage.css, frontend/tests/chat-page.test.tsx, docs/chat-ui.md, .spec/features/foundation-workspace/tasks.md
- Notas: injected ChatTransport, scoped typed command, bounded accessible message list, ordered assistant deltas, cancel/error/retry states and stale/foreign event isolation; no direct storage/provider/Tauri implementation

## T-385 — Adicionar safe Markdown renderer [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: frontend/src/chat/markdown/SafeMarkdown.tsx, frontend/src/chat/markdown/SafeMarkdown.css, frontend/src/chat/ChatPage.tsx, frontend/tests/safe-markdown.test.tsx, docs/safe-markdown.md, .spec/features/foundation-workspace/tasks.md
- Notas: bounded deterministic Markdown subset, escaped raw HTML, http/https-only external links, plain fallback for unsafe schemes, no arbitrary HTML/JS and ChatPage integration; fenced code blocks remain PR-093 scope

## T-386 — Adicionar safe code-block renderer [concluida]
- Refs: US-301, AC-301, AC-303, AC-304
- Arquivos: frontend/src/chat/code-block/CodeBlock.tsx, frontend/src/chat/code-block/CodeBlock.css, frontend/src/chat/markdown/SafeMarkdown.tsx, frontend/tests/code-block.test.tsx, frontend/tests/safe-markdown.test.tsx, docs/code-block.md, .spec/features/foundation-workspace/tasks.md
- Notas: fenced code integration, language allowlist, escaped plain text, ANSI/control sanitization, bounded block, explicit clipboard gesture/status and no execution/autolink/file/shell capability
