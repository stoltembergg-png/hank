# Native evaluation contract

`test_support::evaluation` define a boundary declarativa para a primeira
versão do Harness Evaluation. O contrato é mantido em `test-support` para que
o domínio de produção não dependa do executor de testes.

## O que é congelado

`EvaluationCase` fixa `project_id`, `run_id`, `trace_id`, scenario/task
digests, fixture e scorer versionados, schema de métricas, policy/schema
revisions, classe de modelo, budget, cancelamento, idempotency key e
artifacts esperados. `HoldoutMarker` identifica explicitamente a suíte, a
revisão da partição e se o caso é de training ou holdout.

`MetricSchema` aceita somente nomes conhecidos e associa cada métrica a um
tipo (`boolean`, `count`, `duration_ms`, `ratio` ou `category`), direção e
limites. Observações usam esses tipos; não há campo para prompt, transcript,
chain-of-thought, payload de provider ou secret.

## Autoridade e evidência

Os únicos efeitos declarados são `read_only` e efeitos virtuais de teste.
`external_write` falha fechado. Fixture não determinística, authority
ausente, terminal esperado ausente e holdout ausente também falham fechado.

`BaselineReport` repete a identidade do caso e exige correspondência de
métricas, fixture, scorer, holdout e artifacts. A evidência registra SHA/tree,
digests de policy/schema/fixture/environment, artifacts e um estado
`pass`/`fail`/`blocked`/`no_proof`. O digest do relatório é recalculado para
detectar alteração stale. O relatório é sempre evidência: `can_activate()` é
permanentemente falso.

## Próximas extensões

Este card não executa runner nem adiciona corpus. As próximas extensões podem
consumir o contrato para fixtures de benchmark, comparação baseline/candidate
e holdout, mantendo a separação entre avaliação e ativação.
