# Spec: adversarial planning E2E

> feature: planning-adversarial-e2e
> status: auditada

## Contexto

O pipeline de planejamento precisa demonstrar, em uma única execução
determinística, que reviewers virtuais bounded produzem findings, que findings
duplicados são reconciliados e que o resultado final permanece um artefato de
dados. A fixture usa somente os contratos reais de planejamento e de
Claim/Evidence; não chama modelos, providers, filesystem, rede, store ou efeitos
externos.

### US-1430 — Provar o caminho completo de planejamento

Como operador do Harness, quero uma execução E2E que percorra plano, reviewers,
reconciliação e resultado final.

#### AC-1430 — Plan-to-final e evidência verificada

- **Dado** um plano e uma fixture de reviewer com evidência resolverizada.
- **Quando** reviewers, reconciliação e binding Claim/Evidence são executados.
- **Então** o `FinalPlan` é determinístico e a evidência verificada mantém o
  estado `VERIFIED` sem conceder autoridade de execução.

### US-1431 — Provar dedupe e escalada

Como sistema de confiança, quero preservar provenance quando reviewers
discordam sobre o mesmo finding.

#### AC-1431 — Duplicata e reviewer hostil produzem HUMAN_REQUIRED

- **Dado** dois reviewers com a mesma chave canônica e disposições conflitantes,
  incluindo texto hostil tratado apenas como dado.
- **Quando** o `FinalPlan` é calculado.
- **Então** findings e provenance permanecem presentes, a duplicata é contada,
  o status é `HUMAN_REQUIRED` e nenhuma operação pode ser executada, aprovada
  ou mergeada.

### US-1432 — Provar corpus negativo bounded

Como boundary de planejamento, quero impedir autoaprovação, loops e consumo
acima do orçamento da fixture.

#### AC-1432 — Conflito crítico, self-review, round overflow e budget falham

- **Dado** conflito crítico, reviewer com identidade do planner, round acima do
  limite ou a sexta chamada de reviewer.
- **Quando** a fixture é executada.
- **Então** o conflito escala a `HUMAN_REQUIRED`, os demais casos falham
  fechados e nenhum write effect é produzido.

### US-1433 — Provar identidade da evidência no pipeline

Como consumidor do plano final, quero rejeitar evidência stale, foreign ou
fabricada depois da reconciliação.

#### AC-1433 — Evidence binding não promove referência adulterada

- **Dado** um `FinalPlan` e uma referência de evidência.
- **Quando** o resolver record não existe, tem status diferente ou pertence a
  outro trace.
- **Então** o binding falha fechado e o plano não ganha autoridade implícita.

### US-1434 — Provar replay e cancelamento

Como operador, quero repetir ou cancelar a execução sem efeitos duplicados.

#### AC-1434 — Replay é idempotente e cancelamento não finaliza plano

- **Dado** a mesma entrada ou uma entrada cancelada.
- **Quando** a pipeline é processada.
- **Então** fingerprints e artefatos são iguais no replay, e cancelamento não
  produz `FinalPlan` nem write effect.

## Fora de escopo

- execução de modelos, providers, tools, rede, filesystem ou persistência;
- publicação, aprovação, merge ou qualquer mutação externa;
- implementação de um novo coordenador de produção;
- E2E visual ou Tauri; a fixture é de contratos de domínio.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Fixture E2E determinística sobre os contratos reais, dedupe e
`HUMAN_REQUIRED`, corpus negativo de segurança, binding de evidência por
identidade, replay/cancelamento, teste tagged por AC, documentação e verificação
ONP passando.
