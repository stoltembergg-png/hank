# Tasks: Tauri Desktop

> feature: tauri-desktop

<!--
  Como ler este arquivo (o formato é verificado por `onp-spec audit`):
  - T-xxx = tarefa (código de rastreio, único no projeto inteiro).
  - Toda tarefa referencia em `Refs:` pelo menos uma história de usuário
    (US-xxx) ou critério de aceite (AC-xxx).
  - Toda tarefa lista os arquivos que cria/altera em `Arquivos:` — capriche:
    é o que decide o que `onp-spec plano` roda em PARALELO (arquivos
    disjuntos) e o que roda em sequência.
  - Campos opcionais por tarefa, usados pelo plano de execução:
    `- Modelo: claude-sonnet-5` e `- Esforço: alto` (baixo|medio|alto|xalto|max).
  - Uma tarefa só pode virar [concluida] quando os critérios de aceite dela
    tiverem prova PASS registrada por `onp-spec verify`.
  Status: pendente | em-andamento | concluida
    (atalho: `onp-spec tarefa <feature> <T-xxx> <status>`)
-->

## T-311 — Inicializar projeto Tauri 2 (apps/desktop) [pendente]

- Refs: US-101, AC-101, AC-102, AC-103
- Arquivos: apps/desktop/src-tauri/Cargo.toml, apps/desktop/src-tauri/build.rs, apps/desktop/src-tauri/tauri.conf.json, apps/desktop/src-tauri/src/main.rs
- Notas: Tauri 2 app mínimo; lifecycle de janela; bridge sem comandos de produto

## T-312 — Configurar manifest Tauri com capabilities mínimas [pendente]

- Refs: US-101, AC-102, AC-103
- Arquivos: apps/desktop/src-tauri/tauri.conf.json
- Notas: Sem allowlist perigosa; CSP restritivo; apenas window capabilities

## T-313 — Implementar lifecycle de janela (open/close/ready) [pendente]

- Refs: US-101, AC-101, AC-105
- Arquivos: apps/desktop/src-tauri/src/main.rs
- Notas: Logs estruturados boot/ready/close; exit code 0

## T-314 — Testes de aceitação: manifest, CSP, bridge, logs [pendente]

- Refs: US-101, AC-101, AC-102, AC-103, AC-104, AC-105
- Arquivos: apps/desktop/src-tauri/tests/tauri_ac_tests.rs
- Notas: Testes cobrindo AC-101..105: build, manifest, CSP, bridge sem comandos, logs

## T-315 — Verificar build do workspace [pendente]

- Refs: US-101, AC-101, AC-102, AC-103, AC-104, AC-105
- Arquivos: apps/desktop/src-tauri/tests/tauri_ac_tests.rs, apps/desktop/src-tauri/Cargo.toml, .github/workflows/build-tauri.yml
- Notas: cargo check/test e fmt em Ubuntu com WebKitGTK; valida AC-101..105