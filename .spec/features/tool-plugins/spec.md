# Spec: tool plugins

> feature: tool-plugins
> status: auditada

### US-1399 — Permissioned tool plugin boundary

Como plataforma, quero expor tools de plugin pelo contrato `Tool` sem perder schema, sandbox e decisão de permissão.

#### AC-1399 — Valid tool plugin delegates bounded calls

- **Dado** plugin aprovado, schema válido e capability autorizada
- **Quando** a tool é chamada
- **Então** a requisição é validada pelo contrato `Tool` e a resposta permanece bounded e rastreável.

#### AC-1400 — Denied tool plugin has no unauthorized effect

- **Dado** plugin não aprovado, capability ausente, schema/version inválido ou policy deny
- **Quando** a tool é chamada
- **Então** retorna erro tipado antes da delegação, sem shell, filesystem, rede ou secrets.

## Segurança

- O adapter não executa código arbitrário; apenas delega a uma implementação `Tool` já fornecida.
- A aprovação do plugin e a policy decision são pré-requisitos independentes.
- Schema, capability, versão, input/output e timeout continuam bounded pelo contrato existente.

## Suposições

- ASM-1399: sandbox e Permission Engine são aplicados pelo contexto/adapter externo; este wrapper não concede capabilities.

## Perguntas em aberto

Nenhuma.
