# Spec: Confirmation policies

> feature: confirmation-policies
> status: em-implementacao

## Contexto

PR-111 inicia o ciclo de aprovação humana para efeitos sensíveis de tools. O
contrato neutro do `tool-core` é exposto pela primeira fronteira de aplicação:
o request é bounded, o artefato carrega apenas hashes de schema/argumentos e o
ledger autoriza somente o contexto apresentado, com expiração, revogação e
replay controlados.

## Histórias

### US-617 — Autorizar efeito sensível com artefato verificável

Como Tool Runtime, quero emitir e validar um artefato de confirmação vinculado
à execução, para que uma aprovação não possa ser reutilizada em outro contexto
ou expor payload sensível.

#### AC-669 — Binding exato e payload redigido

- **Dado** um request com projeto, agente, tool, versão, schema, argumentos,
  efeito, budget, trace e actor
- **Quando** a aprovação é emitida e validada
- **Então** o ledger aceita somente o request apresentado e mantém apenas
  hashes bounded do schema/argumentos, sem guardar o payload bruto

#### AC-670 — Expiração e revogação fail-closed

- **Dado** um artefato expirado ou revogado
- **Quando** o runtime tenta autorizar o efeito
- **Então** a autorização é rejeitada antes de liberar a execução

#### AC-671 — Políticas e replay

- **Dado** `ask_every_time` ou `ask_once`
- **Quando** uma aprovação é consumida ou o mesmo escopo bounded é reapresentado
- **Então** `ask_every_time` rejeita replay e `ask_once` reutiliza somente o
  escopo idêntico, sem ampliar projeto, agente, actor, trace ou budget

#### AC-672 — Isolamento de identidade e policy

- **Dado** uma alteração de projeto, agente, actor, policy ou identidade da tool
- **Quando** o grant é apresentado
- **Então** a validação falha fechadamente sem executar o efeito

#### AC-673 — Permission gate consome o artefato

- **Dado** um efeito sensível avaliado pelo `PermissionEvaluator`
- **Quando** a fronteira fornece um request/grant do ledger
- **Então** somente o artefato compatível libera a decisão; ausência de grant
  mantém `ask` e incompatibilidade/expiração/replay resulta em negação

#### AC-674 — Application API transporta artefatos sem payload bruto

- **Dado** um request de confirmação submetido pela fronteira de aplicação
- **Quando** a API registra, aprova, revoga ou autoriza o request
- **Então** ela transporta somente o artefato serializável bounded, exige o
  actor apresentado e mantém as mesmas regras fail-closed do ledger, sem
  expor schema ou argumentos brutos

## Fora de escopo

- UI visual/Tauri bridge, autenticação completa do actor e integração no loop
  de execução de cada handler.
- Persistência entre processos, notificações push, pagamentos e escolha de
  políticas por projeto/agente.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-628 | A fronteira de aplicação pode transportar um artefato serializável sem payload bruto. | confirmada | `ApprovalRequest` e `ApprovalGrant` são serializáveis e carregam hashes SHA-256, IDs e metadados bounded. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-617 | A aprovação deve ser persistida entre processos? | respondida | Não neste incremento; o ledger é bounded e em memória. Persistência entre processos fica fora do escopo e exigirá contrato próprio na ponte runtime/Application API. |
