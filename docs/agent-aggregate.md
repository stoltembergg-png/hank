# Agent aggregate contract

An Agent is always bound to a `ProjectId` and remains a domain-only aggregate. Its
identity, name, lifecycle, personality and policy are serialized without provider,
Tauri, SQLite or repository dependencies. Names and personality fields are bounded;
validation rejects empty/oversized values and oversized trait collections.

Repository, execution loop, provider adapters and UI are separate increments. Lifecycle
changes are explicit and observable through the domain error contract; project scope is
never inferred from frontend input.
