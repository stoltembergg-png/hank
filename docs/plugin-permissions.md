# Plugin permissions

`security-core` define permissões de plugin com default deny e vínculo exato a plugin ID, digest, versão, capability, projeto, agente e policy revision. As ações são separadas em `Install`, `Start` e `Use`; grants revogados não voltam a autorizar o vínculo, e upgrades com digest/versão/capability diferente exigem novo consentimento.

O contrato é puro: não acessa secrets, não inicia plugins e não muta lifecycle ou runtime. Nenhuma capability de MCP, browser, UI, filesystem, rede ou processo é herdada implicitamente.
