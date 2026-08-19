# Spec: CI Build

> feature: ci-build
> status: rascunho

<!--
  Como ler este arquivo (o formato é verificado por `onp-spec audit`):
  - US-xxx = história de usuário · AC-xxx = critério de aceite
  - ASM-xxx = suposição · Q-xxx = pergunta em aberto
  - São códigos de rastreio: ligam a especificação às tarefas e aos testes.
  - Toda história de usuário precisa de pelo menos um critério de aceite.
  - Todo critério de aceite precisa de Dado/Quando/Então completos.
  - Os códigos são únicos no projeto inteiro (nunca reutilize um número).
  - Suposições e Perguntas em aberto são OBRIGATÓRIAS: se não há nenhuma,
    escreva "Nenhuma." — mas desconfie: quase toda feature esconde uma.
-->

## Contexto

Esta feature implementa PR-004 da queue executável: criar workflow de build verificável do workspace Rust e frontend em CI. O build local isolado não detecta deriva entre ambientes nem produz identidade de evidência.

## Histórias

### US-401 — Workflow de build Rust e frontend em CI

Como engenheiro de CI, quero um workflow que compile Rust e frontend em cada PR elegível, para que deriva de ambiente seja detectada e identidade de evidência (SHA/tree/digest) seja produzida.

#### AC-401 — Workflow válido executa build Rust completo

- **Dado** um PR elegível com mudanças no workspace Rust
- **Quando** o workflow de build roda em CI
- **Então** `cargo check --workspace` e `cargo build --workspace` terminam com exit code 0; artifacts com digest SHA-256 são publicados

#### AC-402 — Workflow válido executa build frontend completo

- **Dado** um PR elegível com mudanças no workspace frontend
- **Quando** o workflow de build roda em CI
- **Então** `npm ci && npm run build` terminam com exit code 0; artifact do bundle com digest SHA-256 é publicado

#### AC-403 — Cache controlado por Cargo.lock e package-lock.json

- **Dado** o workflow rodando em CI
- **Quando** `Cargo.lock` e `package-lock.json` não mudaram
- **Então** cache de dependências é restaurado e build é acelerado; cache miss é logado

#### AC-404 — Matrix mínima suportada definida

- **Dado** a configuração do workflow
- **Quando** inspeciono a matrix de OS/toolchain
- **Então** pelo menos `ubuntu-latest` + Rust stable + Node 20 estão definidos; sem matrix excessiva

#### AC-405 — Falha em erro, timeout ou artefato ausente

- **Dado** o workflow rodando
- **Quando** ocorre erro de compilação, timeout (>15min) ou artifact não gerado
- **Então** o workflow falha explicitamente com reason code identificável

## Fora de escopo

- Publicação, signing, release, secrets ou status externo assumido como protegido
- Testes unitários, Clippy, fmt (são PRs separados)
- Deploy, staging ou promoção de ambiente

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-401 | Rust stable + Node 20 disponíveis em GitHub Actions | aberta | — |
| ASM-402 | `Cargo.lock` e `package-lock.json` versionados | confirmada | PR-001/PR-003 |
| ASM-403 | Workspace Rust e frontend compilam localmente | confirmada | Verificado |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-401 | Publicar artifacts no GitHub Packages ou apenas upload transient? | aberta | — |
| Q-402 | Incluir Windows/macOS na matrix ou só Linux? | aberta | — |
| Q-403 | Timeout padrão 15min é suficiente? | aberta | — |