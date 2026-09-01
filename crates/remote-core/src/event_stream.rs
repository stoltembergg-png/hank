//! Bounded, redacted, transport-neutral event stream bound to an authenticated
//! daemon lease.
//!
//! This module models only the ordering, ack, buffer, replay and redaction
//! boundaries of a remote event channel. It does not open a socket, WebSocket,
//! TLS listener, or dispatch events over the network; concrete adapters belong
//! to later cards.

use crate::{AuthenticatedDaemon, DaemonLease, DaemonSessionState, PeerAuthenticator};
use thiserror::Error;

/// Maximum retained events in the bounded buffer.
pub const DEFAULT_MAX_BUFFERED_EVENTS: usize = 256;
/// Maximum single event payload size.
pub const DEFAULT_MAX_EVENT_PAYLOAD: usize = 64 * 1024;
/// Maximum replay window (events behind the acked watermark that can be resumed).
pub const DEFAULT_REPLAY_WINDOW: usize = 64;

/// Bounded event-stream policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventStreamPolicy {
    pub max_buffered_events: usize,
    pub max_event_payload: usize,
    pub replay_window: usize,
}

impl EventStreamPolicy {
    pub fn bounded(
        max_buffered_events: usize,
        max_event_payload: usize,
        replay_window: usize,
    ) -> Result<Self, EventStreamError> {
        if max_buffered_events == 0
            || max_event_payload == 0
            || replay_window == 0
            || replay_window > max_buffered_events
        {
            return Err(EventStreamError::InvalidPolicy);
        }
        Ok(Self {
            max_buffered_events,
            max_event_payload,
            replay_window,
        })
    }
}

impl Default for EventStreamPolicy {
    fn default() -> Self {
        Self {
            max_buffered_events: DEFAULT_MAX_BUFFERED_EVENTS,
            max_event_payload: DEFAULT_MAX_EVENT_PAYLOAD,
            replay_window: DEFAULT_REPLAY_WINDOW,
        }
    }
}

/// A single ordered event with a bounded, already-redacted payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEvent {
    pub sequence: u64,
    pub payload: Vec<u8>,
}

/// Errors of the bounded event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EventStreamError {
    #[error("event stream policy is invalid")]
    InvalidPolicy,
    #[error("event stream is not bound to an active lease")]
    NoActiveLease,
    #[error("event stream lease is stale")]
    StaleLease,
    #[error("event payload exceeds maximum size")]
    PayloadTooLarge,
    #[error("event buffer is full")]
    BufferFull,
    #[error("event sequence out of order")]
    OutOfOrder,
    #[error("ack beyond last emitted sequence")]
    UnknownAck,
    #[error("reconnect replay outside window")]
    ReplayOutOfWindow,
    #[error("event stream state lock unavailable")]
    StateUnavailable,
}

struct BoundLease {
    id: u64,
}

struct StreamState {
    bound: Option<BoundLease>,
    last_sequence: u64,
    acked_sequence: u64,
    buffer: Vec<StreamEvent>,
    closed: bool,
}

/// Bounded, redacted event stream bound to exactly one authenticated lease.
///
/// Every payload admission validates the bound lease against the daemon at
/// `now_ms`, so an expired, revoked or superseded lease fails closed (AC-1461).
pub struct EventStream<'a, A: PeerAuthenticator> {
    daemon: &'a AuthenticatedDaemon<A>,
    policy: EventStreamPolicy,
    state: std::sync::Mutex<StreamState>,
}

impl<'a, A: PeerAuthenticator> EventStream<'a, A> {
    pub fn new(daemon: &'a AuthenticatedDaemon<A>, policy: EventStreamPolicy) -> Self {
        Self {
            daemon,
            policy,
            state: std::sync::Mutex::new(StreamState {
                bound: None,
                last_sequence: 0,
                acked_sequence: 0,
                buffer: Vec::with_capacity(policy.max_buffered_events),
                closed: false,
            }),
        }
    }

    /// Binds the stream to an active lease of the daemon. Rejects a stale or
    /// unknown lease so a replacement session cannot be bound by mistake.
    pub fn bind(&self, lease: &DaemonLease, now_ms: u64) -> Result<(), EventStreamError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventStreamError::StateUnavailable)?;
        if state.bound.is_some() {
            return Err(EventStreamError::StaleLease);
        }
        if !self.lease_active(lease.id, now_ms) {
            return Err(EventStreamError::NoActiveLease);
        }
        state.bound = Some(BoundLease { id: lease.id });
        Ok(())
    }

    fn lease_active(&self, lease_id: u64, now_ms: u64) -> bool {
        matches!(
            self.daemon.expire(lease_id, now_ms),
            Ok(DaemonSessionState::Ready)
        )
    }

    /// Pushes an event, assigning the next monotonic sequence. Applies payload
    /// and buffer bounds fail-closed. The bound lease must still be active at
    /// `now_ms`; revoked, expired or unknown leases reject with NoActiveLease.
    pub fn push(&self, now_ms: u64, payload: Vec<u8>) -> Result<u64, EventStreamError> {
        if payload.len() > self.policy.max_event_payload {
            return Err(EventStreamError::PayloadTooLarge);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventStreamError::StateUnavailable)?;
        let Some(bound) = state.bound.as_ref() else {
            return Err(EventStreamError::NoActiveLease);
        };
        if state.closed {
            return Err(EventStreamError::NoActiveLease);
        }
        if !self.lease_active(bound.id, now_ms) {
            return Err(EventStreamError::NoActiveLease);
        }
        if state.buffer.len() >= self.policy.max_buffered_events {
            return Err(EventStreamError::BufferFull);
        }
        let sequence = state
            .last_sequence
            .checked_add(1)
            .ok_or(EventStreamError::StateUnavailable)?;
        state.last_sequence = sequence;
        state.buffer.push(StreamEvent { sequence, payload });
        Ok(sequence)
    }

    /// Acks a delivered sequence, advancing the acked watermark and evicting
    /// acked events from the buffer.
    pub fn ack(&self, sequence: u64) -> Result<(), EventStreamError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventStreamError::StateUnavailable)?;
        if sequence == 0 || sequence > state.last_sequence {
            return Err(EventStreamError::UnknownAck);
        }
        if sequence > state.acked_sequence {
            state.acked_sequence = sequence;
        }
        let acked = state.acked_sequence;
        state.buffer.retain(|e| e.sequence > acked);
        Ok(())
    }

    /// Resumes replay from a sequence within the ack window. Rejects resumes
    /// outside the window (fail-closed reconnect policy). The bound lease must
    /// still be active at `now_ms`.
    pub fn resume(
        &self,
        now_ms: u64,
        from_sequence: u64,
    ) -> Result<Vec<StreamEvent>, EventStreamError> {
        let state = self
            .state
            .lock()
            .map_err(|_| EventStreamError::StateUnavailable)?;
        let Some(bound) = state.bound.as_ref() else {
            return Err(EventStreamError::NoActiveLease);
        };
        if state.closed {
            return Err(EventStreamError::NoActiveLease);
        }
        if !self.lease_active(bound.id, now_ms) {
            return Err(EventStreamError::NoActiveLease);
        }
        let window_start = state
            .acked_sequence
            .saturating_sub(self.policy.replay_window as u64);
        if from_sequence < window_start || from_sequence > state.last_sequence {
            return Err(EventStreamError::ReplayOutOfWindow);
        }
        let replay = state
            .buffer
            .iter()
            .filter(|e| e.sequence > from_sequence)
            .cloned()
            .collect::<Vec<_>>();
        Ok(replay)
    }

    /// Closes the stream for a bound lease. Unknown or stale lease fails closed.
    pub fn close(&self, lease_id: u64) -> Result<(), EventStreamError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventStreamError::StateUnavailable)?;
        match &state.bound {
            Some(bound) if bound.id == lease_id => {
                state.closed = true;
                state.buffer.clear();
                Ok(())
            }
            Some(_) => Err(EventStreamError::StaleLease),
            None => Err(EventStreamError::NoActiveLease),
        }
    }

    /// Current last sequence (observability).
    pub fn last_sequence(&self) -> u64 {
        self.state
            .lock()
            .map(|s| s.last_sequence)
            .unwrap_or_default()
    }

    /// Current acked sequence (observability).
    pub fn acked_sequence(&self) -> u64 {
        self.state
            .lock()
            .map(|s| s.acked_sequence)
            .unwrap_or_default()
    }

    /// Number of buffered events (observability).
    pub fn buffered_len(&self) -> usize {
        self.state
            .lock()
            .map(|s| s.buffer.len())
            .unwrap_or_default()
    }
}
