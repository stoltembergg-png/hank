# Tasks: CI Build

> feature: ci-build

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

## T-401 — Criar workflow de build Rust (.github/workflows/build-rust.yml) [pendente]
- Refs: US-401, AC-401, AC-403, AC-404, AC-405
- Arquivos: .github/workflows/build-rust.yml, tools/ci/require-artifact-digest.sh
- Notas: cargo check/build, cache Cargo.lock, matrix ubuntu-latest

## T-402 — Criar workflow de build frontend (.github/workflows/build-frontend.yml) [pendente]
- Refs: US-401, AC-402, AC-403, AC-404, AC-405
- Arquivos: .github/workflows/build-frontend.yml, tools/ci/require-artifact-digest.sh
- Notas: npm ci/build, cache package-lock.json, matrix ubuntu-latest + Node 20

## T-403 — Adicionar artifact upload com digest SHA-256 [pendente]
- Refs: US-401, AC-401, AC-402
- Arquivos: .github/workflows/build-rust.yml, .github/workflows/build-frontend.yml, tools/ci/require-artifact-digest.sh
- Notas: actions/upload-artifact com retention, digest no nome

## T-404 — Validar actionlint e schema do workflow [pendente]
- Refs: US-401, AC-405
- Arquivos: .github/workflows/build-rust.yml, .github/workflows/build-frontend.yml
- Notas: actionlint passa; schema GitHub Actions válido

## T-405 — Adicionar fixture de erro/falta de artifact [pendente]
- Refs: US-401, AC-405
- Arquivos: .github/workflows/build-rust.yml, .github/workflows/build-frontend.yml, tools/ci/require-artifact-digest.sh
- Notas: teste negativo que falha se build ok mas artifact ausente