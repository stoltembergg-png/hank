# Spec: execution history

> feature: execution-history
> status: em-implementacao

### US-1277 — Consultar e reter histórico de execução

Como operador de automações, quero consultar outcomes históricos por projeto/job e aplicar retenção bounded, para investigar falhas sem expor lease owners ou payloads.

#### AC-1278 — Histórico project-scoped e determinístico
- **Dado** runs de jobs em projetos diferentes
- **Quando** o operador consulta um projeto com filtros opcionais de job/status e paginação bounded
- **Então** somente o projeto solicitado é retornado, em ordem determinística `due_at_ms, run_id`, sem lease owner ou payload bruto;

#### AC-1279 — Paginação e retenção bounded
- **Dado** histórico maior que o limite
- **Quando** a consulta usa `limit`/`offset` e a retenção recebe cutoff e limite
- **Então** a página é bounded e determinística, e somente runs completed antigos até o limite são removidos;

#### AC-1280 — Isolamento e redaction
- **Dado** uma consulta ou retenção com projeto/job inexistente ou foreign
- **Quando** a operação é executada
- **Então** ela não atravessa o escopo, não retorna lease owner/payload e não altera runs de outro projeto.

## Suposições
- ASM-1281: a retenção é chamada por operador/worker existente e não cria um novo loop autônomo nesta PR.

## Perguntas em aberto
Nenhuma.
