# Spec: rate limiting

> feature: rate-limiting
> status: em-implementacao

## User stories

### US-2570 — Limitar triggers e ingressos por escopo

Como operador de um sistema multiagente, quero limitar requests e triggers por
escopo autenticado, para que loops, retries, catch-up e ingressos remotos não
consumam quota indefinidamente nem atravessem o isolamento de outro projeto.

#### AC-2571 — Burst e janela são bounded

- **Dado** uma policy de token bucket com capacidade, refill e janela monotônica
- **Quando** requests consomem o burst permitido e uma nova request chega antes
  do refill
- **Então** o limite retorna uma classificação explícita e `retry_after_ms`
  determinístico, sem atraso implícito ou silent drop

#### AC-2572 — Chaves e revisão vinculam o escopo

- **Dado** chaves de user, project, agent, provider, tool ou node e uma revisão
  de policy
- **Quando** uma request usa outra chave ou uma revisão stale
- **Então** o orçamento é isolado por chave e a revisão stale falha fechado

#### AC-2573 — Retry idempotente não cobra duas vezes

- **Dado** uma request idempotente admitida com identidade de retry bounded
- **Quando** a mesma request é repetida com a mesma classe e custo
- **Então** ela é admitida sem nova cobrança; conflito de identidade falha
  fechado e requests não idempotentes continuam consumindo o orçamento

#### AC-2574 — Recovery tem exceção finita

- **Dado** uma request marcada como recovery
- **Quando** o orçamento normal está esgotado e o orçamento de recovery ainda
  está disponível
- **Então** a request pode prosseguir somente dentro do bucket de recovery;
  após seu esgotamento, novas requests são negadas

#### AC-2575 — Relógio e persistência falham fechado

- **Dado** estado de buckets e snapshot vinculados à revisão da policy
- **Quando** o relógio monotônico retrocede, o snapshot é incompatível ou o
  número de chaves excede o bound
- **Então** a operação falha fechado sem aceitar estado parcial ou criar estado
  ilimitado; restore válido retoma o refill bounded e reset somente reseta uma
  chave existente em ponto monotônico autorizado

#### AC-2576 — Métricas são redigidas

- **Dado** requests admitidas e negadas em múltiplos escopos
- **Quando** métricas são consultadas
- **Então** somente contadores bounded são retornados, sem IDs de projeto,
  provider, ferramenta, request ou qualquer input cru

#### AC-2577 — Ingresso remoto exige binding antes do limite

- **Dado** um bootstrap remoto com credencial autenticada e binding exato de
  peer, node e project
- **Quando** o node excede o orçamento de bootstrap
- **Então** o daemon nega com `retry_after_ms` e audita a denial; identidade
  não autenticada ou fora do binding não pode consumir esse orçamento

#### AC-2578 — Trigger do scheduler é limitado antes do claim

- **Dado** um scheduler worker com rate limiter por project
- **Quando** um tick tenta exceder o orçamento configurado
- **Então** o tick falha com razão explícita antes de reivindicar uma lease
  adicional

## Segurança

- O limiter é transport-neutral e recebe somente chaves construídas após a
  autenticação/ownership correspondente; ele não interpreta payload para criar
  identidade.
- O relógio é fornecido pelo boundary chamador e deve ser monotônico. Não há
  leitura de wall clock, rede, credencial ou segredo no `security-core`.
- Recovery não é bypass: possui capacidade própria, refill e estado bounded.
- Snapshot/restore exige a mesma revisão de policy, valida chaves únicas e
  substitui o conjunto de buckets atomicamente sob o lock.
- Métricas e erros não carregam valores de escopo, payload ou material secreto.

## Suposições

- ASM-2570: persistência durável de snapshots e distribuição de policy serão
  conectadas por adapters posteriores; o core fornece contrato e snapshot
  validável, não um backend de storage.
- ASM-2571: limites de CPU/memória/disk permanecem no PR-258 e não são
  inferidos a partir do consumo de tokens desta feature.

## Perguntas em aberto

Nenhuma.
