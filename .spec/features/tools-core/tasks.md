# Tarefas — tools-core

## T-389 — Adicionar tool trait e contrato (PR-096) [concluida]
- Refs: US-601, AC-601, AC-602, AC-603, AC-604, AC-605, AC-606, AC-607, AC-608, AC-609
- Arquivos: crates/tool-core/Cargo.toml, crates/tool-core/src/lib.rs, crates/tool-core/src/context.rs, crates/tool-core/src/error.rs, crates/tool-core/src/request.rs, crates/tool-core/src/response.rs, crates/tool-core/src/schema.rs, crates/tool-core/src/trait_def.rs, crates/tool-core/tests/trait_contract.rs, .github/workflows/onp-sdd-evidence.yml, Cargo.toml, crates/agent-protocol/src/ids.rs, crates/test-support/src/arch_fixtures_test.rs
- Notas: Tool trait async com execute/can_handle, ToolSchema com input/output JSON schema, ToolContext com project/agent/session/capability/policy/budget/trace, PolicyDecision Allow/AskOnce/AskEveryTime/Deny, ToolEnvironment Host/Sandbox/Python/Remote, ToolError taxonomy, 34 testes de contrato cobrindo validação, serialização, trait behavior, policy decisions, environment variants e error taxonomy

## T-390 — Adicionar spec tools-core [concluida]
- Refs: US-601, AC-601, AC-602, AC-603, AC-604, AC-605, AC-606, AC-607, AC-608, AC-609
- Arquivos: .spec/features/tools-core/spec.md, .spec/features/tools-core/tasks.md
- Notas: spec ONP completa com 9 critérios de aceite, 4 suposições confirmadas, 3 perguntas respondidas; fora de escopo ferramentas concretas, registry, permission evaluator, sandbox

## T-391 — Implementar validação semântica e payload do ToolSchema (PR-097) [concluida]
- Refs: US-601, AC-610, AC-611, AC-612, AC-613, AC-614, AC-615
- Arquivos: crates/tool-core/src/schema.rs, crates/tool-core/tests/schema_contract.rs, crates/tool-core/Cargo.toml, Cargo.lock, docs/tool-schema.md, .spec/features/tools-core/spec.md, .spec/features/tools-core/tasks.md
- Notas: validação bounded de schema/payload, policy strict/permissive para unknown fields, compatibilidade semver explícita, rejeição fail-closed de shape/sensitive metadata e 10 contract tests

## T-392 — Implementar Tool Registry determinístico e isolado (PR-098) [concluida]
- Refs: US-601, AC-616, AC-617, AC-618, AC-619, AC-620, AC-621, AC-622, AC-623
- Arquivos: crates/tool-core/src/registry.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/registry_contract.rs, docs/tool-registry.md, .spec/features/tools-core/spec.md, .spec/features/tools-core/tasks.md
- Notas: registry bounded com RwLock/BTreeMap, key por name/version/scope, lifecycle, origem autorizada, lookup project-first/global, capability filter, unregister/restore, seal e concorrência; registrar não executa handler
