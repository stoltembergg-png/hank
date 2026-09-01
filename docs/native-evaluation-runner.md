# Native evaluation runner

O `NativeEvaluationRunner` é o executor offline do corpus core de avaliação.
Ele existe em `test-support` para manter a implementação fora do runtime de
produção e aceita somente `CoreEvaluationFixture` versionado, uma identidade
de ambiente explícita e um `FixtureWorkspace` temporário do chamador.

## Garantias

- exige correspondência exata de `head_sha`, `tree_sha`, policy, schema e
  ambiente com os baselines do corpus;
- valida todos os cases antes da primeira materialização;
- rejeita fixture não determinística, `ExternalWrite`, terminal inesperado,
  artifact ausente, idempotency key duplicada e ambiente incomparável;
- materializa apenas arquivos sintéticos no workspace controlado pelo teste;
- reusa fixture idêntica já existente sem sobrescrever conteúdo;
- executa replay sequencial e limitado para manter o digest determinístico;
- reconstrói `BaselineReport` com identidade, fixture, artifacts, terminal,
  métricas e evidência bounded.

O runner não acessa provider, rede, secrets, ferramentas, repositório real,
filesystem de produção, UI ou persistência. O `run_digest` é evidência do
replay; ele não concede ativação nem substitui gates de CI ou revisão humana.

## Uso em teste

```rust
let corpus = core_evaluation_corpus()?;
let environment = NativeEvaluationEnvironment::from_evidence(
    &corpus[0].baseline.evidence,
)?;
let tempdir = tempfile::tempdir()?;
let workspace = FixtureWorkspace::create(tempdir.path().join("native-runner"))?;
let run = NativeEvaluationRunner::default().run(&corpus, &environment, &workspace)?;
```

Para reexecutar o mesmo run, mantenha o workspace controlado: fixtures
idênticas são lidas e reutilizadas, enquanto conteúdo divergente falha fechado.
Um ambiente diferente deve iniciar outro baseline explícito; ele não pode ser
comparado silenciosamente com o run anterior.

## Quality gates

```text
cargo test -p test-support --test evaluation_runner --locked
HANK_SKIP_TAURI=1 CI=1 node tools/run-feature-tests.mjs native-evaluation-runner
HANK_SKIP_TAURI=1 CI=1 node tools/ci/run-onp-spec.mjs verify native-evaluation-runner
```

O verify ONP grava um artefato temporário em `.spec/verification/`; esse
artefato não deve ser commitado.
