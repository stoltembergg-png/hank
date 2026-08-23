# Memory core boundary

`agent-core::Memory` is a pure domain entity. It carries project identity,
optional agent/session scope, closed memory type, provenance, confidence,
content, status and version.

New memories are `candidate`. Content is untrusted data; it is not an
instruction and is never executed by this entity. Approval is an explicit
transition. Archived memories must be restored before approval can be
requested again.

The entity validates bounded content and summary sizes, finite confidence and
importance in the range 0..1. Lifecycle mutations increment the version.

This slice deliberately does not persist memory, extract memory from model
output, retrieve or embed content, or provide UI. Those concerns require
separate policy and repository contracts.
