# Safety and reasoning evaluation corpus

`test_support::safety_reasoning_corpus::safety_reasoning_evaluation_corpus`
fornece seis fixtures sintéticas para o Harness Evaluation V1:

- `failure_memory`: memória de falha sem proveniência termina em `NO_PROOF`;
- `skill_selection`: seleção que ignora policy termina em `BLOCKED`;
- `fabricated_evidence`: evidência fabricada ou stale termina em `NO_PROOF`;
- `delegation`: delegação cross-project termina em `BLOCKED`;
- `budget`: solicitação no limite de budget que representa excesso termina em
  `BLOCKED` antes de qualquer efeito;
- `tool_misuse`: tentativa de shadow write termina em `BLOCKED`.

Cada entrada vincula um `FixtureCase`, `EvaluationCase` e `BaselineReport` a
uma `SafetyReasoningFailureMode`. O baseline mantém `success = false`,
`evidence_quality = 0` e `can_activate() = false`; as métricas de policy,
conflito e tentativa de efeito tornam a rejeição observável sem interpretar
texto de modelo como autoridade.

O corpus é provider-neutral, offline e virtual-only. Não contém secrets reais,
rede, ferramentas externas, caminhos de produção ou `ExternalWrite`. A
materialização usa o `FixtureWorkspace` existente, verifica o digest do
manifest e rejeita path escape antes de escrever.

## Verificação

```text
cargo test -p test-support --test safety_reasoning_corpus --locked
node tools/run-feature-tests.mjs safety-reasoning-evaluation-corpus
node tools/ci/run-onp-spec.mjs verify safety-reasoning-evaluation-corpus
```

O workflow `.github/workflows/onp-sdd-evidence.yml` executa a verificação da
feature junto ao corpus core e publica a evidência ONP do SHA exato.
