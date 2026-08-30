//! Transport-neutral MCP framing and lifecycle contract.
const MAX_FRAME: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    V1,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Request,
    Cancel,
    Close,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet(u8);
impl CapabilitySet {
    pub const READ: Self = Self(1);
    pub const CANCEL: Self = Self(2);
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
impl std::ops::BitOr for CapabilitySet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub version: ProtocolVersion,
    pub correlation_id: u64,
    pub kind: FrameKind,
    pub size: usize,
    caps: CapabilitySet,
}
impl Envelope {
    pub fn new(
        v: ProtocolVersion,
        id: u64,
        k: FrameKind,
        size: usize,
        caps: CapabilitySet,
    ) -> Result<Self, TransportError> {
        if id == 0 {
            return Err(TransportError::InvalidCorrelation);
        }
        Ok(Self {
            version: v,
            correlation_id: id,
            kind: k,
            size,
            caps,
        })
    }
    pub fn correlation_id(&self) -> u64 {
        self.correlation_id
    }
    pub fn capabilities(&self) -> CapabilitySet {
        self.caps
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedVersion,
    FrameTooLarge,
    InvalidCorrelation,
    Backpressure,
    ReconnectDenied,
}
pub struct Transport;
impl Transport {
    pub fn accept(e: Envelope) -> Result<Envelope, TransportError> {
        if e.version != ProtocolVersion::V1 {
            return Err(TransportError::UnsupportedVersion);
        }
        if e.size > MAX_FRAME {
            return Err(TransportError::FrameTooLarge);
        }
        if e.correlation_id == 0 {
            return Err(TransportError::InvalidCorrelation);
        }
        Ok(e)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Cancelled,
    Closed,
}
pub struct Session {
    state: SessionState,
    queue: usize,
    limit: usize,
}
impl Session {
    pub fn new(queue: usize, limit: usize) -> Self {
        Self {
            state: SessionState::Active,
            queue,
            limit,
        }
    }
    pub fn cancel(&mut self) -> SessionState {
        if self.state == SessionState::Active {
            self.state = SessionState::Cancelled
        }
        self.state
    }
    pub fn close(&mut self) -> SessionState {
        self.state = SessionState::Closed;
        self.state
    }
    pub fn enqueue(&mut self) -> Result<(), TransportError> {
        if self.state != SessionState::Active || self.queue >= self.limit {
            return Err(TransportError::Backpressure);
        }
        self.queue += 1;
        Ok(())
    }
    pub fn reconnect(&self) -> Result<(), TransportError> {
        Err(TransportError::ReconnectDenied)
    }
}
