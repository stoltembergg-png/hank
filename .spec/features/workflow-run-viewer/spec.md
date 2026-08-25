# Spec: workflow run viewer

> feature: workflow-run-viewer
> status: em-implementacao

### US-1100 — Consultar execução sem sobrescrever dados novos

Como operador, quero visualizar estado, DAG e logs de um run bounded, sem exibir secrets ou
aceitar respostas stale.

#### AC-1101 — Snapshot/DAG/timeline
- **Dado** snapshot do project/run com nodes e eventos
- **Quando** o viewer recebe o snapshot
- **Então** estado, nodes e timeline são exibidos em ordem determinística e bounded.

#### AC-1102 — Stale e scope fail-closed
- **Dado** snapshot novo já aplicado
- **Quando** chega resposta stale ou de outro projeto
- **Então** a projeção atual não é sobrescrita.

#### AC-1103 — Logs redigidos e acessíveis
- **Dado** logs com URLs, paths, tokens ou page content
- **Quando** são projetados/renderizados
- **Então** esses valores não aparecem e estados unknown/paused/recovered continuam explícitos.

## Suposições
- ASM-1104: comandos de cancel/resume/reconcile exigem Application API autorizada e ficam fora desta fatia read-only.

## Perguntas em aberto
Nenhuma.
