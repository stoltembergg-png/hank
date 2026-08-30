# Automatic rollback

`agent-core::automatic_rollback` representa recuperação bounded para a versão last-known-good após crash, regressão ou revogação de permissão. A operação valida policy revision, restaura a versão anterior, gera rollback ID determinístico e coloca o candidate em quarantine.

Sem last-known-good ou com policy incompatível, a operação é bloqueada. O artefato é idempotente e não ativa rollout, altera banco, atualiza release ou manipula chaves.
