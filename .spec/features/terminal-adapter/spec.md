# Spec: Terminal adapter

> feature: terminal-adapter
> status: implementada

## Contexto

PR-104 expõe um adaptador de terminal sobre o process primitive, sem criar caminho de execução paralelo ou shell bypass.

## Histórias

### US-610 — Terminal bounded

Como runtime, quero executar comandos estruturados via o primitive já validado, para obter uma interface terminal sem duplicar segurança.

#### AC-649 — Delegação e dedupe

- **Dado** ProcessSpec válido e operation key
- **Quando** o terminal executa
- **Então** delega ao process primitive, retorna round 1 e rejeita operation key duplicada, key vazia ou round cap inválido

#### AC-650 — Erros do primitive preservados

- **Dado** permission pendente, shell ou cancelamento no ProcessSpec
- **Quando** o terminal executa
- **Então** o erro/estado terminal permanece vindo do primitive, sem bypass ou fallback de shell

## Fora de escopo

- PTY, terminal persistente, shell livre, sudo, instalação e UI.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-617 | Um round inicial é suficiente para o adaptador sem PTY. | confirmada | `round: 1`; loops futuros terão contrato próprio. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-610 | Terminal persistente será necessário? | respondida | Fora deste card; requer lifecycle/PTY explícitos. |
