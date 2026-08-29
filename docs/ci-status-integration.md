# CI status integration

`agent-core::ci_status_integration` classifica somente resultados bounded de contexts allowlisted. Cada resultado deve carregar repository/PR/event/head/tree/policy/run/digest e coincidir com o contexto atual.

Missing, duplicate, skipped, cancelled, timeout, malformed, failed ou stale resulta em `unknown`/`blocked`, nunca em autorização. `merge_group` usa seu próprio evento e identidade; policy `not-applicable` é explícita. O contrato não acessa API, rede, cache, GitHub ou credenciais e nunca autoriza merge.
