# Tasks: authenticated remote daemon

> feature: authenticated-remote-daemon

## T-1452 — Core de bootstrap autenticado e lease [concluida]

- Refs: US-1449, AC-1457, AC-1458, AC-1459
- Arquivos: Cargo.toml, crates/remote-core/Cargo.toml, crates/remote-core/src/lib.rs,
  crates/remote-core/tests/authenticated_daemon_contract.rs,
  crates/test-support/src/arch_fixtures_test.rs

## T-1453 — Auditoria redigida e evidência SDD [concluida]

- Refs: US-1449, AC-1460
- Arquivos: crates/remote-core/src/lib.rs, crates/remote-core/tests/authenticated_daemon_contract.rs,
  .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js,
  docs/authenticated-remote-daemon.md

## T-1454 — Negociação de protocolo, lease IDs e auditoria bounded [concluida]

- Refs: US-1449, AC-1457, AC-1458, AC-1459, AC-1460
- Adiciona: negociação de protocol revision no bootstrap (rejeita versão desconhecida),
  lease IDs para evitar que stale cleanup feche uma sessão substituta,
  auditoria bounded com MAX_AUDIT_EVENTS e rotação via VecDeque,
  audit de tentativas de bootstrap rejeitadas (authentication, authorization, protocol),
  campo `authenticated` nos eventos de auditoria.
- Arquivos: crates/remote-core/src/lib.rs, crates/remote-core/tests/authenticated_daemon_contract.rs,
  crates/tool-core/src/git_worktree.rs, crates/tool-core/tests/git_worktree_contract.rs

## Suposições

- ASM-1454: adapters concretos de auth/secret, socket, bind e dispatch remoto pertencem a cards posteriores.
