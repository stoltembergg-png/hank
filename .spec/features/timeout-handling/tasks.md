# Tasks: Tool timeout handling

> feature: timeout-handling
> status: implementada

## T-655 — Implementar janela comum de execução [concluida]

- Refs: US-616, AC-665, AC-666, AC-668
- Arquivos: crates/tool-core/src/timeout.rs, crates/tool-core/src/lib.rs
- Notas: deadline monotônico, remaining bounded, token compartilhado e terminalização idempotente com precedência de cancelamento.

## T-656 — Integrar adapters bounded e cleanup [concluida]

- Refs: US-616, AC-666, AC-667, AC-668
- Arquivos: crates/tool-core/src/process.rs, crates/tool-core/src/http.rs, crates/tool-core/src/filesystem_read.rs, crates/tool-core/src/filesystem_write.rs
- Notas: process usa a janela sem quebra de API; HTTP limita o client ao restante da janela; filesystem falha fechado antes do acesso e rollback de write quando a janela termina.

## T-657 — Cobrir contratos e evidência SDD [concluida]

- Refs: US-616, AC-665, AC-666, AC-667, AC-668
- Arquivos: crates/tool-core/tests/timeout_contract.rs, crates/tool-core/tests/http_contract.rs, crates/tool-core/tests/filesystem_read_contract.rs, crates/tool-core/tests/filesystem_write_contract.rs, .spec/verification/timeout-handling.json
- Notas: testes determinísticos para deadline, corrida cancel/timeout, cancelamento compartilhado, pre-dispatch fail-closed e rollback sem mutação persistida.
