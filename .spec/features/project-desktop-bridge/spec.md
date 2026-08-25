# Spec: Desktop project lifecycle bridge

> feature: project-desktop-bridge
> status: implementada

### US-110 — Usar o lifecycle real de Project no desktop

Como usuário do Hank Desktop, quero criar, listar, abrir, atualizar e arquivar
projects através da bridge Tauri, para que a UI opere sobre o mesmo SQLite
persistente inicializado no boot.

#### AC-111 — Os cinco commands existem e usam o estado do boot

- **Dado** o shell Tauri com migrations concluídas
- **Quando** a bridge é inspecionada
- **Então** `create_project`, `list_projects`, `get_project`, `update_project` e `archive_project` estão registrados e compartilham o pool do Project state.

#### AC-112 — DTOs frontend/backend têm contrato explícito

- **Dado** os DTOs de Project no frontend e runtime
- **Quando** atravessam a bridge
- **Então** nomes, status, timestamps, IDs, settings e paginação são mapeados explicitamente; erros usam envelope tipado.

#### AC-113 — Ausência da bridge falha fechado

- **Dado** um ambiente sem IPC Tauri
- **Quando** o client desktop tenta operar em Project
- **Então** retorna `PROJECT_BRIDGE_UNAVAILABLE` e nunca inventa um project em memória.

#### AC-114 — Persistência e lifecycle não são duplicados na bridge

- **Dado** comandos válidos de Project
- **Quando** executados
- **Então** a bridge chama os application services reais, preserva migrations/SQLite e não duplica validações ou regras de domínio.

## Fora de escopo

- nodes, edges, workflow engine, scheduler, provider e execução de agents;
- bypass de policy, DB alternativo, fallback sintético e UI otimista sem confirmação.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-115 | O pool criado no boot pode ser compartilhado por repositories clonáveis. | confirmada | `SqliteProjectRepository` clona o pool sqlx. |

## Perguntas em aberto

Nenhuma.
