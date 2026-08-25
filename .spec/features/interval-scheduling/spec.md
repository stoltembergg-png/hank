# Spec: interval scheduling

> feature: interval-scheduling
> status: em-implementacao

### US-1140 — Calcular próximas execuções por intervalo

Como scheduler, quero calcular a próxima ocorrência a partir de um anchor persistido sem drift,
busy loop ou dependência de threads.

#### AC-1141 — Cálculo determinístico
- **Dado** anchor, intervalo e relógio fake
- **Quando** a próxima ocorrência é calculada
- **Então** ela é a primeira ocorrência estritamente posterior ao relógio e permanece igual após restart.

#### AC-1142 — Bounds e overflow
- **Dado** intervalo zero, abaixo do mínimo, acima do limite ou arithmetic overflow
- **Quando** o schedule é normalizado/calculado
- **Então** a operação falha com erro tipado sem loop ou saturação silenciosa.

#### AC-1143 — Estado disabled
- **Dado** schedule disabled
- **Quando** next_due é solicitado
- **Então** nenhum agendamento é retornado.

## Suposições
- ASM-1144: a semântica de intervalo é UTC/epoch-ms e independente de DST; timezone só será aplicado por cron.

## Perguntas em aberto
Nenhuma.
