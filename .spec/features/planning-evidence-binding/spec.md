# Spec: planning evidence binding

> feature: planning-evidence-binding
> status: auditada

## Contexto

O Harness precisa ligar cada `ReviewerFinding` ao contrato Claim/Evidence sem
tratar a referência textual do reviewer como prova. O binding é puro,
provider-neutral e limitado ao projeto, run, trace e identidade esperada. Ele
aceita somente `EvidenceRecord` produzido por um resolver; não acessa UI,
store, filesystem, rede, provider ou autorização.

### US-1420 — Vincular finding ao estado factual

Como boundary de planejamento, quero projetar um finding em um `Claim` e em
registros de evidência resolvidos.

#### AC-1420 — Finding sem prova permanece NO_PROOF

- **Dado** um finding sem registros de resolver ou com referência marcada como
  ausente.
- **Quando** o binding é executado.
- **Então** o claim permanece `NO_PROOF` e uma disposição `MITIGATE` não é
  mantida como mitigável.

### US-1421 — Exigir correspondência exata

Como sistema de confiança, quero rejeitar referências fabricadas ou
desalinhadas.

#### AC-1421 — Digest, claim, escopo e status precisam coincidir

- **Dado** um finding e registros de resolver.
- **Quando** uma referência não tem registro, tem digest/status divergente ou
  o registro pertence a outro claim/identidade.
- **Então** o binding falha fechado sem promover o claim.

### US-1422 — Preservar estados de evidência

Como consumidor de planejamento, quero distinguir prova válida, stale,
conflitante e insuficiente.

#### AC-1422 — Apenas evidência verificada pode liberar mitigação

- **Dado** evidência `VERIFIED`, `UNVERIFIED`, `STALE` ou `CONFLICTING`.
- **Quando** o finding é projetado.
- **Então** o estado e as métricas correspondem ao resolver e somente
  `VERIFIED` pode manter `MITIGATE`.

### US-1423 — Manter lifecycle bounded

Como boundary de integração, quero replay determinístico e cancelamento sem
efeitos.

#### AC-1423 — Replay e cancelamento são seguros

- **Dado** a mesma entrada, uma entrada cancelada ou registros não referidos.
- **Quando** o binding é processado.
- **Então** o resultado é idempotente, cancelamento não produz claim e
  evidência órfã é rejeitada.

### US-1424 — Versionar e observar o binding

Como operador do Harness, quero schema bounded e métricas de estado.

#### AC-1424 — Schema desconhecido e limites falham fechados

- **Dado** payload com versão incompatível, campo desconhecido ou input acima
  dos limites.
- **Quando** lido ou construído.
- **Então** o payload é rejeitado e o resultado válido expõe contadores por
  estado sem criar autoridade operacional.

## Fora de escopo

- resolvers concretos de filesystem, Git, PR ou CI;
- persistência, UI, scheduler, providers, secrets e efeitos externos;
- importação de texto de reviewer como fato;
- E2E adversarial do pipeline completo, reservado para a PR-392.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Binding puro em `agent-core`, contrato versionado e bounded, mapping de
resolver com identidade exata, estados negativos, métricas, testes de
contrato, documentação e verificação/auditoria ONP passando.
