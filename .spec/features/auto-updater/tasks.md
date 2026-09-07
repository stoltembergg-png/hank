# Tasks: auto-updater

> feature: auto-updater
> card: PR-268

## T-2681 — Implement signed staged updater contract [concluída]

- Refs: US-2680, AC-2681, AC-2682, AC-2683, AC-2684, AC-2685, AC-2686, AC-2687
- Arquivos: tools/updater-contract.mjs, tools/updater-contract.spec.mjs, docs/updater-manifest.json
- Testes: signature/digest, policy matrix, bounds, consent, staging and profile preservation.

## T-2682 — Document updater boundary [concluída]

- Refs: AC-2681, AC-2683, AC-2686, AC-2687
- Arquivos: docs/updater-tests.md
- Testes: no network, no unattended update, bounded staging.

## T-2683 — Integrate updater contract into CI and ONP [concluída]

- Refs: AC-2681, AC-2682, AC-2683, AC-2684, AC-2685, AC-2686, AC-2687
- Arquivos: tools/security/updater-feature-tests.mjs, .github/workflows/ci-auto-updater.yml, .github/workflows/onp-sdd-evidence.yml, onpspec.config.json
- Testes: strict TAP runner, workflow integrity and ONP verify.
