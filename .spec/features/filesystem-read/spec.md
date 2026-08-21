# Spec: Filesystem read

> feature: filesystem-read
> status: implementada

## Contexto

PR-100 oferece leitura read-only confinada às raízes autorizadas de um projeto, sem traversal, symlink escape, execução ou payload ilimitado.

## Histórias

### US-606 — Leitura confinada e bounded

Como runtime de tools, quero ler arquivos somente dentro das raízes canônicas do projeto, para consultar dados sem atravessar projetos ou modificar o filesystem.

#### AC-635 — Lê arquivo autorizado

- **Dado** uma raiz existente, projeto autorizado e permissão `Allowed`
- **Quando** leio um path relativo existente
- **Então** retorno conteúdo UTF-8, bytes lidos, path lógico, trace e indicação de truncamento sem metadata sensível

#### AC-636 — Nega path/projeto/permissão inválidos

- **Dado** path absoluto, traversal, projeto diferente ou decisão não permitida
- **Quando** solicito leitura
- **Então** a operação falha com erro tipado antes de ler o arquivo

#### AC-637 — Nega symlink escape e limita payload

- **Dado** symlink fora da raiz ou arquivo maior que o limite
- **Quando** leio
- **Então** symlink é rejeitado e conteúdo é truncado explicitamente sem mutação

#### AC-638 — Falha fechada para root/UTF-8 inválidos

- **Dado** root ausente, limite inválido ou bytes não UTF-8
- **Quando** construo ou uso a tool
- **Então** erro tipado não expõe conteúdo bruto

#### AC-639 — Read não executa nem modifica

- **Dado** conteúdo não confiável com prompt injection e arquivo existente
- **Quando** leio
- **Então** o conteúdo permanece dado, nenhum processo é executado e o arquivo não é alterado

## Fora de escopo

- Escrita, remoção, rename, listagem, shell, processos, secrets discovery e execução de conteúdo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-609 | Roots autorizadas já são fornecidas por Project settings. | confirmada | A tool recebe roots explícitas; resolução de projeto fica fora do handler. |
| ASM-610 | UTF-8 estrito é o contrato inicial do resultado. | confirmada | Bytes inválidos falham; encoding alternativo terá card próprio. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-606 | Deve haver suporte a encoding binário/base64? | respondida | Não neste card; leitura é UTF-8 estrita e bounded. |
