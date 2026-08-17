# Queue validator contract — AB-038 / AI-038

## Entrada

O validator lê os três arquivos `.planning/queue/queue-*.md` e `queue-index.md` no mesmo tree. O parser reconhece somente headings `### PR-NNN — título` e os 19 campos canônicos do schema `queue-card.schema.json`.

## Regras

1. O conjunto de IDs deve ser exatamente `PR-001..PR-270`, sem duplicata ou lacuna.
2. Cada card deve preencher os 19 campos; `Arquivos prováveis` é planejamento e não prova de existência.
3. Cada dependência deve ser um ID existente e a relação deve ser acíclica.
4. Categoria deve pertencer ao enum canônico; label textual divergente não pode ser normalizada silenciosamente.
5. M16 deve aparecer como cards numerados `PR-252..PR-270` e `PR-001` deve ser único e explícito.
6. O relatório deve conter `PASS`, `BLOCKED` ou `NO_PROOF`, motivo, card, SHA, tree e revisão do schema/policy.

## Estados

- `PASS`: estrutura completa e identidade de evidência presente.
- `BLOCKED`: inconsistência determinística ou dependência inválida.
- `NO_PROOF`: documentação existe, mas falta execução/evidência vinculada.

O validator nunca promove uma fila apenas porque todos os headings foram encontrados; a ausência de prova de execução mantém o fechamento do blocker em `PARTIAL/NO_PROOF`.
