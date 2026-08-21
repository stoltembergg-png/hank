# Spec: Tool timeout handling

> feature: timeout-handling
> status: implementada

## Contexto

PR-110 introduz uma janela de execução comum para que adapters de Tool Runtime
compartilhem deadline monotônico, cancelamento e uma única transição terminal.
O incremento cobre o primitive de processo e os caminhos bounded de HTTP e
filesystem sem alterar as políticas específicas de cada tool.

## Histórias

### US-616 — Encerrar execução de tool de forma determinística

Como Tool Runtime, quero aplicar uma janela de execução comum, para que timeout,
cancelamento e cleanup não deixem efeitos ou processos órfãos.

#### AC-665 — Deadline monotônico e bounded

- **Dado** um timeout positivo recebido pela execução
- **Quando** a janela é criada e consultada
- **Então** o deadline usa relógio monotônico e o tempo restante nunca excede o timeout solicitado; timeout zero falha fechado

#### AC-666 — Precedência e terminalização idempotente

- **Dado** cancelamento e timeout concorrentes ou chamadas repetidas de finalização
- **Quando** a janela é consultada
- **Então** cancelamento vence a corrida, exatamente um estado terminal é reivindicado e chamadas posteriores preservam esse estado

#### AC-667 — Adapters respeitam a janela antes de efeitos

- **Dado** uma execução expirada ou cancelada
- **Quando** process, HTTP ou filesystem são invocados com a janela
- **Então** a operação falha com estado categorizado antes de iniciar novo efeito; processos em execução são encerrados e writes são revertidos quando necessário

#### AC-668 — Cancelamento compartilhado e observável

- **Dado** um token/flag compartilhado entre o chamador e o adapter
- **Quando** o chamador cancela a execução
- **Então** o adapter observa o cancelamento sem bypass de permission, redaction, limites ou isolamento

## Fora de escopo

- Definir valores específicos por tool, retry policy geral de provider ou workflow recovery.
- Reestruturar budget/trace persistence ou criar uma política de confirmação; esses contratos dependem da janela comum.
- Interromper uma chamada síncrona de filesystem já iniciada pelo sistema operacional; o adapter verifica o estado antes/depois e mantém rollback atômico para writes.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-627 | Adapters síncronos podem receber uma janela compartilhada sem depender de provider-core. | confirmada | `tool-core` mantém a abstração neutra e os adapters expõem métodos `*_with_window`; o process primitive preserva sua API de flag existente. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-616 | Onde a janela será ligada ao budget/trace do runtime? | respondida | A propagação de budget/trace será conectada na fronteira de execução do Tool Runtime; este incremento fornece a janela e os estados que essa fronteira consumirá. |
