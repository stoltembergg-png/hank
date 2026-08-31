//! Versioned, provider-neutral contracts for native Harness evaluation.
//!
//! The contract records only bounded identities, deterministic fixture/scorer
//! descriptors, structured metrics and redacted evidence. It has no runner,
//! storage, provider or external-effect path. `test-support` owns this module
//! so production `agent-core` never depends on a test platform implementation.

use agent_protocol::ids::{ProjectId, RunId, TraceId};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const EVALUATION_CASE_SCHEMA_VERSION: u32 = 1;
pub const METRIC_SCHEMA_VERSION: u32 = 1;
pub const BASELINE_REPORT_SCHEMA_VERSION: u32 = 1;

const MAX_CASE_ID_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 256;
const MAX_DIGEST_BYTES: usize = 128;
const MAX_METRICS: usize = 32;
const MAX_EFFECTS: usize = 8;
const MAX_ARTIFACTS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluationContractError {
    #[error("evaluation schema version is unsupported")]
    UnsupportedSchemaVersion,
    #[error("evaluation contract field is outside its bound")]
    BoundsExceeded,
    #[error("evaluation contract shape is invalid")]
    InvalidShape,
    #[error("evaluation authority is required")]
    MissingAuthority,
    #[error("expected terminal state is required")]
    MissingExpectedTerminal,
    #[error("holdout metadata is required")]
    MissingHoldout,
    #[error("fixture is not deterministic and bounded")]
    UnsafeFixture,
    #[error("evaluation effect is not virtual or read-only")]
    UnsafeEffect,
    #[error("metric is duplicated or incompatible with its schema")]
    InvalidMetric,
    #[error("metric observation is missing")]
    MissingMetric,
    #[error("metric observation is not declared by the schema")]
    UnexpectedMetric,
    #[error("metric observation value is invalid")]
    InvalidMetricValue,
    #[error("sensitive value is not allowed in evaluation metadata")]
    SensitiveValue,
    #[error("evaluation identity does not match the case")]
    IdentityMismatch,
    #[error("evaluation evidence does not match the case")]
    EvidenceMismatch,
    #[error("evaluation digest is invalid or stale")]
    InvalidDigest,
    #[error("evaluation report evidence status is incompatible with terminal state")]
    InvalidEvidenceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationAuthority {
    ReadOnly,
    VirtualOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationEffect {
    ReadOnly,
    VirtualToolCall,
    VirtualFilesystem,
    VirtualProcess,
    ExternalWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationTerminal {
    Pass,
    Fail,
    Blocked,
    Cancelled,
    NoProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldoutPartition {
    Training,
    Holdout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldoutMarker {
    pub partition: HoldoutPartition,
    pub suite_id: String,
    pub partition_revision: String,
    pub case_key: String,
}

impl HoldoutMarker {
    pub fn new(
        partition: HoldoutPartition,
        suite_id: impl Into<String>,
        partition_revision: impl Into<String>,
        case_key: impl Into<String>,
    ) -> Result<Self, EvaluationContractError> {
        let marker = Self {
            partition,
            suite_id: suite_id.into(),
            partition_revision: partition_revision.into(),
            case_key: case_key.into(),
        };
        marker.validate()?;
        Ok(marker)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        validate_text(&self.suite_id, MAX_CASE_ID_BYTES)?;
        validate_text(&self.partition_revision, MAX_CASE_ID_BYTES)?;
        validate_text(&self.case_key, MAX_CASE_ID_BYTES)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureDescriptor {
    pub fixture_id: String,
    pub fixture_revision: String,
    pub fixture_digest: String,
    pub seed: u64,
    pub deterministic: bool,
}

impl FixtureDescriptor {
    pub fn new(
        fixture_id: impl Into<String>,
        fixture_revision: impl Into<String>,
        fixture_digest: impl Into<String>,
        seed: u64,
        deterministic: bool,
    ) -> Result<Self, EvaluationContractError> {
        let fixture = Self {
            fixture_id: fixture_id.into(),
            fixture_revision: fixture_revision.into(),
            fixture_digest: fixture_digest.into(),
            seed,
            deterministic,
        };
        fixture.validate()?;
        Ok(fixture)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        validate_text(&self.fixture_id, MAX_CASE_ID_BYTES)?;
        validate_text(&self.fixture_revision, MAX_CASE_ID_BYTES)?;
        validate_digest(&self.fixture_digest)?;
        if !self.deterministic {
            return Err(EvaluationContractError::UnsafeFixture);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScorerDescriptor {
    pub scorer_id: String,
    pub scorer_version: String,
    pub scorer_digest: String,
}

impl ScorerDescriptor {
    pub fn new(
        scorer_id: impl Into<String>,
        scorer_version: impl Into<String>,
        scorer_digest: impl Into<String>,
    ) -> Result<Self, EvaluationContractError> {
        let scorer = Self {
            scorer_id: scorer_id.into(),
            scorer_version: scorer_version.into(),
            scorer_digest: scorer_digest.into(),
        };
        scorer.validate()?;
        Ok(scorer)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        validate_text(&self.scorer_id, MAX_CASE_ID_BYTES)?;
        validate_text(&self.scorer_version, MAX_CASE_ID_BYTES)?;
        validate_digest(&self.scorer_digest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricName {
    Success,
    TerminalState,
    TestsPassing,
    ToolCalls,
    FailedToolCalls,
    Retries,
    Tokens,
    Cost,
    LatencyMs,
    HumanIntervention,
    EvidenceQuality,
    PolicyViolations,
    ContextMisses,
    MemoryHits,
    EvidenceConflicts,
    SkillSelection,
    ExternalSideEffectAttempts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricValueKind {
    Boolean,
    Count,
    DurationMs,
    Ratio,
    Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
    Exact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricDefinition {
    pub name: MetricName,
    pub value_kind: MetricValueKind,
    pub direction: MetricDirection,
    pub required: bool,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

impl MetricDefinition {
    pub fn new(
        name: MetricName,
        value_kind: MetricValueKind,
        direction: MetricDirection,
        required: bool,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Self {
        Self {
            name,
            value_kind,
            direction,
            required,
            minimum,
            maximum,
        }
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        if self
            .minimum
            .into_iter()
            .chain(self.maximum)
            .any(|value| !value.is_finite() || value < 0.0)
        {
            return Err(EvaluationContractError::InvalidMetric);
        }
        if self
            .minimum
            .zip(self.maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            return Err(EvaluationContractError::InvalidMetric);
        }
        match self.value_kind {
            MetricValueKind::Boolean | MetricValueKind::Category => {
                if self.minimum.is_some() || self.maximum.is_some() {
                    return Err(EvaluationContractError::InvalidMetric);
                }
            }
            MetricValueKind::Ratio => {
                if self.minimum.is_some_and(|value| value > 1.0)
                    || self.maximum.is_some_and(|value| value > 1.0)
                {
                    return Err(EvaluationContractError::InvalidMetric);
                }
            }
            MetricValueKind::Count | MetricValueKind::DurationMs => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSchema {
    pub schema_version: u32,
    pub revision: String,
    pub metrics: Vec<MetricDefinition>,
}

impl MetricSchema {
    pub fn new(
        revision: impl Into<String>,
        metrics: Vec<MetricDefinition>,
    ) -> Result<Self, EvaluationContractError> {
        let schema = Self {
            schema_version: METRIC_SCHEMA_VERSION,
            revision: revision.into(),
            metrics,
        };
        schema.validate()?;
        Ok(schema)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        if self.schema_version != METRIC_SCHEMA_VERSION {
            return Err(EvaluationContractError::UnsupportedSchemaVersion);
        }
        validate_text(&self.revision, MAX_CASE_ID_BYTES)?;
        if self.metrics.is_empty() || self.metrics.len() > MAX_METRICS {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        let mut names = BTreeSet::new();
        for metric in &self.metrics {
            metric.validate()?;
            if !names.insert(metric.name) {
                return Err(EvaluationContractError::InvalidMetric);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricValue {
    Boolean(bool),
    Count(u64),
    DurationMs(u64),
    Ratio(f64),
    Category(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricObservation {
    pub name: MetricName,
    pub value: MetricValue,
}

impl MetricObservation {
    pub fn boolean(name: MetricName, value: bool) -> Self {
        Self {
            name,
            value: MetricValue::Boolean(value),
        }
    }

    pub fn count(name: MetricName, value: u64) -> Self {
        Self {
            name,
            value: MetricValue::Count(value),
        }
    }

    pub fn duration_ms(name: MetricName, value: u64) -> Self {
        Self {
            name,
            value: MetricValue::DurationMs(value),
        }
    }

    pub fn ratio(name: MetricName, value: f64) -> Self {
        Self {
            name,
            value: MetricValue::Ratio(value),
        }
    }

    pub fn category(name: MetricName, value: impl Into<String>) -> Self {
        Self {
            name,
            value: MetricValue::Category(value.into()),
        }
    }

    fn validate_against(
        &self,
        definition: &MetricDefinition,
    ) -> Result<(), EvaluationContractError> {
        let numeric = match (&self.value, definition.value_kind) {
            (MetricValue::Boolean(_), MetricValueKind::Boolean)
            | (MetricValue::Category(_), MetricValueKind::Category) => None,
            (MetricValue::Count(value), MetricValueKind::Count)
            | (MetricValue::DurationMs(value), MetricValueKind::DurationMs) => Some(*value as f64),
            (MetricValue::Ratio(value), MetricValueKind::Ratio)
                if value.is_finite() && (0.0..=1.0).contains(value) =>
            {
                Some(*value)
            }
            _ => return Err(EvaluationContractError::InvalidMetricValue),
        };

        if let MetricValue::Category(value) = &self.value {
            validate_text(value, MAX_TEXT_BYTES)?;
        }
        if let Some(value) = numeric {
            if definition.minimum.is_some_and(|minimum| value < minimum)
                || definition.maximum.is_some_and(|maximum| value > maximum)
            {
                return Err(EvaluationContractError::InvalidMetricValue);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancellationPolicy {
    pub cancellable: bool,
    pub max_duration_ms: u64,
    pub max_output_bytes: usize,
}

impl CancellationPolicy {
    pub fn new(
        cancellable: bool,
        max_duration_ms: u64,
        max_output_bytes: usize,
    ) -> Result<Self, EvaluationContractError> {
        let policy = Self {
            cancellable,
            max_duration_ms,
            max_output_bytes,
        };
        policy.validate()?;
        Ok(policy)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        if self.max_duration_ms == 0 || self.max_output_bytes == 0 {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationBudget {
    pub max_tool_calls: u32,
    pub max_tokens: u64,
    pub max_cost_micros: u64,
}

impl EvaluationBudget {
    pub fn new(
        max_tool_calls: u32,
        max_tokens: u64,
        max_cost_micros: u64,
    ) -> Result<Self, EvaluationContractError> {
        let budget = Self {
            max_tool_calls,
            max_tokens,
            max_cost_micros,
        };
        budget.validate()?;
        Ok(budget)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        if self.max_tool_calls == 0 || self.max_tokens == 0 || self.max_cost_micros == 0 {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Result,
    Trace,
    Evidence,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRequirement {
    pub kind: ArtifactKind,
    pub digest: String,
}

impl ArtifactRequirement {
    pub fn new(
        kind: ArtifactKind,
        digest: impl Into<String>,
    ) -> Result<Self, EvaluationContractError> {
        let requirement = Self {
            kind,
            digest: digest.into(),
        };
        requirement.validate()?;
        Ok(requirement)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        validate_digest(&self.digest)
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationCaseSpec {
    pub schema_version: u32,
    pub case_id: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub scenario_id: String,
    pub task_contract_digest: String,
    pub fixture: FixtureDescriptor,
    pub scorer: ScorerDescriptor,
    pub metric_schema: MetricSchema,
    pub authority: Option<EvaluationAuthority>,
    pub allowed_effects: Vec<EvaluationEffect>,
    pub expected_terminal: Option<EvaluationTerminal>,
    pub holdout: Option<HoldoutMarker>,
    pub policy_revision: String,
    pub schema_revision: String,
    pub model_class: String,
    pub idempotency_key: String,
    pub cancellation: CancellationPolicy,
    pub budget: EvaluationBudget,
    pub artifact_requirements: Vec<ArtifactRequirement>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub schema_version: u32,
    pub case_id: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub scenario_id: String,
    pub task_contract_digest: String,
    pub fixture: FixtureDescriptor,
    pub scorer: ScorerDescriptor,
    pub metric_schema: MetricSchema,
    pub authority: EvaluationAuthority,
    pub allowed_effects: Vec<EvaluationEffect>,
    pub expected_terminal: EvaluationTerminal,
    pub holdout: HoldoutMarker,
    pub policy_revision: String,
    pub schema_revision: String,
    pub model_class: String,
    pub idempotency_key: String,
    pub cancellation: CancellationPolicy,
    pub budget: EvaluationBudget,
    pub artifact_requirements: Vec<ArtifactRequirement>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationCaseWire {
    schema_version: u32,
    case_id: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    scenario_id: String,
    task_contract_digest: String,
    fixture: FixtureDescriptor,
    scorer: ScorerDescriptor,
    metric_schema: MetricSchema,
    authority: EvaluationAuthority,
    allowed_effects: Vec<EvaluationEffect>,
    expected_terminal: EvaluationTerminal,
    holdout: HoldoutMarker,
    policy_revision: String,
    schema_revision: String,
    model_class: String,
    idempotency_key: String,
    cancellation: CancellationPolicy,
    budget: EvaluationBudget,
    artifact_requirements: Vec<ArtifactRequirement>,
}

impl<'de> Deserialize<'de> for EvaluationCase {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EvaluationCaseWire::deserialize(deserializer)?;
        let case = Self {
            schema_version: wire.schema_version,
            case_id: wire.case_id,
            project_id: wire.project_id,
            run_id: wire.run_id,
            trace_id: wire.trace_id,
            scenario_id: wire.scenario_id,
            task_contract_digest: wire.task_contract_digest,
            fixture: wire.fixture,
            scorer: wire.scorer,
            metric_schema: wire.metric_schema,
            authority: wire.authority,
            allowed_effects: wire.allowed_effects,
            expected_terminal: wire.expected_terminal,
            holdout: wire.holdout,
            policy_revision: wire.policy_revision,
            schema_revision: wire.schema_revision,
            model_class: wire.model_class,
            idempotency_key: wire.idempotency_key,
            cancellation: wire.cancellation,
            budget: wire.budget,
            artifact_requirements: wire.artifact_requirements,
        };
        case.validate().map_err(D::Error::custom)?;
        Ok(case)
    }
}

impl EvaluationCase {
    pub fn new(spec: EvaluationCaseSpec) -> Result<Self, EvaluationContractError> {
        let authority = spec
            .authority
            .ok_or(EvaluationContractError::MissingAuthority)?;
        let expected_terminal = spec
            .expected_terminal
            .ok_or(EvaluationContractError::MissingExpectedTerminal)?;
        let holdout = spec
            .holdout
            .ok_or(EvaluationContractError::MissingHoldout)?;
        let case = Self {
            schema_version: spec.schema_version,
            case_id: spec.case_id,
            project_id: spec.project_id,
            run_id: spec.run_id,
            trace_id: spec.trace_id,
            scenario_id: spec.scenario_id,
            task_contract_digest: spec.task_contract_digest,
            fixture: spec.fixture,
            scorer: spec.scorer,
            metric_schema: spec.metric_schema,
            authority,
            allowed_effects: spec.allowed_effects,
            expected_terminal,
            holdout,
            policy_revision: spec.policy_revision,
            schema_revision: spec.schema_revision,
            model_class: spec.model_class,
            idempotency_key: spec.idempotency_key,
            cancellation: spec.cancellation,
            budget: spec.budget,
            artifact_requirements: spec.artifact_requirements,
        };
        case.validate()?;
        Ok(case)
    }

    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        if self.schema_version != EVALUATION_CASE_SCHEMA_VERSION {
            return Err(EvaluationContractError::UnsupportedSchemaVersion);
        }
        if self.project_id.as_uuid().is_nil()
            || self.run_id.as_uuid().is_nil()
            || self.trace_id.as_uuid().is_nil()
        {
            return Err(EvaluationContractError::InvalidShape);
        }
        validate_text(&self.case_id, MAX_CASE_ID_BYTES)?;
        validate_text(&self.scenario_id, MAX_CASE_ID_BYTES)?;
        validate_digest(&self.task_contract_digest)?;
        validate_text(&self.policy_revision, MAX_CASE_ID_BYTES)?;
        validate_text(&self.schema_revision, MAX_CASE_ID_BYTES)?;
        validate_text(&self.model_class, MAX_CASE_ID_BYTES)?;
        validate_text(&self.idempotency_key, MAX_CASE_ID_BYTES)?;
        self.fixture.validate()?;
        self.scorer.validate()?;
        self.metric_schema.validate()?;
        self.holdout.validate()?;
        self.cancellation.validate()?;
        self.budget.validate()?;
        validate_effects(self.authority, &self.allowed_effects)?;

        if self.artifact_requirements.is_empty() || self.artifact_requirements.len() > MAX_ARTIFACTS
        {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        let mut artifacts = BTreeSet::new();
        for artifact in &self.artifact_requirements {
            artifact.validate()?;
            if !artifacts.insert((&artifact.kind, &artifact.digest)) {
                return Err(EvaluationContractError::InvalidShape);
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> String {
        digest_bytes(&serde_json::to_vec(self).expect("evaluation case is serializable"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationEvidenceStatus {
    Pass,
    Fail,
    Blocked,
    NoProof,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidence {
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_digest: String,
    pub schema_digest: String,
    pub fixture_digest: String,
    pub environment_digest: String,
    pub artifact_digests: Vec<String>,
    pub status: EvaluationEvidenceStatus,
}

impl EvaluationEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        policy_digest: impl Into<String>,
        schema_digest: impl Into<String>,
        fixture_digest: impl Into<String>,
        environment_digest: impl Into<String>,
        artifact_digests: Vec<String>,
        status: EvaluationEvidenceStatus,
    ) -> Result<Self, EvaluationContractError> {
        let evidence = Self {
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            policy_digest: policy_digest.into(),
            schema_digest: schema_digest.into(),
            fixture_digest: fixture_digest.into(),
            environment_digest: environment_digest.into(),
            artifact_digests,
            status,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    fn validate(&self) -> Result<(), EvaluationContractError> {
        validate_digest(&self.head_sha)?;
        validate_digest(&self.tree_sha)?;
        validate_digest(&self.policy_digest)?;
        validate_digest(&self.schema_digest)?;
        validate_digest(&self.fixture_digest)?;
        validate_digest(&self.environment_digest)?;
        if self.artifact_digests.is_empty() || self.artifact_digests.len() > MAX_ARTIFACTS {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        for digest in &self.artifact_digests {
            validate_digest(digest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineReport {
    pub schema_version: u32,
    pub case_id: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub fixture_digest: String,
    pub scorer_digest: String,
    pub policy_revision: String,
    pub schema_revision: String,
    pub model_class: String,
    pub holdout: HoldoutMarker,
    pub terminal: EvaluationTerminal,
    pub metrics: Vec<MetricObservation>,
    pub evidence: EvaluationEvidence,
    pub report_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BaselineReportWire {
    schema_version: u32,
    case_id: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    fixture_digest: String,
    scorer_digest: String,
    policy_revision: String,
    schema_revision: String,
    model_class: String,
    holdout: HoldoutMarker,
    terminal: EvaluationTerminal,
    metrics: Vec<MetricObservation>,
    evidence: EvaluationEvidence,
    report_digest: String,
}

impl<'de> Deserialize<'de> for BaselineReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BaselineReportWire::deserialize(deserializer)?;
        let report = Self {
            schema_version: wire.schema_version,
            case_id: wire.case_id,
            project_id: wire.project_id,
            run_id: wire.run_id,
            trace_id: wire.trace_id,
            fixture_digest: wire.fixture_digest,
            scorer_digest: wire.scorer_digest,
            policy_revision: wire.policy_revision,
            schema_revision: wire.schema_revision,
            model_class: wire.model_class,
            holdout: wire.holdout,
            terminal: wire.terminal,
            metrics: wire.metrics,
            evidence: wire.evidence,
            report_digest: wire.report_digest,
        };
        report
            .validate_shape()
            .and_then(|()| report.validate_digest())
            .map_err(D::Error::custom)?;
        Ok(report)
    }
}

impl BaselineReport {
    pub fn from_case(
        case: &EvaluationCase,
        terminal: EvaluationTerminal,
        metrics: Vec<MetricObservation>,
        evidence: EvaluationEvidence,
    ) -> Result<Self, EvaluationContractError> {
        let report = Self {
            schema_version: BASELINE_REPORT_SCHEMA_VERSION,
            case_id: case.case_id.clone(),
            project_id: case.project_id,
            run_id: case.run_id,
            trace_id: case.trace_id,
            fixture_digest: case.fixture.fixture_digest.clone(),
            scorer_digest: case.scorer.scorer_digest.clone(),
            policy_revision: case.policy_revision.clone(),
            schema_revision: case.schema_revision.clone(),
            model_class: case.model_class.clone(),
            holdout: case.holdout.clone(),
            terminal,
            metrics,
            evidence,
            report_digest: String::new(),
        };
        report.validate_against_inner(case, true)?;
        let mut report = report;
        report.report_digest = report.content_digest();
        Ok(report)
    }

    pub fn validate_against(&self, case: &EvaluationCase) -> Result<(), EvaluationContractError> {
        self.validate_against_inner(case, false)
    }

    pub fn can_activate(&self) -> bool {
        false
    }

    fn validate_against_inner(
        &self,
        case: &EvaluationCase,
        allow_empty_digest: bool,
    ) -> Result<(), EvaluationContractError> {
        self.validate_shape()?;
        if self.case_id != case.case_id
            || self.project_id != case.project_id
            || self.run_id != case.run_id
            || self.trace_id != case.trace_id
            || self.fixture_digest != case.fixture.fixture_digest
            || self.scorer_digest != case.scorer.scorer_digest
            || self.policy_revision != case.policy_revision
            || self.schema_revision != case.schema_revision
            || self.model_class != case.model_class
            || self.holdout != case.holdout
        {
            return Err(EvaluationContractError::IdentityMismatch);
        }
        if self.evidence.fixture_digest != case.fixture.fixture_digest {
            return Err(EvaluationContractError::EvidenceMismatch);
        }
        if !case
            .artifact_requirements
            .iter()
            .all(|requirement| self.evidence.artifact_digests.contains(&requirement.digest))
        {
            return Err(EvaluationContractError::EvidenceMismatch);
        }
        if self.metrics.len() < case.metric_schema.metrics.len() {
            return Err(EvaluationContractError::MissingMetric);
        }
        if self.metrics.len() > case.metric_schema.metrics.len() {
            return Err(EvaluationContractError::UnexpectedMetric);
        }
        let mut observations = BTreeSet::new();
        for (observation, definition) in self.metrics.iter().zip(&case.metric_schema.metrics) {
            if observation.name != definition.name {
                return Err(
                    if case
                        .metric_schema
                        .metrics
                        .iter()
                        .any(|candidate| candidate.name == observation.name)
                    {
                        EvaluationContractError::MissingMetric
                    } else {
                        EvaluationContractError::UnexpectedMetric
                    },
                );
            }
            if !observations.insert(observation.name) {
                return Err(EvaluationContractError::InvalidMetric);
            }
            observation.validate_against(definition)?;
        }
        if !evidence_status_matches(self.terminal, self.evidence.status) {
            return Err(EvaluationContractError::InvalidEvidenceStatus);
        }
        if !allow_empty_digest {
            self.validate_digest()?;
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), EvaluationContractError> {
        if self.schema_version != BASELINE_REPORT_SCHEMA_VERSION {
            return Err(EvaluationContractError::UnsupportedSchemaVersion);
        }
        if self.project_id.as_uuid().is_nil()
            || self.run_id.as_uuid().is_nil()
            || self.trace_id.as_uuid().is_nil()
        {
            return Err(EvaluationContractError::InvalidShape);
        }
        validate_text(&self.case_id, MAX_CASE_ID_BYTES)?;
        validate_digest(&self.fixture_digest)?;
        validate_digest(&self.scorer_digest)?;
        validate_text(&self.policy_revision, MAX_CASE_ID_BYTES)?;
        validate_text(&self.schema_revision, MAX_CASE_ID_BYTES)?;
        validate_text(&self.model_class, MAX_CASE_ID_BYTES)?;
        self.holdout.validate()?;
        self.evidence.validate()?;
        if self.metrics.is_empty() || self.metrics.len() > MAX_METRICS {
            return Err(EvaluationContractError::BoundsExceeded);
        }
        Ok(())
    }

    fn validate_digest(&self) -> Result<(), EvaluationContractError> {
        if self.report_digest.is_empty() || self.report_digest != self.content_digest() {
            return Err(EvaluationContractError::InvalidDigest);
        }
        Ok(())
    }

    fn content_digest(&self) -> String {
        let mut content = self.clone();
        content.report_digest.clear();
        digest_bytes(&serde_json::to_vec(&content).expect("baseline report is serializable"))
    }
}

fn validate_effects(
    authority: EvaluationAuthority,
    effects: &[EvaluationEffect],
) -> Result<(), EvaluationContractError> {
    if effects.len() > MAX_EFFECTS {
        return Err(EvaluationContractError::BoundsExceeded);
    }
    let mut unique = BTreeSet::new();
    for effect in effects {
        if !unique.insert(effect) {
            return Err(EvaluationContractError::InvalidShape);
        }
        if *effect == EvaluationEffect::ExternalWrite
            || (authority == EvaluationAuthority::ReadOnly && *effect != EvaluationEffect::ReadOnly)
        {
            return Err(EvaluationContractError::UnsafeEffect);
        }
    }
    Ok(())
}

fn evidence_status_matches(terminal: EvaluationTerminal, status: EvaluationEvidenceStatus) -> bool {
    match terminal {
        EvaluationTerminal::Pass => status == EvaluationEvidenceStatus::Pass,
        EvaluationTerminal::Fail => status == EvaluationEvidenceStatus::Fail,
        EvaluationTerminal::Blocked
        | EvaluationTerminal::Cancelled
        | EvaluationTerminal::NoProof => {
            matches!(
                status,
                EvaluationEvidenceStatus::Blocked | EvaluationEvidenceStatus::NoProof
            )
        }
    }
}

fn validate_text(value: &str, max_bytes: usize) -> Result<(), EvaluationContractError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(EvaluationContractError::BoundsExceeded);
    }
    let lowercase = value.to_ascii_lowercase();
    if [
        "-----begin",
        "api_key",
        "apikey",
        "authorization:",
        "chain_of_thought",
        "password",
        "prompt",
        "secret",
        "token=",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
    {
        return Err(EvaluationContractError::SensitiveValue);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), EvaluationContractError> {
    validate_text(value, MAX_DIGEST_BYTES)
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
