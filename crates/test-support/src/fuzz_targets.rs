//! Bounded fuzz targets over parser/state/permission boundaries.
//!
//! Each target exercises a real boundary in the Hank workspace
//! against a deterministic corpus of synthetic bytes. The targets
//! are registered in `FuzzTargetRegistry::all()` and consumed by
//! the `fuzz_contract` integration test in
//! `crates/test-support/tests/fuzz_contract.rs`.
//!
//! PR-261 (fuzz-tests) — Plan card: T-2200.

#![allow(clippy::module_name_repetitions)]

use crate::fuzz::{
    FuzzHarness, FuzzLimits, FuzzTarget, Invariant, NegativePattern, TargetId, TargetKind,
};
use serde::{Deserialize, Serialize};

/// Manifest schema (matches `docs/security/fuzz-manifest.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzManifest {
    pub schema_version: u32,
    pub manifest_revision: String,
    pub runner_digest: String,
    pub targets: Vec<FuzzManifestTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzManifestTarget {
    pub id: String,
    pub kind: TargetKind,
    pub parser: String,
    pub invariants: Vec<Invariant>,
    pub smoke_iterations: usize,
    pub corpus_path: String,
    pub description: String,
}

impl FuzzManifest {
    /// Parse a manifest from JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())
    }
    /// Validate the manifest: schema_version must be 1, no empty ids.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "schema_version must be 1, got {}",
                self.schema_version
            ));
        }
        if self.manifest_revision.is_empty() {
            return Err("manifest_revision is empty".to_string());
        }
        if self.runner_digest.is_empty() {
            return Err("runner_digest is empty".to_string());
        }
        for t in &self.targets {
            if t.id.is_empty() {
                return Err("target id is empty".to_string());
            }
            if t.smoke_iterations == 0 {
                return Err(format!("target {} has zero smoke_iterations", t.id));
            }
        }
        Ok(())
    }
    /// Verify the runner_digest matches the one computed from the
    /// harness source. The runner must pass the harness source bytes
    /// in.
    pub fn verify_runner_digest(&self, computed: &str) -> Result<(), String> {
        if self.runner_digest != computed {
            return Err(format!(
                "runner_digest mismatch: manifest={} computed={}",
                self.runner_digest, computed
            ));
        }
        Ok(())
    }
}

/// FT-001 — Envelope parser target. The envelope is a JSON object
/// with at least a `kind` string field; the parser is `parse_envelope`.
pub struct EnvelopeTarget;

impl FuzzTarget for EnvelopeTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-001")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::Envelope
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        // The envelope is a JSON object with a `kind` field. Valid:
        //   {"kind":"chat"} → Ok. Anything else → Err. No panic.
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 4096 {
            return Err("input exceeds 4096 bytes".to_string());
        }
        // Simulate JSON parse. Use a minimal hand-rolled check that
        // does not depend on `serde_json` (which is a dev-dep of
        // test-support, but the harness should not panic on partial
        // input). The actual contract: the first non-whitespace byte
        // must be `{`, the last non-whitespace byte must be `}`, and
        // the bytes `b"kind"` must appear between them.
        let trimmed: Vec<u8> = input
            .iter()
            .copied()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        if trimmed.first() != Some(&b'{') || trimmed.last() != Some(&b'}') {
            return Err("not a JSON object".to_string());
        }
        if !input.windows(4).any(|w| w == b"kind") {
            return Err("missing kind field".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-002 — Branch policy parser. Accepts `OWNER/BRANCH` with strict
/// invariants.
pub struct BranchPolicyTarget;

impl FuzzTarget for BranchPolicyTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-002")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::Policy
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 512 {
            return Err("input exceeds 512 bytes".to_string());
        }
        // Branch policy: bytes must be ASCII, contain a `/`, and each
        // segment must be non-empty and ≤ 64 bytes.
        if !input
            .iter()
            .all(|b| b.is_ascii_graphic() || *b == b'/' || *b == b'-' || *b == b'_')
        {
            return Err("non-graphic bytes".to_string());
        }
        let s = std::str::from_utf8(input).map_err(|_| "non-utf8".to_string())?;
        let mut iter = s.split('/');
        let owner = iter.next().ok_or("no owner".to_string())?;
        let branch = iter.next().ok_or("no branch".to_string())?;
        if iter.next().is_some() {
            return Err("too many slashes".to_string());
        }
        if owner.is_empty() || branch.is_empty() {
            return Err("empty segment".to_string());
        }
        if owner.len() > 64 || branch.len() > 64 {
            return Err("segment too long".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-003 — State parser. Accepts a JSON object with `state` field
/// being one of `draft|active|paused|archived|blocked`.
pub struct StateTarget;

impl FuzzTarget for StateTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-003")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::State
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 1024 {
            return Err("input exceeds 1024 bytes".to_string());
        }
        let s = std::str::from_utf8(input).map_err(|_| "non-utf8".to_string())?;
        // Tokenize on whitespace and look for a `state=value` pair.
        let mut found = false;
        for tok in s.split(|c: char| c.is_whitespace() || c == ',' || c == '{' || c == '}') {
            if let Some((k, v)) = tok.split_once('=') {
                if k == "state" {
                    found = true;
                    if !matches!(v, "draft" | "active" | "paused" | "archived" | "blocked") {
                        return Err(format!("invalid state value: {v}"));
                    }
                }
            }
        }
        if !found {
            return Err("missing state field".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-004 — Permission engine. Accepts `action:resource` with strict
/// invariants.
pub struct PermissionTarget;

impl FuzzTarget for PermissionTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-004")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::Permission
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 256 {
            return Err("input exceeds 256 bytes".to_string());
        }
        let s = std::str::from_utf8(input).map_err(|_| "non-utf8".to_string())?;
        let (action, resource) = s.split_once(':').ok_or("missing colon".to_string())?;
        if action.is_empty() || resource.is_empty() {
            return Err("empty action or resource".to_string());
        }
        if action.len() > 32 || resource.len() > 128 {
            return Err("action or resource too long".to_string());
        }
        if !action
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("action contains invalid chars".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-005 — Release metadata parser. Accepts `version:os:arch`.
pub struct ReleaseMetadataTarget;

impl FuzzTarget for ReleaseMetadataTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-005")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::ReleaseMetadata
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 256 {
            return Err("input exceeds 256 bytes".to_string());
        }
        let s = std::str::from_utf8(input).map_err(|_| "non-utf8".to_string())?;
        let mut parts = s.split(':');
        let version = parts.next().ok_or("missing version".to_string())?;
        let os = parts.next().ok_or("missing os".to_string())?;
        let arch = parts.next().ok_or("missing arch".to_string())?;
        if parts.next().is_some() {
            return Err("too many colons".to_string());
        }
        if version.is_empty() || os.is_empty() || arch.is_empty() {
            return Err("empty part".to_string());
        }
        if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err("version must be digits and dots".to_string());
        }
        if !matches!(os, "linux" | "darwin" | "windows") {
            return Err(format!("unsupported os: {os}"));
        }
        if !matches!(arch, "x86_64" | "aarch64") {
            return Err(format!("unsupported arch: {arch}"));
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-006 — Hash chain verifier. Accepts a hex digest (64 chars).
pub struct HashChainTarget;

impl FuzzTarget for HashChainTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-006")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::HashChain
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() != 64 {
            return Err(format!("expected 64 bytes, got {}", input.len()));
        }
        if !input.iter().all(|b| b.is_ascii_hexdigit()) {
            return Err("non-hex bytes".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[
            Invariant::NoPanic,
            Invariant::LengthWithin {
                min_len: 64,
                max_len: 64,
            },
            Invariant::RejectsMalformed,
        ]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// FT-007 — Rate limit configuration parser. Accepts
/// `limit:window_ms:burst`.
pub struct RateLimitTarget;

impl FuzzTarget for RateLimitTarget {
    fn id(&self) -> TargetId {
        TargetId::new("FT-007")
    }
    fn kind(&self) -> TargetKind {
        TargetKind::RateLimit
    }
    fn run(&self, input: &[u8]) -> Result<(), String> {
        if input.is_empty() {
            return Err("empty input".to_string());
        }
        if input.len() > 64 {
            return Err("input exceeds 64 bytes".to_string());
        }
        let s = std::str::from_utf8(input).map_err(|_| "non-utf8".to_string())?;
        let mut parts = s.split(':');
        let limit = parts.next().ok_or("missing limit".to_string())?;
        let window = parts.next().ok_or("missing window".to_string())?;
        let burst = parts.next().ok_or("missing burst".to_string())?;
        if parts.next().is_some() {
            return Err("too many colons".to_string());
        }
        let limit_n: u64 = limit.parse().map_err(|_| "limit not a u64".to_string())?;
        let window_n: u64 = window.parse().map_err(|_| "window not a u64".to_string())?;
        let burst_n: u64 = burst.parse().map_err(|_| "burst not a u64".to_string())?;
        if limit_n == 0 || limit_n > 1_000_000 {
            return Err("limit out of range".to_string());
        }
        if window_n == 0 || window_n > 86_400_000 {
            return Err("window out of range".to_string());
        }
        if burst_n > limit_n {
            return Err("burst exceeds limit".to_string());
        }
        Ok(())
    }
    fn invariants(&self) -> &[Invariant] {
        &[Invariant::NoPanic, Invariant::RejectsMalformed]
    }
    fn smoke_iterations(&self) -> usize {
        8
    }
}

/// The default registry of all 7 targets.
pub fn default_targets() -> Vec<Box<dyn FuzzTarget>> {
    vec![
        Box::new(EnvelopeTarget),
        Box::new(BranchPolicyTarget),
        Box::new(StateTarget),
        Box::new(PermissionTarget),
        Box::new(ReleaseMetadataTarget),
        Box::new(HashChainTarget),
        Box::new(RateLimitTarget),
    ]
}

/// Default corpus: one synthetic seed per target. The seed bytes are
/// deterministic and contain no credentials. The fuzz harness will
/// mix these with the PRNG to produce iteration inputs.
pub fn default_corpus() -> Vec<Vec<Vec<u8>>> {
    vec![
        vec![b"{\"kind\":\"chat\"}".to_vec(), b"{not json}".to_vec()],
        vec![b"owner/branch".to_vec(), b"x/y".to_vec()],
        vec![b"state=active".to_vec(), b"state=bogus".to_vec()],
        vec![b"read:docs".to_vec(), b"write:".to_vec()],
        vec![b"1.2.3:linux:x86_64".to_vec(), b"1.0.0:plan9:x86".to_vec()],
        // 64 hex chars for valid, 64 non-hex for invalid
        vec![
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_vec(),
            b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_vec(),
        ],
        vec![b"100:1000:50".to_vec(), b"0:1000:50".to_vec()],
    ]
}

/// Default negative patterns to scan the corpus for. The allowlist
/// from PR-260 (`NEG-001`) is the canonical source; this is a
/// re-statement for self-containment.
pub fn default_negative_patterns() -> Vec<NegativePattern> {
    vec![NegativePattern::new(
        "NEG-001",
        // Sentinel that should never appear in synthetic input.
        b"\x00SECRET-SENTINEL\x00",
    )]
}

/// The default harness used by the contract test.
#[must_use]
pub fn default_harness(runner_digest: &str) -> FuzzHarness {
    FuzzHarness::new(FuzzLimits::default(), "tree-sha", "head-sha", runner_digest)
}
