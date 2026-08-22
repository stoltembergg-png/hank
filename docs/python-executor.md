# Python Executor

O executor (`crates/agent-runtime/src/python_executor.rs`) é o **único
caminho de execução** de tools Python registradas: registry → permission
evaluator → lifecycle → worker protocol sobre JSON-RPC. Não existe
subprocesso ad hoc.

## Fluxo

1. **Validação do request** (`ToolRequest::validate`).
2. **Registry**: `ToolRegistry::resolve` por nome/versão/projeto/capability
   — somente tools ativas, escopo correto e `ToolEnvironment::Python`.
3. **Handler gate**: `Tool::can_handle` (identidade, capability, policy).
4. **Permission evaluator**: execução Python é efeito `Execute` (mutante);
   `AskOnce`/`AskEveryTime` sem artefato de aprovação negam antes do
   dispatch; budget indisponível nega.
5. **Input bound**: bytes do input ≤ `schema.max_input_bytes`.
6. **Lifecycle**: `begin_request` reserva budget e deduplica o operation
   key (retry nega sem novo dispatch); estado ≠ Ready nega.
7. **Janela**: timeout = min(request, schema implícito, config) com
   cancelamento.
8. **Dispatch**: `WorkerMessage::Request` (request_id novo, WorkerContext
   com projeto/sessão/task/trace, capability `tool:execute:<cap>`, payload)
   via frame JSON-RPC correlado por id incremental.
9. **Resposta**: desserializa e valida como `WorkerMessage`; request_id e
   contexto devem casar exatamente com o enviado.

## Mapeamento de resultados

| Worker (`TerminalResult`) | Outcome |
|---|---|
| `succeeded` (output ≤ bound) | `Success` com payload |
| `cancelled` | `Cancelled` |
| `timed_out` | `Timeout` |
| `rejected/failed/not_supported/blocked` | `Failed` com detalhe bounded |
| canal fecha / flood de frames | `SandboxError` (worker crash/reaped) |
| janela expira | `Timeout` (worker parado) |

Bounds de saída: `min(schema.max_output_bytes, 65_536)`; detalhes de erro
truncados em 256 chars e nunca ecoam payload.

## Trust boundary

- O resultado do worker é **dado não confiável**: atravessa sem
  interpretação; identidade divergente (request/contexto) nega sem ecoar
  conteúdo.
- O transporte é o trait `WorkerTransport` — testes rodam contra um
  fixture worker em memória (zero dependência de Python no core).
- O lifecycle é o dono do processo: crash do canal → `crash()` com kill +
  reaping (sem órfão); timeout → worker parado e budget liberado.

## Limits e troubleshooting

- Timeout padrão 30s (teto); output 64 KiB (teto); 16 frames por dispatch.
- `Failed: "operation key already consumed"` → retry de chave usada
  (idempotência por design).
- `SandboxError: "worker channel closed"` → worker morreu; o lifecycle
  registra crash e admite restart limitado.
- `PermissionDenied: "approval artifact"` → fluxo de confirmação
  (confirmation-policies) deve aprovar antes.

Rollback: o executor não muta estado durável; reverter o commit do adapter
retorna ao comportamento deny-by-default do registro declarativo.
