# Spec: rate limiting

> feature: rate-limiting
> status: em-implementacao

### US-2000 — Limitar triggers e ingresso remoto sem bypass de identidade

Como runtime de segurança, quero aplicar limites token-bucket por identidade e projeto,
para impedir bursts, retry storms e consumo cruzado de quota sem transformar recovery em
um bypass ilimitado.

#### AC-2001 — Burst e janela são bounded e monotônicos

- **Dado** uma política com burst, janela e revisão válidos
- **Quando** requests autenticados consumirem tokens usando um relógio monotônico
- **Então** o burst permitido é finito, o refill respeita a janela, o relógio regressivo
  falha fechado e o estado retido permanece dentro do limite configurado.

#### AC-2002 — A chave é vinculada à identidade autenticada e ao projeto

- **Dado** duas identidades autenticadas em projetos diferentes
- **Quando** cada uma consumir a mesma classe de limite
- **Então** a quota de um projeto não é consumida pela outra, a revisão de policy é
  verificada e nenhum campo livre do payload pode trocar a chave efetiva.

#### AC-2003 — Retry e recovery continuam bounded

- **Dado** uma request idempotente, uma classe de recovery ou uma métrica
- **Quando** a request for admitida repetidamente dentro da janela
- **Então** retry idempotente e recovery recebem decisões explícitas e bounded, sem
  isenção ilimitada nem crescimento não limitado do estado.

#### AC-2004 — Ingresso remoto rejeita excesso após autenticação

- **Dado** um peer autenticado e autorizado no daemon remoto
- **Quando** o peer exceder o limite de ingresso configurado
- **Então** o bootstrap retorna uma classificação de rate limit, não abre lease, registra
  somente auditoria redigida e não altera o estado de outro projeto.

#### AC-2005 — Trigger de agente usa o mesmo contrato fail-closed

- **Dado** um dispatch de agente autenticado, com projeto e agente vinculados
- **Quando** o gate de trigger for avaliado
- **Então** excesso retorna motivo explícito sem produzir dispatch, entradas de projeto
  distintas permanecem isoladas e o resultado expõe revisão/janela sem dados sensíveis.

## Segurança

- A policy é pura quanto a efeitos externos e recebe somente identidade já autenticada,
  IDs bounded e relógio monotônico fornecido pelo chamador.
- Project, agent, provider, tool e node fazem parte da chave; a chave não é derivada de
  um campo de payload mutável depois da autenticação.
- Denial é fail-closed, recovery é uma classe com orçamento próprio e métricas não
  concedem execução nem bypass.
- O estado é limitado por `max_keys`; nenhuma API aceita segredos, tokens ou conteúdo
  bruto de requests.
- O remote daemon só aplica o limite depois da autenticação e autorização exatas; falhas
  não criam lease e o audit registra apenas identidade redigida/classificação.

## Suposições

- ASM-2006: `security-core` é a boundary aprovada para policy pura; os adapters de
  `remote-core` e `agent-runtime` fornecem a identidade autenticada e o relógio.
- ASM-2007: o repositório ainda não possui um scheduler-core separado; a integração de
  trigger desta fatia usa `AgentDispatchRequest` e não inventa um novo executor.
- ASM-2008: persistência distribuída, rate limiting por IP e enforcement em transporte
  real permanecem fora deste incremento e exigem contratos próprios.

## Perguntas em aberto

Nenhuma.
