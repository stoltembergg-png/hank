//! Concrete OS composition for the transport-neutral remote broker.
//!
//! This crate is the infrastructure boundary: `remote-core` depends only on
//! `BrokerClock` and `BrokerEntropy`, while this adapter selects the system
//! clock and operating-system CSPRNG for desktop/runtime composition.

use remote_core::credential_broker::{
    BrokerClock, BrokerEntropy, CredentialBroker, CredentialBrokerError,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock implementation used by the runtime composition root.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl BrokerClock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// OS-backed source of per-broker entropy.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsEntropy;

impl BrokerEntropy for OsEntropy {
    fn next_seed(&self) -> Result<[u8; 16], CredentialBrokerError> {
        let mut seed = [0u8; 16];
        getrandom::fill(&mut seed).map_err(|_| CredentialBrokerError::EntropyUnavailable)?;
        Ok(seed)
    }
}

/// Builds a production broker with explicit OS clock and CSPRNG adapters.
pub fn new_credential_broker() -> Result<CredentialBroker, CredentialBrokerError> {
    CredentialBroker::with_clock_and_entropy(Arc::new(SystemClock), Arc::new(OsEntropy))
}
