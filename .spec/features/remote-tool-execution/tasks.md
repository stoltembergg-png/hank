# Tasks: remote tool execution

> feature: remote-tool-execution

## T-1510 — Amarrar dispatch à lease e à política bounded [concluida]

- Refs: US-3001, AC-3007, AC-3008, AC-3009, AC-3010, AC-3011, AC-3012, AC-3013, AC-3014
- Arquivos: `crates/remote-core/src/lib.rs`, `crates/remote-core/src/tool_dispatch.rs`, `crates/remote-core/Cargo.toml`, `crates/tool-core/src/response.rs`, `Cargo.lock`
- Notas: adiciona contexto de lease, allowlist de tool/capability, limites de timeout/payload,
  ledger com fingerprint por `OperationKey`, DTO remoto mínimo, cancelamento e estado
  `UnknownOutcome`; nenhum transport real é criado.

## T-1511 — Provar execução, falha, cancelamento e redaction com fixture offline [concluida]

- Refs: US-3001, AC-3007, AC-3008, AC-3009, AC-3010, AC-3011, AC-3012, AC-3013, AC-3014
- Arquivos: `crates/remote-core/tests/remote_tool_dispatch_contract.rs`, `test/remote-tool-execution-onp.test.mjs`
- Notas: fixture sintética não usa rede, shell, provider real ou credencial real; resposta sensível é rejeitada fail-closed.

## T-1512 — Registrar fronteira de produção e executar verify explícito [concluida]

- Refs: US-3001, AC-3007, AC-3010, AC-3014
- Arquivos: `.spec/features/remote-tool-execution/spec.md`, `.spec/features/remote-tool-execution/tasks.md`, `.github/workflows/onp-sdd-evidence.yml`, `onpspec.config.json`, `docs/remote-tool-execution.md`
- Notas: `CONTRACT PASS` não é prova de runtime remoto operacional; adapter de produção permanece posterior.

## Suposições

- ASM-2105 está registrada na especificação: o adapter de transporte autenticado é trabalho posterior.
