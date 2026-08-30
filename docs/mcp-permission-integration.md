# MCP permission integration

`security-core::mcp_permission` aplica default deny a capabilities MCP e exige escopo exato de server, tool, origin, project, agent e ação (discovery ou execution).

Grants possuem duração bounded (one-shot, session ou persistent), expiração e revoke. Requests repetidos são rejeitados como replay e policy revisions diferentes são stale. Discovery nunca concede execution. O contrato não implementa discovery, UI, credential storage ou plugin lifecycle.
