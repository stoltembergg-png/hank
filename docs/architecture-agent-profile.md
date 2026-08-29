# Architecture Agent Profile

O `architecture-agent-profile` é um contrato puro, bounded e provider-neutral em
`agent-core`. Ele produz análise advisory do manifesto arquitetural; não é um
gate autoritativo e não substitui policy, decisão humana ou ratificação de ADR.

## Boundary

O profile aceita somente dados tipados de layers, edges, documentos e evidence.
Valida schema/policy revision, project/task/repository/worktree/branch, SHA/tree,
limites de coleções e paths relativos. Não acessa Git, filesystem, rede,
providers, secrets ou processos.

A execução de leitura e a coleta de artifacts pertencem a adapters externos. O
módulo não interpreta comandos, source, diff ou texto como instrução executável.
Descrições instruction-like, paths inseguros, edges desconhecidos e manifests
malformed falham fechado.

## Graph checks

O manifesto deve declarar layers únicos, owners únicos, dependências conhecidas,
edges permitidos e edges proibidos. O evaluator reporta:

- `FORBIDDEN_EDGE` para atravessamento de boundary proibida;
- `UNDECLARED_EDGE` para edge fora da allowlist;
- `CYCLE` para ciclo no grafo;
- `MISSING_DOCUMENT` para architecture document ou ADR ausente.

Esses findings são dados de revisão. O profile não edita edges, aplica refactor
ou altera o graph gate.

## Evidence e documentação

Cada check (`graph`, `dependencies`, `documents`, `ADR impact`) deve possuir
identity exata de `head_sha`, `tree_sha`, `graph_revision` e `policy_revision`,
além de path relativo, bytes bounded e digest. Evidence `Passed` de todos os
checks é necessária para `Pass`; missing, skipped, no-run, malformed ou stale
produzem `NoProof`. Documento ausente produz `Blocked`, e edge/cycle ou evidence
failed produz `Failed`.

O report retém somente referências, status, severity, códigos e digests. Não há
conteúdo bruto de source, logs, prompts ou artifacts.

## Handoff advisory

Relatórios não provados ou falhos podem gerar handoff com códigos dos findings e
digest do evaluator. O handoff é sempre advisory:

- não edita arquitetura;
- não ratifica ADR;
- não aprova mudanças;
- não altera gates nem faz bypass;
- não publica, faz merge ou libera release.

Hypothesis, missing, stale, malformed e blockers permanecem não provados. O
rollback é remover o módulo, testes, documentação e a etapa ONP desta feature;
nenhuma arquitetura ou infraestrutura externa é modificada.
