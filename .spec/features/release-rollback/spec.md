# Spec: release rollback boundary

> feature: release-rollback
> status: implementada

## História de usuário

### US-3003 — Retornar a last-known-good quando um gate de release falha

Como runtime de atualização, quero selecionar atomically o slot last-known-good verificado
quando boot, health ou verificação falham, para que um update defeituoso não torne o app
inicializável nem propague regressão sem stop/recovery.

#### AC-3806 — Slot known-good é registrado somente para proof válido

- **Dado** um proof de artefato válido (identidade não vazia/bounded, digest não vazio/bounded)
- **Quando** o coordenador registra o known-good
- **Então** a versão fica disponível como alvo de rollback.

#### AC-3807 — Boot ou health falho seleciona o known-good anterior

- **Dado** um known-good registrado e um current version saudável
- **Quando** o current é avaliado como saudável
- **Então** `Continue`; quando avaliado como boot/health/verificação falho, retorna `Rollback` para o known-good.

#### AC-3808 — Versão revogada não pode ser restaurada

- **Dado** o known-good registrado que depois é revogado
- **Quando** uma falha tenta selecionar um alvo de rollback
- **Então** retorna `NoKnownGoodAvailable` fail-closed, sem restaurar a versão revogada.

#### AC-3809 — Rollback repetido converge sem loop

- **Dado** um rollback já aplicado
- **Quando** uma nova falha ocorre com tentativas acima do limite
- **Então** retorna `AttemptsExhausted`/`Blocked`, sem repetir indefinidamente.

#### AC-3810 — Proof inválido é rejeitado na construção

- **Dado** artefato com identidade vazia, digest vazio ou caractere de controle
- **Quando** o proof é construído
- **Então** retorna `InvalidProof`.

#### AC-3811 — Known-good avança estritamente para versão mais nova

- **Dado** um known-good e um proof de versão mais nova
- **Quando** registrado
- **Então** o known-good avança; uma versão mais antiga não avança o slot.

#### AC-3812 — Incidente de rollback é auditado sem material sensível

- **Dado** uma decisão de rollback
- **Quando** registrada no audit
- **Então** o registro preserva versões/attempts sem expor o digest do proof.

#### AC-3813 — Material de proof é redigido da observabilidade

- **Dado** um proof
- **Quando** renderizado via `Debug`/`Display`
- **Então** o digest cru nunca é exposto.

## Segurança e fronteira de prova

- O coordenador é transport-neutral e não contém updater, signer, filesystem, banco ou rede.
- Somente proof verificado pode ser last-known-good; versão revogada nunca é restaurada.
- O digest do proof é acessível apenas via getter para binding; `Debug`/`Display` redigem.
- Rollback é bounded (limite de tentativas) e idempotente; não há downgrade arbitrário de dados,
  operação de chave de signer nem claim de zero downtime.
- Esta feature prova somente o contrato offline; a classificação de produção permanece `NO_PROOF`.

## Suposições

- ASM-2107: a operação de chave de signer e a publicação automatizada de release são trabalho
  posterior; este card entrega só a decisão fail-closed de rollback.

## Perguntas em aberto

Nenhuma.