# Improvement scoring

`agent-core::improvement_scoring` combina qualidade, segurança, regressão e custo com pesos fixos versionados no contrato. O cálculo é bounded e determinístico, e o fingerprint vincula policy/evidence.

Falha de segurança, regressão ou evidence stale tem precedência sobre qualquer valor e resulta em `NoGo`; métrica ausente resulta em `Unknown`. O score é evidência explicável e não ativa rollout.
