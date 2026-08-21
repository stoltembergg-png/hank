# Spec: Filesystem write

> feature: filesystem-write
> status: implementada

## Contexto

PR-101 permite escrita atômica e reversível somente em roots autorizadas, após permission e com dedupe por operation key.

## Histórias

### US-607 — Escrita autorizada e rollback

Como runtime de tools, quero escrever arquivos de projeto de forma atômica e reversível, para que falhas não destruam o estado anterior.

#### AC-640 — Escrita bounded e atomicamente substituída

- **Dado** projeto, root, path relativo, payload limitado e decisão `Allowed`
- **Quando** escrevo com operation key
- **Então** arquivo novo ou existente é substituído atomicamente e resultado contém bytes, path lógico, trace e key

#### AC-641 — Rollback restaura snapshot

- **Dado** escrita nova ou substituição de arquivo existente
- **Quando** executo rollback da operation key
- **Então** arquivo novo é removido e arquivo existente retorna exatamente ao snapshot anterior

#### AC-642 — Rejeições e dedupe fail-closed

- **Dado** path traversal, projeto errado, permission pendente, payload acima do limite ou operation key repetida
- **Quando** escrevo
- **Então** a rejeição não modifica filesystem e repetição deduplicada não aplica novo conteúdo

## Fora de escopo

- Delete/rename independente, listagem, shell/processos, secrets scanning e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-611 | Rename no mesmo filesystem fornece substituição atômica suficiente para este contrato. | confirmada | Temp file é criado no parent e renomeado antes de registrar snapshot. |
| ASM-612 | Snapshot bounded em memória atende rollback de uma operação durante o processo. | confirmada | Persistência futura terá card próprio. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-607 | Rollback deve sobreviver a restart? | respondida | Não neste card; rollback é explicitamente process-local. |
