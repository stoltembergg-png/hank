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
