# Spec: improvement observation event

> feature: improvement-observation-event
> status: auditada

## Contexto

PR-218 registra observações de melhoria como dados não confiáveis e append-only, sem interpretar conteúdo como instrução ou mutação.

## Histórias

### US-1349 — Bounded observation

Como sistema, quero aceitar somente tipos/fontes válidos com provenance e payload bounded.

#### AC-1349 — Schema and trust boundary

- **Dado** observação válida com source/type/project/run/trace e payload bounded.
- **Quando** o envelope é criado.
- **Então** ele é aceito como `Untrusted` e não possui capability mutante.
- **Dado** versão desconhecida, payload oversized, fonte/tipo inválido ou conteúdo secret-like.
- **Quando** o envelope é criado.
- **Então** ele é rejeitado ou redigido de forma determinística.

### US-1350 — Dedup and privacy

Como sistema, quero deduplicar observações e preservar classe de privacidade/retention.

#### AC-1350 — Deterministic coalescing

- **Dado** duas observações com a mesma dedup key.
- **Quando** são comparadas.
- **Então** a segunda é classificada como duplicata determinística.
- **Dado** conteúdo instruction-like.
- **Quando** é criado o evento.
- **Então** o conteúdo permanece dado não confiável e não ganha autoridade.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Envelope puro, bounded, versionado, redigido/deduplicável e sem proposta, avaliação, persistência, workflow ou mutação.
