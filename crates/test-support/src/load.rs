//! Deterministic, bounded load-smoke contracts for PR-262.
//!
//! This module measures the admission/backpressure model only. It does not read
//! host metrics, contact providers, or claim production capacity.

use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub const MAX_PROFILES: usize = 3;
pub const CANONICAL_FIXTURE_DIGEST: &str =
    "8caf08c83a68555b411c01ee6dd6b34c140123f98c81e735ca38a2ae599fa031";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LoadProfile {
    S,
    M,
    L,
}

impl LoadProfile {
    #[must_use]
    pub const fn all() -> [Self; MAX_PROFILES] {
        [Self::S, Self::M, Self::L]
    }

    #[must_use]
    pub const fn limits(self) -> ProfileLimits {
        match self {
            Self::S => ProfileLimits {
                concurrency: 2,
                requests: 8,
                queue: 4,
                duration_ms: 100,
            },
            Self::M => ProfileLimits {
                concurrency: 4,
                requests: 32,
                queue: 16,
                duration_ms: 250,
            },
            Self::L => ProfileLimits {
                concurrency: 8,
                requests: 128,
                queue: 64,
                duration_ms: 500,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileLimits {
    pub concurrency: usize,
    pub requests: usize,
    pub queue: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkloadManifest {
    pub revision: String,
    pub seed: u64,
    pub warmup_iterations: usize,
    pub repetitions: usize,
    pub profiles: Vec<LoadProfile>,
    pub fixture_digest: String,
}

impl Default for WorkloadManifest {
    fn default() -> Self {
        Self {
            revision: "PR-262".into(),
            seed: 26_200,
            warmup_iterations: 1,
            repetitions: 2,
            profiles: LoadProfile::all().to_vec(),
            fixture_digest: digest_fixture("PR-262:synthetic:redacted"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoadMetrics {
    pub profile: LoadProfile,
    pub concurrency: usize,
    pub requests: usize,
    pub admitted: usize,
    pub rejected: usize,
    pub completed: usize,
    pub cancelled: usize,
    pub peak_queue: usize,
    pub max_in_flight: usize,
    pub bounded_duration_ms: u64,
    pub seed: u64,
    pub warmup_iterations: usize,
    pub repetitions: usize,
    pub fixture_digest: String,
    pub artifact_digest: String,
    pub status: LoadStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadStatus {
    Pass,
    AdmissionBound,
    InvalidManifest,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ManifestValidationError {
    EmptyRevision,
    WarmupOutOfBounds,
    RepetitionsOutOfBounds,
    ProfilesMismatch,
    FixtureDigestMismatch,
    InvalidProfileLimits,
    PlanWarmupOutOfBounds,
    PlanRepetitionsOutOfBounds,
}

impl Display for ManifestValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ManifestValidationError {}

pub fn validate_manifest(manifest: &WorkloadManifest) -> bool {
    validate_manifest_result(manifest).is_ok()
}

pub fn validate_manifest_result(
    manifest: &WorkloadManifest,
) -> Result<(), ManifestValidationError> {
    if manifest.revision.is_empty() {
        return Err(ManifestValidationError::EmptyRevision);
    }
    if manifest.warmup_iterations > 8 {
        return Err(ManifestValidationError::WarmupOutOfBounds);
    }
    if !(1..=8).contains(&manifest.repetitions) {
        return Err(ManifestValidationError::RepetitionsOutOfBounds);
    }
    if manifest.profiles != LoadProfile::all() {
        return Err(ManifestValidationError::ProfilesMismatch);
    }
    if manifest.fixture_digest != CANONICAL_FIXTURE_DIGEST {
        return Err(ManifestValidationError::FixtureDigestMismatch);
    }
    if !manifest.profiles.iter().all(|profile| {
        let limits = profile.limits();
        limits.concurrency > 0
            && limits.requests >= limits.concurrency
            && limits.queue > 0
            && limits.duration_ms > 0
    }) {
        return Err(ManifestValidationError::InvalidProfileLimits);
    }
    Ok(())
}

/// Runs a bounded admission model with no wall-clock or host-resource sampling.
pub fn run_profile(
    profile: LoadProfile,
    seed: u64,
    fixture_digest: &str,
) -> Result<LoadMetrics, ManifestValidationError> {
    run_profile_with_plan(profile, seed, fixture_digest, 0, 1)
}

pub fn run_profile_with_plan(
    profile: LoadProfile,
    seed: u64,
    fixture_digest: &str,
    warmup_iterations: usize,
    repetitions: usize,
) -> Result<LoadMetrics, ManifestValidationError> {
    if warmup_iterations > 8 {
        return Err(ManifestValidationError::PlanWarmupOutOfBounds);
    }
    if !(1..=8).contains(&repetitions) {
        return Err(ManifestValidationError::PlanRepetitionsOutOfBounds);
    }
    let limits = profile.limits();
    let available = limits.concurrency.saturating_add(limits.queue);
    let admitted_one = limits.requests.min(available);
    let rejected_one = limits.requests.saturating_sub(admitted_one);
    let cancelled_one = admitted_one / 8;
    let completed_one = admitted_one.saturating_sub(cancelled_one);
    let peak_queue = admitted_one
        .saturating_sub(limits.concurrency)
        .min(limits.queue);
    let status = if rejected_one > 0 {
        LoadStatus::AdmissionBound
    } else {
        LoadStatus::Pass
    };
    let _warmup_work = (0..warmup_iterations).map(|_| admitted_one).sum::<usize>();
    let mut admitted = 0usize;
    let mut rejected = 0usize;
    let mut cancelled = 0usize;
    let mut completed = 0usize;
    for _ in 0..repetitions {
        admitted += admitted_one;
        rejected += rejected_one;
        cancelled += cancelled_one;
        completed += completed_one;
    }
    let mut metrics = LoadMetrics {
        profile,
        concurrency: limits.concurrency,
        requests: limits.requests,
        admitted,
        rejected,
        completed,
        cancelled,
        peak_queue,
        max_in_flight: limits.concurrency.min(admitted),
        bounded_duration_ms: limits.duration_ms * repetitions as u64,
        seed,
        warmup_iterations,
        repetitions,
        fixture_digest: fixture_digest.into(),
        artifact_digest: String::new(),
        status,
    };
    metrics.artifact_digest = digest_metrics(&metrics);
    Ok(metrics)
}

pub fn run_manifest(
    manifest: &WorkloadManifest,
) -> Result<Vec<LoadMetrics>, ManifestValidationError> {
    validate_manifest_result(manifest)?;
    manifest
        .profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| {
            run_profile_with_plan(
                *profile,
                manifest.seed + index as u64,
                &manifest.fixture_digest,
                manifest.warmup_iterations,
                manifest.repetitions,
            )
        })
        .collect()
}

#[must_use]
pub fn digest_fixture(value: &str) -> String {
    hex(digest(&SHA256, value.as_bytes()).as_ref())
}

fn digest_metrics(metrics: &LoadMetrics) -> String {
    let material = format!(
        "{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        metrics.profile,
        metrics.seed,
        metrics.requests,
        metrics.admitted,
        metrics.rejected,
        metrics.completed,
        metrics.cancelled,
        metrics.peak_queue,
        metrics.max_in_flight,
        metrics.bounded_duration_ms,
        metrics.warmup_iterations,
        metrics.repetitions,
        metrics.fixture_digest
    );
    digest_fixture(&material)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_manifest_is_bounded_and_valid() {
        let manifest = WorkloadManifest::default();
        assert!(validate_manifest(&manifest));
        assert_eq!(
            run_manifest(&manifest).expect("valid manifest").len(),
            MAX_PROFILES
        );
    }

    #[test]
    fn admission_is_bounded_and_deterministic() {
        let first = run_profile(LoadProfile::L, 42, "fixture").expect("valid plan");
        let second = run_profile(LoadProfile::L, 42, "fixture").expect("valid plan");
        assert_eq!(first, second);
        assert_eq!(
            first.admitted,
            first.concurrency + LoadProfile::L.limits().queue
        );
        assert_eq!(first.admitted + first.rejected, first.requests);
        assert!(first.max_in_flight <= first.concurrency);
        assert!(first.peak_queue <= LoadProfile::L.limits().queue);
    }

    #[test]
    fn invalid_manifest_fails_closed_without_execution() {
        let manifest = WorkloadManifest {
            repetitions: 0,
            ..WorkloadManifest::default()
        };
        assert!(!validate_manifest(&manifest));
        assert_eq!(
            run_manifest(&manifest).expect_err("invalid manifest must fail"),
            ManifestValidationError::RepetitionsOutOfBounds
        );
    }
}
