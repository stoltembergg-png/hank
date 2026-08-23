# Spec: Release prereleases

> feature: release-prereleases
> status: implementada

## Contexto

A plataforma precisa publicar prereleases reais, baixáveis e reproduzíveis depois de cada atualização funcional integrada em `main`, sem transformar uma execução manual em falsa evidência de release.

## Histórias

### US-602 — Prerelease reproduzível após integração

Como mantenedor, quero que uma alteração funcional integrada em `main` gere uma prerelease imutável, para que usuários possam testar exatamente o commit que passou pelos gates.

#### AC-624 — Tag determinística e SemVer

- **Dado** uma versão estável válida e um commit completo de `main`
- **Quando** a pipeline calcula a prerelease
- **Então** produz `vMAJOR.MINOR.PATCH-dev.<SHA completo>`, válido em SemVer, determinístico e único por commit

#### AC-625 — Consistência de versões

- **Dado** Cargo, crate Tauri, frontend, configuração Tauri, manifesto e versão exibida
- **Quando** o preflight valida os manifestos
- **Então** qualquer divergência interrompe a publicação antes de criar tag

#### AC-626 — Proveniência e checks pós-merge

- **Dado** um commit integrado em `main`
- **Quando** o preflight consulta PRs relacionadas e checks obrigatórios
- **Então** falha se não houver PR, se o commit não for o `main` confirmado ou se qualquer check estiver ausente, pendente ou falho

#### AC-627 — Idempotência e tags imutáveis

- **Dado** uma execução repetida
- **Quando** a tag/release correspondente já existir
- **Então** a pipeline só faz no-op se commit e manifesto forem idênticos; tag órfã, commit divergente ou manifesto divergente falham sem sobrescrever

#### AC-628 — Manifesto, changelog e artefatos baixáveis

- **Dado** um preflight aprovado
- **Quando** a pipeline empacota a release
- **Então** publica manifesto com SHA exato, cartão PR-xxx, PRs relacionadas, classificação, changelog automático, instruções de teste, hashes e artefato baixável, marcado explicitamente como prerelease

### US-603 — Política de publicação controlada

Como mantenedor, quero classificar alterações e restringir permissões, para evitar releases indevidas e manter rollback explícito.

#### AC-629 — Política funcional/documentação/CI/dependência

- **Dado** commits classificados como funcional, documentação, CI ou dependência
- **Quando** a política decide publicar
- **Então** funcional gera prerelease independente; demais categorias só publicam quando a política estiver habilitada; release estável exige marco explícito

#### AC-630 — Permissão mínima e ações fixadas

- **Dado** o workflow de prerelease
- **Quando** a pipeline é analisada
- **Então** apenas o job de publicação possui `contents: write`, os demais são read-only, ações externas estão fixadas por SHA e não há publicação com permissão insuficiente

#### AC-631 — Rollback explícito

- **Dado** uma prerelease publicada com problema
- **Quando** alguém solicita rollback
- **Então** o plano identifica tag, release e SHA exatos, exige aprovação explícita e nunca apaga silenciosamente estado remoto

### US-604 — Promoção explícita de milestone

Como mantenedor, quero promover uma prerelease já validada para o milestone correspondente, para que a versão estável avance de acordo com o roadmap sem seleção automática ou reutilização de artefatos incorretos.

#### AC-632 — Manifesto estável com proveniência preservada

- **Dado** um manifesto de prerelease com tag, commit e árvore completos
- **Quando** a promoção recebe a versão estável e o identificador do milestone
- **Então** converte somente a combinação exata `v<versão>-dev.<SHA>` em `v<versão>`, marca `stable: true`, preserva o SHA e registra a tag de origem
- **E** rejeita versão divergente, manifesto já estável, identidade incompleta ou milestone inválido

#### AC-633 — Workflow manual de promoção fail-closed

- **Dado** uma prerelease publicada e aprovada pelos checks pós-merge
- **Quando** um operador dispara `release-milestone.yml` com tag, versão e milestone
- **Então** o workflow valida o mapeamento versionado, o release prerelease, o commit exato, os hashes e publica uma única release estável com `contents: write`
- **E** não possui gatilho `push`, não sobrescreve tag e é idempotente somente quando release, commit e manifesto coincidem

## Fora de escopo

- Seleção automática de milestone, bump automático de versão ou promoção estável sem ação explícita do operador.
- Assinatura criptográfica, notarização e distribuição por lojas.
- Publicação automática de documentação, CI ou dependências sem política habilitada.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-605 | O GitHub Actions possui `gh`, `jq`, `git` e Node disponíveis no runner Ubuntu. | confirmada | Ferramentas presentes no runner padrão e usadas somente em jobs bounded. |
| ASM-606 | A API de commits do GitHub associa o commit squash de main às PRs relacionadas. | confirmada | Preflight falha fechadamente quando a associação não existe. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-604 | Qual marco de produto autoriza release estável? | respondida | O mapa versionado em `release-milestones.json` associa M0–M2 a v0.1.0, M3–M4 a v0.2.0 e M5–M6 a v0.3.0; a promoção exige `workflow_dispatch` explícito com a combinação correspondente. |
