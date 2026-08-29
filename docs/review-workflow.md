# Review workflow

`agent-core::review_workflow` agrega evidência de reviewer, QA, security e architecture em um relatório advisory puro.

O contrato exige identidade exata de projeto/tarefa/repositório/worktree/branch/commit/tree/policy. Evidência `missing`, `skipped`, `cancelled`, `stale` ou `malformed` resulta em `blocked`; nenhum estado desconhecido vira sucesso.

Findings são bounded e dados externos, inclusive texto `approved` de IA. O relatório nunca concede `ready`, approval ou merge: `can_mark_ready`, `can_approve` e `can_merge` permanecem falsos. O fingerprint determinístico só identifica o relatório e não é autorização.
