# Resource limiting

`agent_core::resource` é um ledger puro de admission control. Ele não mede CPU/memória/disk,
termina processos, acessa o sistema operacional, persiste estado ou autentica identidade.

- `ResourceScope` usa IDs estruturados (`Project`, `Agent`, `Task`, `Node`, `Global`), nunca uma
  chave livre do payload.
- `ResourceQuota` e `ResourceDemand` validam dimensões bounded: CPU em millicores, memória/disco
  em bytes, handles, slots de fila e subprocessos.
- `reserve` valida todos os scopes e só então muta o ledger; falha em uma dimensão não deixa
  reserva parcial.
- `release` devolve a demanda a todos os scopes. `reap_expired` é a recuperação de timeout para
  crash/cancelamento, usando relógio monotônico injetado.
- Medição real, persistência, kill/quarantine e integração com scheduler são adapters/etapas
  posteriores; este contrato não faz claims de enforcement no host.
