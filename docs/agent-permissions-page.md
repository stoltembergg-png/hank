# Agent Permissions Page contract

`PermissionsPage` edits a tool permission policy through a dedicated `ToolPermissionApiClient` boundary. It does not execute tools, access shell/filesystem, invoke providers, or grant permissions automatically.

## Security invariants

- `default_effect` is always `deny` and is not editable;
- Rules are bounded to 128 entries;
- Scope IDs are required, bounded to 160 characters, and reject wildcard/newline syntax;
- Duplicate logical rules are rejected before update;
- Privileged wildcard capabilities are rejected;
- Sensitive/destructive operations (`secret`, or file/process/network execute/invoke/delete) cannot use `allow`; they require `ask` (human approval) or `deny`;
- `ask` is rendered as `Aprovação humana necessária`;
- No executor, registry, shell, filesystem, credential, payment, package-install, or grant-automation controls are present.

## Rule schema

Each rule contains only the provider-neutral permission contract:

```ts
{
  capability: { resource, action, scope? },
  effect: 'allow' | 'ask' | 'deny',
  scope: 'project' | 'agent' | 'session',
  scope_id: string,
  expires_at?: string | null
}
```

The page exposes bounded resource/action/scope selectors, effect selector, scope ID, optional expiration, add/remove operations, and a read-only effective-rule view.

## Service boundary

`DesktopToolPermissionApiClient` invokes only typed bridge commands:

- `get_agent_tool_permissions`
- `update_agent_tool_permissions`

Without a Permission Engine bridge, `get` returns `null`; the UI shows that default deny remains implicit and does not fabricate any grant.

## Lifecycle and errors

- Loading and unsupported states use `role="status"`;
- Malformed snapshots show `role="alert"` and are never rendered as active rules;
- Stale/concurrency failures remain on the page without navigation;
- Cancel without changes returns immediately; unsaved changes require explicit confirmation;
- Save is disabled without changes or while submitting.

## Tests

`frontend/tests/agent_permissions_ac_tests.test.tsx` covers:

- Loading, default deny, effective rules, and approval mode;
- Safe rule add/update payload;
- Wildcard/malformed scope rejection;
- Destructive process execution requiring `ask`/`deny`;
- Duplicate conflict rejection;
- Malformed policy snapshot rejection;
- Stale conflict handling;
- Unsupported/no-engine state;
- Cancel confirmation and accessibility/security metadata.

## ONP mapping

- T-346 — Adicionar página de permissões do Agent [concluida]