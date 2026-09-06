# Tasks: installers

> feature: installers
> card: PR-267

## T-2671 — Implement bounded installer contract [concluída]

- Refs: US-2670, AC-2671, AC-2672, AC-2673, AC-2674, AC-2675, AC-2676, AC-2677
- Arquivos: tools/installer-contract.mjs, tools/installer-contract.spec.mjs, docs/installer-manifest.json
- Testes: matrix, clean install, digest/platform identity, uninstall preservation, paths and migration.

## T-2672 — Document installer boundary [concluída]

- Refs: AC-2671, AC-2674, AC-2676, AC-2677
- Arquivos: docs/installer-tests.md
- Testes: security policy and support classification.

## T-2673 — Integrate installer contract into CI and ONP [concluída]

- Refs: AC-2671, AC-2672, AC-2673, AC-2675, AC-2676
- Arquivos: tools/security/installer-feature-tests.mjs, tools/security/bind-installer-evidence.mjs, .github/workflows/ci-installers.yml, .github/workflows/onp-sdd-evidence.yml, onpspec.config.json
- Testes: strict TAP runner, workflow integrity and ONP verify.
