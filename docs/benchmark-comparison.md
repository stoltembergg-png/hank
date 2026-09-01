# Skill version benchmark comparison

`test_support::benchmark_comparison::BenchmarkComparison` compara uma única
skill baseline com uma candidata em uma suíte nativa congelada. O contrato
atual usa a suíte core de seis casos; quatro são training e dois são holdout.
O chamador não fornece uma lista de casos, portanto não pode remover o
holdout, relabelar partições ou escolher somente os cenários favoráveis.

## Garantias

- baseline e candidate precisam ter a mesma suíte, schema de corpus, policy,
  schema e ambiente compartilhado; SHA e árvore podem identificar versões
  distintas e são preservados no artifact;
- todos os reports são validados contra os mesmos cases canônicos, fixtures,
  scorer, modelo, orçamento, timeout, autoridade e efeitos permitidos;
- a baseline precisa ser byte-equivalente ao baseline congelado do corpus;
- todas as 17 métricas declaradas pelo schema (`success`, estado terminal,
  qualidade, ferramentas, segurança, custo, latência, memória e seleção)
  geram deltas por case e partição; thresholds específicos continuam sendo
  aplicados a sucesso, violações de policy, evidência, custo, latência e
  falhas de ferramenta;
- thresholds excedidos e mudanças de terminal no holdout, subset ausente ou
  identidade incomparável falham fechado ou produzem `Regression` explícita;
- perdas de sucesso no training continuam sujeitas exclusivamente ao
  threshold de sucesso configurado;
- o artifact requer revisão independente vinculada aos IDs e digests dos dois
  runs e ao digest exato da policy. Reviewer e candidata não podem ser a
  mesma identidade, e a revisão precisa de assinatura Ed25519 verificada por
  uma chave pública injetada pelo Harness confiável;
- `BenchmarkComparisonReport::validate()` verifica somente shape, corpus e
  digest do payload. Um relatório desserializado só é evidência após
  `BenchmarkComparison::verify_report()` recomputar o resultado contra os
  runs exatos e verificar a assinatura;
- thresholds têm limites bounded derivados dos limites do corpus; ainda
  assim, a policy exata precisa estar coberta pela revisão assinada;
- o relatório é versionado, bounded, redigido e protegido por digest
  determinístico.

O resultado é evidência de comparação. Ele não cria, promove, ativa, executa
ou altera uma skill, e não concede autoridade de merge ou rollout. A baseline
anterior permanece preservada para rollback posterior.

## Uso em teste

```rust
let report = BenchmarkComparison::compare(
    "skill-baseline-v1",
    "skill-candidate-v2",
    &baseline_run,
    &candidate_run,
    &policy,
    Some(&independent_review),
    &trusted_reviewer_verifier,
)?;
BenchmarkComparison::verify_report(
    &report,
    &baseline_run,
    &candidate_run,
    &trusted_reviewer_verifier,
)?;
```

`independent_review` deve ser emitida pelo serviço revisor com a chave
privada correspondente ao `trusted_reviewer_verifier`; a chave privada não é
armazenada no report nem no repositório.

Runs candidatos são criados com `NativeEvaluationRun::from_reports`; a
validação final continua sendo responsabilidade do comparador contra a suíte
canônica. `BenchmarkComparisonStatus::Pass` não é promoção: a etapa posterior
deve aplicar seus próprios gates e aprovação humana.

## Quality gates

```text
cargo fmt --all -- --check
cargo clippy -p test-support --all-targets --locked -- -D warnings
cargo test -p test-support --test benchmark_comparison --locked
HANK_SKIP_TAURI=1 CI=1 node tools/run-feature-tests.mjs benchmark-comparison
HANK_SKIP_TAURI=1 CI=1 node tools/ci/run-onp-spec.mjs verify benchmark-comparison
```
