# Spec: audit logs

> feature: audit-logs
> status: em-implementacao

## Contexto

O runtime Hank precisa registrar eventos de segurança (autorização, migração,
recovery, plugin/remote, denial, release) como trilha auditável, redigida e
tamper-evident, sem transformar auditoria em aprovação de ações.

## Histórias

### US-2020 — Registrar eventos de segurança como trilha auditável, redigida e tamper-evident

Como runtime de segurança, quero registrar eventos sensíveis como entradas
append-only com integridade verificável, retenção bounded, redaction
obrigatória e serialização determinística, para que auditoria seja
evidência, não aprovação.

#### AC-2021 — Evento de auditoria é append-only, ordenado e bounded

- **Dado** um `AuditPolicy` com capacidade máxima, retenção, revisão e escopo
- **Quando** vários eventos forem gravados, sobrescrevendo o limite configurado
- **Então** a ordem é monotônica por sequência, o efeito líquido é bounded
  (eventos antigos são descartados de forma explícita), nenhum evento é
  mutável depois de registrado e a sequência nunca regride.

#### AC-2022 — A integridade do log é verificável e qualquer adulteração quebra a verificação

- **Dado** um `AuditLog` com eventos encadeados por hash e por sequência
- **Quando** um evento for adulterado, removido, inserido fora de ordem ou
  tiver payload divergente
- **Então** a verificação de integridade (`verify_chain`) falha com
  classificação explícita (`Broken`, `Missing`, `OutOfOrder` ou
  `HashMismatch`) e o evento adulterado é identificável por índice e
  identificador.

#### AC-2023 — Redaction obrigatória de valores sensíveis e bounded de payload

- **Dado** um `AuditEvent` com payload arbitrário em modo estruturado
- **Quando** ele for registrado, exportado, consultado ou comparado
- **Então** valores em `RedactedField::Secret` aparecem como `[REDACTED]`,
  o payload tem tamanho bounded, nenhum valor de credencial cruza a
  boundary do contrato e o output serializado é determinístico para o mesmo
  evento.

#### AC-2024 — Retenção é respeitada e exports são bounded

- **Dado** um `AuditLog` com retenção por duração e capacidade
- **Quando** `retain` for chamado, `export` for solicitado e uma consulta
  for emitida por actor/resource/policy_revision/intervalo
- **Então** eventos fora da retenção são descartados de forma explícita, o
  export respeita o limite de linhas, consultas sem critério vazio ou com
  critérios inválidos falham fechado e o resultado nunca inclui eventos
  removidos nem dados sensíveis.

#### AC-2025 — Falha de sink nunca autoriza uma ação nem esconde o estado inconsistente

- **Dado** um `AuditSink::write` que retorna erro
- **Quando** um chamador de `record` tentar registrar um evento relevante
  para decisão
- **Então** o erro é propagado tipado, a chamada externa é classificada
  como falhada (não "ok"), a decisão associada ao evento não é concedida e
  o `AuditLog` mantém consistência de sequência/integridade, mesmo após a
  falha.

## Fora de escopo

- I/O de disco, rede, relógio real, secret store ou sink concreto.
- Persistência durável, SQL/index, forwarding SIEM, agregação por janela.
- Aprovação de ações baseada em auditoria.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-2026 | `security-core` é a boundary aprovada para um log append-only puro; adapters de `agent-runtime`, `remote-core`, `storage-core` e `telemetry-core` são responsáveis por fornecer sink, relógio e identidade autenticada. | confirmada | adotada pelo card PR-259 |
| ASM-2027 | O repositório ainda não possui um `telemetry-core` nem um `audit-sink` concreto; este incremento entrega apenas o contrato, o ledger e a serialização. | confirmada | registrada em `docs/audit-logs.md` |
| ASM-2028 | Persistência durável, consulta por SQL/index, agregação por janela e forwarding para SIEM permanecem fora deste incremento e exigem contrato próprio. | confirmada | registrada no `Não-escopo` e em `docs/audit-logs.md` |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-2021 | Como o `audit-sink` concreto será plugado em `agent-runtime` sem reintroduzir dependência de Tauri/SQLx na boundary pura? | aberta | — |
| Q-2022 | Quando o forwarding para SIEM será introduzido e qual `audit-sink` adapter será o portador? | aberta | — |
