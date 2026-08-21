//! Deterministic, fail-closed permission evaluation for tool calls.

use crate::context::PolicyDecision;
use agent_core::ids::ProjectId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::RwLock;

pub const MAX_CACHED_APPROVALS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    Read,
    Write,
    Execute,
    Network,
    Credentials,
    Payment,
    InstallPackage,
    ForcePush,
}

impl ToolEffect {
    fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::Write
                | Self::Execute
                | Self::Credentials
                | Self::Payment
                | Self::InstallPackage
                | Self::ForcePush
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub project_id: Option<ProjectId>,
    pub tool_name: String,
    pub tool_version: String,
    pub capability: String,
    pub effect: ToolEffect,
    pub policy: PolicyDecision,
    pub budget_available: bool,
    pub confirmation_approved: bool,
}

impl PermissionRequest {
    pub fn validate(&self) -> Result<(), PermissionError> {
        if self.project_id.is_none() {
            return Err(PermissionError::MissingProject);
        }
        if self.tool_name.trim().is_empty() || self.tool_version.trim().is_empty() {
            return Err(PermissionError::InvalidToolIdentity);
        }
        if self.capability.trim().is_empty() {
            return Err(PermissionError::MissingCapability);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allowed { reason: &'static str },
    NeedsConfirmation { scope: String },
    Denied { reason: PermissionError },
}

impl PermissionDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissionError {
    #[error("project identity is required")]
    MissingProject,
    #[error("tool identity is invalid")]
    InvalidToolIdentity,
    #[error("capability is required")]
    MissingCapability,
    #[error("policy denies tool execution")]
    PolicyDenied,
    #[error("budget is unavailable")]
    BudgetUnavailable,
    #[error("confirmation is required")]
    ConfirmationRequired,
    #[error("approval cache is full")]
    ApprovalCacheFull,
}

#[derive(Debug, Default)]
pub struct PermissionEvaluator {
    approvals: RwLock<BTreeSet<String>>,
}

impl PermissionEvaluator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(&self, request: &PermissionRequest) -> PermissionDecision {
        if let Err(error) = request.validate() {
            return PermissionDecision::Denied { reason: error };
        }
        if request.policy == PolicyDecision::Deny {
            return PermissionDecision::Denied {
                reason: PermissionError::PolicyDenied,
            };
        }
        if !request.budget_available {
            return PermissionDecision::Denied {
                reason: PermissionError::BudgetUnavailable,
            };
        }
        if !request.effect.requires_confirmation() {
            return PermissionDecision::Allowed {
                reason: "policy-and-budget-allow-read-only-effect",
            };
        }

        let scope = approval_scope(request);
        match request.policy {
            PolicyDecision::Allow => PermissionDecision::Allowed {
                reason: "explicit-policy-allow",
            },
            PolicyDecision::AskEveryTime => {
                if request.confirmation_approved {
                    PermissionDecision::Allowed {
                        reason: "explicit-confirmation",
                    }
                } else {
                    PermissionDecision::NeedsConfirmation { scope }
                }
            }
            PolicyDecision::AskOnce => {
                let cached = self
                    .approvals
                    .read()
                    .map(|approvals| approvals.contains(&scope))
                    .unwrap_or(false);
                if cached || request.confirmation_approved {
                    if request.confirmation_approved {
                        let _ = self.cache_approval(scope.clone());
                    }
                    PermissionDecision::Allowed {
                        reason: if cached {
                            "cached-scoped-confirmation"
                        } else {
                            "explicit-confirmation-cached"
                        },
                    }
                } else {
                    PermissionDecision::NeedsConfirmation { scope }
                }
            }
            PolicyDecision::Deny => unreachable!("deny policy handled before effect evaluation"),
        }
    }

    pub fn clear_project(&self, project_id: &ProjectId) {
        if let Ok(mut approvals) = self.approvals.write() {
            let prefix = format!("{}:", project_id);
            approvals.retain(|scope| !scope.starts_with(&prefix));
        }
    }

    fn cache_approval(&self, scope: String) -> Result<(), PermissionError> {
        let mut approvals = self
            .approvals
            .write()
            .map_err(|_| PermissionError::ApprovalCacheFull)?;
        if approvals.len() >= MAX_CACHED_APPROVALS && !approvals.contains(&scope) {
            return Err(PermissionError::ApprovalCacheFull);
        }
        approvals.insert(scope);
        Ok(())
    }
}

fn approval_scope(request: &PermissionRequest) -> String {
    format!(
        "{}:{}:{}:{}",
        request.project_id.as_ref().expect("validated project"),
        request.tool_name,
        request.tool_version,
        request.capability
    )
}
