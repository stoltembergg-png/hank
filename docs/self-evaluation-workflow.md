# Self-evaluation workflow

`agent-core::self_evaluation_workflow` produz um decision record bounded para avaliação externa de um candidate. O record fixa candidate/version/SHA e declara os estágios `Validation`, `Tests` e `Security`.

Ausência de policy, tests ou security bloqueia o workflow; crash do evaluator produz decisão `Blocked`. Uma decisão `Approved` continua sem capacidade de ativação: rollout, mutação de runtime, branch, issue e aprovação humana permanecem fora deste módulo.
