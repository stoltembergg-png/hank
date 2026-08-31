# Native evaluation corpus

`test_support::evaluation_corpus::core_evaluation_corpus` fornece seis
fixtures sintéticas para o Harness Evaluation V1:

- `rust_bug`
- `ci_failure`
- `architecture_violation`
- `vulnerable_dependency`
- `unsafe_operation`
- `interrupted_task`

Cada entrada combina um `FixtureCase`, um `EvaluationCase` e um
`BaselineReport`. O fixture é versionado, determinístico e tem manifest digest
igual ao digest preso no case. O baseline contém as métricas estruturadas da
versão core, terminal esperado e evidência de SHA/tree/policy/schema/fixture/
environment/artifacts.

Os quatro primeiros cenários usam terminal `PASS` para representar uma
execução sintética concluída. `unsafe_operation` termina em `BLOCKED` e
`interrupted_task` em `CANCELLED` com `NO_PROOF`; nenhum dos dois pode ativar
configuração. Todos os efeitos são virtuais ou read-only, e os payloads não
contêm rede, secrets ou caminhos de produção.

`CoreEvaluationFixture::materialize` escreve somente no
`FixtureWorkspace` fornecido pelo teste e falha se o manifest digest divergir.
IDs de fixture são limitados a um componente de caminho seguro; o workspace
também rejeita alvos existentes que resolvam fora da sua raiz. As métricas de
contagem têm teto explícito de 32 eventos, enquanto tokens e custo respeitam
os limites do budget core de 100.000 e 1.000.000 micros, respectivamente.
O runner de benchmark, a comparação candidate/baseline e o corpus de
segurança/reasoning ficam para cards posteriores.
