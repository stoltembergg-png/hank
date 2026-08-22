# Spec: JSON-RPC transport

> feature: json-rpc-transport
> status: em-implementacao

## Contexto

PR-114 transporta as worker messages via JSON-RPC 2.0 com framing
`Content-Length`, substituindo o NDJSON ad hoc do PR-113. O codec vive no
`agent-protocol` (Rust) e no `python/runtime/transport.py` (Python),
espelhados; o transporte é bounded, correlacionado por id e não confia em
texto de modelo — métodos fora da allowlist do worker protocol falham
fechados. O core continua sem exigir Python.

## Histórias

### US-621 — Transportar mensagens do worker com framing reproduzível

Como Agent Runtime, quero um transporte JSON-RPC bounded e correlacionado
para o worker Python, para que framing, backpressure, cancelamento e erros
sejam reproduzíveis nos dois lados.

#### AC-688 — Framing golden determinístico com fragmentação e coalescing

- **Dado** mensagens JSON-RPC válidas codificadas como frames `Content-Length`
- **Quando** os bytes chegam fragmentados ou colados no decoder
- **Então** a codificação é byte-estável (golden) e frames fragmentados ou
  coalescidos decodificam exatamente a mesma mensagem

#### AC-689 — Parse fail-closed com estado definido

- **Dado** bytes malformados, JSON inválido, frames excedentes ou disconnect
  com frame parcial
- **Quando** o decoder os processa
- **Então** erros são tipados e bounded (`OversizeFrame`/`InvalidJson`/
  `InvalidMessage`) sem panic, fuzz nunca decodifica mensagem válida e o
  disconnect descarta parciais com estado definido

#### AC-690 — Correlação por id com estados definidos

- **Dado** request ids registrados com deadline
- **Quando** ids duplicados em voo, capacidade excedida, conclusão,
  expiração ou cancelamento ocorrem
- **Então** os estados são definidos (`Completed`/`UnknownId`/`Expired`;
  duplicado e backpressure falham fechado com limite 256)

#### AC-691 — Mensagens válidas atravessam nos dois lados

- **Dado** o worker Python real via processo
- **Quando** handshake/health/request/shutdown trafegam com ids
- **Então** cada resposta correlaciona o id exato, requests resultam em
  `not_supported` bounded e shutdown ack encerra com exit 0

#### AC-692 — Erros determinísticos e redigidos

- **Dado** métodos desconhecidos, estrutura inválida ou ids duplicados
- **Quando** o worker responde
- **Então** respostas de erro carregam códigos documentados (-32601/
  -32600/-32011) com mensagens fixas, sem ecoar payload, e o canal
  permanece utilizável

#### AC-693 — Core não exige Python

- **Dado** o codec e a correlação em Rust
- **Quando** validam mensagens sem qualquer processo Python
- **Então** a allowlist de métodos, códigos de erro e correlação funcionam
  in-process e o worker continua sem dependências/env/exec

## Fora de escopo

- TCP remoto, autenticação remota, SDK e lifecycle supervisor (PR-116+).
- Compression, batching JSON-RPC e multi-transporte.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-633 | `Content-Length` framing (estilo LSP) é suficiente sobre stdio. | confirmada | Cobertura de fragmentação/coalescing/oversize testada nos dois lados. |
| ASM-634 | Ids JSON-RPC numéricos não negativos bastam para correlação. | confirmada | Ids são sequenciais do runtime; duplicados em voo e replay janela 256 falham fechado. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-621 | Reuso de id após conclusão é permitido? | respondida | Sim, desde que não esteja em voo; a janela bounded de replay (256) protege contra reuso imediato malicioso no worker. |
| Q-622 | Notifications precisam de resposta de erro? | respondida | Não: notifications não têm alvo de resposta; violações estruturais são descartadas bounded sem eco. |
