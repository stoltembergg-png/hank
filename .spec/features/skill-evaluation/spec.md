# Spec: Non-activating Skill evaluation

> feature: skill-evaluation
> status: implementada

## Contexto

Esta feature avalia uma candidata de Skill contra uma versão baseline imutável
usando evidência de fixtures determinísticas. A avaliação produz um relatório
bounded, redigido e reproduzível; ela não cria versões, promove, ativa, executa
conteúdo ou altera o baseline.

Toda solicitação é vinculada a projeto, actor, capability de leitura,
policy, budget e trace não nulo. O candidato permanece não confiável mesmo
quando o documento parece válido. A aprovação do relatório de validação é
reconfirmada antes da comparação.

## História

### US-648 — Avaliar evolução sem autoaprovação

Como mantenedor de Skills, quero comparar uma candidata com a versão conhecida,
para que regressões, injeção, custo e evidência incompleta nunca virem
ativação silenciosa.

#### AC-810 — Candidata segura produz relatório PASS

- **Dado** baseline de projeto imutável, candidata compatível, validação PASS e
  evidência determinística correspondente
- **Quando** o evaluator recebe contexto governado
- **Então** produz `Passed`, scores bounded, hashes de conteúdo/evidência e
  versão de rollback sem copiar instruções ou artefatos.

#### AC-811 — Regressão não altera baseline

- **Dado** evidência controlada com resultado inferior ao baseline
- **Quando** a candidata é avaliada
- **Então** produz `Failed` e delta negativo, sem promover, ativar ou mutar o
  baseline.

#### AC-812 — Injection ou validação adulterada entra em quarentena

- **Dado** candidata quarentenada, capability drift ou relatório stale/tampered
- **Quando** a avaliação reconfirma a evidência
- **Então** produz `Quarantined` com razão estável e não concede aprovação.

#### AC-813 — Budget e timeout são bounded

- **Dado** limite de testes excedido ou passos acima da policy
- **Quando** o evaluator calcula a execução
- **Então** produz estado não ativo (`Quarantined` ou `TimedOut`) sem executar
  passos fora do limite.

#### AC-814 — Evidência inconclusiva não passa

- **Dado** fixture reportada como inconclusiva/flaky
- **Quando** a candidata é avaliada
- **Então** produz `Inconclusive`, nunca `Passed` ou estado ativo.

#### AC-815 — Reexecução é determinística e deduplicável

- **Dado** a mesma baseline, candidata, policy, budget, evidência e trace
- **Quando** o evaluator é executado novamente
- **Então** retorna relatório byte-equivalente com o mesmo `report_digest`.

#### AC-816 — Escopo e capability não podem derivar

- **Dado** projeto divergente, baseline fora do projeto ou capability fora da
  policy
- **Quando** a avaliação é solicitada
- **Então** falha fechada sem atravessar escopo nem acessar conteúdo externo.

## Fora de escopo

- Criar, persistir, publicar, promover ou ativar Skills.
- Gerar candidatas ou alterar runtime code, policy ou instruction layers.
- Executar scripts, rede, providers, filesystem ou tools reais.
- Rollout automático; o relatório apenas registra o rollback conhecido.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-817 | Fixtures e reports são evidência controlada, não autoridade de ativação. | confirmada | O runner valida identidade/digest e o evaluator só produz estados e hashes. |
| ASM-818 | A baseline permanece imutável durante toda a comparação. | confirmada | O serviço recebe `SkillRecord` por valor, não possui repositório nem chama lifecycle APIs. |

## Perguntas em aberto

Nenhuma.
