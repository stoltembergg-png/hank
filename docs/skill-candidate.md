# Skill candidate generation

`agent_runtime::skill_candidate` é uma boundary data-only para transformar uma
proposta de Skill e referências de observações em um candidato project-scoped.
O serviço não recebe repositório, não persiste conversa e não chama criação,
promoção, ativação, execução, provider, tool ou Git.

## Contexto e estados

- `Draft`: proposta parseada, project-scoped, bound à policy/capability/budget/
  trace e sem sinais de poisoning;
- `Quarantined`: parser detectou override de instrução, capability escalation,
  escopo/trace divergente, script ou marcador sensível;
- `Discarded`: transição terminal e idempotente do draft, preservando digest e
  versão base de rollback.

As observações são somente referências `(observation_id, digest, source)`.
IDs são ordenados e deduplicados; a mesma ID com evidência conflitante falha
fechada. Limites de quantidade, tamanho, actor, versão, budget e documento são
checados antes do parse.

## Handoff para avaliação

O handoff contém apenas `candidate_digest`, `source_digest`, `policy_digest`,
`budget_digest`, identidade, trace, status e `rollback_version`. O digest é
determinístico para a mesma entrada e muda quando o contexto governado muda.
Não há Markdown, instruções, conversa, script, segredo ou payload bruto no
handoff. A etapa seguinte pode montar uma solicitação para o evaluator
não-activating; este incremento não executa essa etapa nem concede autorização
de lifecycle.
