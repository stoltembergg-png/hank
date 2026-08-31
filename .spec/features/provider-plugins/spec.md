# Spec: provider plugins

> feature: provider-plugins
> status: auditada

### US-1397 — Normalized provider plugin boundary

Como plataforma, quero adaptar um provider plugin ao contrato normalizado sem acoplar SDK ou transportes ao domínio.

#### AC-1397 — Approved plugin delegates normalized contract

- **Dado** um plugin aprovado e compatível encapsulando um `ModelProvider`
- **Quando** uma requisição normalizada é enviada
- **Então** ID, versão, capabilities, resposta e stream permanecem no contrato `provider-core`.

#### AC-1398 — Unapproved or unsupported plugin fails closed

- **Dado** plugin não aprovado ou capability não suportada
- **Quando** complete, stream ou operação incompatível é solicitada
- **Então** a chamada retorna erro tipado sem acesso a credenciais, rede ou efeitos externos.

## Segurança

- O adapter recebe somente `CredentialRef` opaco; não armazena nem expõe secrets.
- Nenhum SDK, HTTP client, processo ou provider específico entra em `provider-core`.
- Capabilities `Supported`, `Unsupported` e `Unknown` permanecem explícitas.

## Suposições

- ASM-1397: lifecycle e permission engine externos já validaram aprovação; o adapter somente aplica a decisão recebida.

## Perguntas em aberto

Nenhuma.
