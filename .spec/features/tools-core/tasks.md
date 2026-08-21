# Tarefas — tools-core

## T-389 — Adicionar tool trait e contrato (PR-096) [concluida]
- Refs: US-601, AC-601, AC-602, AC-603, AC-604, AC-605, AC-606, AC-607, AC-608, AC-609
- Arquivos: crates/tool-core/Cargo.toml, crates/tool-core/src/lib.rs, crates/tool-core/src/context.rs, crates/tool-core/src/error.rs, crates/tool-core/src/request.rs, crates/tool-core/src/response.rs, crates/tool-core/src/schema.rs, crates/tool-core/src/trait_def.rs, crates/tool-core/tests/trait_contract.rs, .github/workflows/onp-sdd-evidence.yml, Cargo.toml, crates/agent-protocol/src/ids.rs, crates/test-support/src/arch_fixtures_test.rs
- Notas: Tool trait async com execute/can_handle, ToolSchema com input/output JSON schema, ToolContext com project/agent/session/capability/policy/budget/trace, PolicyDecision Allow/AskOnce/AskEveryTime/Deny, ToolEnvironment Host/Sandbox/Python/Remote, ToolError taxonomy, 34 testes de contrato cobrindo validação, serialização, trait behavior, policy decisions, environment variants e error taxonomy

## T-390 — Adicionar spec tools-core [concluida]
- Refs: US-601, AC-601, AC-602, AC-603, AC-604, AC-605, AC-606, AC-607, AC-608, AC-609
- Arquivos: .spec/features/tools-core/spec.md, .spec/features/tools-core/tasks.md
- Notas: spec ONP completa com 9 critérios de aceite, 4 suposições confirmadas, 3 perguntas respondidas; fora de escopo ferramentas concretas, registry, permission evaluator, sandbox
