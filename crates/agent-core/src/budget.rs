//! Políticas de orçamento e tracking de custos.

use crate::ids::{AgentId, ProjectId, SessionId, TaskId, WorkflowId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rastreamento de orçamento por escopo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetTracker {
    pub project_budgets: HashMap<ProjectId, ProjectBudget>,
    pub agent_budgets: HashMap<AgentId, AgentBudget>,
    pub session_budgets: HashMap<SessionId, SessionBudget>,
    pub workflow_budgets: HashMap<WorkflowId, WorkflowBudget>,
    pub task_budgets: HashMap<TaskId, TaskBudget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBudget {
    pub project_id: ProjectId,
    pub max_tokens: u64,
    pub max_cost_usd: f64,
    pub used_tokens: u64,
    pub used_cost_usd: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBudget {
    pub agent_id: AgentId,
    pub project_id: ProjectId,
    pub max_tokens_per_request: u64,
    pub max_tokens_per_session: u64,
    pub max_cost_usd_per_session: f64,
    pub used_tokens: u64,
    pub used_cost_usd: f64,
    pub parallel_invocations: u32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBudget {
    pub session_id: SessionId,
    pub agent_id: AgentId,
    pub max_tokens: u64,
    pub max_cost_usd: f64,
    pub used_tokens: u64,
    pub used_cost_usd: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBudget {
    pub workflow_id: WorkflowId,
    pub max_tokens: u64,
    pub max_cost_usd: f64,
    pub max_wall_time_seconds: u64,
    pub used_tokens: u64,
    pub used_cost_usd: f64,
    pub elapsed_seconds: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBudget {
    pub task_id: TaskId,
    pub max_tokens: u64,
    pub max_cost_usd: f64,
    pub used_tokens: u64,
    pub used_cost_usd: f64,
    pub updated_at: DateTime<Utc>,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            project_budgets: HashMap::new(),
            agent_budgets: HashMap::new(),
            session_budgets: HashMap::new(),
            workflow_budgets: HashMap::new(),
            task_budgets: HashMap::new(),
        }
    }

    pub fn check_project_budget(
        &self,
        project_id: ProjectId,
        tokens: u64,
        cost: f64,
    ) -> Result<(), crate::error::DomainError> {
        if let Some(budget) = self.project_budgets.get(&project_id) {
            if budget.used_tokens + tokens > budget.max_tokens {
                return Err(crate::error::DomainError::BudgetExceeded {
                    budget_type: "project_tokens".into(),
                    limit: budget.max_tokens.to_string(),
                    used: (budget.used_tokens + tokens).to_string(),
                });
            }
            if budget.used_cost_usd + cost > budget.max_cost_usd {
                return Err(crate::error::DomainError::BudgetExceeded {
                    budget_type: "project_cost".into(),
                    limit: budget.max_cost_usd.to_string(),
                    used: (budget.used_cost_usd + cost).to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn consume_project_budget(&mut self, project_id: ProjectId, tokens: u64, cost: f64) {
        let budget = self
            .project_budgets
            .entry(project_id)
            .or_insert(ProjectBudget {
                project_id,
                max_tokens: 1_000_000,
                max_cost_usd: 100.0,
                used_tokens: 0,
                used_cost_usd: 0.0,
                updated_at: Utc::now(),
            });
        budget.used_tokens += tokens;
        budget.used_cost_usd += cost;
        budget.updated_at = Utc::now();
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}
