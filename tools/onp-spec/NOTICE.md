# Vendored ONP Spec snapshot

This directory contains the exact `onp-spec-driven` v3.6.0 tool snapshot used by
Hank's clean-room SDD verification. The snapshot source was the installed Hermes
skill available during this preparation. The source directory had no Git remote or
release manifest, so this repository records per-file SHA-256 hashes in `manifest.json`
and intentionally does not claim upstream release provenance.

`tools/ci/run-onp-spec.mjs` verifies the manifest before importing the tool. Missing,
changed, unsafe or unexpected tool files fail closed. Updating the snapshot requires a
separate dependency/provenance review and a new manifest.
