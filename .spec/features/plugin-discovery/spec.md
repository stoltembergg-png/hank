# Spec: plugin discovery

> feature: plugin-discovery
> status: auditada

### US-1391 — Safe plugin source discovery

Como plataforma, quero inspecionar somente fontes autorizadas e colocar plugins válidos em staging sem carregar código.

#### AC-1391 — Allowlisted source staging

- **Dado** uma fonte declarada dentro de uma raiz allowlist
- **Quando** o catálogo for descoberto
- **Então** manifests válidos são ordenados deterministicamente, vinculados ao digest e permanecem `Staged` sem execução.

### US-1392 — Fail-closed discovery boundaries

Como plataforma, quero rejeitar fontes inseguras, duplicatas e manifests incompatíveis.

#### AC-1392 — Fail-closed source validation

- **Dado** uma fonte fora da raiz, plugin duplicado ou API incompatível
- **Quando** o catálogo for descoberto
- **Então** a descoberta falha fechada e nenhum plugin é ativado.

## Segurança

- Nenhum código é carregado ou executado durante a descoberta.
- Paths são comparados de forma determinística e limitados à raiz autorizada.
- Entradas staged permanecem não confiáveis; não há instalação, rede ou registro provider/tool.

## Suposições

- ASM-1391: o adapter externo fornece manifests já lidos; este contrato recebe apenas metadados bounded.

## Perguntas em aberto

- Nenhuma.
