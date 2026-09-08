# Release rollback boundary

## O que esta fatia entrega

`recovery-core::rollback` modela a decisão fail-closed de retornar a uma versão
last-known-good quando um gate de boot, health ou verificação falha. O coordenador:

- registra um `ReleaseProof` (identidade + digest bounded) como known-good,
  somente para proof válido;
- seleciona o known-good anterior em falha (`Rollback`), continua quando saudável;
- recusa restaurar versão revogada (`NoKnownGoodAvailable`);
- avança o known-good apenas para versão estritamente mais nova;
- limita tentativas (`AttemptsExhausted`) para convergir sem loop;
- audita decisões sem expor material de proof (o digest é redigido de `Debug`/`Display`).

## O que não está provado

O module é transport-neutral e offline. Não há updater, signer, filesystem, banco
ou rede; não há downgrade arbitrário de dados, operação de chave de signer nem
publicação automatizada. Esta feature pode obter `CONTRACT PASS`, mas continua
`PRODUCTION NO_PROOF`.

## Verificação local

```bash
CARGO_BUILD_JOBS=1 cargo test -p recovery-core --test release_rollback_contract --locked --offline
```

O teste cobre seleção de known-good, revogação, convergência de rollback, rejeição
de proof inválido, avanço estritamente mais novo, auditoria redigida e redação do
Digest. Ele não substitui a prova de um drill de rollback real.