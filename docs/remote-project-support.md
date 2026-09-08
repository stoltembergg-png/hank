# Remote project support boundary

## O que esta fatia entrega

`remote-core::remote_project` vincula um `ProjectId` a um node/peer exato e versionado,
sem tocar storage root, filesystem, rede ou credential store. Antes de resolver um
projeto, o registry valida:

- node e peer exatos do bind;
- projeto conhecido (rejeita `ProjectNotBound` para projeto nunca vinculado);
- versão — a mesma versão é idempotente; uma mais antiga conflita (`VersionConflict`);
- `reconcile` avança somente para versão estritamente mais nova;
- capability scope bounded e explícito (deny-by-default);
- referências de workflow/sessão/artefato como IDs tipados, nunca caminhos crus.

## O que não está provado

O module é deliberadamente transport-neutral e offline. A fixture de contrato não abre
socket, não inicia daemon, não sincroniza arquivos, não resolve credential e não chama
provider. Esta feature pode obter `CONTRACT PASS`, mas continua `PRODUCTION NO_PROOF`
até existir um adapter de sync/replicação real e testes de integração vinculados ao mesmo
SHA/tree.

## Verificação local

```bash
CARGO_BUILD_JOBS=1 cargo test -p remote-core --test remote_project_contract --locked --offline
```

O teste cobre bind no node/peer exato, rejeição cross-project e de node divergente,
conflito de versão e reconciliação, capability bounded, referências tipadas, idempotência
e reconstrução do ledger. Ele não substitui a prova de sincronização ou produção.