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
5. O segundo replay do mesmo `recovery_id` retorna `AlreadyReplayed` após claim
   durável e não chama callback novamente.

## Segurança e limites

`MAX_PENDING_CLASSES`, `MAX_OPAQUE_REFS` e `MAX_OPAQUE_REF_LEN` limitam o input.
O crash bundle serializado mantém apenas `recovery_id`, épocas e classes; actor,
capability references, credential references e `last_safe_action` não são incluídos.

## Não-objetivos desta PR

Esta PR não implementa SQLite/sled/fsync, migrações, execução de workflow,
transporte remoto, rotação efetiva de capability ou revogação efetiva de credencial.
Esses adapters dependem deste contrato em cards posteriores.
