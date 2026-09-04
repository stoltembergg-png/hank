# Crash recovery contract

`recovery-core` define o contrato portátil de recuperação no startup. Ele classifica
um dirty marker antes de qualquer tentativa de retomar efeitos, aplica quarentena
fail-closed para estados ambíguos/corrompidos e delega ao storage claims e
conclusões duráveis de `recovery_id` por namespace de projeto, impedindo replay duplicado.

## Fronteira

- `RecoveryMarker` contém somente identificadores opacos, época, classes de efeito e
  referências bounded para revalidação.
- `RecoveryStorage` é a porta para adapters concretos. `write_marker` deve ser
  atômico: ou o marker inteiro fica visível, ou nenhum marker fica visível.
- `RecoveryCallbacks` injeta efeitos de replay e revalidação; o core não conhece
  SQLite, filesystem, rede, Tauri, tokens ou credenciais.
- `InMemoryStorage` existe apenas para testes de contrato e não é persistente.

## Transições

1. `Clean`: não há pendências e `epoch == last_known_good_epoch`.
2. `Recoverable`: somente `TransactionWrite`/`JournalAppend`, com `epoch > 0`.
3. `Unknown`/`Corrupt`: nunca são automaticamente executados; entram em
   `Quarantined`.
4. `CredentialRevocationPending` ou `CapabilityRotationPending` exigem
   `RevalidateRequired` quando não há classe de quarentena; os conjuntos opacos
   são enviados ao callback. Markers mistos entram em `Quarantined`.
5. O claim durável cobre também `Clean`: o primeiro processamento retorna
   `Replayed` e chamadas seguintes retornam `AlreadyReplayed`.
6. Em `RecoveryMode::Resume`, `Recoverable` entra em `Quarantined` com estado
   durável `Deferred`; não há callback e um startup posterior em `Safe` pode
   reivindicar o mesmo ID. Uma falha de callback é `Failed` e não é retry automático.
7. Um marker de outro `project_id` é rejeitado antes de classificação, claim,
   callback ou auditoria.
8. O segundo replay do mesmo `recovery_id` retorna `AlreadyReplayed` após claim
   durável e não chama callback novamente.

## Segurança e limites

`MAX_PENDING_CLASSES`, `MAX_OPAQUE_REFS`, `MAX_OPAQUE_REF_LEN` e
`MAX_LAST_SAFE_ACTION_LEN` limitam o input. O crash bundle serializado mantém apenas
`recovery_id`, épocas e classes; actor, capability references, credential references
e `last_safe_action` não são incluídos. A entrada de auditoria também não contém o
`RecoveryOutcome` completo: revalidation conserva apenas contagens dos conjuntos
opacos.

## Não-objetivos desta PR

Esta PR não implementa SQLite/sled/fsync, migrações, execução de workflow,
transporte remoto, rotação efetiva de capability ou revogação efetiva de credencial.
Esses adapters dependem deste contrato em cards posteriores.
