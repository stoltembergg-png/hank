# Governed Skill activation policy

Esta boundary decide se uma candidata pode avançar para ativação com base na
política de autonomia e evidências redigidas. Ela não persiste estado, altera
o ponteiro ativo, executa a candidata ou concede capabilities.

L3/L4 podem decidir Allowed quando validation, evaluation e autonomous-test
possuem digests válidos. L2 requer aprovação humana explícita; L1/L0 negam sem
aprovação. Identidade, budget ou evidência incompleta falham fechados.
