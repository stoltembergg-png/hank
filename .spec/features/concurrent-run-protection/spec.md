# Spec: concurrent-run protection

> feature: concurrent-run-protection
> status: em-implementacao

### US-1240 — Admitir runs sem sobreposição

Como scheduler, quero admitir runs por chave de concorrência com limite e fencing, para impedir
sobreposição não autorizada sem criar um lock distribuído.

#### AC-1241 — Admission atômica
- **Dado** dois workers e limite 1 na mesma chave
- **Quando** ambos solicitam admission
- **Então** exatamente um é admitido e o outro recebe rejeição bounded.

#### AC-1242 — Expiry e cancelamento
- **Dado** um slot admitido com lease
- **Quando** o lease expira ou o run é cancelado pelo owner
- **Então** o slot pode ser reutilizado e owner diferente não pode liberar o slot ativo.

#### AC-1243 — Isolamento por projeto e limite protegido
- **Dado** a mesma chave em projetos diferentes ou limite inválido
- **Quando** a admission é solicitada
- **Então** projetos não colidem e o limite é validado na boundary, não pelo payload do job.

## Suposições
- ASM-1244: `concurrency_key` já foi derivada por uma boundary autorizada; esta PR não interpreta payloads.

## Perguntas em aberto
Nenhuma.
