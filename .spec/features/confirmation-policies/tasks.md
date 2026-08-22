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

## T-663 — Expor confirmation service na Application API [concluida]

- Refs: US-617, AC-674
- Arquivos: crates/agent-runtime/Cargo.toml, crates/agent-runtime/src/confirmation_application.rs, crates/agent-runtime/src/lib.rs, docs/confirmation-application.md
- Notas: facade tipada para submit/approve/revoke/authorize; transporta somente ApprovalRequest/ApprovalGrant e delega as invariantes ao ConfirmationLedger.

## T-664 — Cobrir contrato da Application API [concluida]

- Refs: US-617, AC-674
- Arquivos: crates/agent-runtime/tests/confirmation_application_contract.rs, .spec/verification/confirmation-policies.json
- Notas: testes cobrem redaction, actor binding, revogação e replay.

## T-665 — Expor comandos tipados na ponte Tauri [concluida]

- Refs: US-618, AC-675
- Arquivos: apps/desktop/src-tauri/src/confirmations.rs, apps/desktop/src-tauri/src/main.rs, apps/desktop/src-tauri/Cargo.toml, .spec/features/tauri-desktop/spec.md, apps/desktop/src-tauri/tests/tauri_ac_tests.rs
- Notas: state gerenciado delega ao ConfirmationApplicationService; comandos submit/approve/revoke transportam somente artefato bounded; evento `request_submitted` emitido no contrato vigente com sequência monotônica; AC-104 atualizado para handlers limitados ao ciclo de confirmação.

## T-666 — Publicar contrato e cliente tipado no frontend [concluida]

- Refs: US-618, AC-675
- Arquivos: frontend/src/contracts/confirmation.ts, frontend/src/api/confirmations.ts
- Notas: guard `isConfirmationEvent` aceita somente schema vigente e request com chaves exatas do artefato bounded (rejeita payload bruto); cliente invoker injetado chama `submit/approve/revoke_confirmation_request`.

## T-667 — Renderizar card de confirmação acessível [concluida]

- Refs: US-618, AC-676
- Arquivos: frontend/src/chat/confirmation/ConfirmationCard.tsx, frontend/src/chat/confirmation/ConfirmationCard.css, frontend/tests/confirmation-bridge.test.tsx, .spec/verification/confirmation-policies.json
- Notas: card expõe metadados bounded com ações acessíveis de aprovar/revogar vinculando actor e momento apresentados; sem schema hash ou payload bruto.
