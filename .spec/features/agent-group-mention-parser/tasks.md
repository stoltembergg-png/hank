# Tasks: AgentGroup mention parser

> feature: agent-group-mention-parser

## T-881 — Implementar parser tipado sem side effect [concluida]

- Refs: US-876, AC-877, AC-878, AC-879
- Arquivos: `crates/agent-core/src/mention_parser.rs`, `crates/agent-core/src/lib.rs`, `crates/agent-core/tests/mention_parser_contract.rs`, `docs/agent-group-mention-parser.md`
- Notas: parser usa snapshot de membership e nunca cria invocation ou chama provider/tool.
