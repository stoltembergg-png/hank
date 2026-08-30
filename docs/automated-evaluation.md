# Automated evaluation

`agent-core::automated_evaluation` produz evidência bounded para comparar baseline e candidate com o mesmo manifest, fixtures e seed. A identidade do candidate é verificada contra o SHA esperado; métricas cobrem qualidade, segurança, custo e latência.

Timeout, skip, recurso excedido e regressão abaixo dos thresholds não produzem aprovação. O relatório é determinístico e proposal/evidence-only: não acessa dados de produção, não usa credenciais e não ativa rollout.
