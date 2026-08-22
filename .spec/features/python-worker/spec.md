# Spec: Python worker

> feature: python-worker
> status: em-implementacao

## Contexto

PR-113 cria o processo sidecar mínimo que implementa o worker protocol
definido em PR-112 (`crates/agent-protocol/src/worker.rs`). Desde PR-114 o
worker fala JSON-RPC 2.0 com framing `Content-Length` (ver spec
json-rpc-transport), usa apenas a biblioteca padrão do Python e mantém o
Python opcional: o core continua compilando, testando e operando sem
qualquer runtime Python (D-006, AI-016).

## Histórias

### US-620 — Estender o runtime com worker Python opcional

Como Agent Runtime, quero um processo worker mínimo que implemente o
handshake do protocolo, para que a extensão Python exista sem instalar
dependências, executar código de mensagens ou acoplar o core ao Python.

#### AC-683 — Handshake válido inicia worker com ciclo determinístico

- **Dado** o worker iniciado via lifecycle autorizado (sem argumentos)
- **Quando** o runtime envia handshake válido, health e shutdown
- **Então** o worker responde `handshake_accepted` com a versão vigente,
  `health_report` healthy e `shutdown_ack`, encerrando com exit 0 — sem
  ecoar variáveis de ambiente no canal

#### AC-684 — Handshake e argumentos inválidos negam fail-closed

- **Dado** uma mensagem antes do handshake, handshake com versão inválida
  ou argumentos fora da allowlist
- **Quando** o worker as processa
- **Então** responde erro bounded (`invalid_state`/`unsupported_version`) e
  encerra com código de falha distinto (1 para protocolo, 2 para argumentos)

#### AC-685 — Canal sobrevive a mensagens desconhecidas com erro bounded

- **Dado** a sessão estabelecida
- **Quando** chegam linhas malformadas ou kinds desconhecidos
- **Então** o worker responde `invalid_message` com detalhe bounded e o
  canal continua utilizável até o shutdown

#### AC-686 — Requests nunca executam payload

- **Dado** um request com payload arbitrário (incluindo instruções)
- **Quando** o worker o processa
- **Então** responde `not_supported` com erro bounded, sem ecoar o payload,
  e a resposta desserializa e valida como `WorkerMessage` do contrato

#### AC-687 — Worker sem dependências e core sem Python

- **Dado** o diretório `python/runtime` e a crate core
- **Quando** inspeciono manifestos e fonte
- **Então** não há manifestos de dependência, a fonte do worker não acessa
  env/processo/arquivo/exec e a sessão do contrato funciona em processo
  Rust sem qualquer Python

### US-622 — Supervisionar o processo Python opcional

Como Agent Runtime, quero supervisionar o processo Python com identidade,
limites e cleanup determinísticos, para que crash, timeout, cancelamento e
restart não criem processos órfãos nem repitam operações.

#### AC-694 — Lifecycle feliz é explícito e observável

- **Dado** uma configuração válida e uma identidade project/session/task/trace
- **Quando** o runtime faz spawn, recebe readiness, inicia e conclui uma operação
- **Então** as transições `Stopped → Starting → Ready → Busy → Ready → Stopped`
  são válidas e cada uma produz evento sem conteúdo sensível

#### AC-695 — Crash, timeout e cancelamento encerram o processo

- **Dado** um worker iniciado
- **Quando** ocorre crash, timeout ou cancelamento
- **Então** o processo é coletado, o estado terminal é explícito, o budget ativo é
  liberado e nenhum processo órfão permanece

#### AC-696 — Restart é bounded e não repete operação

- **Dado** um worker em `Crashed` e uma policy com limite de restart
- **Quando** o runtime reinicia ou recebe a mesma operation key
- **Então** restart respeita o limite/backoff e a operation key duplicada é
  rejeitada sem executar novamente

#### AC-697 — Identidade e isolamento são obrigatórios

- **Dado** workers de dois projetos
- **Quando** uma operação é associada a um worker
- **Então** a identidade completa permanece vinculada ao worker e um worker de
  outro project não pode ser reutilizado implicitamente

#### AC-698 — Configuração do processo é fail-closed

- **Dado** comando, argumentos ou identidade inválidos
- **Quando** o runtime tenta iniciar o worker
- **Então** a operação falha sem spawn inseguro, sem propagação de ambiente e
  sem expor segredos em eventos ou erros

### US-623 — Expor SDK Python limitado ao protocol

Como integrador Python, quero um SDK pequeno para falar o worker protocol,
para que requests tenham contexto, versão e limites sem criar runtime autônomo.

#### AC-699 — SDK faz handshake e request/response correlato

- **Dado** streams framed e identidade/capability válidas
- **Quando** o cliente executa handshake e request
- **Então** as mensagens JSON-RPC são versionadas, correlatas e o contexto
  project/session/task/trace é preservado

#### AC-700 — Contexto, IDs e payloads inválidos falham antes do write

- **Dado** identidade vazia, request id inválido, capability inválida ou payload
  acima do limite
- **Quando** o SDK prepara a chamada
- **Então** retorna erro bounded sem escrever uma chamada insegura no stream

#### AC-701 — Cancelamento e shutdown respeitam o protocolo

- **Dado** sessão handshaked
- **Quando** o cliente cancela ou encerra
- **Então** cancel é notification sem resposta esperada e shutdown exige ack
  correlato antes de fechar o cliente

#### AC-702 — Erros do worker são tipados e redigidos

- **Dado** resposta JSON-RPC de erro, EOF ou resultado inválido
- **Quando** o SDK decodifica a resposta
- **Então** produz `SdkError` bounded sem payload bruto, segredo ou instrução
  executável

#### AC-703 — SDK não concede execução

- **Dado** capability, contexto ou texto de modelo/skill no request
- **Quando** o cliente serializa a mensagem
- **Então** não registra tool, não executa subprocesso e não transforma capability
  em autorização

## Fora de escopo

- Adapter dedicado do runtime ao transporte, tools Python e
  execução de código de mensagens.
- Instalação de dependências, telemetria e empacotamento distribuído.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-631 | NDJSON em stdin/stdout basta como framing mínimo antes do transporte formal. | confirmada | O contrato PR-112 é agnóstico de transporte; o framing por linha é determinístico e testável em processo. |
| ASM-632 | Python 3 presente no ambiente de execução é condição dos testes de processo, não do core. | confirmada | Testes de processo pulam com aviso quando não há runtime; CI (ubuntu) sempre executa; core não depende de Python. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-619 | O worker precisa de identity persistente entre processos? | respondida | Não neste incremento: `worker_id` é atribuído pelo runtime no handshake; identidade persistente exige persistência própria em incremento futuro. |
| Q-620 | Exit codes devem ser contratuais? | respondida | Sim, os três atuais (0 ok, 1 protocolo, 2 argumentos) são contratuais e cobertos por teste; novos códigos exigem atualização desta spec. |
