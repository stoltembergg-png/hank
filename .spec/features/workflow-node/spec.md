# Spec: Workflow node schema

> feature: workflow-node
> status: implementada

### US-948 — Declarar nodes versionados e tipados

Como workflow core, quero nodes tipados com schemas bounded e capability
requirements explícitos, para validar composição sem executar handlers.

#### AC-949 — Todos os tipos iniciais são explícitos

- **Dado** um node workflow
- **Quando** seu tipo é serializado
- **Então** Agent, Tool, Python, Condition, Parallel, Delay, Approval e SubWorkflow são aceitos; tipos desconhecidos falham.

#### AC-950 — Payload, identidade e políticas são bounded

- **Dado** node_id, workflow_id, schema de input/output, timeout e retry
- **Quando** a definição é validada
- **Então** campos obrigatórios, payload máximo, timeout e tentativas inválidas falham cedo.

#### AC-951 — Capabilities são deny-by-default e versionadas

- **Dado** um node sem capability requirements
- **Quando** é criado
- **Então** sua lista é vazia; capabilities declaradas são bounded e o schema version é explícito.

## Fora de escopo

- execução de handlers, scheduler, persistência, editor e resolução de edges;
- secrets plaintext, shell irrestrito e tipos aceitos silenciosamente.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
