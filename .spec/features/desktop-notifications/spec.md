# Spec: desktop notifications

> feature: desktop-notifications
> status: em-implementacao

### US-1284 — Notificações seguras de automação

Como usuário de um projeto desktop, quero receber sinais bounded de conclusão,
falha e approval para não perder jobs assíncronos, sem expor prompts,
credenciais ou links não validados.

#### AC-1285 — Eventos permitidos e severidade
- **Dado** um evento de conclusão, falha ou approval com project/run válidos
- **Quando** a política de notificação o avalia
- **Então** ela produz uma notificação com severidade explícita e título/corpo redacted.

#### AC-1286 — Deduplicação por projeto/run/evento
- **Dado** o mesmo sinal recebido duas vezes
- **Quando** a política o avalia pela segunda vez
- **Então** a segunda decisão é suprimida sem alterar o conteúdo da primeira.

#### AC-1287 — Rate limit bounded e preferência opt-in
- **Dado** preferência desativada ou limite de notificações atingido
- **Quando** um sinal é avaliado
- **Então** a decisão é suprimida com motivo determinístico e sem bloquear o produtor.

#### AC-1288 — Deep link seguro
- **Dado** um destino para uma execução
- **Quando** o destino não corresponde ao project/run permitido ou contém dados extras
- **Então** o link é rejeitado e nenhum path, token ou conteúdo bruto é retornado.

#### AC-1295 — Registro oficial e capability mínima
- **Dado** o bootstrap Tauri e a capability da janela principal
- **Quando** a aplicação é compilada
- **Então** o plugin oficial 2.3.3 é registrado e somente permissões mínimas de permission state/request/notify são concedidas.

#### AC-1296 — Adapter isolado e sem panic externo
- **Dado** uma decisão já produzida pela policy interna
- **Quando** o adapter Tauri é compilado e usado
- **Então** ele implementa `NotificationSink`, mapeia erros/permission states e não expõe tipos Tauri ao runtime nem usa panic em boundary externa.

#### AC-1297 — Entrega concedida uma única vez
- **Dado** um sink com permissão `Granted`
- **Quando** o worker processa uma decisão de entrega
- **Então** o sink é chamado exatamente uma vez e o resultado é `Delivered`.

#### AC-1298 — Falha do sink não derruba o runtime
- **Dado** um sink concedido que retorna falha
- **Quando** o worker tenta entregar
- **Então** retorna `Failed` de forma controlada e permanece utilizável para o scheduler.

## Suposições

| ID | Suposição | Status | Resolução |
| --- | --- | --- | --- |
| ASM-1289 | O worker de OS notification consumirá uma decisão pura desta política; este slice não introduz chamada direta de sistema operacional. | confirmada | Boundary OS será T-1292. |
| ASM-1290 | `project_id` e `run_id` são identificadores opacos e permanecem parâmetros bounded do deep link. | confirmada | IDs são validados por allowlist. |

## Perguntas em aberto

Nenhuma.

## Segurança e não-escopo

- Não renderizar raw prompt, payload ou credenciais.
- Não auto-aprovar ações.
- Não aceitar URLs externas, paths locais ou parâmetros desconhecidos.
- Não bloquear scheduler por falha de entrega.
- Não alegar permissão OS concedida sem boundary real.
