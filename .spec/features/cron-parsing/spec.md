# Spec: cron parsing

> feature: cron-parsing
> status: em-implementacao

### US-1160 — Validar e calcular cron em timezone declarado

Como scheduler, quero aceitar uma grammar cron pequena e determinística em timezone IANA,
sem aceitar expressões ambíguas, frequentes demais ou capazes de gerar busca sem limite.

#### AC-1161 — Grammar e corpus
- **Dado** cinco campos `minute hour day-of-month month day-of-week`
- **Quando** a expressão é parseada
- **Então** wildcards, valores, ranges e listas válidos são normalizados; malformed/extra fields falham com erro estável.

#### AC-1162 — Timezone e DST
- **Dado** timezone IANA válido e relógio UTC
- **Quando** a próxima ocorrência é calculada
- **Então** gap de DST é pulado, fold usa a ocorrência UTC mais antiga e o resultado é bounded.

#### AC-1163 — Segurança e frequência
- **Dado** input longo, timezone desconhecido ou expressão a cada minuto
- **Quando** é validado
- **Então** falha sem eval dinâmico, sem loop infinito e sem aceitar frequência abaixo do mínimo.

## Suposições
- ASM-1164: a grammar inicial não inclui nomes de mês/dia, `L`, `W`, `#` ou steps; upgrades exigem nova versão.

## Perguntas em aberto
Nenhuma.
