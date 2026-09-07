# Updater contract — PR-268

The updater contract accepts only signed metadata for an allowlisted channel/platform and
stages the verified artifact before installation. It rejects bad signature/digest, channel,
platform, downgrade, expiry, size, and revoked signer evidence.

The fixture is offline and synthetic. Explicit user consent is mandatory; unattended rollout,
real endpoints, network downloads, key storage, and rollback execution are out of scope.
Staging is bounded and preserves the current version/profile when validation or staging fails.
