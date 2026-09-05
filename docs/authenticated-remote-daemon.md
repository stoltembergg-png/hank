# Authenticated Remote Daemon Contract

`remote-core` define a control plane in-memory e transport-neutral para o daemon
remoto antes de qualquer adapter de socket ou listener.

## Fluxo

1. O adapter entrega uma `CredentialRef` opaca a um `PeerAuthenticator` injetado.
2. O protocol revision do `Handshake` é negociado contra as capacidades da policy;
   versão desconhecida é rejeitada com `ProtocolNegotiationDenied`.
3. A identidade autenticada precisa corresponder exatamente a peer/node no
   `Handshake` e ao binding peer/node/project da `DaemonPolicy`.
4. Apenas então o daemon cria uma lease `Ready` bounded com ID único.
5. Expiração, revogação e stop encerram a lease exata pelo ID;
   stale cleanup não pode fechar uma sessão substituta (retorna `StaleLease`).
6. A auditoria bounded (max 256 eventos) registra peer/node/project, protocol revision,
   reason e flag `authenticated` — nunca segredo, token ou material de credencial.
   Tentativas de bootstrap rejeitadas também são registradas.
7. Depois da autenticação e autorização exatas, `security-core::rate_limit` avalia a
   classe `RemoteIngress`; excesso retorna `RateLimited`, não cria lease e mantém a
   decisão observável na auditoria redigida.

## Limites de segurança

- Nenhum socket, bind, HTTP/WebSocket, listener público, OAuth callback,
  keychain, Stronghold ou dispatch de tool é implementado aqui.
- A porta de autenticação recebe somente `CredentialRef`, não credenciais cruas.
- Auth adapters concretos podem combinar `auth-core` e `secrets-core` em uma
  fatia posterior sem alterar este core.
- Bindings de policy são exatos; mismatch e lease inválida falham fechado.
