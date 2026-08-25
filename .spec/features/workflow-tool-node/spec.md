# Spec: ToolNode adapter

> feature: workflow-tool-node
> status: em-implementacao

### US-975 — Executar ToolNode sob Permission Engine

Como executor de workflow, quero encaminhar um ToolNode ao Tool Runtime para que somente
ferramentas registradas, autorizadas, bounded e canceláveis produzam outcome.

#### AC-976 — Registry, schema e permission são pré-condições

- **Dado** uma ferramenta registrada e um request tipado
- **Quando** o ToolNode é admitido
- **Então** registry, input schema e PermissionEvaluator são consultados antes do handler; unknown, capability mismatch, denied e oversized falham sem execução.

#### AC-977 — Outcome é bounded, correlacionado e cancelável

- **Dado** uma permission allow
- **Quando** a ferramenta executa dentro do timeout
- **Então** o adapter devolve `ToolResponse` correlacionado por operation/trace e cancellation ou timeout não produz sucesso.

#### AC-978 — Duplicate operation é idempotente

- **Dado** a mesma operation key
- **Quando** o ToolNode é submetido novamente
- **Então** o handler não é chamado uma segunda vez e o outcome original é devolvido.

## Suposições

- ASM-979: handlers existentes são a única superfície de execução; o adapter não cria shell, sandbox ou ferramenta nova.

## Perguntas em aberto

Nenhuma.
