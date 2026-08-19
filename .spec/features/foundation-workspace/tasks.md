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






