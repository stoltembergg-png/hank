# Spec: ApprovalNode ledger

> feature: workflow-approval-node
> status: em-implementacao

### US-1012 — Pausar workflow para aprovação humana

Como executor de workflow, quero criar uma aprovação ligada ao contexto exato do run para
impedir que texto de modelo ou decisão replayada continue um fluxo sensível.

#### AC-1013 — Allow/deny/expiry/cancel são estados terminais

- **Dado** uma aprovação pending
- **Quando** o actor autorizado decide, expira ou cancela
- **Então** o ledger retorna o estado correspondente e não permite continuação indevida.

#### AC-1014 — Binding e identidade são fail-closed

- **Dado** actor, project, workflow, run, node ou generation divergentes
- **Quando** uma decisão/resume é apresentada
- **Então** ela é rejeitada sem alterar o estado pending/approved.

#### AC-1015 — Decisão é one-time e bounded

- **Dado** uma aprovação aprovada
- **Quando** o token é consumido ou o ledger excede sua capacidade
- **Então** o primeiro resume é aceito uma vez; replay e excesso falham tipados.

## Suposições

- ASM-1016: persistência, transporte de eventos e UI humana são fornecidos por camadas posteriores; o ledger desta fatia é bounded em memória.

## Perguntas em aberto

Nenhuma.
