# Improvement candidate entity

`agent-core::improvement_candidate` modela um candidato versionado e isolado por projeto/owner, vinculado a observações, policy snapshot, target, risco e digest da proposta.

O lifecycle é `Draft → Evaluating → Approved/Rejected → RolledBack`. Transições fora da ordem e autorização cross-project são rejeitadas. Mesmo aprovado, o candidato não possui capacidade de ativação: aplicação, rollout, branch, issue e runtime permanecem fora do domínio.
