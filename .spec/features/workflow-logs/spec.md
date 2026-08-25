# Spec: workflow logs

> feature: workflow-logs
> status: em-implementacao

### US-1060 — Registrar eventos estruturados e seguros

Como operador, quero consultar eventos de um run por correlação sem expor prompt, token, URL,
path ou conteúdo de página.

#### AC-1061 — Redação e allowlist
- **Dado** campos estruturados e texto não confiável
- **Quando** um evento é emitido
- **Então** somente campos permitidos são retidos e valores sensíveis são redigidos.

#### AC-1062 — Ordem, duplicação e isolamento
- **Dado** eventos de vários runs/projects
- **Quando** são emitidos e consultados
- **Então** a ordem temporal é monotônica, duplicatas são rejeitadas e project scope é aplicado.

#### AC-1063 — Export bounded e retenção
- **Dado** um sink cheio ou export grande
- **Quando** eventos são adicionados/exportados
- **Então** a retenção e o tamanho são bounded, com métricas de dropped/redacted.

## Suposições
- ASM-1064: retenção em memória bounded é suficiente nesta fatia; persistência/telemetry cloud não fazem parte do card.

## Perguntas em aberto
Nenhuma.
