# Spec: workflow editor

> feature: workflow-editor
> status: em-implementacao

### US-1080 — Editar drafts de workflow com comandos tipados

Como usuário de um projeto, quero criar e validar um DAG bounded no editor, revisar erros e
salvar uma versão explicitamente, sem acesso direto ao banco ou execução automática.

#### AC-1081 — Draft válido e edge inválida
- **Dado** um projeto e draft bounded
- **Quando** nós/arestas são editados
- **Então** nós duplicados, edges desconhecidas, self-edge e ciclos são rejeitados sem mutação.

#### AC-1082 — Comando tipado e segurança de renderização
- **Dado** conteúdo de label não confiável
- **Quando** o comando/UI é produzido
- **Então** project scope, version e limites são preservados e label não é HTML executável.

#### AC-1083 — Submit idempotente e stale fail-closed
- **Dado** uma API tipada de validate/save
- **Quando** há submit duplicado ou expected version stale
- **Então** não há segunda mutação nem avanço local indevido.

## Suposições
- ASM-1084: a Application API concreta será fornecida por uma fatia posterior; esta PR usa uma interface injetada e não acessa Tauri/SQLite.

## Perguntas em aberto
Nenhuma.
