# Authenticated Remote Daemon Contract

`remote-core` define a control plane in-memory e transport-neutral para o daemon
remoto antes de qualquer adapter de socket ou listener.

## Fluxo

1. O adapter entrega uma `CredentialRef` opaca a um `PeerAuthenticator` injetado.
2. A identidade autenticada precisa corresponder exatamente a peer/node no
   `Handshake` e ao binding peer/node/project da `DaemonPolicy`.
3. Apenas então o daemon cria uma lease `Ready` bounded.
4. Expiração, revogação e stop encerram a lease; chamadas repetidas permanecem
   `Closed`.
5. A auditoria registra peer/node/project, protocol revision e reason — nunca
   segredo, token ou material de credencial.

## Limites de segurança

- Nenhum socket, bind, HTTP/WebSocket, listener público, OAuth callback,
  keychain, Stronghold ou dispatch de tool é implementado aqui.
- A porta de autenticação recebe somente `CredentialRef`, não credenciais cruas.
- Auth adapters concretos podem combinar `auth-core` e `secrets-core` em uma
  fatia posterior sem alterar este core.
- Bindings de policy são exatos; mismatch e lease inválida falham fechado.
