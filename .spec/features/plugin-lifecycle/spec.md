# Spec: plugin lifecycle

> feature: plugin-lifecycle
> status: auditada

### US-1393 — Bounded plugin lifecycle

Como plataforma, quero controlar estados de plugin aprovado sem deixar processos órfãos ou ativar manifestos inválidos.

#### AC-1393 — Lifecycle state machine

- **Dado** um manifest válido, aprovado e compatível
- **Quando** o lifecycle recebe `Start`
- **Então** transita para `Ready`; `Stop` é determinístico e idempotente.

#### AC-1394 — Fail-closed failure handling

- **Dado** crash, hang, revoke ou incompatibilidade de versão
- **Quando** o lifecycle recebe o evento correspondente
- **Então** termina em `Quarantined` ou `Stopped`, sem restart ilimitado nem estado `Ready`.

## Segurança

- Este contrato não cria processos nem carrega código; adapters concretos ficam fora do domínio.
- Manifest, digest, API revision e permissão são vinculados antes de `Start`.
- Restart é bounded e falhas não produzem ativação implícita.

## Suposições

- ASM-1393: execução concreta será fornecida por adapter isolado em etapa posterior do lifecycle.

## Perguntas em aberto

Nenhuma.
