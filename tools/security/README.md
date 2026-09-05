# tools/security

Pura tooling para a camada de regressão de segurança (PR-260).

- `threat-regression.mjs` — runner Node, sem I/O fora do workspace, que
  carrega o manifest, valida schema, executa as verificações `NEG-001..004`
  e produz `security/reports/threat-regression.json` com `tree_sha`,
  `runner_revision` e `artifact_digest` estáveis.
- `threat-regression.spec.mjs` — suíte `node --test` que valida o
  contrato do runner e da matriz. Cada teste carrega tag `@spec:AC-21NN`
  para o ONP.

Os runners são executados em `ubuntu-24.04` pelo workflow
`.github/workflows/ci-security.yml`. O runner nunca afirma ausência de
vulnerabilidade; ele apenas confirma que o manifest e a suíte
permanecem coerentes.
