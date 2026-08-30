# Workflow improvement proposal

`agent-core::workflow_improvement_proposal` representa uma alteração before/after de DAG sem mutar o workflow ativo. O contrato valida limites de nós/arestas, referências, ciclos e preserva a versão anterior para rollback.

Capability privilegiada, budget escalation e quebra de compatibilidade de estado são bloqueadas. O digest é determinístico; a proposal é somente input de avaliação e não pode ativar scheduler, subworkflow ou runtime.
