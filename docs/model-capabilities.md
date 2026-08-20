# Model Capability Schema contract

`provider-core::capabilities` defines the stable, provider-neutral capability report consumed before adapter execution. It does not perform discovery, network calls, provider mapping, or model selection UI.

## Schema

`CapabilityReport` contains:

- schema version;
- opaque `ProviderId` and `ModelId`;
- bounded capability version and source (`provider`, `cache`, `unknown`);
- modality states for text/image/audio/video;
- feature states for streaming/tool use/vision/audio input;
- optional bounded context/output limits.

Each capability is explicitly `supported`, `unsupported`, or `unknown`. Missing entries resolve to `unknown`.

## Compatibility semantics

`CapabilityReport::check_compatibility` rejects before an adapter when:

- a required modality is unsupported or unknown;
- a required feature is unsupported or unknown;
- context/output limits are insufficient.

Unknown is never treated as supported. Errors are typed and include no endpoint, key, credential, prompt, or token payload.

## Bounds and determinism

- Schema version is pinned to 1;
- At most four modalities and sixteen features are stored;
- Capability version is non-empty, control-free, and max 64 characters;
- Context limit is max 2,000,000 tokens;
- Output limit is max 1,000,000 tokens;
- `BTreeMap`/`BTreeSet` provide deterministic serialization and comparison.

## Tests

`crates/provider-core/tests/capability_contract.rs` covers:

- deterministic serde roundtrip;
- supported vs unknown semantics;
- unsupported/unknown compatibility rejection;
- typed feature and limit incompatibility;
- malformed schema/version/oversized limit fail-closed behavior.

## ONP mapping

- T-350 — Definir schema de capabilities de modelo [concluida]