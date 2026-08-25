# Interval scheduling

`IntervalSchedule` é um cálculo puro baseado em `anchor_ms` e duração em segundos.

- mínimo: 60 segundos;
- máximo: 31 dias;
- `next_due` retorna a primeira ocorrência estritamente posterior ao relógio;
- o anchor persistido evita drift após restart;
- arithmetic overflow falha com erro tipado;
- schedule disabled retorna `None`;
- não cria threads, timers, workers ou loops.

A semântica usa epoch milliseconds e é independente de DST. Timezone e cron permanecem
responsabilidade da calculadora cron futura.
