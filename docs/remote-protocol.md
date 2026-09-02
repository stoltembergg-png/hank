# Remote Protocol Contract

Contrato de protocolo remoto versionado que opera sobre o transporte
runtime-neutral (`agent-protocol::runtime_transport`). Define:

- **Handshake e negociação** — `Handshake` declara protocol/API revision,
  identidade de peer/node, project scope e capabilities. `negotiate` valida
  compatibilidade de major/minor e exige que toda capability declarada seja
  conhecida (capability é *negociada*, nunca concedida).
- **Catálogo de comandos tipado** — `CommandCatalog` expõe comandos com
  flag de idempotência (`ping`, `get_state`, `subscribe`, `cancel`).
  Comando desconhecido é rejeitado.
- **Correlação de requests** — `RequestTracker` bounded com estados
  pending/completed/cancelled; rejeita correlação duplicada, stale e
  desconhecida de forma fail-closed.
- **Ordenação de eventos** — `EventSequence` exige sequência estritamente
  crescente; fora de ordem é rejeitado.
- **Identidade esperada** — `ExpectedIdentity` verifica peer/node antes de
  qualquer dispatch; divergência é `IdentityMismatch`.
- **Limite de payload** — `PayloadBound` rejeita payloads acima de 64 KiB.
- **Modelo de erro** — `ProtocolError` tipado e estável.

## Limites de segurança

- Este contrato **não** autentica peers, **não** abre sockets, **não** cria
  daemon, **não** executa tools e **não** transporta credenciais.
- Autenticação, WebSocket, remote tool dispatch e isolamento de credenciais
  pertencem aos cards PR-246+.
- Identity e capabilities são dados declarativos verificados por este
  contrato; dispatch autorizado pertence a etapa posterior.
