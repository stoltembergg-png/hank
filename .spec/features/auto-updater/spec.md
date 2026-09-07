# Spec: auto updater

> feature: auto-updater
> status: implementada

## Histórias

### US-2680 — Atualizar com prova e consentimento

Como usuário, quero que atualizações sejam verificadas e staged antes da instalação,
para preservar a versão atual e meu perfil quando a prova falhar.

#### AC-2681 — metadata assinada stageia
- **Dado** metadata assinada para canal e plataforma permitidos
- **Quando** há consentimento explícito
- **Então** o artefato é validado e stageado antes da instalação.

#### AC-2682 — assinatura ou digest inválido bloqueia
- **Dado** assinatura ou digest divergente
- **Quando** o updater verifica a metadata
- **Então** o staging é bloqueado.

#### AC-2683 — canal ou plataforma divergente bloqueia
- **Dado** canal, OS ou arquitetura incompatível
- **Quando** a metadata é avaliada
- **Então** o update é rejeitado.

#### AC-2684 — downgrade bloqueia
- **Dado** versão menor ou igual à mínima/current
- **Quando** o update é avaliado
- **Então** o downgrade é rejeitado.

#### AC-2685 — expiry e tamanho são bounded
- **Dado** metadata expirada ou acima do limite
- **Quando** o update é avaliado
- **Então** ele é rejeitado.

#### AC-2686 — consentimento e perfil
- **Dado** um perfil existente e ausência de consentimento
- **Quando** o update é solicitado
- **Então** nada é stageado e o perfil permanece intacto.

#### AC-2687 — falha de staging é segura
- **Dado** escrita parcial ou tamanho divergente
- **Quando** o staging falha
- **Então** o artefato parcial não é aceito e a versão atual permanece executável.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Não-escopo

Download real, publicação, unattended rollout, endpoint arbitrário, armazenamento de chave e rollback completo.
