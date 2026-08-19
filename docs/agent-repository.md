# Agent repository contract

`SqliteAgentRepository` persists Agents using the existing migrated `agents` table.
Every read/write includes `project_id`; a lookup from another project returns no
record. IDs, personality and policy are serialized through the domain contract, SQL
is parameterized, list limits are bounded, and database failures map to domain errors.

This repository does not implement provider credentials, execution, policies or UI.
Migration ownership remains with the existing migration runner; rollback is reverting
this repository integration while preserving the schema contract.
