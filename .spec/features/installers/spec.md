# Spec: installers

> feature: installers
> status: implementada

## Histórias

### US-2670 — Instalar desktop com identidade e preservação

Como usuário, quero instalar e remover o desktop sem perder meu perfil,
para que a migração e os artefatos permaneçam verificáveis.

#### AC-2671 — matriz de plataformas declarada

- **Dado** os sistemas e arquiteturas suportados pelo plano
- **Quando** o manifest de instaladores é validado
- **Então** cada alvo tem formato, política de perfil e sidecars bounded.

#### AC-2672 — clean install bounded

- **Dado** um artefato com digest válido
- **Quando** a instalação limpa é simulada
- **Então** somente os caminhos app/profile declarados são criados.

#### AC-2673 — identidade divergente rejeita

- **Dado** plataforma ou digest diferente do manifest
- **Quando** o instalador é preparado
- **Então** a instalação falha antes de criar o app.

#### AC-2674 — uninstall preserva perfil

- **Dado** uma instalação válida com perfil existente
- **Quando** o uninstall é executado
- **Então** os arquivos do app são removidos e o perfil permanece.

#### AC-2675 — paths são canonicalizados

- **Dado** metadata com path absoluto ou traversal
- **Quando** o pacote é validado
- **Então** o path é rejeitado fail-closed.

#### AC-2676 — metadata insegura rejeita

- **Dado** metadata que executa shell ou embute segredo
- **Quando** o manifest é validado
- **Então** o manifest é rejeitado.

#### AC-2677 — migração antecede uso

- **Dado** um perfil que exige migração
- **Quando** o pacote é instalado
- **Então** o perfil é preservado e a política exige migração antes do uso.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Não-escopo

Publicação, suporte de produção, atualização automática, endpoints, assinatura real e remoção do perfil.
