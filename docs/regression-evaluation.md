# Regression evaluation

O gate `agent-core::regression_evaluation` usa corpus e revision identificados, impacto declarado e identidade exata de baseline/candidate. Resultados incompletos ou críticos (`fixture missing`, `skip`, `no-run`, identidade stale, classificador desconhecido e falha crítica) são `NoGo`.

O relatório é bounded, determinístico e somente evidência. Não altera testes, seleciona casos de forma controlável pelo candidate, certifica release ou autoriza rollout.
