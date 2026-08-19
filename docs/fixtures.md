# Test fixture catalog

The `test-support::fixtures` module is dev-only and offline. It provides:

- `FixtureCase`: synthetic bounded payload, explicit version and seed;
- deterministic FNV-1a manifest hash over canonical JSON;
- `FixtureWorkspace`: isolated directory owner with read/write and Drop cleanup.

Fixtures must use synthetic data, remain below 64 KiB, avoid secrets and be reproducible
from `(id, version, seed, payload)`. Tests must assert lifecycle/cleanup and malformed
input behavior. Production crates must not depend on fixture state or fixture paths.

Changing the fixture schema or hash algorithm requires a new version and a focused
compatibility note; existing fixtures are not silently rewritten.
