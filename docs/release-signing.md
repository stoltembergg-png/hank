# Release signing contract

PR-266 defines the release-proof boundary without storing or exercising a real signing secret.

The attestation binds the artifact digest to repository, event, ref, commit, tree, workflow,
policy, channel, operating system, and versioned signer key ID. Ed25519 verification is
independent from the producer and fails closed for malformed, substituted, stale, revoked,
or identity-mismatched evidence.

Private keys are restricted to the protected `release-signing` environment. The prerelease
workflow downloads the exact archive, Windows installer and Linux AppImage, signs each one with
`tools/release-artifact-signing.mjs`, and publishes only after the publish job independently
verifies every attestation against the exact commit/tree and signer key ID. Stable milestone
promotion never reuses prerelease attestations: after the immutable bytes are renamed, the
promotion job signs them again with `channel=stable` and `release-stable-v1`, verifies the trusted
key, and only then publishes the stable manifest. The public key and per-artifact attestations are
released alongside the binaries; the private key is never written to disk, logged, or passed to a
downstream job. The environment must provide
`HANK_RELEASE_SIGNING_PRIVATE_KEY_PEM` as a secret, `HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM` and
`HANK_RELEASE_SIGNER_KEY_ID` as protected variables; missing values or a private/public key mismatch
fail closed.

Fixtures generate an ephemeral synthetic key pair in memory; they never contact a provider,
network, filesystem secret store, or production release endpoint. Local contract tests therefore
prove the signer/verifier behavior but do not prove that the protected environment is configured or
that a production key signed a published artifact. That evidence remains `NO_PROOF` until the
workflow runs successfully with the protected environment and its retained report.
