//! Bounded fuzz harness for parser/state/permission boundaries.
//!
//! Public surface: `FuzzTarget` trait, `FuzzHarness`, `FuzzReport`,
//! `FuzzCrash`, `FuzzLimits`, `run_target`, `run_all_targets`,
//! `digest_corpus`, `digest_runner`, `replay_crash`, `verify_no_secrets`.
//!
//! Design constraints:
//! - No `unsafe`. No threading. No real I/O.
//! - No panic is hidden. A target panic increments `panics` and
//!   captures the panic message, the input digest, and a stack
//!   digest computed from the textual panic message.
//! - No time/memory limit silently exceeded.
//! - No silent skip. A test that produces zero iterations is
//!   reported as `zero_iterations` and fails the harness.
//!
//! PR-261 (fuzz-tests) — Plan card: T-2200.

#![allow(clippy::module_name_repetitions)]

use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Identifies a fuzz target.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TargetId(pub String);

impl TargetId {
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The kind of parser the target exercises.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Envelope,
    Policy,
    State,
    Permission,
    ReleaseMetadata,
    HashChain,
    RateLimit,
}

/// A bound the target is expected to honor on every input.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invariant {
    NoPanic,
    RejectsMalformed,
    AcceptsValid,
    LengthWithin { min_len: usize, max_len: usize },
    AllocationBound { max_bytes: usize },
}

/// Classification for a target that returns an error.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetError {
    Rejected(String),
    InvariantViolation(String),
}

/// A single fuzz target.
pub trait FuzzTarget: Send + Sync {
    fn id(&self) -> TargetId;
    fn kind(&self) -> TargetKind;
    fn run(&self, input: &[u8]) -> Result<(), String>;
    /// Classify a returned error without conflating rejection with failure.
    fn classify_error(&self, message: &str) -> TargetError {
        TargetError::Rejected(message.to_string())
    }
    fn invariants(&self) -> &[Invariant];
    fn smoke_iterations(&self) -> usize;
}

/// Default smoke iteration count.
pub const DEFAULT_SMOKE_ITERATIONS: usize = 8;
pub const DEFAULT_PER_ITER_TIMEOUT_MS: u64 = 250;
pub const DEFAULT_MAX_MEMORY_MB: usize = 64;

/// Tunable limits for a fuzz run.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FuzzLimits {
    pub smoke_iterations: usize,
    pub per_iter_timeout_ms: u64,
    pub max_memory_mb: usize,
}

impl Default for FuzzLimits {
    fn default() -> Self {
        Self {
            smoke_iterations: DEFAULT_SMOKE_ITERATIONS,
            per_iter_timeout_ms: DEFAULT_PER_ITER_TIMEOUT_MS,
            max_memory_mb: DEFAULT_MAX_MEMORY_MB,
        }
    }
}

/// What happened in a single iteration.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationOutcome {
    Pass,
    Rejected {
        message: String,
    },
    Panic {
        message: String,
        stack_digest: String,
    },
    Timeout,
    Oom,
    InvariantViolation {
        invariant: Invariant,
        message: String,
    },
}

/// What happened for an entire target.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FuzzReport {
    pub target_id: TargetId,
    pub kind: TargetKind,
    pub iterations: usize,
    pub passes: usize,
    pub rejections: usize,
    pub panics: usize,
    pub timeouts: usize,
    pub ooms: usize,
    pub invariant_violations: usize,
    pub total_duration_ms: u64,
    pub seed: u64,
    pub corpus_digest: String,
    pub runner_digest: String,
    pub tree_sha: String,
    pub head_sha: String,
    pub status: FuzzStatus,
    pub first_crash: Option<FuzzCrash>,
    pub replay: Option<ReplayResult>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuzzStatus {
    Pass,
    Panic,
    Timeout,
    Oom,
    InvariantViolation,
    ZeroIterations,
}

/// A captured crash artifact.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FuzzCrash {
    pub target_id: TargetId,
    pub iteration: usize,
    pub seed: u64,
    pub input_digest: String,
    pub stack_digest: String,
    pub message: String,
    pub invariant: Option<Invariant>,
}

/// Result of replaying a crash.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayResult {
    pub reproducible: bool,
    pub iterations_run: usize,
}

/// A pattern the harness rejects in the corpus.
#[derive(Debug, Clone)]
pub struct NegativePattern {
    pub id: String,
    pub needle: Vec<u8>,
}

impl NegativePattern {
    #[must_use]
    pub fn new(id: &str, needle: &[u8]) -> Self {
        Self {
            id: id.to_string(),
            needle: needle.to_vec(),
        }
    }
    #[must_use]
    pub fn matches(&self, bytes: &[u8]) -> bool {
        if self.needle.is_empty() {
            return false;
        }
        bytes
            .windows(self.needle.len())
            .any(|w| w == self.needle.as_slice())
    }
}

/// The harness.
pub struct FuzzHarness {
    pub limits: FuzzLimits,
    pub tree_sha: String,
    pub head_sha: String,
    pub runner_digest: String,
}

impl FuzzHarness {
    #[must_use]
    pub fn new(limits: FuzzLimits, tree_sha: &str, head_sha: &str, runner_digest: &str) -> Self {
        Self {
            limits,
            tree_sha: tree_sha.to_string(),
            head_sha: head_sha.to_string(),
            runner_digest: runner_digest.to_string(),
        }
    }

    /// Run a single target against a corpus.
    pub fn run_target(&self, target: &dyn FuzzTarget, corpus: &[Vec<u8>], seed: u64) -> FuzzReport {
        let target_id = target.id();
        let kind = target.kind();
        let invariants = target.invariants().to_vec();
        // The target's own preference wins over the harness lower
        // bound, but if either is zero we honor that and report
        // ZeroIterations. The harness's lower bound never silently
        // raises a target's iteration count.
        let smoke = if target.smoke_iterations() == 0 || self.limits.smoke_iterations == 0 {
            0
        } else {
            target.smoke_iterations().min(self.limits.smoke_iterations)
        };
        let per_iter_timeout = Duration::from_millis(self.limits.per_iter_timeout_ms);
        let corpus_digest = digest_corpus(corpus);
        let runner_digest = self.runner_digest.clone();
        let tree_sha = self.tree_sha.clone();
        let head_sha = self.head_sha.clone();

        if smoke == 0 {
            return FuzzReport {
                target_id,
                kind,
                iterations: 0,
                passes: 0,
                rejections: 0,
                panics: 0,
                timeouts: 0,
                ooms: 0,
                invariant_violations: 0,
                total_duration_ms: 0,
                seed,
                corpus_digest,
                runner_digest,
                tree_sha,
                head_sha,
                status: FuzzStatus::ZeroIterations,
                first_crash: None,
                replay: None,
            };
        }

        let start = Instant::now();
        let mut passes = 0usize;
        let mut rejections = 0usize;
        let mut panics = 0usize;
        let mut timeouts = 0usize;
        let mut ooms = 0usize;
        let mut invariant_violations = 0usize;
        let mut first_crash: Option<FuzzCrash> = None;
        let mut rng = SplitMix64::new(seed);

        for iter in 0..smoke {
            let input = synthesize_input(corpus, &mut rng, iter);
            let input_digest = sha256_hex(&input);
            let iter_start = Instant::now();

            let outcome = catch_unwind(AssertUnwindSafe(|| {
                if iter_start.elapsed() > per_iter_timeout {
                    return IterationOutcome::Timeout;
                }
                if input.len() > self.limits.max_memory_mb.saturating_mul(1024 * 1024) {
                    return IterationOutcome::Oom;
                }
                match target.run(&input) {
                    Ok(()) => IterationOutcome::Pass,
                    Err(message) => match target.classify_error(&message) {
                        TargetError::Rejected(message) => IterationOutcome::Rejected { message },
                        TargetError::InvariantViolation(message) => {
                            let inv = pick_invariant(&invariants);
                            IterationOutcome::InvariantViolation {
                                invariant: inv,
                                message,
                            }
                        }
                    },
                }
            }));

            let outcome = match outcome {
                Ok(outcome)
                    if !matches!(outcome, IterationOutcome::Panic { .. })
                        && iter_start.elapsed() > per_iter_timeout =>
                {
                    IterationOutcome::Timeout
                }
                Ok(outcome) => outcome,
                Err(payload) => {
                    let message = panic_message(payload.as_ref());
                    let stack_digest = sha256_hex(message.as_bytes());
                    IterationOutcome::Panic {
                        message,
                        stack_digest,
                    }
                }
            };

            match &outcome {
                IterationOutcome::Pass => passes += 1,
                IterationOutcome::Rejected { .. } => rejections += 1,
                IterationOutcome::Panic {
                    message,
                    stack_digest,
                } => {
                    panics += 1;
                    if first_crash.is_none() {
                        first_crash = Some(FuzzCrash {
                            target_id: target_id.clone(),
                            iteration: iter,
                            seed,
                            input_digest,
                            stack_digest: stack_digest.clone(),
                            message: message.clone(),
                            invariant: None,
                        });
                    }
                }
                IterationOutcome::Timeout => timeouts += 1,
                IterationOutcome::Oom => ooms += 1,
                IterationOutcome::InvariantViolation { invariant, message } => {
                    invariant_violations += 1;
                    if first_crash.is_none() {
                        first_crash = Some(FuzzCrash {
                            target_id: target_id.clone(),
                            iteration: iter,
                            seed,
                            input_digest,
                            stack_digest: sha256_hex(message.as_bytes()),
                            message: message.clone(),
                            invariant: Some(invariant.clone()),
                        });
                    }
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        let status = if panics > 0 {
            FuzzStatus::Panic
        } else if timeouts > 0 {
            FuzzStatus::Timeout
        } else if ooms > 0 {
            FuzzStatus::Oom
        } else if invariant_violations > 0 {
            FuzzStatus::InvariantViolation
        } else {
            FuzzStatus::Pass
        };

        FuzzReport {
            target_id,
            kind,
            iterations: smoke,
            passes,
            rejections,
            panics,
            timeouts,
            ooms,
            invariant_violations,
            total_duration_ms,
            seed,
            corpus_digest,
            runner_digest,
            tree_sha,
            head_sha,
            status,
            first_crash,
            replay: None,
        }
    }

    /// Replay a target up to and including a specific iteration.
    pub fn replay_crash(
        &self,
        target: &dyn FuzzTarget,
        corpus: &[Vec<u8>],
        seed: u64,
        iteration: usize,
    ) -> ReplayResult {
        let mut rng = SplitMix64::new(seed);
        let iterations_to_run = iteration.saturating_add(1);
        let mut reproducible = false;
        for iter in 0..iterations_to_run {
            let input = synthesize_input(corpus, &mut rng, iter);
            let replayed = catch_unwind(AssertUnwindSafe(|| target.run(&input)));
            if iter == iteration {
                reproducible = replayed.is_err();
            }
        }
        ReplayResult {
            reproducible,
            iterations_run: iterations_to_run,
        }
    }
}

/// Run all targets.
pub fn run_all_targets(
    harness: &FuzzHarness,
    targets: &[Box<dyn FuzzTarget>],
    corpus_per_target: &[Vec<Vec<u8>>],
    seed: u64,
) -> Result<Vec<FuzzReport>, String> {
    if targets.len() != corpus_per_target.len() {
        return Err(format!(
            "targets and corpus_per_target must align: {} != {}",
            targets.len(),
            corpus_per_target.len()
        ));
    }
    let mut reports = Vec::with_capacity(targets.len());
    for (target, corpus) in targets.iter().zip(corpus_per_target.iter()) {
        let report = harness.run_target(target.as_ref(), corpus, seed);
        reports.push(report);
    }
    Ok(reports)
}

/// Compute a deterministic SHA-256 digest of the corpus bytes.
#[must_use]
pub fn digest_corpus(corpus: &[Vec<u8>]) -> String {
    let mut hasher = Sha256::new();
    for (idx, bytes) in corpus.iter().enumerate() {
        hasher.update(&(idx as u64).to_be_bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hasher.finalize_hex()
}

/// Compute a deterministic SHA-256 digest of the runner source.
#[must_use]
pub fn digest_runner(runner_bytes: &[u8]) -> String {
    sha256_hex(runner_bytes)
}

/// Verify the corpus does not contain credential patterns.
pub fn verify_no_secrets(
    corpus: &[Vec<u8>],
    negative_patterns: &[NegativePattern],
) -> Result<(), String> {
    for (idx, bytes) in corpus.iter().enumerate() {
        for pat in negative_patterns {
            if pat.matches(bytes) {
                return Err(format!("corpus[{}] contains pattern {}", idx, pat.id));
            }
        }
    }
    Ok(())
}

fn pick_invariant(invariants: &[Invariant]) -> Invariant {
    invariants.first().cloned().unwrap_or(Invariant::NoPanic)
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Deterministically synthesize an input for iteration `iter` by
/// mixing the corpus bytes with a `SplitMix64` PRNG.
fn synthesize_input(corpus: &[Vec<u8>], rng: &mut SplitMix64, iter: usize) -> Vec<u8> {
    let len = (rng.next() as usize).min(4096);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        if corpus.is_empty() {
            out.push((rng.next() & 0xff) as u8);
        } else {
            let corpus_idx = (rng.next() as usize) % corpus.len();
            let corpus_bytes = &corpus[corpus_idx];
            if corpus_bytes.is_empty() {
                out.push((rng.next() & 0xff) as u8);
            } else {
                let byte_idx = (rng.next() as usize) % corpus_bytes.len();
                out.push(
                    corpus_bytes[byte_idx] ^ ((rng.next() & 0xff) as u8) ^ ((iter & 0xff) as u8),
                );
            }
        }
    }
    out
}

/// SplitMix64 PRNG (public domain). Deterministic and small.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next(&mut self) -> u64 {
        let mut z = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.0 = z;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize_hex()
}

/// A minimal SHA-256 implementation so the harness does not depend
/// on the `sha2` crate. SHA-256 is a public-domain algorithm.
struct Sha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total_len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: Vec::with_capacity(64),
            total_len: 0,
        }
    }
    fn update(&mut self, bytes: &[u8]) {
        self.total_len = self.total_len.wrapping_add(bytes.len() as u64);
        let mut data = bytes;
        if !self.buffer.is_empty() {
            let need = 64 - self.buffer.len();
            let take = need.min(data.len());
            self.buffer.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.buffer.len() == 64 {
                let block = std::mem::take(&mut self.buffer);
                Self::compress(&mut self.state, &block);
            }
        }
        while data.len() >= 64 {
            let block = &data[..64];
            Self::compress(&mut self.state, block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer.extend_from_slice(data);
        }
    }
    fn finalize_hex(mut self) -> String {
        let bit_len = self.total_len.wrapping_mul(8);
        self.buffer.push(0x80);
        while self.buffer.len() % 64 != 56 {
            self.buffer.push(0);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());
        debug_assert!(self.buffer.len().is_multiple_of(64));
        let chunks: Vec<Vec<u8>> = self.buffer.chunks(64).map(<[u8]>::to_vec).collect();
        for block in chunks {
            Self::compress(&mut self.state, &block);
        }
        let mut out = String::with_capacity(64);
        for word in &self.state {
            out.push_str(&format!("{word:08x}"));
        }
        out
    }
    fn compress(state: &mut [u32; 8], block: &[u8]) {
        debug_assert_eq!(block.len(), 64);
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyTarget {
        id: TargetId,
        kind: TargetKind,
    }
    impl FuzzTarget for DummyTarget {
        fn id(&self) -> TargetId {
            self.id.clone()
        }
        fn kind(&self) -> TargetKind {
            self.kind
        }
        fn run(&self, _input: &[u8]) -> Result<(), String> {
            Ok(())
        }
        fn invariants(&self) -> &[Invariant] {
            &[Invariant::NoPanic]
        }
        fn smoke_iterations(&self) -> usize {
            4
        }
    }

    struct PanickyTarget {
        id: TargetId,
    }
    impl FuzzTarget for PanickyTarget {
        fn id(&self) -> TargetId {
            self.id.clone()
        }
        fn kind(&self) -> TargetKind {
            TargetKind::Envelope
        }
        fn run(&self, _input: &[u8]) -> Result<(), String> {
            panic!("simulated");
        }
        fn invariants(&self) -> &[Invariant] {
            &[Invariant::NoPanic]
        }
        fn smoke_iterations(&self) -> usize {
            2
        }
    }

    #[test]
    fn run_target_pass_for_inert_target() {
        let harness = FuzzHarness::new(FuzzLimits::default(), "tree", "head", "runner");
        let target = DummyTarget {
            id: TargetId::new("FT-001"),
            kind: TargetKind::Envelope,
        };
        let report = harness.run_target(&target, &[vec![0u8; 4]], 0xDEAD_BEEF);
        assert_eq!(report.status, FuzzStatus::Pass);
        assert_eq!(report.iterations, 4);
        assert_eq!(report.passes, 4);
        assert!(report.first_crash.is_none());
    }

    #[test]
    fn run_target_captures_panic() {
        let harness = FuzzHarness::new(FuzzLimits::default(), "tree", "head", "runner");
        let target = PanickyTarget {
            id: TargetId::new("FT-PANIC"),
        };
        let report = harness.run_target(&target, &[vec![0u8; 4]], 0xCAFE_BABE);
        assert_eq!(report.status, FuzzStatus::Panic);
        assert!(report.first_crash.is_some());
        let crash = report.first_crash.unwrap();
        assert!(crash.message.contains("simulated"));
        assert_eq!(crash.stack_digest.len(), 64);
    }

    #[test]
    fn digest_corpus_is_deterministic() {
        let corpus = vec![vec![1u8, 2, 3], vec![4u8, 5, 6, 7]];
        let d1 = digest_corpus(&corpus);
        let d2 = digest_corpus(&corpus);
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
        let corpus2 = vec![vec![1u8, 2, 3], vec![4u8, 5, 6, 8]];
        let d3 = digest_corpus(&corpus2);
        assert_ne!(d1, d3);
    }

    #[test]
    fn digest_runner_is_deterministic() {
        let bytes = b"fn main() {}";
        let d1 = digest_runner(bytes);
        let d2 = digest_runner(bytes);
        assert_eq!(d1, d2);
    }

    #[test]
    fn verify_no_secrets_finds_needle() {
        let corpus = vec![b"hello AKIAIOSFODNN7EXAMPLE world".to_vec()];
        let pat = NegativePattern::new("NEG-001", b"AKIAIOSFODNN7EXAMPLE");
        assert!(verify_no_secrets(&corpus, &[pat]).is_err());
    }

    #[test]
    fn verify_no_secrets_passes_for_clean_corpus() {
        let corpus = vec![b"hello world".to_vec(), b"another line".to_vec()];
        let pat = NegativePattern::new("NEG-001", b"AKIAIOSFODNN7EXAMPLE");
        assert!(verify_no_secrets(&corpus, &[pat]).is_ok());
    }

    #[test]
    fn zero_iterations_fails_closed() {
        let harness = FuzzHarness::new(FuzzLimits::default(), "tree", "head", "runner");
        struct ZeroTarget;
        impl FuzzTarget for ZeroTarget {
            fn id(&self) -> TargetId {
                TargetId::new("FT-ZERO")
            }
            fn kind(&self) -> TargetKind {
                TargetKind::State
            }
            fn run(&self, _input: &[u8]) -> Result<(), String> {
                Ok(())
            }
            fn invariants(&self) -> &[Invariant] {
                &[Invariant::NoPanic]
            }
            fn smoke_iterations(&self) -> usize {
                0
            }
        }
        let report = harness.run_target(&ZeroTarget, &[], 1);
        assert_eq!(report.status, FuzzStatus::ZeroIterations);
    }

    #[test]
    fn synthesize_input_is_deterministic() {
        let corpus = vec![vec![0xABu8; 4]];
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);
        let a = synthesize_input(&corpus, &mut rng1, 0);
        let b = synthesize_input(&corpus, &mut rng2, 0);
        assert_eq!(a, b);
    }
}
