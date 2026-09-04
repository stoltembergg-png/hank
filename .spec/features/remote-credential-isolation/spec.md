# Spec: remote credential isolation

> feature: remote-credential-isolation
> status: em-implementacao

### US-1451 — Broker de credencial remoto scoped e redigido

Como runtime remoto, quero emitir referências opacas de credencial vinculadas a
node/project/actor com lease, expiração e revogação, para que nenhum node consiga
resolver material de credencial fora do seu escopo e nenhum payload remoto
transporte segredo.

#### AC-1466 — Broker emite referência opaca sem material de segredo

- **Dado** uma `CredentialRef` existente e um escopo exato (node/project/actor)
- **Quando** o broker emite uma referência remota scoped
- **Então** o payload contém somente um handle opaco com hash; nenhum segredo,
  token ou valor cru é retido ou serializado.

#### AC-1467 — Resolução falha fechado para escopo divergente

- **Dado** referência emitida para um escopo (node/project/actor)
- **Quando** um peer com node, project ou actor diferente tenta resolver
- **Então** a resolução retorna erro tipado e nenhuma credencial é exposta.

#### AC-1468 — Referência expirada ou revogada falha fechado

- **Dado** referência com lease bounded e deadline
- **Quando** a referência é consultada após expirar ou após revogação
- **Então** a operação falha fechado e a referência não pode mais ser usada.

#### AC-1469 — Broker é bounded e auditoria é redigida

- **Dado** broker com limite máximo de referências ativas
- **Quando** o limite é atingido ou eventos de auditoria são registrados
- **Então** novas emissões falham fechado e o log nunca contém material de
  credencial, token ou valor cru.

#### AC-1470 — Lease binding impede uso cruzado entre agentes

- **Dado** referência vinculada a um actor específico
- **Quando** outro agente ou projeto tenta emitir ou resolver com a mesma
  referência
- **Então** a operação é negada; a vinculação é revalidada em cada operação.

## Segurança

- O broker nunca armazena ou transmite material de segredo; ele resolve
  somente referências opacas já existentes no escopo local.
- Escopo exato (node/project/actor) é revalidado em toda emissão e resolução.
- Lease, expiração e revogação são fail-closed; stale cleanup não reabre.
- Sem `keychain`, OS backend, socket, TLS, OAuth callback ou dispatch remoto
  nesta fatia — o broker é transport-neutral e o backend de secrets é injetado.
- `OsEntropy` obtém a seed de 128 bits do CSPRNG do sistema via `getrandom`;
  indisponibilidade da fonte aborta a construção com erro tipado, sem fallback
  para timestamp, contador ou outro valor previsível.

## Suposições

- ASM-1461: adapters concretos de OS keychain/Stronghold, transporte de
  referência e migração de secrets existentes (PR-256) pertencem a cards
  posteriores, mantendo este core sem material de segredo.

## Perguntas em aberto

Nenhuma.
