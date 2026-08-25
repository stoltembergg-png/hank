# Cron parsing

A versão inicial da grammar é deliberadamente pequena e versionável:

```text
minute hour day-of-month month day-of-week
```

Suporta `*`, valores, ranges, listas e steps (`*/5`). Não suporta nomes, `L`, `W`, `#` ou
extensions. A expressão é limitada a 128 bytes e cada campo a 64 bytes.

Timezone usa IANA via `chrono-tz`. O cálculo busca no máximo 366 dias, sem eval dinâmico:

- gap DST: ocorrência local inexistente é pulada;
- fold DST: escolhe a ocorrência UTC mais antiga ainda futura;
- timezone inválido: rejeição tipada;
- every-minute: rejeitado como frequência excessiva.

O cálculo é puro e não cria worker, timer ou loop persistente. Mudanças de grammar exigem nova
versão e novo corpus.
