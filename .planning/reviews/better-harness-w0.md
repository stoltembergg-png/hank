# Better Harness review — W0 execution contract

**Escopo:** revisão do Harness de execução por agentes contra `PR-EXECUTION-CONTRACT.md`, `agent-development-policy.md`, a fila de 270 cards e os contratos W0. Esta revisão é documental e não prova enforcement externo, GitHub ou runtime.

## Findings

| ID | Consequência | Owner | Limite de evidência | Verifier/estado |
|---|---|---|---|---|
| BH-001 | Prompt sem schema permite interpretações diferentes do contrato. | execution harness + security | contrato original era prosa; agora há `PR-EXECUTION-CONTRACT.schema.json` e estados no gate. | validar schema + manifest no mesmo SHA; enforcement ainda `NO_PROOF` |
| BH-002 | Policy textual não impõe isolamento, comandos, reviewer ou secrets. | agent runtime + governance | policy e contract descrevem obrigações; workflow local só testa fixtures. | clean-room com dois agentes e evidence manifest; `PARTIAL/NO_PROOF` |
| BH-003 | Branch/worktree compartilhado pode alterar `main` ou trabalho alheio. | orchestration/runtime | `execution-gate-contract.md` e fixture de `main` existem; não há runner que intercepte escrita real. | tentativa real em `main`/path fora allowlist deve retornar `BLOCKED`; `NO_PROOF` |
| BH-004 | Self-approval ou reviewer não autenticado pode liberar mudança. | governance/security | schema exige author/reviewer e teste rejeita igualdade; identidade GitHub protegida ainda não foi exercitada. | reviewer distinto, autenticado e vinculado ao SHA; `NO_PROOF` |
| BH-005 | Evidence manifest não prova diff, comando, artifact, SHA/tree/policy se não for validado. | execution harness | schema e contract existem; workflow atual prova testes/validators, não uma execução de agente. | manifest machine-readable com digests e revalidação; `PARTIAL/NO_PROOF` |
| BH-006 | Scope drift e 35 label mismatches deixam a fila não determinística. | planning/governance | parser exige 270 IDs/campos e rejeita dependências/ciclos; labels históricos ainda precisam de decisão explícita. | relatório de normalização e fixture de arquivo provável; `PARTIAL/NO_PROOF` |
| BH-007 | Secret pode entrar em env, log ou artifact sem scanner/policy efetiva. | security/runtime | constituição define proibição e fixtures nomeiam o caso; não há secret scanner real no workflow. | scanner + artifact/env/log negative fixture; `NO_PROOF` |
| BH-008 | DAG/queue schema sem enforcement permite card incompleto ou stale. | planning/governance | `queue-card.schema.json`, parser e teste 270/270 existem e passam localmente. | CI `w0-contract-gate` no SHA protegido + normalização de labels; comportamento local `PASS`, enforcement externo `NO_PROOF` |
| BH-009 | Crash, timeout, retry, cancelamento e duplicate dispatch podem abandonar ou duplicar runs. | orchestration/runtime | contrato menciona lifecycle, retry/cancel e invalidação; não há lease, idempotency key, fencing ou fault runner. | fault matrix com crash/timeout/cancel/retry/duplicate; `NO_PROOF` |
| BH-010 | Sem preflight/gate runner machine-readable, dois agentes podem produzir resultados incompatíveis. | execution harness + security | schemas, validator local e workflow W0 existem; clean-room e identidade de autoridade ainda não. | dois agentes em clean-room, `PASS/NO_PROOF/BLOCKED` fail-closed; `PARTIAL/NO_PROOF` |

## Reconciliation

Os commits T-001–T-006 fecham a especificação, schemas, fixtures, validator documental, testes e workflow de W0. Eles não fecham automaticamente os findings BH-002/003/004/005/007/009/010 porque ainda faltam enforcement de runtime/orquestração, secret scanner, leases/idempotência, identidade protegida e execução autenticada do check.

O resultado do Harness, portanto, não autoriza declarar ARCH-001, ARCH-002, GOV-001, GOV-002 ou GOV-003 `RESOLVED`. O status correto permanece `PARTIAL/NO_PROOF` até que o verifier de cada finding produza evidência com repository, SHA, tree, policy/schema revision, run/check identity e reviewer distinto.

## Próximos gates

1. Publicar a branch e executar o workflow `w0-contract-gate` em um PR real.
2. Reconsultar branch protection e required check com autenticação; não assumir que a configuração existe.
3. Adicionar secret scanner e fixtures de artifact/env/log antes de ativar o check como required.
4. Definir e testar lease/idempotency/retry/cancel/fencing antes de permitir execução concorrente de agentes.
5. Revalidar a fila após resolver os 35 label mismatches; snapshot stale reabre GOV-001/GOV-002.
