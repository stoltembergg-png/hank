# Spec: Governed Skill validation

> feature: skill-validation
> status: implementada

## Contexto

Esta fatia adiciona a boundary confiável que decide se uma Skill candidata
pode avançar no lifecycle. Ela recebe um `ParsedSkill` e evidências explícitas
de teste, mas não executa conteúdo, resolve referências, acessa o host ou
altera a versão ativa. O resultado é bounded, versionado, determinístico e
redigido para auditoria.

## História

### US-646 — Validar uma Skill candidata antes de qualquer transição

Como mantenedor de Skills, quero validar manifest, parser, policy,
capabilities, referências, dependências, testes e budget antes de promover ou
restaurar uma versão, para que conteúdo incompatível ou injetado permaneça em
quarentena.

#### AC-796 — Candidata segura passa os gates

- **Dado** um manifesto de projeto válido, com teste determinístico e budget
  dentro da policy
- **Quando** o runtime valida a candidata
- **Então** produz relatório PASS com identidade, trace, regras e hashes sem
  expor instruções ou artefatos brutos.

#### AC-797 — Quarentena e ausência de testes falham fechadas

- **Dado** parser em quarentena ou manifesto sem testes/evidência
- **Quando** a candidata é validada
- **Então** o resultado é QUARANTINED com razão estável e sem ativação.

#### AC-798 — Capabilities incompatíveis são negadas

- **Dado** capability não suportada ou ausente na policy do projeto
- **Quando** a candidata é validada
- **Então** a transição é bloqueada com diagnóstico de capability, sem
  conceder autoridade ao manifesto.

#### AC-799 — Paths e dependências são bounded

- **Dado** referência com escape de caminho, ciclo ou profundidade excessiva
- **Quando** a candidata é validada
- **Então** permanece em quarentena antes de qualquer resolução ou efeito.

#### AC-800 — Teste e budget precisam corresponder à identidade

- **Dado** evidência de teste de outra versão/trace ou budget acima do limite
- **Quando** a candidata é validada
- **Então** o runtime rejeita o resultado e não permite lifecycle transition.

#### AC-801 — Validação é determinística e não mutante

- **Dado** a mesma candidata, policy e evidência
- **Quando** o gate é reexecutado
- **Então** o relatório é idêntico e o objeto candidato permanece inalterado.

#### AC-802 — Evidência de lifecycle é verificável

- **Dado** relatório PASS associado a uma candidata
- **Quando** o repositório promove ou restaura a versão
- **Então** verifica schema, identidade, digest e regras antes da mutação;
  relatório ausente, stale ou adulterado bloqueia a operação.

## Fora de escopo

- Criação automática de Skills ou geração de candidatos.
- Execução de scripts, processos, rede, filesystem real ou providers.
- Resolver/installar dependências ou ativar global Skills sem contexto de projeto.
- UI e persistência de relatórios de auditoria em uma tabela dedicada.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-803 | O parser e o harness determinístico são as fontes de evidência de conteúdo e teste. | confirmada | A boundary recebe `ParsedSkill` e `SkillTestReport`, sem parser alternativo ou execução arbitrária. |
| ASM-804 | O repositório é a última fronteira antes de promover/restaurar uma Skill. | confirmada | `promote` e `rollback` exigem relatório PASS verificável e recusam evidência ausente ou stale. |

## Perguntas em aberto

Nenhuma.
