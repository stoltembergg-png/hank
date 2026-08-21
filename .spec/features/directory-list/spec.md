# Spec: Directory list

> feature: directory-list
> status: implementada

## Contexto

PR-102 lista somente metadata de entries em diretórios autorizados, com filtros bounded e ordenação determinística.

## Histórias

### US-608 — Inspeção de diretório segura

Como runtime de tools, quero listar entries de um diretório do projeto sem conteúdo, para explorar estrutura sem vazar ou atravessar roots.

#### AC-643 — Listagem bounded e determinística

- **Dado** diretório autorizado e permissão `Allowed`
- **Quando** listo com filtros prefix/suffix
- **Então** entries são metadata-only, ordenadas por nome, dotfiles ficam ocultos por padrão e limite gera truncamento explícito

#### AC-644 — Isolamento e validação

- **Dado** projeto errado, permission pendente, traversal ou filtro inválido
- **Quando** listo
- **Então** erro tipado ocorre antes de retornar entries

#### AC-645 — Symlink escape fail-closed

- **Dado** symlink em uma directory root apontando para fora
- **Quando** listo
- **Então** a operação rejeita o escape e não retorna conteúdo do destino

## Fora de escopo

- Conteúdo de arquivos, watch, glob irrestrito, mutação e execução.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-613 | Metadata de tamanho é suficiente para a primeira versão. | confirmada | Conteúdo permanece fora do contrato. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-608 | Dotfiles exigem policy adicional? | respondida | São ocultos por padrão; `include_hidden` é explícito e continua sujeito a root/permission. |
