# Contract evidence scope

This document is the human-readable boundary for the three M16 increments that were
implemented as offline contracts.

| Card | Contract result | Production result | Honest interpretation |
| --- | --- | --- | --- |
| PR-261 / #447 | `PASS` | `NO_PROOF` | Bounded fuzz harness over synthetic targets; not fuzzing of the production parser. |
| PR-266 / #452 | `PASS` | `NO_PROOF` | Ed25519 verifier contract with ephemeral synthetic fixtures; no real release signing or publication. |
| PR-267 / #453 | `PASS` | `NO_PROOF` | Installer identity and simulated state contract; every declared platform remains `contract-only`. |

The machine-readable authority is [`docs/evidence-scope-manifest.json`](evidence-scope-manifest.json).
The contract runner is `node tools/security/evidence-scope-feature-tests.mjs`.

## Visual evidence policy

The dedicated check generates `security/reports/evidence-scope.svg` and
`security/reports/evidence-scope.html` from the actual TAP result and exact Git commit/tree.
The workflow uploads them as an artifact and writes a link in the GitHub check summary.
The image is a visual proof of the **contract run**, not of the application running in
production. It intentionally displays `NOT PRODUCTION PROOF`.

A screenshot or screen recording of the desktop application is not an acceptable substitute
for the missing evidence here: PR-261, PR-266 and PR-267 do not execute a production parser,
real signing ceremony, native installer, launch or uninstall. Those claims require new,
platform- or environment-specific increments with real CI execution and evidence bound to
their exact SHA/tree.

## Evidence states

- `PASS`: the declared bounded contract executed without skipped tests.
- `NO_PROOF`: the corresponding production behavior was not executed or cannot be tied to the exact SHA/tree.
- `STALE`: a stored result belongs to another source identity and must not be reused.
