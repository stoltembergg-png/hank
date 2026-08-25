# Explicit Skill rollback

A boundary de rollback valida identidade, digest do target e orçamento, e
produz uma decisão determinística para uma versão conhecida. Repetições quando
o ponteiro já está no target são idempotentes. Esta fatia não persiste, não
remove provenance e não altera cache ou binding; esses efeitos exigem a
operação transacional posterior.
