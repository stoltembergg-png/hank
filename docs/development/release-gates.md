# Required checks da `main`

A fonte versionada dos gates protegidos é `.github/required-checks.json`.

A proteção efetiva é o Ruleset do repositório `main-required-checks` aplicado a
`refs/heads/main`. O preflight consulta a representação pública e sem privilégio
administrativo em `/repos/{owner}/{repo}/rules/branches/main` e compara o
Ruleset com o manifesto antes de aguardar os check-runs.

Para verificar localmente com uma resposta salva da API:

```bash
node tools/release-required-checks.mjs validate \
  --manifest .github/required-checks.json \
  --rules /tmp/active-rules.json \
  --output /tmp/required-checks.txt
```

A verificação falha quando um gate existe somente no manifesto, somente no
Ruleset, aparece duplicado ou quando a política strict não está ativa. A
proteção clássica de branch permanece ativa durante a migração e continua
sendo administrada separadamente.
