# Spec: plugin manifest

> feature: plugin-manifest
> status: auditada

### US-1389 — Canonical plugin metadata

Como plataforma, quero validar manifestos de plugins antes de qualquer loader.

#### AC-1389 — Explicit trust and capabilities

- **Dado** manifest com ID, versão, API revision, entrypoint, isolamento e capabilities explícitos.
- **Quando** validado.
- **Então** o manifest é canônico, bounded e recebe digest estável.
- **Dado** signer/provenance ausente, capability desconhecida/overbroad ou campo obrigatório ausente.
- **Quando** validado.
- **Então** é staged como `Untrusted` ou rejeitado fail-closed.

### US-1390 — Dependency and compatibility safety

Como plataforma, quero rejeitar dependências cíclicas e incompatibilidades.

#### AC-1390 — Dependency graph

- **Dado** grafo acíclico e suporte de OS compatível.
- **Quando** validado.
- **Então** o manifest é aceito sem ativação.
- **Dado** ciclo de dependência, secret-like value ou API revision inválida.
- **Quando** validado.
- **Então** retorna erro tipado.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Manifest canônico e bounded precede lifecycle/discovery; nenhum install, loader ou trust automático.
