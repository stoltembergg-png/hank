//! Runtime-neutral transport envelope and bounded session lifecycle.
use thiserror::Error;

const MAX_ID: usize = 128;
const MAX_FRAME: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    V1,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Request,
    Response,
    Cancel,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet(u8);

impl CapabilitySet {
    pub const CANCEL: Self = Self(1);
    pub const STREAM: Self = Self(2);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for CapabilitySet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionId(String);

impl ConnectionId {
    pub fn new(value: &str) -> Result<Self, TransportError> {
        validate_id(value)?;
        Ok(Self(value.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: &str) -> Result<Self, TransportError> {
        validate_id(value)?;
        Ok(Self(value.into()))
    }
}

fn validate_id(value: &str) -> Result<(), TransportError> {
    if value.is_empty() || value.len() > MAX_ID || value.chars().any(char::is_control) {
        return Err(TransportError::InvalidIdentity);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnvelope {
    version: ProtocolVersion,
    connection: ConnectionId,
    session: SessionId,
    correlation_id: u64,
    kind: FrameKind,
    size: usize,
    capabilities: CapabilitySet,
}

impl RuntimeEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: ProtocolVersion,
        connection: ConnectionId,
        session: SessionId,
        correlation_id: u64,
        kind: FrameKind,
        size: usize,
        capabilities: CapabilitySet,
    ) -> Result<Self, TransportError> {
        if correlation_id == 0 {
            return Err(TransportError::InvalidCorrelation);
        }
        Ok(Self {
            version,
            connection,
            session,
            correlation_id,
            kind,
            size,
            capabilities,
        })
    }

    pub fn correlation_id(&self) -> u64 {
        self.correlation_id
    }

    pub fn connection(&self) -> &ConnectionId {
        &self.connection
    }

    pub fn session(&self) -> &SessionId {
        &self.session
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("invalid transport identity")]
    InvalidIdentity,
    #[error("invalid correlation ID")]
    InvalidCorrelation,
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("frame exceeds maximum size")]
    FrameTooLarge,
    #[error("session queue backpressure")]
    Backpressure,
    #[error("reconnect denied by default")]
    ReconnectDenied,
}

pub struct RuntimeTransport;

impl RuntimeTransport {
    pub fn accept(envelope: RuntimeEnvelope) -> Result<RuntimeEnvelope, TransportError> {
        if envelope.version != ProtocolVersion::V1 {
            return Err(TransportError::UnsupportedVersion);
        }
        if envelope.size > MAX_FRAME {
            return Err(TransportError::FrameTooLarge);
        }
        Ok(envelope)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Cancelled,
    Closed,
}

pub struct RuntimeSession {
    queue: usize,
    limit: usize,
    state: SessionState,
}

impl RuntimeSession {
    pub fn new(queue: usize, limit: usize) -> Result<Self, TransportError> {
        if limit == 0 || queue > limit {
            return Err(TransportError::Backpressure);
        }
        Ok(Self {
            queue,
            limit,
            state: SessionState::Active,
        })
    }

    pub fn enqueue(&mut self) -> Result<(), TransportError> {
        if self.state != SessionState::Active || self.queue >= self.limit {
            return Err(TransportError::Backpressure);
        }
        self.queue += 1;
        Ok(())
    }

    pub fn cancel(&mut self) -> SessionState {
        if self.state == SessionState::Active {
            self.state = SessionState::Cancelled;
        }
        self.state
    }

    pub fn close(&mut self) -> SessionState {
        self.state = SessionState::Closed;
        self.state
    }

    pub fn reconnect(&self) -> Result<(), TransportError> {
        Err(TransportError::ReconnectDenied)
    }
}
