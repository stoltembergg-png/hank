# Python Worker Logs

Logs estruturados bounded do sidecar Python: `crates/agent-runtime/src/python_logs.rs`
(captura/redação/retenção) e `python/runtime/logging.py` (emissor no
worker). Logs são **dados não confiáveis** — nunca executados, nunca mudam
policy — e são redigidos antes de qualquer retenção.

## Pipeline

```
worker stderr (JSON single-line)  ─┐
worker stdout (linhas avulsas)     ├─> PythonLogRedactor ─> PythonLogCapture (bounded)
lifecycle events (descrições)     ─┘
```

Cada registro carrega `sequence` monotônica, `timestamp_ms`, `level`,
`source` (stdout/stderr/lifecycle) e correlação completa
(worker/project/session/task/trace — validada bounded).

## Níveis

Prefixos explícitos (`ERROR`/`WARN`/`DEBUG`/`INFO`, case-insensitive)
vencem; stderr sem nível padrão é `Warn`; lifecycle é `Info`.

## Redação (determinística, antes de reter)

- Secretos mascarados com `[redacted]`: `token=…`, `password: …`,
  `api_key=…` e cadeias `Authorization: Bearer <jwt>` (encadeadas)
- Caracteres de controle/ANSI convertidos em espaço; `..` neutralizado
- Linha acima de 2 KiB truncada com `...[truncated]`; mensagem retida ≤512
  chars; espaço normalizado (linha redigida é dado derivado)
- Redações contabilizadas (`redacted_count`) para métrica

## Limites e rotação

| Limite | Valor |
|---|---|
| Linha crua | 2.048 bytes |
| Mensagem retida | 512 chars |
| Registros (capacidade) | 256 (configurável) |
| Budget de retenção | 256 KiB (configurável) |

Excedentes: mais antigos descartados com contador `dropped`; `rotate()`
drena o buffer (estado definido, `rotations` incrementa); registro maior
que o budget **não é retido** (fail-closed). Retenção é em memória —
persistência em disco exige contrato próprio.

## Isolamento e segurança

- `records_for_project` devolve somente registros do projeto consultado.
- Payload de frames nunca é ecoado em log; o e2e prova que a rejeição de
  frame loga nível+mensagem sem o conteúdo.
- Log nunca altera policy nem é tratado como comando (igualdade de
  conteúdo é a prova de não-interpretação).

## Diagnóstico

- `dropped`/`rotations`/`redacted_count`/`total_bytes` expõem a saúde da
  captura ligados ao trace do worker.
- Worker sem stderr estruturado indica versão antiga do sidecar; o
  contrato e2e valida o formato vigente.
