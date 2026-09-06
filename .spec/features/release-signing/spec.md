# Spec: release signing

> feature: release-signing
> status: implementada

## Histórias

### US-2660 — Provar artefatos de release

Como mantenedor, quero verificar assinaturas e provenance independentemente do produtor,
para rejeitar artefatos substituídos ou com identidade divergente.

#### AC-2661 — tuple válida verifica

- **Dado** um attestation Ed25519 válido
- **Quando** o verificador recebe a chave pública independente
- **Então** a tupla exata de artefato e identidade é aceita.

#### AC-2662 — digest substituído rejeita

- **Dado** um artefato diferente
- **Quando** o digest não corresponde à assinatura
- **Então** a prova é rejeitada.

#### AC-2663 — identidade divergente rejeita

- **Dado** commit, tree, canal ou policy divergente
- **Quando** a prova é verificada
- **Então** ela falha fechado.

#### AC-2664 — signer revogado rejeita

- **Dado** um signer não permitido
- **Quando** a prova é verificada
- **Então** ela não autoriza o artefato.

#### AC-2665 — prova incompleta rejeita

- **Dado** metadata ausente, malformada ou schema desconhecido
- **Quando** ela é verificada
- **Então** ela é rejeitada.

#### AC-2666 — limites são aplicados

- **Dado** campo de identidade acima do limite
- **Quando** o attestation é verificado
- **Então** o attestation é rejeitado.

#### AC-2667 — digest é determinístico

- **Dado** o mesmo conteúdo
- **Quando** o digest é calculado mais de uma vez
- **Então** o digest SHA-256 é idêntico.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Não-escopo

Nenhuma chave real, publicação, auto-update, ou assinatura de artefato de produção é executada.
