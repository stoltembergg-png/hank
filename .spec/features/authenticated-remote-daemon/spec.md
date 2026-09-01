# Spec: authenticated remote daemon

> feature: authenticated-remote-daemon
> status: em-implementacao

### US-1449 — Control plane autenticado e fail-closed

Como runtime remoto, quero admitir um peer somente após autenticação opaca e autorização
explícita de peer/node/project, para que nenhum comando remoto possa cruzar o boundary de daemon
com identidade ausente, expirada, revogada ou fora de escopo.

#### AC-1457 — Bootstrap não expõe daemon sem autenticação válida

- **Dado** bootstrap com handshake remote protocol e `CredentialRef` opaca
- **Quando** o autenticador rejeita a referência, ela está ausente, ou a identidade autenticada
  diverge do handshake
- **Então** o daemon retorna `AuthenticationDenied`, não cria sessão Ready e não registra segredo.

#### AC-1458 — Policy autoriza o escopo exato peer/node/project

- **Dado** peer autenticado e policy com bindings explícitos
- **Quando** o bootstrap pede peer, node ou project não autorizado
- **Então** retorna `AuthorizationDenied`; somente a tripla exata autorizada pode abrir lease Ready.

#### AC-1459 — Lease expira, revoga e encerra de modo idempotente

- **Dado** lease Ready bounded com deadline
- **Quando** ocorre expiração, revogação ou stop repetido
- **Então** o estado termina Closed; novos comandos são negados e stop/revoke não reabrem sessão.

#### AC-1460 — Auditoria redigida preserva identidade e razão

- **Dado** tentativa de bootstrap, revogação ou stop
- **Quando** o daemon registra o evento
- **Então** o registro contém peer/node/project, revision e reason sem material de credencial ou token.

## Segurança

- `CredentialRef` é opaca; nenhum segredo ou token cru é armazenado, retornado ou logado.
- A autenticação é uma porta injetada; OAuth, keychain/Stronghold, socket e bind de rede
  permanecem fora desta fatia.
- Policy negada, lease expirada, revogação e comando sem sessão válida falham fechado.
- O daemon não executa tools, não aceita listener público e não oferece UI.

## Suposições

- ASM-1454: a implementação concreta de autenticação consumirá `auth-core` e `secrets-core`
  em adapter posterior, mantendo este core sem material de segredo.

## Perguntas em aberto

Nenhuma.
