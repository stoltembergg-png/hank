# Tasks: Confirmation policies

> feature: confirmation-policies

## T-658 — Implementar artefato de aprovação bounded [concluida]

- Refs: US-617, AC-669, AC-672
- Arquivos: crates/tool-core/src/confirmation.rs, crates/tool-core/src/lib.rs, crates/tool-core/Cargo.toml, Cargo.lock
- Notas: hashes canônicos SHA-256, vínculo de projeto/agente/tool/schema/args/effect/budget/trace/actor e ausência de payload bruto.

## T-659 — Implementar ledger de ciclo de aprovação [concluida]

- Refs: US-617, AC-670, AC-671
- Arquivos: crates/tool-core/src/confirmation.rs
- Notas: registro bounded, aprovação serializável, expiração monotônica por timestamp injetado, revogação, ask_once scoped e consumo único de ask_every_time.

## T-660 — Cobrir contrato e evidência SDD [concluida]

- Refs: US-617, AC-669, AC-670, AC-671, AC-672
- Arquivos: crates/tool-core/tests/confirmation_contract.rs, .spec/verification/confirmation-policies.json
- Notas: testes determinísticos para binding, redaction, expiração, revogação, replay, isolamento e policy.

## T-661 — Integrar com PermissionEvaluator [concluida]

- Refs: US-617, AC-673
- Arquivos: crates/tool-core/src/permission.rs, crates/tool-core/tests/permission_contract.rs
- Notas: nova entrada runtime valida request/grant pelo ledger antes de liberar efeito sensível; a API booleana existente permanece compatível durante a migração.

## T-662 — Cobrir gate integrado [concluida]

- Refs: US-617, AC-673
- Arquivos: crates/tool-core/tests/permission_contract.rs, .spec/verification/confirmation-policies.json
- Notas: testes cobrem aprovação válida, request alterado, grant ausente e negação fail-closed.

## Próximo incremento

- Conectar o ledger à fronteira Application API/UI, preservando os mesmos
  bindings e emitindo eventos auditáveis sem payload bruto.
