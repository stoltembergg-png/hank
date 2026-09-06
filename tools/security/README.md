# tools/security

Tooling pura para as lanes de regressão de segurança.

- `threat-regression.mjs` — runner da matriz de ameaças da PR-260; valida o
  manifest, executa as verificações negativas e produz
  `security/reports/threat-regression.json`.
- `threat-regression.spec.mjs` — contrato Node da matriz de ameaças.
- `fuzz-runner.mjs` — runner da PR-261; valida o manifest versionado, executa
  `test-support::fuzz_contract` com dependências travadas e produz
  `security/reports/fuzz.json`.
- `fuzz-tests.spec.mjs` — contrato Node do manifest e do relatório de fuzz.
- `fuzz-feature-tests.mjs` — wrapper TAP opcional para combinar a execução
  Rust e a evidência por AC.

Os runners não carregam credenciais, não acessam providers ou rede de
produção e não afirmam ausência de vulnerabilidades. A lane de fuzz é
bounded e reproduzível; o CI roda em `ubuntu-24.04` com `contents: read` por
`.github/workflows/ci-fuzz.yml`. Dependências Rust usam `--locked`; o CI pode
resolver o conteúdo do lockfile em um checkout limpo, enquanto os gates locais
podem usar `--offline` quando o cache estiver completo.
