# Maximum invocation depth

`DepthLimiter` calcula a profundidade percorrendo somente a ancestry registrada,
com limite u16 explícito, validação de project scope e consistência entre o
campo declarado e o valor calculado.

A consulta é read-only, determinística e fail-closed para overflow, mismatch,
limite inválido ou ancestry ausente. Não há override por task/prompt/modelo.
