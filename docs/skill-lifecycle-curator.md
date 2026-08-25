# Skill lifecycle curator

O curator centraliza decisões de transição entre Draft, Testing, Active,
Deprecated, Archived e Blocked. Ele usa a máquina de estados de `agent-core`,
exige evidências bounded para Testing/Active, exige rollback disponível para
Active e mantém todas as decisões project-scoped.

Esta primeira fatia é pura: não persiste, não publica eventos, não altera
ponteiros, cache, bindings ou runtime. Repetições no mesmo estado retornam
`AlreadyApplied` de forma determinística.
