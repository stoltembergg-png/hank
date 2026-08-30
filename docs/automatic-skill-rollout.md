# Automatic skill rollout

`agent-core::automatic_skill_rollout` define apenas elegibilidade staged. Todas as evidências anteriores devem estar presentes; o escopo resultante é `ProjectCanary`, com versão pinada e sem expansão global.

Falha de health ou kill switch retorna `Stopped`; evidência ausente ou escopo não autorizado retorna `Blocked`. O contrato não muta skill ativa, scheduler ou runtime e não autoriza ativação global.
