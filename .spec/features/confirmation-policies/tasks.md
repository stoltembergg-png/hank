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

## T-660 — Cobrir contrato e evidência SDD [em-andamento]

- Refs: US-617, AC-669, AC-670, AC-671, AC-672
- Arquivos: crates/tool-core/tests/confirmation_contract.rs, .spec/verification/confirmation-policies.json
- Notas: testes determinísticos para binding, redaction, expiração, revogação, replay, isolamento e policy.

## Próximo incremento

- Conectar o ledger à fronteira Application API/UI e ao `PermissionEvaluator`,
  preservando os mesmos bindings e emitindo eventos auditáveis sem payload bruto.
