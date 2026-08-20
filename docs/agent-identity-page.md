# Agent Identity Page contract

`AgentIdentityPage` is a project-scoped React component that allows editing an Agent's identity (name and description) via the `AgentApiClient.update` service boundary. It has no direct access to storage, Tauri, or provider SDKs.

## Props

- `projectId: string` — required project scope
- `agentId: string` — required agent scope
- `onBack: () => void` — navigation callback
- `onSaved?: (agent: AgentSummary) => void` — optional success callback
- `apiClient?: AgentApiClient` — injected, defaults to `defaultAgentApi`

## UI States

- **Loading** — `Carregando agent...` with `role="status" aria-live="polite"`
- **Error** — alert box with message and retry button (`role="alert"`)
- **Form Ready** — editable fields with character counts, meta info, save/cancel actions
- **Archived Notice** — shown when agent status is `inactive` or `suspended`; fields disabled

## Form Fields

| Field | Type | Validation | Limits |
|-------|------|------------|--------|
| Name | required text | non-empty, trimmed | 120 chars max |
| Description | optional textarea | trimmed | 500 chars max |

## Validation & UX

- Real-time character counters with `aria-live="polite"`
- Clear validation hints under each field
- Submit button disabled when no changes or submitting
- Double-submit protection via `isSubmitting` state
- Cancel shows confirmation dialog if unsaved changes
- Auto-focus on name field on load

## Error Handling (maps service errors to user messages)

| Service error | User message |
|---------------|--------------|
| stale/version/concurrency | "O agent foi modificado por outro processo. Recarregue e tente novamente." |
| archived/inactive | "Não é possível editar um agent arquivado ou inativo." |
| forbidden/permission | "Você não tem permissão para editar este agent." |
| other | original error message |

## Accessibility

- `aria-label="Identidade do Agent"` on container
- Form has `noValidate` (custom validation)
- Labels properly associated via `htmlFor` / `id`
- Required field marked with `*` and `aria-required`
- Hints linked via `aria-describedby`
- Error state uses `role="alert"`
- Loading state uses `role="status" aria-live="polite"`
- Character counts announced via `aria-live="polite"`
- Status badge uses semantic color classes
- Meta IDs use monospace font for readability

## Optimistic Concurrency

- Fetches `updated_at` on load as `expectedVersion`
- Sends `expected_version` on update
- Service rejects stale versions with `ConcurrencyConflict`

## Tests

`frontend/tests/agent_identity_ac_tests.test.tsx` covers:
- Loading, error, empty states
- Form rendering with agent data
- Required/max-length validation
- Successful update with correct payload
- No-update when no changes
- Stale version, archived, permission errors
- Disabled fields for inactive/suspended agents
- Character counts
- Cancel behavior
- Accessible labels, hints, ARIA
- Submit loading state
- Status badge classes
- Form structure

## ONP Mapping

- T-343 — Adicionar página de identidade do Agent Builder [concluida]