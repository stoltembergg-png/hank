# SubWorkflowNode plan

`workflow_core::subworkflow` planeja composição sem executar child workflows.

- catálogo bounded de referências exatas `project/workflow/version`;
- mapping de inputs determinístico e sem avaliação de expressões;
- cross-project exige grant explícito;
- depth e budget possuem limites finitos;
- ciclo direto é rejeitado antes de criar child plan;
- correlação é determinística por `parent_run/node/generation`;
- cancelamento do child plan é terminal e idempotente;
- nenhuma execução, scheduler, dynamic loading ou persistência é criada.

A persistência e a retomada após restart são responsabilidade da PR-186. O planner não restaura
capabilities nem secrets do pai automaticamente e não inclui payload sensível na correlação.
