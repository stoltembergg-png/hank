# Release signing contract

PR-266 defines the release-proof boundary without storing or exercising a real signing secret.

The attestation binds the artifact digest to repository, event, ref, commit, tree, workflow,
policy, channel, operating system, and versioned signer key ID. Ed25519 verification is
independent from the producer and fails closed for malformed, substituted, stale, revoked,
or identity-mismatched evidence.

Private keys are restricted to protected release environments. Fixtures generate an ephemeral
synthetic key pair in memory; they never contact a provider, network, filesystem secret store,
or production release endpoint. This contract does not claim a real artifact was signed or
published.
