# Rate limiting

`security-core::rate_limit` fornece a policy token-bucket, sem transporte, storage ou
credenciais. O chamador entrega uma identidade já autenticada e um relógio monotônico.
A policy valida revisão, limites e identidade antes de admitir uma unidade de custo.

## Contrato

- `RateLimitPolicy` define `policy_revision`, `window_ms`, `burst` e `max_keys` bounded.
- `RateLimitIdentity` vincula `user`, `project` e os escopos opcionais de agent/provider/tool/node.
- `RateLimitClass` separa `Trigger`, `RemoteIngress`, `Recovery` e `Metrics`; recovery
  e métricas não são ilimitados.
- `RateLimiter::check` retorna `Allowed`, `Duplicate` ou `Denied` com razão tipada,
  revisão, restante e `retry_after_ms`; nenhum resultado executa efeitos externos.
- Relógio regressivo, revisão diferente, custo inválido, identidade inválida e estado
  cheio falham fechado.

## Integrações desta fatia

- `remote-core` chama o limiter depois de autenticar e autorizar o peer, antes de criar
  o lease. `RateLimited` é auditável e não cria sessão.
- `agent-runtime` expõe `AgentDispatchGate`, que aplica a mesma policy ao trigger de
  agente sem mudar a identidade do projeto ou do agente.

## Defaults e operação

Os adapters fornecem a policy explicitamente; os defaults de teste são pequenos e não
representam capacidade de produção. Ajuste `burst`, `window_ms`, `max_keys` e a revisão
por configuração autorizada, nunca a partir do payload. Para parada de emergência, o
chamador deve negar novos requests e revogar leases/dispatches por sua própria policy;
esta crate não executa kill, rede, persistence ou migração.

## Limites honestos

Os testes são determinísticos e em memória. Eles não provam enforcement distribuído,
limite por IP, persistência após processo, transporte real, performance ou ausência de
retry storm em produção.

## Verificação

```bash
CARGO_BUILD_JOBS=1 cargo test -p security-core --test rate_limit_contract --locked --offline
CARGO_BUILD_JOBS=1 cargo test -p remote-core --test authenticated_daemon_contract --locked --offline
CARGO_BUILD_JOBS=1 cargo test -p agent-runtime --test agent_scheduler_contract --locked --offline
CARGO_BUILD_JOBS=1 cargo clippy -p security-core -p remote-core -p agent-runtime --all-targets --locked --offline -- -D warnings
```
