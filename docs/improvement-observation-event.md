# Improvement observation event

`agent-core::improvement_observation` registra sinais bounded como dados não confiáveis. O envelope exige versão, source/type allowlisted, project/run/trace, dedup key, classe de privacidade e retenção.

Payloads secret-like são substituídos deterministicamente por `[REDACTED]`; payloads instruction-like permanecem dados e nunca ganham autoridade. Duplicatas são reconhecidas por `(project_id, dedup_key)`. O módulo não cria propostas, não avalia modelo, não persiste, não publica eventos e não possui capability mutante.
