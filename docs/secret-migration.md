# Secret migration (PR-256)

`secrets-core::migration` define o coordinator que move uma credencial legada
para um destino seguro sem transformar o core em um armazenamento de secrets.
O fluxo é `preflight → inspect → read em memória → seal → encrypted staging →
destination write → opaque verify → legacy revoke → cleanup`.

## Boundaries

- `LegacySecretSource::inspect` identifica disponibilidade sem ler material;
  `read` é a única porta que pode entregar `SecretMaterial` ao coordinator.
- `SecretEnvelopeCodec` deve usar criptografia autenticada e devolver somente
  `EncryptedSecretEnvelope`. O envelope tem limite de 68 KiB e o `Debug`
  redige seu ciphertext.
- `EncryptedSecretStaging` recebe envelope e devolve apenas `StagingReceipt`;
  `MigrationLedger` guarda IDs, bindings, estados e classes de falha, nunca
  bytes de segredo ou envelope.
- `SecretMigrationDestination` grava e verifica apenas no escopo autorizado.
  A verificação recebe o material esperado somente para comparação interna do
  broker e devolve um booleano, nunca o material armazenado ao coordinator. A
  implementação para `SecureSecretStore<B>` continua atrás do backend injetado,
  sem SQLite, `.env`, frontend ou fallback plaintext.
- `MigrationLedger::claim` é um lease exclusivo bounded; `save` usa CAS com o
  lease e o relógio, e `release` encerra o claim. Assim, chamadas concorrentes
  com o mesmo ID recebem `Conflict`, enquanto uma interrupção permite retomada
  depois da expiração do lease.

## Failure and recovery

Preflight falha antes da fonte quando há projeto/actor divergente, autorização
expirada, cancelamento ou ausência de consentimento para revogar o legado.
Depois de `start`, qualquer falha de source, codec, staging, destino ou
verificação persiste `Quarantined`, mantém a fonte e preserva o receipt staged
quando possível. O retry precisa ser explícito; com staging válido ele não relê
a fonte. `Applied` é idempotente pelo `SecretMigrationId`.

A revogação da fonte é deliberadamente a última operação destrutiva. Se a
verificação falhar ou a revogação falhar, a fonte não é removida. Se a remoção
do staging cifrado falhar depois do cutover, o journal retorna `cleanup_pending`
sem desfazer o destino já verificado.
O contrato de `LegacySecretSource::revoke` exige atomicidade: quando retorna
erro, a fonte continua disponível e retryable; adapters concretos devem testar
essa garantia.

## Scope of this increment

Esta PR entrega o contrato transport-neutral e mocks determinísticos. Não
implementa keychain/Stronghold real, parser de formatos legados de produção,
persistência de journal em banco, migração de dados reais ou inspeção de
processo/ambiente em um host de produção. Esses pontos permanecem `NO_PROOF`
até adapters e testes por plataforma serem adicionados.

## Tests

`crates/secrets-core/tests/secret_migration_contract.rs` cobre:

- preflight cross-project, autorização expirada e consentimento;
- staging cifrado sem plaintext no envelope/debug;
- cutover somente após verificação;
- quarentena preservando legado;
- retry a partir do staging sem nova leitura;
- retry idempotente após `Applied`;
- limites de IDs, receipts e ciphertext.
