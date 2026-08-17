# W0 Closure Gate

## Objetivo

Avaliar exclusivamente ARCH-001, ARCH-002, GOV-001, GOV-002 e GOV-003. O gate distingue existência documental de prova de comportamento, enforcement e autoridade.

## Entradas obrigatórias

- architecture graph/schema e fixtures;
- queue card schema, parser report e DAG report;
- PR Execution Contract/evidence manifest schemas;
- negative matrix, test output, SHA/tree/policy/schema revisions;
- reviewer distinto do author quando houver aprovação.

## Veredito

- `PASS`: todos os testes/validators aplicáveis passam no mesmo SHA/tree/policy e o reviewer independente confirma o diff.
- `NO_PROOF`: contrato existe, mas falta execução, identidade ou enforcement verificável.
- `BLOCKED`: caso negativo permitido, dependência inválida, scope drift, secret, branch proibida, reviewer inválido ou gate falho.

## Condição atual

A presença deste documento, dos schemas ou das fixtures não resolve os blockers. Até `PASS` autenticado e reprodutível, W0 permanece `PARTIAL/NO_PROOF` e nenhuma implementação downstream é liberada.
