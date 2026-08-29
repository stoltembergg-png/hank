# Fix-review workflow

`agent-core::fix_review_workflow` transforma um finding blocker em um plano declarativo de correção, preservando o mapping de projeto/tarefa/repositório/worktree/branch/commit/tree/policy e vinculando a revisão superseded.

O contrato rejeita evidência stale e limita o número de ciclos. Ao atingir o cap, retorna `Escalated` sem criar tarefa. A crate não cria tasks/worktrees, não executa Git, não acessa rede, não usa credenciais e não autoriza merge.
