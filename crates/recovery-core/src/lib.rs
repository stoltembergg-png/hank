//! Bounded, fail-closed crash recovery contracts.
//!
//! This crate intentionally contains no filesystem, database, process, or
//! network implementation. Those adapters can depend on these contracts
//! without moving privileged effects into the portable core.

mod coordinator;
mod marker;
mod storage;

pub use coordinator::{
    NoopCallbacks, RecoveryAuditEntry, RecoveryCallbacks, RecoveryClassification,
    RecoveryCoordinator, RecoveryError, RecoveryMode, RecoveryOutcome, RedactedCrashBundle,
};
pub use marker::{
    RecoveryClass, RecoveryMarker, RevalidationRequest, MAX_OPAQUE_REFS, MAX_OPAQUE_REF_LEN,
    MAX_PENDING_CLASSES,
};
pub use storage::{
    InMemoryStorage, RecoveryStorage, ReplayClaim, ReplayCompletion, MAX_AUDIT_ENTRIES,
};
