# Prereleases testáveis

## Política

- Alteração **funcional** integrada em `main`: uma prerelease independente por commit, no formato `vMAJOR.MINOR.PATCH-dev.<SHA completo>`.
- Alteração de **documentação**, **CI** ou **dependência**: não publica por padrão; somente publica se a política do repositório for habilitada no workflow.
- Release **estável**: não é criada automaticamente; exige marco de produto explicitamente definido e disparo manual do workflow de promoção.
- Checks pendentes, falhos ou ausentes bloqueiam a publicação.
- O pipeline não considera dry-run como release nem como prova de publicação.
- Tags são imutáveis: nenhuma tag existente é sobrescrita ou reutilizada.

## Ordem fail-closed

1. O workflow roda somente para `main` e faz checkout do SHA exato.
2. Confirma que o commit é o `main` atual e possui PR relacionada na API do GitHub.
3. Verifica a mesma versão em `Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`, `frontend/package.json`, `tauri.conf.json`, `release-manifest.json` e `frontend/src/version.ts`.
4. Aguarda todos os checks pós-merge obrigatórios concluírem com `success`.
5. Calcula a tag determinística usando o SHA completo e recusa tags existentes.
6. Gera changelog, instruções, hashes, archive e manifesto imutável depois de
   incorporar o instalador Windows produzido pelo job nativo. O manifesto registra
   `artifactDigests` para o archive e o `.exe`; a publicação falha se qualquer digest
   não corresponder ao arquivo baixado.
7. Somente o job `publish` possui `contents: write`; os jobs de preflight e package são read-only.
8. Publica com `gh release create --prerelease --target <SHA>` e lê de volta tag, target e flag prerelease.
9. Em rerun, um release existente só vira no-op se target e manifesto forem idênticos. Tag órfã ou divergente falha.

## Milestones e promoção estável

O mapa versionado em `release-milestones.json` é a fonte da associação entre milestone e versão:

- M0–M2 → `v0.1.0`
- M3–M4 → `v0.2.0`
- M5–M6 → `v0.3.0` (released)
- M7–M8 → `v0.4.0`
- M9 → `v0.5.0`
- M10–M11 → `v0.6.0`
- M12 → `v0.7.0`
- M13 → `v0.8.0`
- M14–M15 → `v0.9.0`
- M16 → `v1.0.0` (ativo)

Para a milestone ativa, `release-milestones.json.active.releaseBoundary` declara o tag estável anterior (`v0.3.0`) e o cartão lógico (`PR-270`). O preflight exige que o tag anterior seja estável e ancestral, calcula PRs e changelog no range completo `previousStableTag..HEAD`, e valida que o cartão declarado esteja mergeado e ancestral. Ele não usa uma janela fixa de commits nem escolhe o primeiro PR retornado pela API.

Após a prerelease correspondente passar pelos checks obrigatórios, o mantenedor deve disparar manualmente `Publish stable milestone release`, informando a tag prerelease exata, a versão e o milestone. O workflow valida o commit e o manifesto, transforma os nomes dos artefatos para a tag estável e publica `prerelease: false`. Não existe promoção automática, seleção implícita de milestone ou sobrescrita de tag.

Exemplo:

```bash
gh workflow run release-milestone.yml --ref main \
  -f prerelease_tag=v1.0.0-dev.<SHA> \
  -f stable_version=1.0.0 \
  -f milestone=M16
```

## Teste de uma prerelease

Baixe `hank-<tag>.tar.gz`, `hank-<tag>-setup.exe`, `release-manifest.json`,
`manifest.sha256` e `SHA256SUMS` da página da release. Verifique os hashes dos dois
artefatos e o manifesto (o workflow de promoção repete essa leitura antes de aceitar
uma prerelease), confirme `provenance.exactCommit` e `provenance.source == "main"`,
depois execute:

```bash
cargo test --workspace --locked
npm --prefix frontend ci
npm --prefix frontend test
```

O workflow também executa `Clean-room Windows install smoke` e `Clean-room Linux install
smoke` em runners novos: confere os hashes/proveniência do download, instala o NSIS ou
abre o AppImage em diretório temporário, valida o WebView nativo, encerra e limpa. Os
relatórios são publicados como artifacts da execução.
Upgrade/rollback do aplicativo ainda não possuem updater integrado; permanecem
`NO_PROOF` e não são apresentados como suporte.

A página e o manifesto informam explicitamente que a versão é prerelease e não estável. O manifesto também registra cartão lógico `PR-xxx`, PRs relacionadas, classificação e instruções de teste.

## Rollback

Rollback não é automático. Para remover uma release, um operador deve revisar o plano gerado por `buildRollbackPlan` contendo tag, release ID e SHA e fornecer aprovação explícita. A pipeline nunca deleta estado remoto silenciosamente.
