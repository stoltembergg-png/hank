# Rate limiting

O rate limiter de `security-core` é uma policy portátil e bounded. O chamador
fornece uma chave criada depois de autenticar e autorizar o contexto, uma revisão
de policy e um timestamp monotônico; o core não lê wall clock, rede, payload ou
credenciais.

## Policy e resposta

Uma policy define capacidade, refill por janela, capacidade separada de recovery,
quantidade máxima de chaves e histórico bounded de retries idempotentes. A resposta
é `Allowed` ou `Denied`; em caso de denial, `retry_after_ms` e a classe de razão
permanecem explícitos. O limiter não dorme nem descarta silenciosamente requests.

As chaves são tipadas por `User`, `Project`, `Agent`, `Provider`, `Tool` ou `Node`
e sempre carregam `project_id` e `subject_id`. O adapter deve derivá-las de
identidade autenticada/ownership, nunca de um campo não verificado do payload.

Retries idempotentes podem ser reapresentados sem cobrança duplicada enquanto a
receipt permanece no limite bounded de requests recentes. Conflito de request ID,
custo ou classe falha fechado. Requests não idempotentes consomem o bucket a cada
admission.

Recovery possui bucket próprio e finito. Ele protege a recuperação contra storms;
não é uma isenção ilimitada nem pode elevar capacidade por payload.

## Integrações atuais

- `AuthenticatedDaemon::new_with_rate_limiter` aplica o limite de bootstrap
  somente após autenticação e binding exato peer/node/project. Uma denial gera
  `DaemonError::RateLimited` e evento redigido `RateLimited`.
- `SchedulerWorker::new_with_rate_limiter` admite cada trigger antes de chamar
  `claim_next_due`. Quando o limite nega, retorna `WorkerError::RateLimited` e
  não reivindica a lease adicional.

## Persistência e operação

`RateLimiter::snapshot` produz buckets e receipts bounded vinculados à revisão.
`from_snapshot`/`restore` rejeitam revisão divergente, relógio retrocedido, chaves
duplicadas, tokens fora da capacidade e histórico acima do limite. O snapshot é
uma porta para um backend posterior; esta feature não grava em SQLite nem oferece
serviço distribuído.

`reset_window` só reseta uma chave já existente em um ponto monotônico fornecido
por um operador; não cria buckets desconhecidos, aceita relógio retrocedido ou
limpa o estado de outras chaves.

As métricas expõem somente a revisão/janela da policy e contadores (`allowed`,
`denied`, `delayed`, retries idempotentes, recovery, chaves saturadas e tokens
restantes agregados), sem valores de escopo ou request. Para tuning, alterações
na revisão da policy devem ser tratadas como mudança explícita e revalidadas no
boundary que persiste o snapshot.

## Rollback e limites

Para rollback, remova a configuração opcional do limiter nos adapters e reverta a
mudança da policy no mesmo SHA; não há migração de dados ou processo externo para
desfazer. O PR não implementa quota de CPU/memória/disk, serviço de abuso
distribuído, auto-tuning, listener remoto ou execução de workflow.
