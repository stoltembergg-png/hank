# Spec: distribution gates

> feature: distribution-gates
> status: implementada

## História de usuário

### US-3004 — Agregar evidência de release em uma decisão de distribuição fail-closed

Como operador de release, quero agregar evidência de segurança, compatibilidade,
signing, installer, updater, backup/migração/secret e recovery em um avaliador
determinístico, para que nenhum artifact seja distribuído sem prova completa, vinculada
e atual, e para que a decisão de distribuição só possa ser tomada por humano/protected.

#### AC-3814 — Toda categoria de evidência exigida precisa estar presente

- **Dado** um conjunto de evidência com qualquer categoria exigida ausente
- **Quando** o avaliador processa
- **Então** retorna `NO_GO` com o motivo da categoria faltante.

#### AC-3815 — Tuple all-success produz apenas eligible-for-human-decision

- **Dado** todas as categorias exigidas com `status=success` e identidade correta
- **Quando** o avaliador processa
- **Então** retorna `ELIGIBLE`, nunca autorizando publicação por si (`authorized=false`).

#### AC-3816 — Evidência failed/skipped/cancelled/timed-out/queued produz NO_GO

- **Dado** qualquer registro com status diferente de `success`
- **Quando** o avaliador processa
- **Então** retorna `NO_GO` com o status não-success.

#### AC-3817 — Identidade errada (repo/ref/sha/tree) produz NO_GO

- **Dado** qualquer registro com repository, ref, sha ou tree divergente do alvo
- **Quando** o avaliador processa
- **Então** retorna `NO_GO`.

#### AC-3818 — Mismatch de channel ou platform produz NO_GO

- **Dado** um registro com channel diferente do alvo
- **Quando** o avaliador processa
- **Então** retorna `NO_GO`.

#### AC-3819 — Evidência malformada ou acima do limite é rejeitada

- **Dado** registro com categoria desconhecida, sha inválido ou campo acima do limite
- **Quando** construído/avaliado
- **Então** rejeita (throw na construção; `NO_GO` fail-closed na avaliação).

#### AC-3820 — Saída de IA/reviewer é ignorada pelo avaliador

- **Dado** entrada `aiApproval` (prosa de IA/reviewer)
- **Quando** o avaliador processa
- **Então** rejeita a entrada; apenas evidência estruturada é considerada.

#### AC-3821 — Tuple elegível é determinístico e reprodutível

- **Dado** o mesmo conjunto de evidência elegível
- **Quando** avaliado duas vezes
- **Então** produz o mesmo veredito e o mesmo `reportDigest` (40 hex).

## Segurança e fronteira de prova

- O avaliador é offline e determinístico; não publica, não assina e não acessa credenciais.
- `ELIGIBLE` nunca autoriza publicação: apenas habilita uma decisão humana/protected.
- Identidade (repo/ref/sha/tree/channel) e tamanho de campos são revalidados a cada
  avaliação (defesa em profundidade), não apenas na construção.
- Esta feature prova somente o contrato do avaliador. A distribuição real permanece `NO_PROOF`.

## Suposições

- ASM-2108: a publicação protegida e a decisão canary automatizada são trabalho posterior;
  este card entrega só o avaliador fail-closed e o catálogo de evidência.

## Perguntas em aberto

Nenhuma.