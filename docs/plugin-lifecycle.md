# Plugin lifecycle

`plugin-core` define a máquina de estados pura do lifecycle: `Pending`, `Ready`, `Stopped` e `Quarantined`. O contrato exige aprovação, permissão e API compatível antes de `Start`; `Stop` é idempotente, falhas de crash/hang/version mismatch entram em quarantine e o orçamento de restart é bounded.

Esta PR não cria processos, carrega módulos, acessa filesystem/rede ou ativa provider/tool plugins. Adapters isolados e efeitos concretos permanecem fora do domínio.
