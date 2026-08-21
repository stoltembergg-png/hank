# Spec: Http tool

> feature: http-tool
> status: implementada

## Contexto

PR-105 fornece requests HTTP bounded com TLS, allowlist de host, bloqueio private-by-default, headers seguros, timeout e response truncation.

## Histórias

### US-611 — HTTP egress controlado

Como runtime de tools, quero executar requests somente a destinos explicitamente permitidos, para evitar SSRF, exfiltração e respostas ilimitadas.

#### AC-651 — Request permitido e resposta bounded

- **Dado** URL HTTP(S) e host allowlisted, permission `Allowed`, timeout e limite válidos
- **Quando** executo a request
- **Então** retorno status, body UTF-8 limitado, truncamento e trace

#### AC-652 — SSRF, host, scheme e headers fail-closed

- **Dado** private/localhost, host não allowlisted, scheme inválido ou header sensível
- **Quando** valido/executo
- **Então** a request falha antes de enviar dados

#### AC-653 — Redirect e permission policy controlados

- **Dado** redirect ou permission pendente
- **Quando** executo
- **Então** redirect não é seguido e permission não permitida é rejeitada

## Fora de escopo

- Browser automation, crawler, download irrestrito, credenciais, bypass de egress, DNS rebinding hardening além da resolução fornecida pelo client.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-619 | `reqwest` blocking com rustls é suficiente para o primeiro contrato. | confirmada | Dependência adicionada apenas a tool-core e lockfile atualizado. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-611 | Client async é necessário agora? | respondida | Não; a primeira tool é bounded e blocking, integração async futura terá adapter explícito. |
