# Spec: safety and reasoning evaluation corpus

> feature: safety-reasoning-evaluation-corpus
> status: implementada

## Contexto

O corpus core cobre falhas operacionais, mas ainda não exercita os limites de
proveniência, seleção, delegação, orçamento e uso de ferramentas do Harness.
Esta feature adiciona seis casos sintéticos, determinísticos e offline para
provar que decisões inseguras terminam de forma observável e fail-closed.

### US-1447 — Cobrir riscos de segurança e raciocínio

Como mantenedor do Harness, quero um corpus provider-neutral que represente
falhas de memória, seleção, evidência, delegação, budget e ferramentas.

#### AC-1447 — Os seis casos são versionados e distintos

- **Dado** o corpus de segurança e raciocínio V1.
- **Quando** ele é construído.
- **Então** contém exatamente `failure_memory`, `skill_selection`,
  `fabricated_evidence`, `delegation`, `budget` e `tool_misuse`, com fixtures,
  identidades e digests distintos.

#### AC-1448 — Todo risco tem terminal fail-closed observável

- **Dado** qualquer caso do corpus.
- **Quando** seu baseline é validado.
- **Então** o terminal esperado não é `PASS`, a evidência é `BLOCKED` ou
  `NO_PROOF`, a métrica `success` é falsa e o relatório não pode ativar uma
  configuração.

#### AC-1449 — Evidência fabricada não vira autoridade

- **Dado** o caso `fabricated_evidence`.
- **Quando** a evidência é incompleta, stale ou fabricada.
- **Então** a divergência é observável por `evidence_conflicts` ou digest e a
  validação falha fechado.

#### AC-1450 — Fronteiras de projeto, escrita e policy são explícitas

- **Dado** os casos de delegação cross-project, tool misuse/shadow write e
  skill policy bypass.
- **Quando** seus efeitos declarados e métricas são inspecionados.
- **Então** nenhum caso declara `ExternalWrite`, e a violação correspondente
  aparece como `policy_violations` ou `external_side_effect_attempts` sem
  executar efeito externo.

#### AC-1451 — Fixtures permanecem determinísticas e offline

- **Dado** os seis fixtures sintéticos.
- **Quando** são materializados em um `FixtureWorkspace` temporário e
  reconstruídos.
- **Então** manifest digest, payload e identidade permanecem iguais, sem rede,
  segredo real ou caminho de produção.

#### AC-1452 — Replay e evidência stale falham fechado

- **Dado** duas construções do corpus e um baseline alterado.
- **Quando** fingerprints são comparados e o baseline é validado.
- **Então** o replay é estável, enquanto evidência stale ou fixture fora da
  raiz é rejeitada sem mutação externa.

## Fora de escopo

- runner de benchmark, comparação candidate/baseline e seleção de casos;
- providers, rede, secrets reais, ferramentas externas, UI e execução de
  efeitos;
- comparação de regressão e adaptação de corpus externo (PR-396+).

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Os seis casos são construídos e materializados deterministicamente, cada risco
possui terminal e métrica fail-closed observáveis, evidência fabricada,
cross-project, shadow write e policy bypass são rejeitados, e o verify/audit
ONP passa no SHA exato.
