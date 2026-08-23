# Tasks: Release prereleases

> feature: release-prereleases

## T-625 — Implementar contrato puro de prerelease e manifesto [concluida]

- Refs: US-602, AC-624, AC-625, AC-627, AC-628
- Arquivos: tools/release-prerelease.mjs, test/release-prerelease.js, release-manifest.json, frontend/src/version.ts, frontend/src/App.tsx
- Notas: SemVer, SHA determinístico, consistência de versões, idempotência, manifesto, changelog, instruções e rollback.

## T-626 — Substituir dry-run por workflow real fail-closed [concluida]

- Refs: US-602, AC-626, AC-628, US-603, AC-630
- Arquivos: .github/workflows/release-prerelease.yml
- Notas: Preflight read-only, package read-only, publicação única com contents: write; tag e release verificadas após criação.

## T-627 — Definir classificação e política de publicação [concluida]

- Refs: US-603, AC-629, AC-631
- Arquivos: tools/release-prerelease.mjs, docs/prerelease-releases.md
- Notas: funcional independente; documentação/CI/dependência condicionais; stable somente por marco explícito; rollback requer aprovação.

## T-628 — Versionar milestones e contrato de promoção [concluida]

- Refs: US-604, AC-632
- Arquivos: release-milestones.json, tools/release-prerelease.mjs, test/release-prerelease.js, manifestos de versão
- Notas: mapa de versões alvo, conversão determinística de manifesto, proveniência da prerelease preservada e rejeição de combinações divergentes.

## T-629 — Publicar milestone estável por workflow manual [concluida]

- Refs: US-604, AC-633
- Arquivos: .github/workflows/release-prerelease.yml, .github/workflows/release-milestone.yml, docs/prerelease-releases.md
- Notas: prerelease passa a usar a versão ativa do mapa; promoção separa validação read-only de publicação write-only, sem gatilho automático e com idempotência fail-closed.
