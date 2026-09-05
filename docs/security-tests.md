# Security tests (PR-260)

A camada de testes de regressão de segurança é composta por três
artefatos canônicos, todos bounded e fail-closed:

- `docs/security/threat-regression-manifest.json` — fonte única de
  `TM-NNN` e `NEG-NNN`. Mudar o manifest é a única forma de adicionar,
  remover ou reclassificar uma ameaça.
- `tools/security/threat-regression.mjs` — runner puro em Node, sem
  efeitos externos, que produz `security/reports/threat-regression.json`
  com `tree_sha`, `head_sha`, `manifest_revision`, `runner_revision`,
  `artifact_digest` e `summary`.
- `crates/security-core/tests/security_regression_contract.rs` — suíte
  Rust que executa as boundaries puras (IPC, remote, filesystem,
  secrets, plugin, evidence, release) contra os fixtures TM-001..TM-007.

O workflow `.github/workflows/ci-security.yml` executa os runners em
`ubuntu-24.04` com `pull_request` + `push: main` e `permissions: contents: read`.

## Limites

| Limite | Valor |
| --- | --- |
| `MAX_THREATS` | 128 |
| `runner timeout` | 10 min |
| `runner artifacts` | 1 MB |
| `negative_fixtures` | 16 |
| `TM-NNN format` | `^TM-\d{3}$` |
| `NEG-NNN format` | `^NEG-\d{3}$` |
| `AC-NNNN format` | `^AC-\d{4}$` (cards 2101..2107) |

## Falhas tipadas

- `MANIFEST_MISSING` — `docs/security/threat-regression-manifest.json` ausente.
- `MANIFEST_INVALID` — schema inválido, `TM-NNN` duplicado, etc.
- `MANIFEST_ORPHAN` — `TM-NNN` sem `AC-NNNN` ou `AC-NNNN` sem tag.
- `MANIFEST_STALE` — `revision` divergente.
- `NEG_FAIL` — `NEG-NNN` produz `match` quando esperado `no-match`
  (ou vice-versa).
- `NEG_BASELINE_GREW` — `security/advisory-baseline.json` adiciona
  exceções sem `mitigation_status` explícito.

## Execução local

```bash
node tools/security/threat-regression.mjs \
  --out security/reports/threat-regression.json
node --test tools/security/threat-regression.spec.mjs
CARGO_BUILD_JOBS=1 cargo test -p security-core \
  --test security_regression_contract --locked --offline
```

## Integração ONP

- `node tools/ci/run-onp-spec.mjs verify security-tests` → 7/7 ACs.
- Step `Verify security tests` em `.github/workflows/onp-sdd-evidence.yml`.
