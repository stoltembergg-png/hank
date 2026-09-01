# Spec: skill version benchmark comparison

> feature: benchmark-comparison
> status: auditada

## Contexto

Esta feature compara exatamente uma skill baseline imutável com uma candidata
em uma suíte nativa canônica. O comparador reutiliza os mesmos fixtures,
policy, modelo, orçamento, timeout e autoridade de ferramentas; training e
holdout são derivados do split congelado da suíte, e não escolhidos pelo
chamador. O resultado é evidência bounded e não possui caminho de promoção,
ativação ou alteração de ponteiro.

### US-1491 — Comparar runs comparáveis

Como mantenedor do Harness, quero comparar uma baseline e uma candidata com
identidade e autoridade comuns para observar deltas reproduzíveis.

#### AC-1491 — Training e holdout produzem deltas determinísticos

- **Dado** um run baseline canônico e um run candidate compatível.
- **Quando** o comparador recebe ambos e uma revisão independente.
- **Então** produz deltas bounded separados em training e holdout, com SHA,
  árvore, policy, schema, fixture, ambiente e digests dos runs.

### US-1492 — Bloquear regressões

Como mantenedor, quero que regressões de qualidade, segurança, custo,
latência ou ferramentas permaneçam explícitas.

#### AC-1492 — Threshold excedido torna o resultado REGRESSION

- **Dado** um delta acima do threshold configurado ou uma mudança de terminal
  no holdout.
- **Quando** a comparação é concluída.
- **Então** o resultado é `Regression`, preserva a baseline e nunca autoriza
  promoção ou ativação.

### US-1493 — Impedir benchmark escolhido pela candidata

Como auditor, quero que o split e a identidade da suíte sejam canônicos para
impedir seleção conveniente ou overfitting.

#### AC-1493 — Subset, holdout adulterado e ambiente incomparável falham fechado

- **Dado** run parcial, case desconhecido, holdout adulterado ou policy/modelo
  divergente.
- **Quando** a comparação é solicitada.
- **Então** ela falha fechado antes de produzir evidência comparável.

### US-1494 — Exigir revisão independente

Como operador, quero que o artifact de comparação seja vinculado a uma
revisão independente e a um schema versionado.

#### AC-1494 — Revisão ausente, autoaprovação ou schema desconhecido não passam

- **Dado** revisão ausente, reviewer igual à candidata ou payload com campo
  desconhecido/digest adulterado, assinatura inválida ou policy diferente da
  policy revisada.
- **Quando** o artifact é validado.
- **Então** a validação rejeita o artifact sem conceder autoridade; a
  assinatura Ed25519 deve ser verificada por uma chave pública confiável e um
  report desserializado só passa como evidência depois de ser recomputado
  contra os runs exatos.

## Fora de escopo

- Promoção, ativação, rollout, rollback ou mutação de ponteiro.
- Execução de provider, rede, secrets, ferramentas reais ou filesystem de
  produção.
- Escolha de cases pelo modelo ou uso de texto da candidata como autoridade.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

O comparador nativo aceita somente runs bounded e canônicos, deriva o split
training/holdout, valida identidade compartilhada, reporta deltas e
regressões por métrica, exige revisão independente, rejeita subset/drift/
autoaprovação/schema desconhecido, limita thresholds, exige revisão assinada,
revalida reports contra os runs-fonte exatos e passa nos testes focais e no
verify/audit ONP sem ativar qualquer candidata.
