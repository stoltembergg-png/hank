# Spec: Synthesis mode

> feature: synthesis
> status: implementada

## História de usuário

### US-924 — Sintetizar resultados provenance-aware

Como sessão de grupo, quero uma resposta bounded a partir de resultados
permitidos, preservando provenance e divergências sem transformar output em
autoridade.

#### AC-925 — Resultados bounded preservam provenance e conflito

- **Dado** resultados permitidos do mesmo projeto
- **Quando** a síntese determinística é executada
- **Então** cada linha preserva source ID, facts divergentes permanecem
  marcados como conflito e o trace fecha.

#### AC-926 — Exclusão fail-closed e dedupe

- **Dado** resultado negado, cross-project ou repetido
- **Quando** a síntese recebe os itens
- **Então** nenhum é incluído; cada exclusão tem razão observável.

#### AC-927 — Injection é dado e budget é obrigatório

- **Dado** conteúdo que tenta instruir o sistema e limite de saída
- **Quando** a síntese é executada
- **Então** o conteúdo é rotulado como data, a saída respeita o limite e o
  fallback é determinístico.

## Fora de escopo

- provider, scheduler, UI, escrita de memória, tool call, alteração automática
  de policy e persistência de output.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-928 | O fallback textual determinístico é suficiente até existir provider contract. | confirmada | A feature não cria provider nem chama modelo. |

## Perguntas em aberto

Nenhuma.
