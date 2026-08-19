# Application event contract

Application events are versioned, project-scoped and bounded. Each event carries a
typed event ID, kind, aggregate ID, logical sequence, timestamp and synthetic/limited
payload. Unknown schema versions, empty aggregates, zero sequence and oversized
payloads fail closed.

The initial catalog includes project/agent/session lifecycle, provider usage and run
success/failure. Transport, event bus, persistence and UI consumers are deliberately
out of scope; this PR defines only the protocol contract.
