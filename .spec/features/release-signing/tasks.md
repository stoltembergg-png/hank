# Tasks: release-signing

> feature: release-signing
> card: PR-266

## T-2661 — Implement release attestation contract [concluída]

- Refs: US-2660, AC-2661, AC-2662, AC-2663, AC-2664, AC-2665, AC-2666, AC-2667
- Arquivos: tools/release-signing.mjs, tools/release-signing.spec.mjs
- Testes: Ed25519 positive/negative verification, identity binding and bounded digest.

## T-2662 — Document signing custody and manifest [concluída]

- Refs: AC-2661, AC-2663, AC-2664, AC-2665
- Arquivos: docs/release-signing-manifest.json, docs/release-signing.md
- Testes: manifest policy and protected key boundary.

## T-2663 — Add release signing CI and ONP evidence [concluída]

- Refs: AC-2661, AC-2662, AC-2663, AC-2664, AC-2665, AC-2666, AC-2667
- Arquivos: tools/security/release-signing-feature-tests.mjs, .github/workflows/ci-release-signing.yml, .github/workflows/onp-sdd-evidence.yml, onpspec.config.json
- Testes: bounded TAP runner, workflow integrity and ONP verify.
