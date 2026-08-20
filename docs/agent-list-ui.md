# Agent List UI contract

`AgentList` is a project-scoped React component that renders a paginated table of Agents using only the `AgentApiClient` boundary. It has no direct access to storage, Tauri, or provider SDKs.

## Props

- `projectId: string` — required project scope
- `apiClient?: AgentApiClient` — injected, defaults to `defaultAgentApi` (desktop bridge or safe fallback)
- `_statusFilter?: AgentStatus` — reserved for future filtering (not yet wired)
- `pageSize?: number` — items per page, bounded 1..100 (default 10)

## UI states

- **Loading** — `Carregando agents...` with `role="status" aria-live="polite"`
- **Error** — red alert box with message and retry button
- **Empty** — `Nenhum agent encontrado para este projeto.`
- **Ready** — table with columns: Nome, Status, Personality, Criado em, Atualizado em

## Accessibility

- `aria-label="Gerenciamento de Agents"` on container
- Table headers with `scope="col"`
- Status badges use semantic color classes
- Pagination buttons have `aria-label` and `aria-live` page info
- Loading state announced via live region
- Error state uses `role="alert"`

## Pagination

- Shows when `totalPages > 1`
- Previous disabled on page 1
- Next disabled on last page
- Page info announced via `aria-live="polite"`

## API contract

Uses `AgentApiClient.list(ListAgentsInput)` where input is `{ project_id, limit, offset }`.
Response shape: `{ agents: AgentSummary[], total, limit, offset }`.

## Tests

`frontend/tests/agent_list_ac_tests.test.tsx` covers:
- Loading state
- Empty state
- Error + retry
- Data rendering
- Pagination navigation
- First/last page button states
- Status badge classes
- Total count in header
- Personality column

## ONP mapping

- T-342 — Adicionar UI de listagem de Agents [concluida]