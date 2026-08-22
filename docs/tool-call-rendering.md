# Tool-call rendering

A UI renderiza tool calls como conteúdo não confiável, consumindo dados já produzidos pela Application API.

## Estados

`pending`, `allowed`, `ask`, `denied`, `running`, `succeeded`, `failed`, `cancelled` e `timeout` têm rótulo e cor distintos. O estado `ask` pode expor approval affordances por callbacks fornecidos pela camada de aplicação; `denied` nunca exibe botão de execução.

## Segurança e limites

- Argumentos são exibidos como JSON em elemento `<pre>` e chaves secret-like (`api_key`, `secret`, `password`, `token`, `authorization`, `bearer`, chaves privadas) são substituídas por `[redigido]`.
- Resultados e erros são inseridos como nós de texto React; HTML e scripts vindos de ferramentas/modelos não são interpretados.
- Strings individuais de argumentos são limitadas a 1.000 caracteres e marcadas com `… [truncado]`; o output visual tem altura máxima e rolagem.
- Trace ID, versão da ferramenta, orçamento e timing são metadados visíveis quando fornecidos.
- O `ChatPage` filtra tool calls por `project_id` e `agent_id` da sessão ativa.

## Boundary

O frontend não executa ferramentas, não acessa SQLite e não avalia autorização. Os callbacks de approval são passados pela aplicação; a UI apenas emite a intenção associada ao `approvalId`. Dados são recebidos via props/eventos e não são tratados como instruções confiáveis.

## Estados terminais

`failed`, `cancelled` e `timeout` permanecem visíveis para auditoria e não oferecem ação de execução. Um resultado truncado mantém a indicação de truncamento; a UI não tenta reconstruir ou buscar o conteúdo omitido.