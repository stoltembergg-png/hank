# Spec: release-agent workflow

> feature: release-agent-workflow
> status: auditada

## Contexto

PR-217 prepara uma proposta de release baseada em evidências exatas, sem publicar ou assinar artefatos.

## Histórias

### US-1347 — Release candidate identity

Como agente, quero montar um manifest vinculado ao repository, commit, tree, policy, artefato e CI corretos.

#### AC-1347 — Identity and fail-closed

- **Dado** artefato e CI com identidade exata, checksum válido e policy correta.
- **Quando** a proposta é montada.
- **Então** o candidato é `Draft` e idempotente.
- **Dado** identidade divergente, checksum inválido ou evidência ausente.
- **Quando** a proposta é montada.
- **Então** o candidato é `NoGo` com razão explícita.

### US-1348 — Protected handoff

Como sistema de release, quero explicitar que signing/provenance/publicação dependem de ambiente protegido.

#### AC-1348 — Human/environment boundary

- **Dado** signing ou provenance ausente.
- **Quando** a proposta é montada.
- **Então** o estado é `NoGo`, sem acesso a segredo e sem aprovação automática.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Manifest determinístico, bounded e fail-closed; nenhum publishing, signing, merge ou acesso a credenciais.
