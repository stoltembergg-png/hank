# Spec: claim and evidence contract

> feature: claim-evidence
> status: auditada

## Contexto

O Harness precisa separar uma afirmação (`Claim`) da evidência produzida por
um resolver (`EvidenceRecord`). Texto, memória ou saída de modelo não são
fatos por si só. O contrato abaixo é puro, provider-neutral e limitado ao
domínio; resolvers concretos, persistência, UI e execução ficam em cards
posteriores.

### US-1410 — Modelar claims e evidências com identidade explícita

Como núcleo de domínio, quero representar claims por digest e evidências por
registros vinculados a projeto, run, trace, commit/tree, policy e schema.

#### AC-1410 — Claim inicia sem prova e verificação exige evidência exata

- **Dado** um claim recém-criado.
- **Quando** nenhuma evidência resolver o claim.
- **Então** seu estado é `NO_PROOF`; `VERIFIED` exige registros bounded,
  resolver-identificados, com identidade digest, escopo e tipos requeridos
  compatíveis.

### US-1411 — Aplicar uma máquina de estados fail-closed

Como sistema de evidências, quero transições determinísticas e replay
idempotente.

#### AC-1411 — Transições ilegais falham sem downgrade implícito

- **Dado** um claim em um estado factual.
- **Quando** uma transição não permitida ou um replay é aplicado.
- **Então** a primeira falha determinísticamente e o replay retorna
  `IDEMPOTENT`, sem mutação adicional.

### US-1412 — Representar stale, conflito e ausência de prova

Como consumidor de fatos, quero distinguir evidência stale, conflitante,
unverified e ausência de prova.

#### AC-1412 — Estados não verificados nunca promovem o claim

- **Dado** evidência stale, conflitante ou ausente.
- **Quando** ela é aplicada ao claim.
- **Então** o estado correspondente é preservado e nenhuma promoção para
  `VERIFIED` ocorre; a razão bounded permanece observável como dado.

### US-1413 — Versionar o wire contract sem aceitar campos desconhecidos

Como boundary de integração, quero schema versionado e rejeição de payloads
forjados ou desconhecidos.

#### AC-1413 — Schema, campos desconhecidos e texto de claim são fail-closed

- **Dado** um payload serializado.
- **Quando** sua versão é incompatível, possui campo desconhecido ou tenta
  transformar texto em autoridade factual.
- **Então** o payload é rejeitado; apenas o digest bounded representa o
  conteúdo do claim.

### US-1414 — Impor limites e higiene de dados

Como boundary de segurança, quero rejeitar duplicatas, payloads ilimitados e
segredos em metadata.

#### AC-1414 — Bounds, duplicatas, segredos e digests malformados falham

- **Dado** input acima dos limites, duplicado, sensível ou malformado.
- **Quando** construído ou aplicado.
- **Então** a operação falha sem armazenar o valor bruto sensível.

### US-1415 — Impor escopo e identidade criptográfica declarada

Como consumidor de evidências, quero que claim e registro pertençam ao mesmo
projeto, run, trace, commit e tree.

#### AC-1415 — Evidência foreign ou SHA/tree divergente não é aceita

- **Dado** registro de outro claim, escopo ou SHA/tree.
- **Quando** usado para resolver um claim.
- **Então** a operação falha com `IDENTITY_MISMATCH` ou `CLAIM_MISMATCH`.

### US-1416 — Manter o contrato sem autoridade operacional

Como arquitetura do Harness, quero que fatos sejam dados auditáveis e não
atalhos para executar, aprovar ou fazer merge.

#### AC-1416 — Roundtrip preserva o contrato, mas não cria autoridade

- **Dado** Claim, EvidenceScope, EvidenceRecord e ClaimResolution válidos.
- **Quando** serializados e lidos novamente.
- **Então** a identidade é preservada e os caminhos de execução, aprovação e
  merge continuam indisponíveis.

## Fora de escopo

- resolvers concretos de filesystem, Git, PR ou CI;
- persistência, UI, scheduler, providers, secrets e autorização humana;
- importação de texto externo como fato;
- adaptação de `ReviewerFinding`, reservada para a PR-391.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Contrato puro em `agent-core`, schema versionado, estados e transições
bounded, testes positivos e negativos, documentação e verificação/auditoria
ONP passando.
