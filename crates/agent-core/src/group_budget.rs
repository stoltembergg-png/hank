//! Group-scoped budget reservation and reconciliation boundary.

use crate::{BudgetAccount, BudgetLimits, BudgetScope, ProjectId, ReservationId};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GroupBudgetError {
    #[error("invocation already has a group budget reservation")]
    DuplicateInvocation,
    #[error("group budget is exceeded")]
    BudgetExceeded,
    #[error("group budget reservation is unknown")]
    UnknownReservation,
    #[error("group budget configuration is invalid")]
    InvalidBudget,
}

#[derive(Debug)]
pub struct GroupBudget {
    project_id: ProjectId,
    group_id: uuid::Uuid,
    account: BudgetAccount,
    reservations: HashMap<uuid::Uuid, ReservationId>,
}

impl GroupBudget {
    pub fn new(
        project_id: ProjectId,
        group_id: uuid::Uuid,
        limits: BudgetLimits,
    ) -> Result<Self, GroupBudgetError> {
        limits
            .validate()
            .map_err(|_| GroupBudgetError::InvalidBudget)?;
        Ok(Self {
            project_id,
            group_id,
            account: BudgetAccount::new(
                BudgetScope::Project(project_id),
                limits,
                chrono::Utc::now(),
            ),
            reservations: HashMap::new(),
        })
    }

    pub fn reserve(
        &mut self,
        invocation_id: uuid::Uuid,
        tokens: u64,
        cost: u64,
    ) -> Result<(), GroupBudgetError> {
        if self.reservations.contains_key(&invocation_id) {
            return Err(GroupBudgetError::DuplicateInvocation);
        }
        let reservation = self
            .account
            .reserve(tokens, cost, chrono::Utc::now())
            .map_err(|_| GroupBudgetError::BudgetExceeded)?;
        self.reservations.insert(invocation_id, reservation);
        Ok(())
    }

    pub fn commit(
        &mut self,
        invocation_id: uuid::Uuid,
        actual_tokens: u64,
        actual_cost: u64,
    ) -> Result<(), GroupBudgetError> {
        let reservation = self
            .reservations
            .remove(&invocation_id)
            .ok_or(GroupBudgetError::UnknownReservation)?;
        self.account
            .commit(reservation, actual_tokens, actual_cost, chrono::Utc::now())
            .map_err(|_| GroupBudgetError::BudgetExceeded)
    }

    pub fn refund(&mut self, invocation_id: uuid::Uuid) -> Result<(), GroupBudgetError> {
        let reservation = self
            .reservations
            .remove(&invocation_id)
            .ok_or(GroupBudgetError::UnknownReservation)?;
        self.account
            .refund(reservation, chrono::Utc::now())
            .map_err(|_| GroupBudgetError::UnknownReservation)
    }

    pub fn used_tokens(&self) -> u64 {
        self.account.used_tokens
    }
    pub fn available_tokens(&self) -> u64 {
        self.account
            .limits
            .max_tokens
            .saturating_sub(self.account.used_tokens + self.account.reserved_tokens)
    }
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }
    pub fn group_id(&self) -> uuid::Uuid {
        self.group_id
    }
}
