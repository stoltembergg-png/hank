# Spec: Tool-call rendering

> feature: tool-call-rendering
> status: implementada

## Contexto

PR-109 adiciona uma representação visual bounded e somente de leitura para tool calls na camada frontend. A UI recebe um read model já produzido pela fronteira da aplicação; não executa tools, acessa providers, filesystem ou Tauri diretamente, nem decide políticas de autorização.

## Histórias

### US-615 — Visualizar execução de tools com segurança

Como usuário do desktop, quero entender o estado e o resultado de uma tool call sem que conteúdo não confiável seja interpretado como markup ou comando, para acompanhar operações auditáveis do agente.

#### AC-662 — Estado e escopo da tool call são visíveis

- **Dado** um read model de tool call válido
- **Quando** a UI renderiza a entrada
- **Então** exibe nome e versão da tool, estado acessível, escopo de projeto/agente e trace ID quando fornecidos, sem executar a operação

#### AC-663 — Conteúdo não confiável é bounded e redigido

- **Dado** uma tool call com argumentos, resultado, erro ou chaves sensíveis
- **Quando** a UI renderiza o conteúdo
- **Então** serializa como texto bounded, redige valores sensíveis e não interpreta o conteúdo como HTML, Markdown ou instrução executável

#### AC-664 — Aprovação permanece na fronteira da aplicação

- **Dado** uma tool call no estado `ask` ou `denied`
- **Quando** o usuário observa ou interage com o card
- **Então** `ask` apenas emite um callback explícito para a camada superior, `denied` não oferece execução, e nenhuma execução local é iniciada

## Fora de escopo

- Executar tools, chamar a Application API ou implementar o backend de aprovação.
- Consumir diretamente eventos de runtime, persistir histórico ou alterar o roteamento global do chat.
- Alterar políticas de permissão, timeout ou cancelamento; esses contratos pertencem a features próprias.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-626 | O componente recebe um read model já validado pela camada superior. | confirmada | `ChatPage` injeta `toolCalls` e o componente limita a apresentação; não há acesso a infraestrutura. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-615 | Onde o stream de eventos de tool call será conectado ao chat? | respondida | A integração será feita em incremento posterior na fronteira runtime/Application API; este card cobre somente o read model e a apresentação. |
