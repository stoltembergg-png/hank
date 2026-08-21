# Tasks: Http tool

> feature: http-tool

## T-640 — Implementar HTTP client bounded e egress policy [concluida]

- Refs: US-611, AC-651, AC-652, AC-653
- Arquivos: crates/tool-core/src/http.rs, crates/tool-core/src/lib.rs, crates/tool-core/tests/http_contract.rs, crates/tool-core/Cargo.toml, Cargo.lock
- Notas: reqwest blocking/rustls, host allowlist, private default deny, header redaction, redirect none, timeout e body bound.

## T-641 — Documentar HTTP boundary [concluida]

- Refs: US-611, AC-651, AC-652
- Arquivos: docs/http-tool.md
- Notas: egress, headers, limits e não-escopo.
