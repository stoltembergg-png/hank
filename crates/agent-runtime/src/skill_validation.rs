//! Deterministic, fail-closed validation boundary for Skill candidates.
//!
//! Validation consumes already parsed, untrusted data and produces a bounded
//! redacted report. It never resolves references, executes artifacts, calls a
//! provider, or mutates a persisted or active Skill.

use crate::skill_testing::SkillTestReport;
use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ParsedSkill, ProjectId, Resource, SkillId,
    SkillScope, TraceId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub const SKILL_VALIDATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_VALIDATION_RULES: usize = 16;
pub const MAX_VALIDATION_REASONS: usize = 16;
pub const MAX_DEPENDENCY_NODES: usize = 32;
pub const MAX_DEPENDENCY_DEPTH: usize = 8;
const MAX_ACTOR_ID_BYTES: usize = 128;
const DIGEST_BYTES: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillValidationPolicy {
    pub allowed_capabilities: CapabilitySet,
}

#[derive(Debug, Clone)]
pub struct SkillValidationRequest {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillValidationPolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub dependency_graph: Vec<SkillDependencyNode>,
}

#[derive(Debug, Clone)]
pub struct SkillDependencyNode {
    pub skill_id: SkillId,
    pub dependencies: Vec<SkillId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillValidationStatus {
    Passed,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillValidationRule {
    Identity,
    Manifest,
    ParserQuarantine,
    Policy,
    Capabilities,
    Paths,
    Dependencies,
    Tests,
    Budget,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillValidationReason {
    IdentityMismatch,
    InvalidActor,
    InvalidManifest,
    ParserQuarantine,
    PolicyRejected,
    CapabilityUnsupported,
    CapabilityNotAllowed,
    PathEscape,
    DependencyCycle,
    DependencyDepthExceeded,
    DependencyGraphInvalid,
    TestsMissing,
    TestEvidenceMismatch,
    TestFailed,
    BudgetExceeded,
    TraceMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillValidationRuleResult {
    pub rule: SkillValidationRule,
    pub passed: bool,
}

/// Redacted validation evidence suitable for audit/event storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillValidationReport {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub trace_id: TraceId,
    pub status: SkillValidationStatus,
    pub rules: Vec<SkillValidationRuleResult>,
    pub quarantine_reasons: Vec<SkillValidationReason>,
    pub policy_digest: String,
    pub budget_digest: String,
    pub content_digest: String,
    pub test_digest: Option<String>,
    pub report_digest: String,
}

pub struct SkillValidationService;

impl SkillValidationService {
    pub fn validate(
        candidate: &ParsedSkill,
        request: &SkillValidationRequest,
        test_report: Option<&SkillTestReport>,
    ) -> SkillValidationReport {
        let mut rules = Vec::with_capacity(MAX_VALIDATION_RULES);
        let mut reasons = Vec::with_capacity(MAX_VALIDATION_REASONS);

        let identity_valid = candidate.manifest.id == request.skill_id
            && candidate.manifest.version == request.version
            && candidate.manifest.scope == SkillScope::Project
            && candidate.provenance.project_id == Some(request.project_id)
            && !request.actor_id.trim().is_empty()
            && request.actor_id.len() <= MAX_ACTOR_ID_BYTES;
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Identity,
            identity_valid,
            if request.actor_id.trim().is_empty() || request.actor_id.len() > MAX_ACTOR_ID_BYTES {
                SkillValidationReason::InvalidActor
            } else {
                SkillValidationReason::IdentityMismatch
            },
        );

        let manifest_valid = candidate.manifest.validate().is_ok();
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Manifest,
            manifest_valid,
            SkillValidationReason::InvalidManifest,
        );

        let parser_clean = !candidate.quarantined
            && !candidate.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == agent_core::SkillDiagnosticSeverity::Quarantine
            });
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::ParserQuarantine,
            parser_clean,
            SkillValidationReason::ParserQuarantine,
        );

        let policy_valid = !candidate.manifest.policy.allow_runtime_mutation
            && !candidate.manifest.policy.allow_instruction_override
            && request
                .policy
                .allowed_capabilities
                .contains(&request.capability);
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Policy,
            policy_valid,
            SkillValidationReason::PolicyRejected,
        );

        let capabilities_valid = candidate.manifest.capabilities.iter().all(|capability| {
            is_supported_capability(capability)
                && request.policy.allowed_capabilities.contains(capability)
        });
        if !candidate
            .manifest
            .capabilities
            .iter()
            .all(is_supported_capability)
        {
            push_reason(&mut reasons, SkillValidationReason::CapabilityUnsupported);
        }
        if !candidate
            .manifest
            .capabilities
            .iter()
            .all(|capability| request.policy.allowed_capabilities.contains(capability))
        {
            push_reason(&mut reasons, SkillValidationReason::CapabilityNotAllowed);
        }
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Capabilities,
            capabilities_valid,
            SkillValidationReason::CapabilityNotAllowed,
        );

        let paths_valid = paths_are_safe(candidate);
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Paths,
            paths_valid,
            SkillValidationReason::PathEscape,
        );

        let dependencies_valid = dependency_graph_is_safe(candidate, request, &mut reasons);
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Dependencies,
            dependencies_valid,
            SkillValidationReason::DependencyGraphInvalid,
        );

        let tests_valid = test_evidence_is_safe(candidate, request, test_report, &mut reasons);
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Tests,
            tests_valid,
            SkillValidationReason::TestEvidenceMismatch,
        );

        let budget_valid = budget_is_safe(candidate, request);
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Budget,
            budget_valid,
            SkillValidationReason::BudgetExceeded,
        );

        let trace_valid = candidate.manifest.trace.trace_id == request.trace_id
            && candidate.provenance.trace_id == request.trace_id
            && !request.trace_id.as_uuid().is_nil();
        push_rule(
            &mut rules,
            &mut reasons,
            SkillValidationRule::Trace,
            trace_valid,
            SkillValidationReason::TraceMismatch,
        );

        reasons.sort_by_key(|reason| *reason as u8);
        reasons.dedup();
        reasons.truncate(MAX_VALIDATION_REASONS);
        rules.truncate(MAX_VALIDATION_RULES);
        let status = if reasons.is_empty() {
            SkillValidationStatus::Passed
        } else {
            SkillValidationStatus::Quarantined
        };
        let policy_digest = digest_json(&request.policy);
        let budget_digest = digest_json(&request.budget);
        let content_digest = digest_json(candidate);
        let test_digest = test_report.map(|report| report.fixture_digest.clone());
        let report_digest = digest_json(&ReportFingerprint {
            schema_version: SKILL_VALIDATION_SCHEMA_VERSION,
            project_id: request.project_id,
            skill_id: request.skill_id,
            version: request.version.clone(),
            trace_id: request.trace_id,
            status,
            rules: rules.clone(),
            quarantine_reasons: reasons.clone(),
            policy_digest: policy_digest.clone(),
            budget_digest: budget_digest.clone(),
            content_digest: content_digest.clone(),
            test_digest: test_digest.clone(),
        });

        SkillValidationReport {
            schema_version: SKILL_VALIDATION_SCHEMA_VERSION,
            project_id: request.project_id,
            skill_id: request.skill_id,
            version: request.version.clone(),
            trace_id: request.trace_id,
            status,
            rules,
            quarantine_reasons: reasons,
            policy_digest,
            budget_digest,
            content_digest,
            test_digest,
            report_digest,
        }
    }

    /// Verifies that a previously generated passing report still describes
    /// this exact candidate before a lifecycle mutation is allowed.
    pub fn report_is_approved(
        candidate: &ParsedSkill,
        project_id: ProjectId,
        skill_id: SkillId,
        version: &str,
        report: &SkillValidationReport,
    ) -> bool {
        if report.schema_version != SKILL_VALIDATION_SCHEMA_VERSION
            || report.status != SkillValidationStatus::Passed
            || !report.quarantine_reasons.is_empty()
            || report.rules.iter().any(|rule| !rule.passed)
            || report.project_id != project_id
            || report.skill_id != skill_id
            || report.version != version
            || candidate.manifest.id != skill_id
            || candidate.manifest.version != version
            || candidate.provenance.project_id != Some(project_id)
            || candidate.provenance.trace_id != report.trace_id
            || report.trace_id.as_uuid().is_nil()
            || report.rules.len() != 10
            || report.content_digest != digest_json(candidate)
            || !is_digest(&report.policy_digest)
            || !is_digest(&report.budget_digest)
            || !report.test_digest.as_deref().is_some_and(is_digest)
        {
            return false;
        }
        let expected_digest = digest_json(&ReportFingerprint {
            schema_version: report.schema_version,
            project_id: report.project_id,
            skill_id: report.skill_id,
            version: report.version.clone(),
            trace_id: report.trace_id,
            status: report.status,
            rules: report.rules.clone(),
            quarantine_reasons: report.quarantine_reasons.clone(),
            policy_digest: report.policy_digest.clone(),
            budget_digest: report.budget_digest.clone(),
            content_digest: report.content_digest.clone(),
            test_digest: report.test_digest.clone(),
        });
        report.report_digest == expected_digest
    }
}

#[derive(Debug, Serialize)]
struct ReportFingerprint {
    schema_version: u32,
    project_id: ProjectId,
    skill_id: SkillId,
    version: String,
    trace_id: TraceId,
    status: SkillValidationStatus,
    rules: Vec<SkillValidationRuleResult>,
    quarantine_reasons: Vec<SkillValidationReason>,
    policy_digest: String,
    budget_digest: String,
    content_digest: String,
    test_digest: Option<String>,
}

fn push_rule(
    rules: &mut Vec<SkillValidationRuleResult>,
    reasons: &mut Vec<SkillValidationReason>,
    rule: SkillValidationRule,
    passed: bool,
    reason: SkillValidationReason,
) {
    rules.push(SkillValidationRuleResult { rule, passed });
    if !passed {
        push_reason(reasons, reason);
    }
}

fn push_reason(reasons: &mut Vec<SkillValidationReason>, reason: SkillValidationReason) {
    if !reasons.contains(&reason) && reasons.len() < MAX_VALIDATION_REASONS {
        reasons.push(reason);
    }
}

fn is_supported_capability(capability: &Capability) -> bool {
    matches!(
        capability.resource,
        Resource::File | Resource::Memory | Resource::Skill
    ) && matches!(capability.action, Action::Read | Action::List)
        && capability
            .scope
            .as_deref()
            .is_none_or(|scope| !scope.trim().is_empty() && !scope.contains('*'))
}

fn paths_are_safe(candidate: &ParsedSkill) -> bool {
    candidate
        .manifest
        .files
        .iter()
        .all(|file| safe_relative_path(&file.path))
        && candidate
            .manifest
            .tests
            .iter()
            .all(|path| safe_relative_path(path))
        && candidate
            .artifacts
            .iter()
            .all(|artifact| safe_relative_path(&artifact.path))
        && candidate.links.iter().all(|link| {
            !matches!(link.kind, agent_core::SkillLinkKind::External)
                && safe_relative_path(&link.source_path)
                && (matches!(link.kind, agent_core::SkillLinkKind::Anchor)
                    || safe_relative_path(&link.target))
        })
}

fn safe_relative_path(path: &str) -> bool {
    !path.trim().is_empty()
        && !path.chars().any(char::is_control)
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains(':')
        && !path.split(['/', '\\']).any(|segment| segment == "..")
}

fn dependency_graph_is_safe(
    candidate: &ParsedSkill,
    request: &SkillValidationRequest,
    reasons: &mut Vec<SkillValidationReason>,
) -> bool {
    if request.dependency_graph.is_empty() || request.dependency_graph.len() > MAX_DEPENDENCY_NODES
    {
        push_reason(reasons, SkillValidationReason::DependencyGraphInvalid);
        return false;
    }
    let mut graph = HashMap::new();
    for node in &request.dependency_graph {
        if graph
            .insert(node.skill_id, node.dependencies.clone())
            .is_some()
        {
            push_reason(reasons, SkillValidationReason::DependencyGraphInvalid);
            return false;
        }
    }
    let Some(candidate_dependencies) = graph.get(&candidate.manifest.id) else {
        push_reason(reasons, SkillValidationReason::DependencyGraphInvalid);
        return false;
    };
    if candidate
        .manifest
        .dependencies
        .iter()
        .any(|dependency| !candidate_dependencies.contains(&dependency.skill_id))
    {
        push_reason(reasons, SkillValidationReason::DependencyGraphInvalid);
        return false;
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for skill_id in graph.keys() {
        if let Some(failure) =
            dependency_cycle_or_depth(*skill_id, 0, &graph, &mut visiting, &mut visited)
        {
            push_reason(
                reasons,
                match failure {
                    DependencyFailure::Cycle => SkillValidationReason::DependencyCycle,
                    DependencyFailure::Depth => SkillValidationReason::DependencyDepthExceeded,
                },
            );
            return false;
        }
    }
    true
}

enum DependencyFailure {
    Cycle,
    Depth,
}

fn dependency_cycle_or_depth(
    skill_id: SkillId,
    depth: usize,
    graph: &HashMap<SkillId, Vec<SkillId>>,
    visiting: &mut HashSet<SkillId>,
    visited: &mut HashSet<SkillId>,
) -> Option<DependencyFailure> {
    if depth > MAX_DEPENDENCY_DEPTH {
        return Some(DependencyFailure::Depth);
    }
    if visiting.contains(&skill_id) {
        return Some(DependencyFailure::Cycle);
    }
    if visited.contains(&skill_id) {
        return None;
    }
    visiting.insert(skill_id);
    if let Some(dependencies) = graph.get(&skill_id) {
        for dependency in dependencies {
            if let Some(failure) =
                dependency_cycle_or_depth(*dependency, depth + 1, graph, visiting, visited)
            {
                return Some(failure);
            }
        }
    }
    visiting.remove(&skill_id);
    visited.insert(skill_id);
    None
}

fn test_evidence_is_safe(
    candidate: &ParsedSkill,
    request: &SkillValidationRequest,
    test_report: Option<&SkillTestReport>,
    reasons: &mut Vec<SkillValidationReason>,
) -> bool {
    if candidate.manifest.tests.is_empty() {
        push_reason(reasons, SkillValidationReason::TestsMissing);
        return false;
    }
    let Some(test_report) = test_report else {
        push_reason(reasons, SkillValidationReason::TestsMissing);
        return false;
    };
    if test_report.project_id != request.project_id
        || test_report.skill_id != request.skill_id
        || test_report.version != request.version
        || test_report.trace_id != request.trace_id
    {
        push_reason(reasons, SkillValidationReason::TestEvidenceMismatch);
        return false;
    }
    if test_report.status != "passed"
        || test_report.activation_requested
        || test_report.steps_executed == 0
        || !is_digest(&test_report.fixture_digest)
    {
        push_reason(reasons, SkillValidationReason::TestFailed);
        return false;
    }
    true
}

fn budget_is_safe(candidate: &ParsedSkill, request: &SkillValidationRequest) -> bool {
    request.budget.validate().is_ok()
        && candidate.manifest.budget.validate().is_ok()
        && candidate.manifest.budget.max_tokens <= request.budget.max_tokens
        && candidate.manifest.budget.max_cost_micro_usd <= request.budget.max_cost_micro_usd
        && candidate.manifest.budget.max_parallel_invocations
            <= request.budget.max_parallel_invocations
        && candidate.manifest.budget.max_wall_time_seconds <= request.budget.max_wall_time_seconds
}

fn is_digest(value: &str) -> bool {
    value.len() == DIGEST_BYTES && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
