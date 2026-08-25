# Spec: one-shot scheduling

> feature: one-shot-scheduling
> status: em-implementacao

### US-1180 — Consumir execução única exatamente uma vez

Como scheduler, quero reclamar um job one-shot devido com chave idempotente, para que retry/restart
não executem o mesmo job duas vezes.

#### AC-1181 — Due-at e policy
- **Dado** job one-shot futuro, passado ou expirado
- **Quando** é reclamado com relógio explícito
- **Então** somente o devido e não expirado é aceito; passado fora da due policy e expirado falham tipados.

#### AC-1182 — Atomic consume
- **Dado** dois claimers no mesmo job
- **Quando** reclamam concorrentemente
- **Então** exatamente um claim é aceito e o estado consumed fica durável.

#### AC-1183 — Replay, scope e lifecycle
- **Dado** claim repetido, actor/project incorreto, disabled ou archived
- **Quando** a operação é repetida
- **Então** mesma chave retorna o mesmo recibo; outras chaves e escopos falham sem mutação.

## Suposições
- ASM-1184: due-at é epoch milliseconds e claim marca consumed atomicamente; execução real continua fora desta PR.

## Perguntas em aberto
Nenhuma.
