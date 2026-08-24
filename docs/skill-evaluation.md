# Skill evaluation

`agent_runtime::skill_evaluation` é uma boundary somente de leitura para
comparar uma candidata de Skill com uma `SkillRecord` baseline imutável. O
serviço não recebe repositório e não chama operações de criação, promoção,
ativação, execução ou rollout.

## Estados e gates

- `Passed`: evidência da candidata passou e seu score não regrediu em relação à
  baseline;
- `Failed`: teste controlado falhou ou o score regrediu;
- `TimedOut`: a evidência excede o limite de passos;
- `Inconclusive`: o report não permite concluir a comparação;
- `Quarantined`: identidade, escopo, capability, fixture, validação, policy ou
  budget não são confiáveis.

Antes da comparação, a avaliação exige actor, capability `skill:evaluate`
(`Skill/Read` escopada ao projeto), policy Allow, budget válido e trace não
nulo. O relatório de validação deve continuar `Passed` e ser verificável pelo
digest do candidato. Fixtures são revalidadas pelo
`DeterministicSkillTestRunner`; script, rede e mutação do host falham fechadas.

## Relatório e rollback

O relatório contém identidade, versões baseline/candidate, score/delta, hashes
de conteúdo e fixtures, digest da validação, razões bounded e a versão baseline
como `rollback_version`, além de digests da policy e do budget efetivamente
avaliados. Não contém Markdown, instruções, scripts, prompts ou payloads. O
digest do relatório é determinístico para a mesma solicitação e muda quando o
contexto governado muda, o que permite deduplicação/replay seguro.

Nenhum estado, inclusive `Passed`, concede autorização de ativação. Uma etapa
posterior deve aplicar sua própria policy e aprovação humana antes de qualquer
transição de lifecycle.
