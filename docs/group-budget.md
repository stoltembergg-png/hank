# Group budget accounting

`GroupBudget` envolve a primitive `BudgetAccount` com project ID, group ID e
invocation ID. A reservation é atômica contra tokens, custo e concorrência;
commit reconcilia uso real e refund libera o restante.

Invocation IDs são deduplicados explicitamente. Nenhum dado de pricing externo,
segredo de billing, scheduler ou provider é introduzido nesta camada.
