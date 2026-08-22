# Python worker lifecycle

O lifecycle do sidecar Python é controlado por `agent-runtime::python_lifecycle::PythonLifecycle`.

## States

```text
Stopped → Starting → Ready → Busy → Ready → Stopped
             │          │       │
             └→ Crashed ←┴──────┴→ TimedOut/Cancelled → Stopped
```

- `Starting` é criado somente por `spawn`; o runtime deve receber `mark_ready` após o handshake/health válido.
- `Busy` possui uma única `operation_key`, deadline de request e reserva de budget.
- `TimedOut`, `Cancelled` e `Crashed` coletam o processo e liberam a reserva antes de qualquer restart.
- `restart` exige policy bounded (`max_restarts` e `restart_backoff`) e nunca reusa uma operation key.

## Identity and isolation

Cada supervisor exige `project_id`, `session_id`, `task_id` e `trace_id` não vazios, limitados e sem caracteres de controle. A identidade fica vinculada ao supervisor; um supervisor de outro projeto não é reutilizado implicitamente.

## Process policy

- argumentos são limitados e rejeitados se contiverem controle;
- o processo inicia sem stdin/stdout/stderr herdados;
- o ambiente é limpo (`env_clear`) para evitar propagação acidental de segredos;
- falha de spawn entra em `Crashed` e não faz retry implícito;
- readiness possui deadline observável; request possui deadline independente;
- cleanup usa kill/wait e não deixa processo filho sob responsabilidade implícita.

O lifecycle não executa tools, não interpreta payloads Python e não implementa SDK. Essas responsabilidades permanecem nos cards posteriores.

## Rollback and diagnostics

Desabilitar o uso do sidecar deve resultar em `Stopped`/`BLOCKED`, nunca em loop de restart. Eventos registram somente state, IDs, operation key, budget, restart count e exit reason; não registram payloads, ambiente ou segredos.
