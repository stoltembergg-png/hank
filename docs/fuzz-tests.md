# Bounded fuzz tests

A lane deterministic smoke exercise of seven registered targets (`FT-001` through
`FT-007`) over synthetic, redacted corpus bytes. The targets exercise the harness
contract and do not claim coverage of production parsers. It is not an exhaustive
fuzzing claim and does not contact providers, production services, or credential stores.

## Execution

```bash
cargo fetch --locked
CARGO_NET_OFFLINE=true node tools/security/fuzz-runner.mjs
CARGO_NET_OFFLINE=true node --test tools/security/fuzz-tests.spec.mjs
```

The runner validates `docs/security/fuzz-manifest.json`, including the manifest
revision, target order/kinds, parser metadata, registry contract, and SHA-256
digest of the runner source. Rust dependencies are fetched once, then the
contract and Node lane run with Cargo offline and `--locked`.

The harness bounds smoke iterations and input allocation, detects returned
iterations that exceed the configured elapsed budget, catches panics, and fails
closed on zero iterations. It deliberately does not inspect host memory or kill
arbitrary processes. The Node runner adds a 120-second watchdog around the
Cargo contract subprocess and records timeout classification in the report.
A non-returning target is therefore bounded by the runner and CI job timeouts;
the report therefore records an immutable snapshot digest rather than a wall
clock timestamp.

The report is written to `security/reports/fuzz.json` and contains the Git tree
and head SHA, a digest of staged/unstaged diffs, runner digest, manifest
revision, exact contract-test count, and failed-test names. Cargo stdout/stderr
and timestamps are not persisted.
