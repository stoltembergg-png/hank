# Tasks: distribution gates

> feature: distribution-gates

## T-3009 — Modelar avaliador deterministic e catálogo de evidência [concluida]

- Refs: US-3004, AC-3814, AC-3815, AC-3819, AC-3820, AC-3821
- Arquivos: `tools/distribution-gates.mjs`
- Notas: avaliador offline fail-closed com `requiredEvidence`, `buildEvidenceRecord` e
  `evaluateDistribution`; `NO_GO`/`ELIGIBLE`; rejeita `aiApproval`; digest determinístico.

## T-3010 — Provar NO_GO/ELIGIBLE, identidade, channel e limites com fixture offline [concluida]

- Refs: US-3004, AC-3816, AC-3817, AC-3818
- Arquivos: `test/distribution-gates.test.mjs`
- Notas: fixture determinística não usa rede, signer, secret ou publicação real.

## T-3011 — Registrar fronteira de produção e executar verify explícito [concluida]

- Refs: US-3004, AC-3815, AC-3819, AC-3820
- Arquivos: `.spec/features/distribution-gates/spec.md`, `.spec/features/distribution-gates/tasks.md`, `onpspec.config.json`, `docs/distribution-gates.md`
- Notas: `CONTRACT PASS` não é prova de publicação; decisão protegida/canary permanecem posteriores.

## Suposições

- ASM-2108 está registrada na especificação: publicação protegida e canary automatizado são trabalho posterior.