# Tasks: resource limiting

> feature: resource-limiting

## T-2010 — Implementar ledger de reserva multidimensional [concluida]

- Refs: US-2010, AC-2011, AC-2012, AC-2013, AC-2014, AC-2015
- Arquivos: `crates/agent-core/src/resource.rs`, `crates/agent-core/src/lib.rs`
- Escopo: quota/demand bounded, scopes tipados, reserva atômica, release e reap de timeout.
- Evidência: teste focal 5/5; `cargo test -p agent-core` e Clippy do crate passaram.

## T-2011 — Contratos de isolamento e recuperação [concluida]

- Refs: AC-2012, AC-2013, AC-2014, AC-2015
- Arquivo: `crates/agent-core/tests/resource_limit_contract.rs`
- Evidência: atomicidade multidimensional com falha posterior em `node`, project isolation,
  monotonicidade, capacity e timeout.

## T-2012 — Documentação e evidência ONP [concluida]

- Refs: US-2010, AC-2011, AC-2012, AC-2013, AC-2014, AC-2015
- Arquivos: `docs/resource-limiting.md`, `.github/workflows/onp-sdd-evidence.yml`
- Evidência local: `verify resource-limiting` 5/5; feature runner 1 comando PASS;
  actionlint, W0/arquitetura e `git diff --check` PASS.
- Não há claim de medição real de OS, kill/quarantine ou persistência nesta fatia.
