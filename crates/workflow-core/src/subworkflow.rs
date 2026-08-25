//! Bounded, deterministic SubWorkflowNode composition planning.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;
const MAX_MAPPING_ENTRIES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubWorkflowReference {
    pub project_id: String,
    pub workflow_id: String,
    pub version: String,
}

impl SubWorkflowReference {
    pub fn new(
        project_id: impl Into<String>,
        workflow_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Self, CompositionError> {
        let reference = Self {
            project_id: project_id.into(),
            workflow_id: workflow_id.into(),
            version: version.into(),
        };
        for value in [
            &reference.project_id,
            &reference.workflow_id,
            &reference.version,
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
                return Err(CompositionError::InvalidReference);
            }
        }
        Ok(reference)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputMapping {
    pub destination: String,
    pub source: String,
}

impl InputMapping {
    pub fn new(
        destination: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, CompositionError> {
        let mapping = Self {
            destination: destination.into(),
            source: source.into(),
        };
        if mapping.destination.trim().is_empty()
            || mapping.source.trim().is_empty()
            || mapping.destination.len() > MAX_ID_BYTES
            || mapping.source.len() > MAX_ID_BYTES
        {
            return Err(CompositionError::InvalidMapping);
        }
        Ok(mapping)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CompositionError {
    #[error("subworkflow reference is invalid")]
    InvalidReference,
    #[error("subworkflow mapping is invalid")]
    InvalidMapping,
    #[error("subworkflow catalog capacity is invalid")]
    InvalidCapacity,
    #[error("subworkflow catalog is full")]
    CapacityFull,
    #[error("subworkflow version is already registered")]
    DuplicateVersion,
    #[error("subworkflow version was not found")]
    VersionNotFound,
    #[error("cross-project subworkflow access is denied")]
    CrossProjectDenied,
    #[error("subworkflow depth limit exceeded")]
    DepthLimit,
    #[error("subworkflow budget exceeded")]
    BudgetExceeded,
    #[error("subworkflow cycle detected")]
    CycleDetected,
    #[error("subworkflow input source is missing")]
    MappingSourceMissing,
    #[error("subworkflow child correlation is invalid")]
    InvalidCorrelation,
}

#[derive(Debug, Clone)]
pub struct SubWorkflowCatalog {
    max_entries: usize,
    versions: BTreeSet<SubWorkflowReference>,
}

impl SubWorkflowCatalog {
    pub fn new(max_entries: usize) -> Result<Self, CompositionError> {
        if max_entries == 0 {
            return Err(CompositionError::InvalidCapacity);
        }
        Ok(Self {
            max_entries,
            versions: BTreeSet::new(),
        })
    }

    pub fn register(&mut self, reference: SubWorkflowReference) -> Result<(), CompositionError> {
        if self.versions.len() >= self.max_entries {
            return Err(CompositionError::CapacityFull);
        }
        if !self.versions.insert(reference) {
            return Err(CompositionError::DuplicateVersion);
        }
        Ok(())
    }

    fn contains(&self, reference: &SubWorkflowReference) -> bool {
        self.versions.contains(reference)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildRunState {
    Planned,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRunPlan {
    pub child_run_id: String,
    pub reference: SubWorkflowReference,
    pub child_inputs: BTreeMap<String, Value>,
    state: ChildRunState,
}

impl ChildRunPlan {
    pub fn cancel(&mut self) -> bool {
        if self.state == ChildRunState::Cancelled {
            return false;
        }
        self.state = ChildRunState::Cancelled;
        true
    }

    pub fn is_cancelled(&self) -> bool {
        self.state == ChildRunState::Cancelled
    }
}

#[derive(Debug, Clone)]
pub struct SubWorkflowPlan {
    parent_project_id: String,
    parent_workflow_id: String,
    parent_run_id: String,
    node_id: String,
    generation: u64,
    reference: SubWorkflowReference,
    mapping: BTreeMap<String, String>,
    depth: u16,
    max_depth: u16,
    budget_cost: u64,
    budget_limit: u64,
}

impl SubWorkflowPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent_project_id: impl Into<String>,
        parent_workflow_id: impl Into<String>,
        parent_run_id: impl Into<String>,
        node_id: impl Into<String>,
        generation: u64,
        reference: SubWorkflowReference,
        mapping: BTreeMap<String, String>,
        depth: u16,
        max_depth: u16,
        budget_cost: u64,
        budget_limit: u64,
    ) -> Result<Self, CompositionError> {
        let plan = Self {
            parent_project_id: parent_project_id.into(),
            parent_workflow_id: parent_workflow_id.into(),
            parent_run_id: parent_run_id.into(),
            node_id: node_id.into(),
            generation,
            reference,
            mapping,
            depth,
            max_depth,
            budget_cost,
            budget_limit,
        };
        for value in [
            &plan.parent_project_id,
            &plan.parent_workflow_id,
            &plan.parent_run_id,
            &plan.node_id,
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
                return Err(CompositionError::InvalidCorrelation);
            }
        }
        if plan.mapping.len() > MAX_MAPPING_ENTRIES || plan.max_depth == 0 {
            return Err(CompositionError::InvalidMapping);
        }
        for (destination, source) in &plan.mapping {
            InputMapping::new(destination.clone(), source.clone())?;
        }
        Ok(plan)
    }

    pub fn resolve(
        &self,
        catalog: &SubWorkflowCatalog,
        cross_project_grant: bool,
        parent_inputs: &BTreeMap<String, Value>,
    ) -> Result<ChildRunPlan, CompositionError> {
        if !catalog.contains(&self.reference) {
            return Err(CompositionError::VersionNotFound);
        }
        if self.reference.project_id != self.parent_project_id && !cross_project_grant {
            return Err(CompositionError::CrossProjectDenied);
        }
        if self.parent_workflow_id == self.reference.workflow_id {
            return Err(CompositionError::CycleDetected);
        }
        if self.depth >= self.max_depth {
            return Err(CompositionError::DepthLimit);
        }
        if self.budget_cost > self.budget_limit {
            return Err(CompositionError::BudgetExceeded);
        }
        let mut child_inputs = BTreeMap::new();
        for (destination, source) in &self.mapping {
            let value = parent_inputs
                .get(source)
                .ok_or(CompositionError::MappingSourceMissing)?;
            child_inputs.insert(destination.clone(), value.clone());
        }
        let child_run_id = format!(
            "child:{}:{}:{}",
            self.parent_run_id, self.node_id, self.generation
        );
        if child_run_id.len() > 3 * MAX_ID_BYTES {
            return Err(CompositionError::InvalidCorrelation);
        }
        Ok(ChildRunPlan {
            child_run_id,
            reference: self.reference.clone(),
            child_inputs,
            state: ChildRunState::Planned,
        })
    }
}
