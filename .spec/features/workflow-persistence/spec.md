# Spec: Workflow persistence

> feature: workflow-persistence
> status: implementada

### US-959 — Persistir definições completas de workflow

Como runtime, quero persistir workflow, nodes e edges em SQLite de forma
transacional e versionada, para recuperar definições sem DAG parcial.

#### AC-960 — Roundtrip da definição

- **Dado** uma definição válida com workflow, nodes e edges
- **Quando** é salva e lida do SQLite
- **Então** identidade, versão e topologia são preservadas.

#### AC-961 — Isolamento e atomicidade

- **Dado** outro project_id ou uma definição com ciclo
- **Quando** a definição é lida/salva
- **Então** leitura cross-project retorna vazio e falha de validação não deixa registros parciais.

#### AC-962 — Optimistic concurrency

- **Dado** uma versão persistida
- **Quando** duas atualizações usam o mesmo expected_version
- **Então** somente a primeira é aceita; a segunda falha com conflito tipado.

## Fora de escopo

- executor, scheduler, UI, backup de produção, kill de processo e recuperação de efeitos externos;
- rollback destrutivo de migrations; correções futuras são forward-only.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
