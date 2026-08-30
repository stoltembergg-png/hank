# Spec: skill improvement proposal

> feature: skill-improvement-proposal
> status: auditada

### US-1355 — Versioned skill proposal

Como avaliador, quero um diff de skill versionado e reproduzível sem alterar a versão ativa.

#### AC-1355 — Bounded proposal

- **Dado** skill/version, source candidate, arquivos declarados, capabilities/tests e diff bounded.
- **Quando** a proposal é criada.
- **Então** ela preserva a versão ativa, calcula hash estável e permanece proposal-only.
- **Dado** source/skill/version ausente, diff vazio ou oversized.
- **Quando** a proposal é criada.
- **Então** falha fechadamente.

### US-1356 — Malicious boundary

Como sistema, quero rejeitar path traversal, hidden files e secret injection.

#### AC-1356 — Declared safe diff

- **Dado** arquivo oculto, path traversal, script/reference não declarado ou conteúdo secret-like.
- **Quando** a proposal é criada.
- **Então** é rejeitada e não possui capability de ativação.
- **Dado** capability delta.
- **Quando** consultado.
- **Então** permanece apenas declarativo e exige avaliação externa.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Proposal versionada, bounded e determinística, sem mutação da skill ativa, instalação, execução ou ativação.
