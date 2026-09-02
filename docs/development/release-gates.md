# Required checks da `main`

A fonte versionada dos gates protegidos é `.github/required-checks.json`. O campo
`requiredChecks` lista somente checks produzidos no commit integrado e consumidos
pelos workflows de release; `pullRequestChecks` lista checks de proteção de PR,
como CodeRabbit e Aikido, que são exigidos pelo Ruleset mas não são esperados em
um SHA pós-merge.

A proteção efetiva é o Ruleset do repositório `main-required-checks` aplicado a
`refs/heads/main`. O preflight consulta a representação pública e sem privilégio
administrativo em `/repos/{owner}/{repo}/rules/branches/main` e verifica se os
dois grupos do manifesto estão cobertos pelo Ruleset antes de aguardar os
check-runs de release. O arquivo de saída usado pelo polling contém apenas
`requiredChecks`.

Para verificar localmente com uma resposta salva da API:

```bash
node tools/release-required-checks.mjs validate \
  --manifest .github/required-checks.json \
  --rules /tmp/active-rules.json \
  --output /tmp/required-checks.txt
```

A verificação falha quando um gate versionado não está no Ruleset, aparece
duplicado ou quando a política strict não está ativa. Checks adicionais do
Ruleset são permitidos para proteção de PR e não são incluídos no polling de
release. A proteção clássica de branch permanece ativa durante a migração e
continua sendo administrada separadamente.
