# Typed ID catalog

All boundary identities are typed newtypes over `Uuid` defined in
`crates/agent-protocol/src/ids.rs` and re-exported by `agent-core`.

## Format

Canonical textual form: `{prefixo}-{uuid}` (ex.: `proj-<uuid>`). The serde
representation is the same prefixed string, so roundtrips preserve identity
and type. Inputs without the correct prefix, longer than `MAX_ID_LEN` (64
bytes), or containing control characters or path traversal (`/`, `\`, `..`)
are rejected at conversion time.

## Catalog

| Type          | Prefix | Use                                    |
|---------------|--------|----------------------------------------|
| ProjectId     | proj   | Projeto (workspace de domínio)         |
| AgentId       | agent  | Agente                                 |
| SessionId     | sess   | Sessão de conversa                     |
| MessageId     | msg    | Mensagem                               |
| WorkflowId    | wf     | Workflow                               |
| NodeId        | node   | Nó de workflow                         |
| RunId         | run    | Execução                               |
| TraceId       | trace  | Rastreio de operação                   |
| RequestId     | req    | Requisição                             |
| SkillId       | skill  | Skill                                  |
| MemoryId      | mem    | Memória                                |
| ToolId        | tool   | Ferramenta                             |
| ProviderId    | prov   | Provedor (LLM etc.)                    |
| CredentialId  | cred   | Credencial                             |
| GroupId       | grp    | Grupo                                  |
| TaskId        | task   | Tarefa                                 |
| ArtifactId    | art    | Artefato                               |

## Rule

Never use raw `String`/`Uuid` as identity in contracts, repositories or
interfaces. Use the typed ID; convert only through the explicit API
(`new`, `from_uuid`, `as_uuid`, `From`/`Into`, `parse`/`FromStr`/
`TryFrom<&str>`). Errors are `IdParseError` with a stable `IdErrorKind`,
redacted display (never the payload) and a deterministic `correlation_id`.

Changing a prefix, the serialized form or `MAX_ID_LEN` is a breaking change:
requires a protocol version bump and a compatibility note.