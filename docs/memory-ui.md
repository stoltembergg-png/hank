# Memory project UI

The memory panel is a read-only review surface attached to project details.
It receives a project-scoped API client and never reads SQLite, localStorage or
model output directly.

The bridge request always includes `project_id`. The panel verifies the
response project identity before rendering and filters any foreign-project
records defensively. It exposes status, type, provenance, confidence,
importance and trace metadata so candidate memory is visibly distinct from
approved memory.

Content is rendered as React text, obvious secret-like values are redacted and
previews are bounded to 320 characters. Loading, empty, error and filter states
are explicit. Editing, activation and automatic persistence are outside this
slice.
