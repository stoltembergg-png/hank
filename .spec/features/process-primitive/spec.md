# Spec: Process primitive

> feature: process-primitive
> status: implementada

## Contexto

PR-103 oferece execução estruturada de processos sem shell, com allowlist, cwd confinado, ambiente explícito, timeout, cancelamento e output bounded.

## Histórias

### US-609 — Processo estruturado seguro

Como runtime de tools, quero executar somente programas allowlisted com recursos limitados, para evitar command injection e processos órfãos.

#### AC-646 — Execução allowlisted sem shell

- **Dado** programa allowlisted, cwd dentro da root e permission `Allowed`
- **Quando** executo argv estruturado
- **Então** o processo termina com status/trace/output bounded e nenhum shell implícito é usado

#### AC-647 — Rejeições fail-closed

- **Dado** shell, permission pendente, cwd fora da root ou ambiente sensível
- **Quando** valido o spec
- **Então** a execução é rejeitada antes do spawn

#### AC-648 — Timeout/cancelamento e redaction

- **Dado** processo longo ou cancelamento solicitado
- **Quando** o primitive executa
- **Então** mata o filho, retorna estado terminal bounded e limita/redige output

## Fora de escopo

- Terminal interativo/PTy, shell livre, sudo, instalação de pacotes e execução automática por LLM.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-615 | Processo estruturado pode iniciar com `env_clear` e ambiente explícito. | confirmada | Implementado; herança implícita é proibida. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-609 | PTY persistente é necessário? | respondida | Não neste primitive; terminal terá contrato próprio. |
