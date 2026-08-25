use agent_core::{BudgetLimits, GroupBudget, GroupBudgetError, ProjectId};

fn budget() -> GroupBudget {
    GroupBudget::new(
        ProjectId::new(),
        uuid::Uuid::new_v4(),
        BudgetLimits {
            max_tokens: 100,
            max_cost_micro_usd: 100,
            max_parallel_invocations: 2,
            ..BudgetLimits::default()
        },
    )
    .unwrap()
}

#[test]
// @spec:AC-910
fn reservations_are_atomic_and_isolated_by_invocation() {
    let mut value = budget();
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    value.reserve(first, 60, 10).unwrap();
    assert_eq!(
        value.reserve(first, 1, 1),
        Err(GroupBudgetError::DuplicateInvocation)
    );
    assert_eq!(
        value.reserve(second, 50, 10),
        Err(GroupBudgetError::BudgetExceeded)
    );
}

#[test]
// @spec:AC-911
fn commit_reconciles_actual_usage_and_dedupes_retry() {
    let mut value = budget();
    let id = uuid::Uuid::new_v4();
    value.reserve(id, 50, 50).unwrap();
    value.commit(id, 30, 20).unwrap();
    assert_eq!(value.used_tokens(), 30);
    assert_eq!(
        value.commit(id, 1, 1),
        Err(GroupBudgetError::UnknownReservation)
    );
}

#[test]
// @spec:AC-912
fn cancellation_refunds_unspent_reservation_and_scope_is_explicit() {
    let mut value = budget();
    let id = uuid::Uuid::new_v4();
    value.reserve(id, 80, 80).unwrap();
    value.refund(id).unwrap();
    assert_eq!(value.available_tokens(), 100);
    assert_eq!(value.refund(id), Err(GroupBudgetError::UnknownReservation));
    assert!(!value.project_id().to_string().is_empty());
    assert!(!value.group_id().is_nil());
}
