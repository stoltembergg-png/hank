use agent_core::resource::{
    ResourceDemand, ResourceDimension, ResourceError, ResourceQuota, ResourceReservationBook,
    ResourceScope, MAX_RESOURCE_TIMEOUT_MS,
};
use agent_core::{NodeId, ProjectId};

fn quota() -> ResourceQuota {
    ResourceQuota::new(2_000, 16_000_000, 32_000_000, 32, 8, 4).unwrap()
}

fn demand() -> ResourceDemand {
    ResourceDemand::new(500, 2_000_000, 4_000_000, 2, 1, 1).unwrap()
}

#[test]
// @spec:AC-2011
fn quotas_demands_and_timeouts_are_bounded() {
    assert!(ResourceQuota::new(0, 1, 1, 1, 1, 1).is_err());
    assert!(ResourceDemand::new(0, 0, 0, 0, 0, 0).is_err());
    assert!(ResourceDemand::new(1_000_001, 1, 1, 1, 1, 1).is_err());

    let mut book = ResourceReservationBook::new();
    let project = ResourceScope::Project(ProjectId::new());
    book.register_scope(project.clone(), quota()).unwrap();
    assert_eq!(
        book.reserve(
            std::slice::from_ref(&project),
            demand(),
            0,
            MAX_RESOURCE_TIMEOUT_MS + 1
        ),
        Err(ResourceError::InvalidTimeout)
    );
    assert_eq!(
        book.reserve(std::slice::from_ref(&project), demand(), 0, 0),
        Err(ResourceError::InvalidTimeout)
    );
}

#[test]
// @spec:AC-2012
fn reservation_is_atomic_across_project_node_and_global_scopes() {
    let mut book = ResourceReservationBook::new();
    let project = ResourceScope::Project(ProjectId::new());
    let node = ResourceScope::Node(NodeId::new());
    let global = ResourceScope::Global;
    for scope in [&project, &node, &global] {
        book.register_scope(scope.clone(), quota()).unwrap();
    }

    let receipt = book
        .reserve(
            &[project.clone(), node.clone(), global.clone()],
            demand(),
            100,
            500,
        )
        .unwrap();
    assert_eq!(book.usage(&project).unwrap().cpu_millis, 500);
    assert_eq!(book.usage(&node).unwrap().queue_slots, 1);
    assert_eq!(receipt.scopes.len(), 3);

    let too_large = ResourceDemand::new(1_000_001, 1, 1, 1, 1, 1).unwrap_err();
    assert_eq!(too_large, ResourceError::InvalidDemand);
    let insufficient = ResourceDemand::new(2_000, 16_000_000, 32_000_000, 32, 8, 4).unwrap();
    assert!(matches!(
        book.reserve(
            &[project.clone(), node.clone(), global.clone()],
            insufficient,
            100,
            500
        ),
        Err(ResourceError::CapacityExceeded {
            dimension: ResourceDimension::CpuMillis,
            ..
        })
    ));
    assert_eq!(book.usage(&project).unwrap().cpu_millis, 500);

    book.release(receipt.reservation_id, 101).unwrap();
    assert_eq!(book.usage(&project).unwrap().cpu_millis, 0);
}

#[test]
// @spec:AC-2013
fn release_is_explicit_and_expiry_recovers_stale_reservations() {
    let mut book = ResourceReservationBook::new();
    let project = ResourceScope::Project(ProjectId::new());
    book.register_scope(project.clone(), quota()).unwrap();
    let receipt = book
        .reserve(std::slice::from_ref(&project), demand(), 100, 50)
        .unwrap();
    assert_eq!(book.reap_expired(149).unwrap(), 0);
    assert_eq!(book.reap_expired(150).unwrap(), 1);
    assert_eq!(book.usage(&project).unwrap().memory_bytes, 0);
    assert_eq!(
        book.release(receipt.reservation_id, 151),
        Err(ResourceError::UnknownReservation)
    );
}

#[test]
// @spec:AC-2014
fn project_capacity_does_not_cross_into_another_project() {
    let mut book = ResourceReservationBook::new();
    let project_a = ResourceScope::Project(ProjectId::new());
    let project_b = ResourceScope::Project(ProjectId::new());
    let small = ResourceQuota::new(500, 1_000_000, 1_000_000, 4, 2, 1).unwrap();
    book.register_scope(project_a.clone(), small).unwrap();
    book.register_scope(project_b.clone(), small).unwrap();
    let one = ResourceDemand::new(500, 1, 1, 1, 1, 1).unwrap();
    book.reserve(std::slice::from_ref(&project_a), one, 0, 100)
        .unwrap();
    assert!(matches!(
        book.reserve(&[project_a], one, 0, 100),
        Err(ResourceError::CapacityExceeded { .. })
    ));
    assert!(book.reserve(&[project_b], one, 0, 100).is_ok());
}

#[test]
// @spec:AC-2015
fn ledger_is_bounded_and_rejects_clock_regression_or_partial_scopes() {
    let mut book = ResourceReservationBook::with_limits(1, 1).unwrap();
    let project = ResourceScope::Project(ProjectId::new());
    let node = ResourceScope::Node(NodeId::new());
    book.register_scope(project.clone(), quota()).unwrap();
    assert_eq!(
        book.reserve(&[project.clone(), node], demand(), 10, 100),
        Err(ResourceError::ScopeNotRegistered)
    );
    let receipt = book
        .reserve(std::slice::from_ref(&project), demand(), 10, 100)
        .unwrap();
    assert_eq!(book.reap_expired(9), Err(ResourceError::ClockWentBackwards));
    assert_eq!(book.active_reservations(), 1);
    assert_eq!(
        book.reserve(&[project], demand(), 10, 100),
        Err(ResourceError::StateCapacityExceeded)
    );
    book.release(receipt.reservation_id, 10).unwrap();
}
