# Audit logs

`security-core::audit` é o contrato puro e fail-closed de auditoria de eventos
de segurança. Ele não executa I/O, não acessa o relógio real e não conhece
sink concreto: o `AuditSink` é injetado pelo composition root (storage-core,
remote-core, agent-runtime).

## Princípios

1. **Append-only**: nenhum evento registrado é mutável.
2. **Tamper-evident**: a integridade é garantida por hash encadeado e por
   sequência monotônica. Qualquer adulteração, remoção, inserção fora de
   ordem ou divergência de payload quebra `verify_chain` com classificação
   explícita.
3. **Redacted**: campos marcados como `RedactedField::secret` são
   serializados, exportados e consultados como `[REDACTED]`. Tokens, senhas,
   chaves, connection strings, paths brutos e conteúdo de página nunca
   atravessam a boundary do contrato.
4. **Bounded**: payload, número de eventos, cardinalidade de export e
   cardinalidade de query são limitados. Eventos fora da retenção são
   descartados de forma explícita.
5. **Fail-closed**: a falha do sink é tipada, propaga, e nunca autoriza
   implicitamente a ação que motivou o registro.
6. **Determinístico**: a serialização de um mesmo evento é idêntica,
   independentemente do sink, do tempo ou do chamador.

## Boundary

- `AuditPolicy`: capacidade, retenção (duração e número de eventos), escopo
  (project/actor/resource), revisão de policy.
- `AuditEvent`: identidade (actor/resource/policy_revision/SHA256 do
  payload), classificação (`Authorization`, `Migration`, `Recovery`,
  `PluginRemote`, `Release`, `Denial`, `Other`), payload estruturado,
  sequência, hash e timestamp monotônico.
- `AuditLog`: ledger puro com `record`, `retain`, `verify_chain`, `query`,
  `export`. Recebe `AuditSink` injetado.
- `AuditSink`: trait com `write(&AuditEvent) -> Result<(), AuditError>`.
  Implementações concretas ficam fora de `security-core`.
- `RedactedField`: enumeração dos campos sensíveis; serialização substitui
  por `[REDACTED]`.

## Não-escopo

- Persistência durável, SQL/index, agregação por janela.
- Forwarding para SIEM, telemetria ou alerting.
- Aprovação de ações baseada em auditoria.
- I/O de disco, rede, relógio real, secret store.

## Limites

| Limite | Valor |
| --- | --- |
| `MAX_EVENT_PAYLOAD_BYTES` | 4096 |
| `MAX_EVENTS_PER_LOG` | 4096 |
| `MAX_ACTOR_ID_LEN` | 128 |
| `MAX_RESOURCE_ID_LEN` | 128 |
| `MAX_POLICY_REVISION_LEN` | 128 |
| `MAX_QUERY_RESULTS` | 1024 |
| `MAX_EXPORT_ROWS` | 1024 |
| `MAX_RETENTION_EVENTS` | 4096 |
| `MAX_SCOPE_KEY_LEN` | 128 |

## Falhas tipadas

- `AuditError::PolicyInvalid`
- `AuditError::EventInvalid`
- `AuditError::PolicyRevisionMismatch`
- `AuditError::ScopeMismatch`
- `AuditError::SinkUnavailable`
- `AuditError::IntegrityBroken { classification, event_id, index }`
- `AuditError::ExportRejected`
- `AuditError::QueryRejected`
- `AuditError::PayloadTooLarge`
- `AuditError::RetentionInvalid`

## Integração

`security-core::audit` é boundary puro. Adapters em `agent-runtime`,
`remote-core`, `storage-core` e `telemetry-core` (futuro) implementam
`AuditSink` com persistência, batching, retry, métricas e forwarding. Eles
não podem alterar a forma do evento, hash, sequência ou classificação.
