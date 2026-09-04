# Spec: crash recovery contract

> feature: crash-recovery-contract
> status: em-implementacao

### US-1500 — Startup recovery coordinator fail-closed

Como runtime, quero classificar o estado da última execução em
`Clean | Recoverable | Unknown | Corrupt` antes de retomar trabalho, para que
nenhuma escrita replay insegura seja feita e o operador possa escolher um
modo seguro.

#### AC-1501 — Startup classifica o estado de uma execução anterior

- **Dado** um dirty marker com `epoch`, `last_known_good_epoch` e
  `pending_classes` (lista de `RecoveryClass`)
- **Quando** o `RecoveryCoordinator::classify(marker)` é executado
- **Então** o resultado é `Clean` se `epoch == last_known_good_epoch` **e**
  `pending_classes` está vazia, `Recoverable` se `pending_classes` é subconjunto de `{TransactionWrite, JournalAppend}` e `epoch > 0`, `Unknown` se `pending_classes` contém `UnknownEffect`, e `Corrupt` se o marker excede os limites, contém `CorruptMarker`, contém `DatabaseMigration`, tem epoch invertido ou tem `epoch == 0` com pendências.

#### AC-1502 — Recovery é idempotente

- **Dado** o mesmo `RecoveryMarker` (epoch + pending_classes)
- **Quando** `RecoveryCoordinator::replay(marker)` é executado duas vezes
- **Então** a primeira chamada retorna `RecoveryOutcome::Replayed` e a
  segunda `RecoveryOutcome::AlreadyReplayed` (mesmo `recovery_id`), sem
  invocar nenhum callback de `on_replay` na segunda chamada.

#### AC-1503 — Modo seguro recusa replay de `Unknown` ou `Corrupt`

- **Dado** um coordinator configurado com `RecoveryMode::Safe` e um
  `RecoveryMarker` cujo `pending_classes` inclui `UnknownEffect`,
  `DatabaseMigration` ou `CorruptMarker`
- **Quando** `replay(marker)` é executado
- **Então** o resultado é `RecoveryOutcome::Quarantined` e nenhum callback
  de `on_replay` ou `on_revalidate` é invocado; a entrada de recovery retém
  o `recovery_id` e as classes em quarentena.

#### AC-1504 — Crash bundle é redigido

- **Dado** um `RecoveryMarker` que contém strings de `actor`, `pending_classes`
  e `last_safe_action: String`
- **Quando** o `redacted_bundle(marker).to_json()` é serializado em JSON
- **Então** o JSON resultante **não** contém o conteúdo literal de
  `last_safe_action` (é substituído por `[REDACTED]`) e mantém apenas
  `epoch`, `pending_classes`, `last_known_good_epoch` e `recovery_id`.

#### AC-1505 — Stale capabilities e credenciais exigem revalidação

- **Dado** um `RecoveryMarker` cuja `pending_classes` inclui
  `CredentialRevocationPending` ou `CapabilityRotationPending`, **sem** incluir
  classe de quarentena (`DatabaseMigration`, `UnknownEffect` ou `CorruptMarker`)
- **Quando** `replay(marker)` é executado em `RecoveryMode::Safe`
- **Então** o resultado é `RecoveryOutcome::RevalidateRequired` e o
  callback `on_revalidate` é invocado uma única vez com
  `capability_set` e `credential_set` derivados do marker.

## Segurança

- O coordinator nunca persiste material de credencial, token, prompt,
  caminho absoluto de arquivo ou conteúdo de página em log ou crash bundle.
- `pending_classes` é bounded (`MAX_PENDING_CLASSES = 32`); marker com
  mais entradas é rejeitado como `Corrupt`.
- Modo `Safe` (default) é fail-closed: qualquer estado ambíguo entra em
  quarentena e bloqueia replay automático.
- Um marker que mistura classe revalidável com classe de quarentena sempre
  entra em quarentena; revalidação só ocorre quando não há classe proibida.
- Claims e conclusões de replay são transições duráveis por `recovery_id` no
  namespace de um único projeto; falha concluída não é automaticamente repetida.

## Suposições

- ASM-1500: adapters concretos de storage (SQLite, sled, fsync) são
  injetados via `RecoveryStorage` trait. Este card define o contrato;
  PRs subsequentes implementam o adapter.
- ASM-1501: a fonte de verdade do dirty marker é o storage que escreve
  o `epoch` atômico antes de qualquer efeito irreversível. O coordinator
  confia nesse storage; ele próprio não reescreve o marker.
- ASM-1502: `RecoveryStorage` é namespace-bound a um `project_id`; adapters
  compartilhados devem separar instâncias ou namespaces por projeto.
- ASM-1503: `claim_replay` e `complete_replay` são atômicos e duráveis no
  adapter real; a fixture em memória modela a máquina de estados para testes.
- ASM-1504: nenhum trabalho é executado por este card em production
  storage. O contrato é validado por unit tests com `InMemoryStorage`.

## Perguntas em aberto

Nenhuma.
