# Spec: native evaluation corpus

> feature: native-evaluation-corpus
> status: auditada

## Contexto

O contrato nativo de avaliação precisa de um corpus pequeno e reproduzível
para exercitar o Harness antes de existir um runner real. O corpus desta fase
é sintético, offline e provider-neutral; nenhum caso aponta para repositório,
rede, segredo ou ferramenta de produção.

### US-1441 — Cobrir cenários operacionais centrais

Como mantenedor do Harness, quero seis cenários determinísticos que
representem diagnóstico, segurança e recuperação.

#### AC-1441 — Os seis cenários são versionados e distintos

- **Dado** o corpus core V1.
- **Quando** ele é construído.
- **Então** contém exatamente `rust_bug`, `ci_failure`,
  `architecture_violation`, `vulnerable_dependency`, `unsafe_operation` e
  `interrupted_task`, com IDs e fixtures distintos.

### US-1442 — Vincular resultado esperado e telemetria

Como comparador, quero que cada case declare terminal, artifacts e as
métricas mínimas do benchmark.

#### AC-1442 — Casos têm métricas e baseline compatíveis

- **Dado** qualquer case do corpus.
- **Quando** seu baseline é validado contra o `EvaluationCase`.
- **Então** terminal, artifacts e métricas de sucesso, testes, ferramentas,
  retries, budget e evidência são aceitos de forma bounded.

### US-1443 — Materializar fixtures de forma reproduzível

Como runner futuro, quero materializar cada fixture em um workspace temporário
e verificar seu manifest digest.

#### AC-1443 — Manifest digest permanece vinculado ao case

- **Dado** um workspace de fixtures controlado pelo teste.
- **Quando** os seis fixtures são materializados e lidos novamente.
- **Então** o conteúdo e o digest coincidem com o descriptor do case.

### US-1444 — Manter a fronteira offline e segura

Como mantenedor de segurança, quero impedir que o corpus introduza efeitos
externos ou dados sensíveis.

#### AC-1444 — Corpus usa somente autoridade virtual

- **Dado** qualquer fixture do corpus.
- **Quando** seus efeitos e payload são inspecionados.
- **Então** não há write externo, rede, secret ou endpoint de produção.

### US-1445 — Exercitar terminais não promocionáveis

Como gate de avaliação, quero que operações inseguras e tarefas interrompidas
permaneçam explícitas.

#### AC-1445 — Blocked e Cancelled não viram sucesso

- **Dado** `unsafe_operation` ou `interrupted_task`.
- **Quando** o baseline é validado.
- **Então** os terminais são `BLOCKED`/`CANCELLED`, a evidência correspondente
  permanece explícita e o relatório não pode ativar configuração.

### US-1446 — Detectar replay e evidência stale

Como comparador, quero que o corpus seja replayable e que evidência alterada
seja rejeitada.

#### AC-1446 — Replay é estável e evidência divergente falha fechado

- **Dado** duas construções do corpus e um relatório com fixture digest
  alterado.
- **Quando** fingerprints e evidência são validados.
- **Então** o replay mantém os mesmos digests e a evidência stale é rejeitada.

## Fora de escopo

- runner de benchmark, comparação candidate/baseline e seleção de casos;
- repositórios reais, providers, rede, filesystem de produção, UI e secrets;
- corpus de Failure Memory, skill selection, delegation e tool misuse (PR-395).

## Definition of Done

Os seis cases são construídos e materializados deterministicamente, todos têm
terminal/artifacts/métricas compatíveis, os casos de segurança permanecem
fail-closed e o verify/audit ONP passa no SHA exato.
