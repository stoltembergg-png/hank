# Release SBOM contract

Every prerelease package job generates `SBOM.spdx.json` with
`tools/release-sbom.mjs`. The document is SPDX 2.3, deterministic, and built only
from the committed `Cargo.lock`, `frontend/package-lock.json`, and
`desktop-e2e/package-lock.json`. Package records are sorted and carry
`NOASSERTION` for license/download fields when lockfiles do not provide
authoritative licensing metadata; this is not a license approval.

The SBOM namespace and creation comment bind the source commit, source tree, and
release version. The publish and milestone workflows verify that tuple and include
the SBOM in `artifactDigests` and `SHA256SUMS`; a missing, substituted, or stale
SBOM therefore fails closed before publication or promotion.

Local contract evidence proves deterministic generation and negative identity
checks. It does not prove that a protected GitHub release was published; that
remains `NO_PROOF` until the protected workflow produces retained artifacts.

To verify a downloaded SBOM:

```bash
node tools/release-sbom.mjs verify --file SBOM.spdx.json \
  --commit <exact-commit-sha> --tree <exact-tree-sha> --version <release-version>
```
