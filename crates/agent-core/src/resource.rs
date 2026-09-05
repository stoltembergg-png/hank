//! Pure, bounded admission ledger for in-flight host resources.
//!
//! This module does not measure the host, kill processes, persist state or
//! authenticate identities. Adapters register typed scopes and must release a
//! receipt on terminal completion; `reap_expired` is the crash/timeout path.

use crate::ids::{AgentId, NodeId, ProjectId, TaskId};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

pub const MAX_RESOURCE_SCOPES: usize = 4_096;
pub const MAX_RESOURCE_RESERVATIONS: usize = 4_096;
pub const MAX_RESOURCE_SCOPE_COUNT_PER_RESERVATION: usize = 16;
pub const MAX_RESOURCE_TIMEOUT_MS: u64 = 604_800_000;
pub const MAX_RESOURCE_CPU_MILLIS: u64 = 1_000_000;
pub const MAX_RESOURCE_MEMORY_BYTES: u64 = 1 << 50;
pub const MAX_RESOURCE_DISK_BYTES: u64 = 1 << 50;
pub const MAX_RESOURCE_HANDLES: u64 = 1 << 20;
pub const MAX_RESOURCE_QUEUE_SLOTS: u64 = 1 << 20;
pub const MAX_RESOURCE_SUBPROCESSES: u64 = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceScope {
    Project(ProjectId),
    Agent(AgentId),
    Task(TaskId),
    Node(NodeId),
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceDimension {
    CpuMillis,
    MemoryBytes,
    DiskBytes,
    Handles,
    QueueSlots,
    Subprocesses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceUsage {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub handles: u64,
    pub queue_slots: u64,
    pub subprocesses: u64,
}

impl ResourceUsage {
    fn checked_add(self, demand: ResourceDemand) -> Option<Self> {
        Some(Self {
            cpu_millis: self.cpu_millis.checked_add(demand.cpu_millis)?,
            memory_bytes: self.memory_bytes.checked_add(demand.memory_bytes)?,
            disk_bytes: self.disk_bytes.checked_add(demand.disk_bytes)?,
            handles: self.handles.checked_add(demand.handles)?,
            queue_slots: self.queue_slots.checked_add(demand.queue_slots)?,
            subprocesses: self.subprocesses.checked_add(demand.subprocesses)?,
        })
    }

    fn checked_sub(self, demand: ResourceDemand) -> Option<Self> {
        Some(Self {
            cpu_millis: self.cpu_millis.checked_sub(demand.cpu_millis)?,
            memory_bytes: self.memory_bytes.checked_sub(demand.memory_bytes)?,
            disk_bytes: self.disk_bytes.checked_sub(demand.disk_bytes)?,
            handles: self.handles.checked_sub(demand.handles)?,
            queue_slots: self.queue_slots.checked_sub(demand.queue_slots)?,
            subprocesses: self.subprocesses.checked_sub(demand.subprocesses)?,
        })
    }

    fn covers(self, demand: ResourceDemand, quota: ResourceQuota) -> bool {
        self.cpu_millis.saturating_add(demand.cpu_millis) <= quota.cpu_millis
            && self.memory_bytes.saturating_add(demand.memory_bytes) <= quota.memory_bytes
            && self.disk_bytes.saturating_add(demand.disk_bytes) <= quota.disk_bytes
            && self.handles.saturating_add(demand.handles) <= quota.handles
            && self.queue_slots.saturating_add(demand.queue_slots) <= quota.queue_slots
            && self.subprocesses.saturating_add(demand.subprocesses) <= quota.subprocesses
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceQuota {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub handles: u64,
    pub queue_slots: u64,
    pub subprocesses: u64,
}

impl ResourceQuota {
    pub fn new(
        cpu_millis: u64,
        memory_bytes: u64,
        disk_bytes: u64,
        handles: u64,
        queue_slots: u64,
        subprocesses: u64,
    ) -> Result<Self, ResourceError> {
        let quota = Self {
            cpu_millis,
            memory_bytes,
            disk_bytes,
            handles,
            queue_slots,
            subprocesses,
        };
        quota.validate()?;
        Ok(quota)
    }

    fn validate(self) -> Result<(), ResourceError> {
        if self.cpu_millis == 0
            || self.cpu_millis > MAX_RESOURCE_CPU_MILLIS
            || self.memory_bytes == 0
            || self.memory_bytes > MAX_RESOURCE_MEMORY_BYTES
            || self.disk_bytes == 0
            || self.disk_bytes > MAX_RESOURCE_DISK_BYTES
            || self.handles == 0
            || self.handles > MAX_RESOURCE_HANDLES
            || self.queue_slots == 0
            || self.queue_slots > MAX_RESOURCE_QUEUE_SLOTS
            || self.subprocesses == 0
            || self.subprocesses > MAX_RESOURCE_SUBPROCESSES
        {
            return Err(ResourceError::InvalidQuota);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDemand {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub handles: u64,
    pub queue_slots: u64,
    pub subprocesses: u64,
}

impl ResourceDemand {
    pub fn new(
        cpu_millis: u64,
        memory_bytes: u64,
        disk_bytes: u64,
        handles: u64,
        queue_slots: u64,
        subprocesses: u64,
    ) -> Result<Self, ResourceError> {
        let demand = Self {
            cpu_millis,
            memory_bytes,
            disk_bytes,
            handles,
            queue_slots,
            subprocesses,
        };
        demand.validate()?;
        Ok(demand)
    }

    fn validate(self) -> Result<(), ResourceError> {
        if self.cpu_millis > MAX_RESOURCE_CPU_MILLIS
            || self.memory_bytes > MAX_RESOURCE_MEMORY_BYTES
            || self.disk_bytes > MAX_RESOURCE_DISK_BYTES
            || self.handles > MAX_RESOURCE_HANDLES
            || self.queue_slots > MAX_RESOURCE_QUEUE_SLOTS
            || self.subprocesses > MAX_RESOURCE_SUBPROCESSES
            || (self.cpu_millis == 0
                && self.memory_bytes == 0
                && self.disk_bytes == 0
                && self.handles == 0
                && self.queue_slots == 0
                && self.subprocesses == 0)
        {
            return Err(ResourceError::InvalidDemand);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceReservationId(uuid::Uuid);

impl ResourceReservationId {
    fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceReservationReceipt {
    pub reservation_id: ResourceReservationId,
    pub scopes: Vec<ResourceScope>,
    pub demand: ResourceDemand,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct ScopeState {
    quota: ResourceQuota,
    usage: ResourceUsage,
}

#[derive(Debug, Clone)]
struct ReservationState {
    scopes: Vec<ResourceScope>,
    demand: ResourceDemand,
    expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResourceError {
    #[error("resource quota is invalid")]
    InvalidQuota,
    #[error("resource demand is invalid")]
    InvalidDemand,
    #[error("resource reservation timeout is invalid")]
    InvalidTimeout,
    #[error("resource reservation has no scopes")]
    EmptyScopes,
    #[error("resource reservation contains a duplicate scope")]
    DuplicateScope,
    #[error("resource scope is already registered")]
    ScopeAlreadyRegistered,
    #[error("resource scope is not registered")]
    ScopeNotRegistered,
    #[error("resource capacity is exceeded for {scope:?}: {dimension:?}")]
    CapacityExceeded {
        scope: ResourceScope,
        dimension: ResourceDimension,
    },
    #[error("resource ledger state capacity was exceeded")]
    StateCapacityExceeded,
    #[error("resource reservation is unknown")]
    UnknownReservation,
    #[error("resource ledger clock moved backwards")]
    ClockWentBackwards,
    #[error("resource ledger arithmetic overflow")]
    ArithmeticOverflow,
    #[error("resource ledger state is inconsistent")]
    StateInconsistent,
}

/// In-memory, bounded, atomic resource admission ledger.
#[derive(Debug)]
pub struct ResourceReservationBook {
    scopes: HashMap<ResourceScope, ScopeState>,
    reservations: HashMap<ResourceReservationId, ReservationState>,
    max_scopes: usize,
    max_reservations: usize,
    last_now_ms: Option<u64>,
}

impl ResourceReservationBook {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
            reservations: HashMap::new(),
            max_scopes: MAX_RESOURCE_SCOPES,
            max_reservations: MAX_RESOURCE_RESERVATIONS,
            last_now_ms: None,
        }
    }

    pub fn with_limits(max_scopes: usize, max_reservations: usize) -> Result<Self, ResourceError> {
        if max_scopes == 0
            || max_scopes > MAX_RESOURCE_SCOPES
            || max_reservations == 0
            || max_reservations > MAX_RESOURCE_RESERVATIONS
        {
            return Err(ResourceError::StateCapacityExceeded);
        }
        Ok(Self {
            scopes: HashMap::new(),
            reservations: HashMap::new(),
            max_scopes,
            max_reservations,
            last_now_ms: None,
        })
    }

    pub fn register_scope(
        &mut self,
        scope: ResourceScope,
        quota: ResourceQuota,
    ) -> Result<(), ResourceError> {
        quota.validate()?;
        if self.scopes.contains_key(&scope) {
            return Err(ResourceError::ScopeAlreadyRegistered);
        }
        if self.scopes.len() >= self.max_scopes {
            return Err(ResourceError::StateCapacityExceeded);
        }
        self.scopes.insert(
            scope,
            ScopeState {
                quota,
                usage: ResourceUsage::default(),
            },
        );
        Ok(())
    }

    pub fn reserve(
        &mut self,
        scopes: &[ResourceScope],
        demand: ResourceDemand,
        now_ms: u64,
        timeout_ms: u64,
    ) -> Result<ResourceReservationReceipt, ResourceError> {
        demand.validate()?;
        if timeout_ms == 0 || timeout_ms > MAX_RESOURCE_TIMEOUT_MS {
            return Err(ResourceError::InvalidTimeout);
        }
        self.validate_scopes(scopes)?;
        if self.reservations.len() >= self.max_reservations {
            return Err(ResourceError::StateCapacityExceeded);
        }
        self.observe(now_ms)?;
        let expires_at_ms = now_ms
            .checked_add(timeout_ms)
            .ok_or(ResourceError::ArithmeticOverflow)?;

        for scope in scopes {
            let state = self
                .scopes
                .get(scope)
                .ok_or(ResourceError::ScopeNotRegistered)?;
            let next = state
                .usage
                .checked_add(demand)
                .ok_or(ResourceError::ArithmeticOverflow)?;
            if next.cpu_millis > state.quota.cpu_millis {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::CpuMillis,
                });
            }
            if next.memory_bytes > state.quota.memory_bytes {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::MemoryBytes,
                });
            }
            if next.disk_bytes > state.quota.disk_bytes {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::DiskBytes,
                });
            }
            if next.handles > state.quota.handles {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::Handles,
                });
            }
            if next.queue_slots > state.quota.queue_slots {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::QueueSlots,
                });
            }
            if next.subprocesses > state.quota.subprocesses {
                return Err(ResourceError::CapacityExceeded {
                    scope: scope.clone(),
                    dimension: ResourceDimension::Subprocesses,
                });
            }
        }

        for scope in scopes {
            let state = self
                .scopes
                .get_mut(scope)
                .ok_or(ResourceError::StateInconsistent)?;
            state.usage = state
                .usage
                .checked_add(demand)
                .ok_or(ResourceError::StateInconsistent)?;
        }
        let reservation_id = ResourceReservationId::new();
        self.reservations.insert(
            reservation_id,
            ReservationState {
                scopes: scopes.to_vec(),
                demand,
                expires_at_ms,
            },
        );
        Ok(ResourceReservationReceipt {
            reservation_id,
            scopes: scopes.to_vec(),
            demand,
            expires_at_ms,
        })
    }

    pub fn release(
        &mut self,
        reservation_id: ResourceReservationId,
        now_ms: u64,
    ) -> Result<(), ResourceError> {
        self.observe(now_ms)?;
        let reservation = self
            .reservations
            .get(&reservation_id)
            .cloned()
            .ok_or(ResourceError::UnknownReservation)?;
        self.validate_release(&reservation)?;
        self.release_state(reservation_id, reservation)
    }

    pub fn reap_expired(&mut self, now_ms: u64) -> Result<usize, ResourceError> {
        self.observe(now_ms)?;
        let expired: Vec<_> = self
            .reservations
            .iter()
            .filter_map(|(id, reservation)| (reservation.expires_at_ms <= now_ms).then_some(*id))
            .collect();
        for reservation_id in &expired {
            let reservation = self
                .reservations
                .get(reservation_id)
                .cloned()
                .ok_or(ResourceError::StateInconsistent)?;
            self.validate_release(&reservation)?;
            self.release_state(*reservation_id, reservation)?;
        }
        Ok(expired.len())
    }

    pub fn usage(&self, scope: &ResourceScope) -> Option<ResourceUsage> {
        self.scopes.get(scope).map(|state| state.usage)
    }

    pub fn active_reservations(&self) -> usize {
        self.reservations.len()
    }

    fn validate_scopes(&self, scopes: &[ResourceScope]) -> Result<(), ResourceError> {
        if scopes.is_empty() || scopes.len() > MAX_RESOURCE_SCOPE_COUNT_PER_RESERVATION {
            return Err(ResourceError::EmptyScopes);
        }
        let mut seen = HashSet::with_capacity(scopes.len());
        for scope in scopes {
            if !seen.insert(scope) {
                return Err(ResourceError::DuplicateScope);
            }
            if !self.scopes.contains_key(scope) {
                return Err(ResourceError::ScopeNotRegistered);
            }
        }
        Ok(())
    }

    fn validate_release(&self, reservation: &ReservationState) -> Result<(), ResourceError> {
        for scope in &reservation.scopes {
            let state = self
                .scopes
                .get(scope)
                .ok_or(ResourceError::StateInconsistent)?;
            if !state.usage.covers(reservation.demand, state.quota) {
                return Err(ResourceError::StateInconsistent);
            }
        }
        Ok(())
    }

    fn release_state(
        &mut self,
        reservation_id: ResourceReservationId,
        reservation: ReservationState,
    ) -> Result<(), ResourceError> {
        for scope in &reservation.scopes {
            let state = self
                .scopes
                .get_mut(scope)
                .ok_or(ResourceError::StateInconsistent)?;
            state.usage = state
                .usage
                .checked_sub(reservation.demand)
                .ok_or(ResourceError::StateInconsistent)?;
        }
        self.reservations.remove(&reservation_id);
        Ok(())
    }

    fn observe(&mut self, now_ms: u64) -> Result<(), ResourceError> {
        if self.last_now_ms.is_some_and(|last| now_ms < last) {
            return Err(ResourceError::ClockWentBackwards);
        }
        self.last_now_ms = Some(now_ms);
        Ok(())
    }
}

impl Default for ResourceReservationBook {
    fn default() -> Self {
        Self::new()
    }
}
