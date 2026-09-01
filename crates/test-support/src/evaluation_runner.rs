//! Deterministic, offline replay runner for the native Harness corpus.
//!
//! The runner consumes only versioned synthetic corpus entries and a caller
//! supplied environment identity. It materializes fixtures through
//! `FixtureWorkspace`, rebuilds bounded baseline reports, and never reaches a
//! provider, network, secret store or production filesystem.

use crate::evaluation::{
    BaselineReport, EvaluationContractError, EvaluationEvidence, EvaluationEvidenceStatus,
};
use crate::evaluation_corpus::{
    core_evaluation_corpus, CoreEvaluationFixture, EvaluationCorpusError, CORE_ENVIRONMENT_DIGEST,
    CORE_EVALUATION_CORPUS_SCHEMA_VERSION, CORE_HEAD_SHA, CORE_POLICY_DIGEST, CORE_SCHEMA_DIGEST,
    CORE_TREE_SHA,
};
use crate::fixtures::{FixtureCase, FixtureWorkspace};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use thiserror::Error;

pub const NATIVE_EVALUATION_RUN_SCHEMA_VERSION: u32 = 1;
pub const MAX_NATIVE_EVALUATION_CASES: usize = 64;
pub const MAX_NATIVE_EVALUATION_PARALLELISM: usize = 8;
pub const DEFAULT_NATIVE_EVALUATION_PARALLELISM: usize = 1;

/// Exact code, policy and runtime identity required for a comparable replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvaluationEnvironment {
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_digest: String,
    pub schema_digest: String,
    pub environment_digest: String,
}

impl NativeEvaluationEnvironment {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        policy_digest: impl Into<String>,
        schema_digest: impl Into<String>,
        environment_digest: impl Into<String>,
    ) -> Result<Self, EvaluationContractError> {
        let environment = Self {
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            policy_digest: policy_digest.into(),
            schema_digest: schema_digest.into(),
            environment_digest: environment_digest.into(),
        };
        environment.validate()?;
        Ok(environment)
    }

    pub fn from_evidence(evidence: &EvaluationEvidence) -> Result<Self, EvaluationContractError> {
        Self::new(
            evidence.head_sha.clone(),
            evidence.tree_sha.clone(),
            evidence.policy_digest.clone(),
            evidence.schema_digest.clone(),
            evidence.environment_digest.clone(),
        )
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        // Reuse the contract's bounded and redacted digest validation without
        // exposing its internal validation helpers to the runner.
        EvaluationEvidence::new(
            self.head_sha.clone(),
            self.tree_sha.clone(),
            self.policy_digest.clone(),
            self.schema_digest.clone(),
            "fixture-placeholder",
            self.environment_digest.clone(),
            vec!["artifact-placeholder".into()],
            EvaluationEvidenceStatus::Pass,
        )
        .map(|_| ())
    }

    fn matches_evidence(&self, evidence: &EvaluationEvidence) -> bool {
        self.head_sha == evidence.head_sha
            && self.tree_sha == evidence.tree_sha
            && self.policy_digest == evidence.policy_digest
            && self.schema_digest == evidence.schema_digest
            && self.environment_digest == evidence.environment_digest
    }

    fn matches_core_identity(&self) -> bool {
        self.head_sha == CORE_HEAD_SHA
            && self.tree_sha == CORE_TREE_SHA
            && self.policy_digest == CORE_POLICY_DIGEST
            && self.schema_digest == CORE_SCHEMA_DIGEST
            && self.environment_digest == CORE_ENVIRONMENT_DIGEST
    }
}

#[derive(Debug, Error)]
pub enum NativeEvaluationRunnerError {
    #[error("native evaluation corpus is empty")]
    EmptyCorpus,
    #[error("native evaluation corpus exceeds the configured case bound")]
    CorpusTooLarge,
    #[error("native evaluation runner configuration is outside its bound")]
    BoundsExceeded,
    #[error("native evaluation environment is invalid: {0}")]
    InvalidEnvironment(#[source] EvaluationContractError),
    #[error("evaluation case `{case_id}` is invalid: {source}")]
    InvalidCase {
        case_id: String,
        #[source]
        source: EvaluationContractError,
    },
    #[error("baseline report for case `{case_id}` is invalid: {source}")]
    InvalidBaseline {
        case_id: String,
        #[source]
        source: EvaluationContractError,
    },
    #[error("generated report for case `{case_id}` is invalid: {source}")]
    InvalidReport {
        case_id: String,
        #[source]
        source: EvaluationContractError,
    },
    #[error("fixture for case `{case_id}` could not be materialized: {source}")]
    Fixture {
        case_id: String,
        #[source]
        source: EvaluationCorpusError,
    },
    #[error("fixture content for case `{case_id}` differs from the pinned fixture")]
    FixtureContentMismatch { case_id: String },
    #[error("fixture digest for case `{case_id}` differs from the case descriptor")]
    FixtureDigestMismatch { case_id: String },
    #[error("case `{case_id}` is bound to a non-deterministic fixture")]
    NondeterministicFixture { case_id: String },
    #[error("case `{case_id}` declares an unsafe external effect")]
    UnsafeEffect { case_id: String },
    #[error("case `{case_id}` is missing required artifact `{artifact_digest}`")]
    MissingArtifact {
        case_id: String,
        artifact_digest: String,
    },
    #[error("case `{case_id}` uses an incomparable evaluation environment")]
    IncomparableEnvironment { case_id: String },
    #[error("case `{case_id}` is bound to a different corpus suite")]
    InconsistentSuite { case_id: String },
    #[error("fixture id `{fixture_id}` has conflicting corpus definitions")]
    ConflictingFixtureDefinitions { fixture_id: String },
    #[error("case id `{case_id}` appears more than once in the corpus")]
    DuplicateCaseId { case_id: String },
    #[error("idempotency key `{key}` appears more than once in the corpus")]
    DuplicateIdempotencyKey { key: String },
    #[error("native evaluation corpus does not match the canonical core corpus")]
    NonCanonicalCorpus,
    #[error("canonical native evaluation corpus could not be built: {0}")]
    CanonicalCorpus(#[source] EvaluationCorpusError),
    #[error("case `{case_id}` expected terminal does not match its baseline")]
    UnexpectedTerminal { case_id: String },
    #[error("fixture for case `{case_id}` is not bound to its case descriptor")]
    InvalidFixtureBinding { case_id: String },
    #[error("generated report for case `{case_id}` exceeds its output bound")]
    OutputBoundExceeded { case_id: String },
    #[error("native evaluation run could not be serialized: {0}")]
    Serialization(#[source] serde_json::Error),
}

/// Provider-neutral runner configuration. Replay is intentionally sequential;
/// the parallelism bound is retained in the contract so a future executor
/// cannot silently introduce unbounded concurrency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEvaluationRunner {
    max_cases: usize,
    max_parallelism: usize,
}

impl Default for NativeEvaluationRunner {
    fn default() -> Self {
        Self {
            max_cases: MAX_NATIVE_EVALUATION_CASES,
            max_parallelism: DEFAULT_NATIVE_EVALUATION_PARALLELISM,
        }
    }
}

impl NativeEvaluationRunner {
    pub fn new(
        max_cases: usize,
        max_parallelism: usize,
    ) -> Result<Self, NativeEvaluationRunnerError> {
        if max_cases == 0
            || max_cases > MAX_NATIVE_EVALUATION_CASES
            || max_parallelism == 0
            || max_parallelism > MAX_NATIVE_EVALUATION_PARALLELISM
        {
            return Err(NativeEvaluationRunnerError::BoundsExceeded);
        }
        Ok(Self {
            max_cases,
            max_parallelism,
        })
    }

    pub fn max_cases(&self) -> usize {
        self.max_cases
    }

    pub fn max_parallelism(&self) -> usize {
        self.max_parallelism
    }

    /// Replays a corpus in a caller-owned fixture workspace.
    ///
    /// All entries are validated before the first materialization. Existing
    /// matching fixture files are read and reused, while divergent files are
    /// rejected rather than overwritten.
    pub fn run(
        &self,
        corpus: &[CoreEvaluationFixture],
        environment: &NativeEvaluationEnvironment,
        workspace: &FixtureWorkspace,
    ) -> Result<NativeEvaluationRun, NativeEvaluationRunnerError> {
        if corpus.is_empty() {
            return Err(NativeEvaluationRunnerError::EmptyCorpus);
        }
        if corpus.len() > self.max_cases {
            return Err(NativeEvaluationRunnerError::CorpusTooLarge);
        }
        environment
            .validate()
            .map_err(NativeEvaluationRunnerError::InvalidEnvironment)?;
        if !environment.matches_core_identity() {
            return Err(NativeEvaluationRunnerError::IncomparableEnvironment {
                case_id: corpus[0].case.case_id.clone(),
            });
        }

        let suite_id = corpus[0].case.holdout.suite_id.clone();
        let mut case_ids = BTreeSet::new();
        let mut idempotency_keys = BTreeSet::new();
        let mut fixture_definitions = BTreeMap::<String, FixtureCase>::new();
        let mut prepared_reports = Vec::with_capacity(corpus.len());
        for entry in corpus {
            let report = self.validate_entry(entry, environment, &suite_id, workspace)?;
            if !case_ids.insert(entry.case.case_id.clone()) {
                return Err(NativeEvaluationRunnerError::DuplicateCaseId {
                    case_id: entry.case.case_id.clone(),
                });
            }
            if let Some(existing) = fixture_definitions.get(&entry.fixture.id) {
                if existing != &entry.fixture {
                    return Err(NativeEvaluationRunnerError::ConflictingFixtureDefinitions {
                        fixture_id: entry.fixture.id.clone(),
                    });
                }
            } else {
                fixture_definitions.insert(entry.fixture.id.clone(), entry.fixture.clone());
            }
            prepared_reports.push(report);
            if !idempotency_keys.insert(entry.case.idempotency_key.clone()) {
                return Err(NativeEvaluationRunnerError::DuplicateIdempotencyKey {
                    key: entry.case.idempotency_key.clone(),
                });
            }
        }
        Self::validate_canonical_corpus(corpus)?;

        // Sequential replay is deliberate for deterministic baseline output;
        // max_parallelism remains a bounded configuration for later executors.
        let _bounded_parallelism = self.max_parallelism;
        // Build the complete run before writing any fixture so serialization and
        // output-bound failures cannot leave a partial workspace behind.
        let run = NativeEvaluationRun::new(
            suite_id,
            CORE_EVALUATION_CORPUS_SCHEMA_VERSION,
            environment.clone(),
            prepared_reports.clone(),
        )?;
        for (entry, _prepared_report) in corpus.iter().zip(prepared_reports) {
            let fixture_digest = self.materialize_idempotently(entry, workspace)?;
            if fixture_digest != entry.case.fixture.fixture_digest {
                return Err(NativeEvaluationRunnerError::FixtureDigestMismatch {
                    case_id: entry.case.case_id.clone(),
                });
            }
        }
        Ok(run)
    }

    fn validate_canonical_corpus(
        corpus: &[CoreEvaluationFixture],
    ) -> Result<(), NativeEvaluationRunnerError> {
        let canonical =
            core_evaluation_corpus().map_err(NativeEvaluationRunnerError::CanonicalCorpus)?;
        if corpus.len() != canonical.len() {
            return Err(NativeEvaluationRunnerError::NonCanonicalCorpus);
        }

        for (actual, expected) in corpus.iter().zip(canonical.iter()) {
            let actual_encoded =
                serde_json::to_vec(&(&actual.case, &actual.fixture, &actual.baseline))
                    .map_err(NativeEvaluationRunnerError::Serialization)?;
            let expected_encoded =
                serde_json::to_vec(&(&expected.case, &expected.fixture, &expected.baseline))
                    .map_err(NativeEvaluationRunnerError::Serialization)?;
            if actual_encoded != expected_encoded {
                return Err(NativeEvaluationRunnerError::NonCanonicalCorpus);
            }
        }
        Ok(())
    }

    fn validate_entry(
        &self,
        entry: &CoreEvaluationFixture,
        environment: &NativeEvaluationEnvironment,
        suite_id: &str,
        workspace: &FixtureWorkspace,
    ) -> Result<BaselineReport, NativeEvaluationRunnerError> {
        let case_id = entry.case.case_id.clone();
        if !entry.case.fixture.deterministic {
            return Err(NativeEvaluationRunnerError::NondeterministicFixture { case_id });
        }

        match entry.case.validate() {
            Ok(()) => {}
            Err(EvaluationContractError::UnsafeFixture) => {
                return Err(NativeEvaluationRunnerError::NondeterministicFixture { case_id });
            }
            Err(EvaluationContractError::UnsafeEffect) => {
                return Err(NativeEvaluationRunnerError::UnsafeEffect { case_id });
            }
            Err(source) => {
                return Err(NativeEvaluationRunnerError::InvalidCase { case_id, source });
            }
        }

        if entry.case.holdout.suite_id != suite_id {
            return Err(NativeEvaluationRunnerError::InconsistentSuite { case_id });
        }
        entry
            .fixture
            .validate()
            .map_err(|source| NativeEvaluationRunnerError::Fixture {
                case_id: entry.case.case_id.clone(),
                source: EvaluationCorpusError::Fixture(source),
            })?;
        if entry.case.fixture.fixture_id != entry.fixture.id
            || entry.case.fixture.fixture_revision != entry.fixture.version.to_string()
            || entry.case.fixture.seed != entry.fixture.seed
        {
            return Err(NativeEvaluationRunnerError::InvalidFixtureBinding {
                case_id: entry.case.case_id.clone(),
            });
        }
        let fixture_digest = entry.fixture.manifest_hash().map_err(|source| {
            NativeEvaluationRunnerError::Fixture {
                case_id: entry.case.case_id.clone(),
                source: EvaluationCorpusError::Fixture(source),
            }
        })?;
        if fixture_digest != entry.case.fixture.fixture_digest {
            return Err(NativeEvaluationRunnerError::FixtureDigestMismatch {
                case_id: entry.case.case_id.clone(),
            });
        }
        self.validate_existing_fixture(entry, workspace)?;
        if entry.baseline.terminal != entry.case.expected_terminal {
            return Err(NativeEvaluationRunnerError::UnexpectedTerminal { case_id });
        }
        for requirement in &entry.case.artifact_requirements {
            if !entry
                .baseline
                .evidence
                .artifact_digests
                .contains(&requirement.digest)
            {
                return Err(NativeEvaluationRunnerError::MissingArtifact {
                    case_id,
                    artifact_digest: requirement.digest.clone(),
                });
            }
        }
        entry
            .baseline
            .validate_against(&entry.case)
            .map_err(|source| NativeEvaluationRunnerError::InvalidBaseline {
                case_id: entry.case.case_id.clone(),
                source,
            })?;
        if !environment.matches_evidence(&entry.baseline.evidence) {
            return Err(NativeEvaluationRunnerError::IncomparableEnvironment {
                case_id: entry.case.case_id.clone(),
            });
        }
        let report = self.build_report(entry, environment)?;
        let output_size = serde_json::to_vec(&report)
            .map_err(NativeEvaluationRunnerError::Serialization)?
            .len();
        if output_size > entry.case.cancellation.max_output_bytes {
            return Err(NativeEvaluationRunnerError::OutputBoundExceeded {
                case_id: entry.case.case_id.clone(),
            });
        }
        Ok(report)
    }

    fn validate_existing_fixture(
        &self,
        entry: &CoreEvaluationFixture,
        workspace: &FixtureWorkspace,
    ) -> Result<(), NativeEvaluationRunnerError> {
        match workspace.read(&entry.fixture.id) {
            Ok(existing) if existing == entry.fixture => Ok(()),
            Ok(_) => Err(NativeEvaluationRunnerError::FixtureContentMismatch {
                case_id: entry.case.case_id.clone(),
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(NativeEvaluationRunnerError::Fixture {
                case_id: entry.case.case_id.clone(),
                source: EvaluationCorpusError::Fixture(source),
            }),
        }
    }

    fn materialize_idempotently(
        &self,
        entry: &CoreEvaluationFixture,
        workspace: &FixtureWorkspace,
    ) -> Result<String, NativeEvaluationRunnerError> {
        match entry.materialize(workspace) {
            Ok(digest) => Ok(digest),
            Err(EvaluationCorpusError::Fixture(error))
                if error.kind() == io::ErrorKind::AlreadyExists =>
            {
                let existing = workspace.read(&entry.fixture.id).map_err(|source| {
                    NativeEvaluationRunnerError::Fixture {
                        case_id: entry.case.case_id.clone(),
                        source: EvaluationCorpusError::Fixture(source),
                    }
                })?;
                if existing != entry.fixture {
                    return Err(NativeEvaluationRunnerError::FixtureContentMismatch {
                        case_id: entry.case.case_id.clone(),
                    });
                }
                let digest = existing.manifest_hash().map_err(|source| {
                    NativeEvaluationRunnerError::Fixture {
                        case_id: entry.case.case_id.clone(),
                        source: EvaluationCorpusError::Fixture(source),
                    }
                })?;
                if digest != entry.case.fixture.fixture_digest {
                    return Err(NativeEvaluationRunnerError::FixtureDigestMismatch {
                        case_id: entry.case.case_id.clone(),
                    });
                }
                Ok(digest)
            }
            Err(source) => Err(NativeEvaluationRunnerError::Fixture {
                case_id: entry.case.case_id.clone(),
                source,
            }),
        }
    }

    fn build_report(
        &self,
        entry: &CoreEvaluationFixture,
        environment: &NativeEvaluationEnvironment,
    ) -> Result<BaselineReport, NativeEvaluationRunnerError> {
        let evidence = EvaluationEvidence::new(
            environment.head_sha.clone(),
            environment.tree_sha.clone(),
            environment.policy_digest.clone(),
            environment.schema_digest.clone(),
            entry.case.fixture.fixture_digest.clone(),
            environment.environment_digest.clone(),
            entry.baseline.evidence.artifact_digests.clone(),
            entry.baseline.evidence.status,
        )
        .map_err(|source| NativeEvaluationRunnerError::InvalidReport {
            case_id: entry.case.case_id.clone(),
            source,
        })?;

        BaselineReport::from_case(
            &entry.case,
            entry.baseline.terminal,
            entry.baseline.metrics.clone(),
            evidence,
        )
        .map_err(|source| NativeEvaluationRunnerError::InvalidReport {
            case_id: entry.case.case_id.clone(),
            source,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvaluationRun {
    pub schema_version: u32,
    pub suite_id: String,
    pub corpus_schema_version: u32,
    pub environment: NativeEvaluationEnvironment,
    pub reports: Vec<BaselineReport>,
    pub run_digest: String,
}

impl NativeEvaluationRun {
    fn new(
        suite_id: String,
        corpus_schema_version: u32,
        environment: NativeEvaluationEnvironment,
        reports: Vec<BaselineReport>,
    ) -> Result<Self, NativeEvaluationRunnerError> {
        let mut run = Self {
            schema_version: NATIVE_EVALUATION_RUN_SCHEMA_VERSION,
            suite_id,
            corpus_schema_version,
            environment,
            reports,
            run_digest: String::new(),
        };
        run.run_digest = run.content_digest()?;
        Ok(run)
    }

    fn content_digest(&self) -> Result<String, NativeEvaluationRunnerError> {
        let mut content = self.clone();
        content.run_digest.clear();
        let encoded =
            serde_json::to_vec(&content).map_err(NativeEvaluationRunnerError::Serialization)?;
        Ok(format!("{:016x}", fnv1a64(&encoded)))
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
