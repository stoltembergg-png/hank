# Python Worker (sidecar mínimo)

Processo sidecar opcional que implementa o [worker protocol](worker-protocol.md)
(PR-112). Vive em `python/runtime/` e usa apenas a biblioteca padrão — não
instala dependências, não lê variáveis de ambiente, não toca sistema de
arquivos e não executa código vindo de mensagens. O core compila, testa e
opera sem qualquer runtime Python (D-006, AI-016).

## Entrypoint e framing

```
python worker.py            # dentro de python/runtime
python -m runtime           # dentro de python/
```

Framing: NDJSON (uma mensagem JSON compacta por linha) em stdin/stdout. O
contrato de mensagens é o `WorkerMessage` do `agent-protocol` (schema
version 1). O transporte formal (JSON-RPC) chega no PR-114.

## Lifecycle

- **Handshake**: a primeira mensagem deve ser `handshake` válido (versão
  vigente, `worker_id` bounded, 1..=32 capabilities) → resposta
  `handshake_accepted`. Mensagem antes do handshake, versão inválida ou
  handshake malformado → erro bounded + exit 1.
- **Pronto**: `request` → resposta `not_supported` com erro bounded (o
  worker mínimo não executa nada e nunca ecoa o payload); `cancel`/`error`
  são controle silencioso; `health` → `health_report healthy`; linhas
  malformadas ou kinds desconhecidos → erro `invalid_message` bounded e o
  canal continua.
- **Shutdown**: `shutdown` → `shutdown_ack` + exit 0. EOF no stdin sem
  shutdown também encerra com exit 0 (canal fechou limpo).

## Exit codes contratuais

| Código | Significado |
|---|---|
| 0 | encerramento limpo (shutdown ack ou EOF) |
| 1 | violação de protocolo (pré-handshake, versão/handshake rejeitado) |
| 2 | argumentos fora da allowlist (o worker não aceita nenhum) |

## Isolamento

- **Env**: o worker não lê `os.environ`; o teste de processo injeta
  sentinela e verifica que o transcript não a ecoa.
- **argv**: allowlist vazia — qualquer argumento encerra com exit 2.
- **Fonte**: contrato de teste proíbe `environ`, `subprocess`, `eval(`,
  `exec(`, `system(`, `open(` e `__import__` na fonte do worker.
- **Payload**: requests nunca executam nem ecoam conteúdo — resposta
  fail-closed `not_supported`.

## Empacotamento e rollback

O sidecar é opcional e sem dependências: distribuir é copiar o diretório
`python/runtime`. Rollback = remover/ignorar o sidecar; o core e os testes
de contrato (PR-112) continuam íntegros sem Python — os testes de processo
pulam com aviso quando não há runtime disponível e sempre rodam no CI.
