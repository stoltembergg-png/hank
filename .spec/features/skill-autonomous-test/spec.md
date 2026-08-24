# Spec: Autonomous bounded Skill testing

> feature: skill-autonomous-test
> status: em implementação

## Contexto

Esta primeira fatia permite executar uma candidata de Skill em um boundary
orquestrado, determinístico e não mutante. O serviço reutiliza o harness de
fixtures declarativas e não executa processos, ferramentas, rede, filesystem
real ou ativação.

## História

### US-650 — Testar uma candidata sob limites sem mutar o runtime

Como mantenedor de Skills, quero testar uma candidata em um ambiente bounded,
para obter evidência antes de qualquer ativação ou transição de lifecycle.

#### AC-831 — Candidata segura produz relatório bounded

- **Dado** um candidato Draft project-scoped, identidade autorizada, fixture
  declarativa e sandbox lógico do projeto
- **Quando** o teste autônomo é executado
- **Então** produz PASS com rounds/depth/steps, digests e indicação de que a
  versão ativa não mudou.

#### AC-832 — Cancelamento e limites terminam sem execução privilegiada

- **Dado** cancelamento solicitado ou limite de steps/rounds/depth excedido
- **Quando** o teste autônomo inicia
- **Então** termina como cancelado ou timeout, sem executar conteúdo privilegiado
  e sem produzir transição de lifecycle.

#### AC-833 — Identidade, capability e sandbox são fail-closed

- **Dado** projeto, trace, capability ou sandbox fora do contexto autorizado
- **Quando** o teste autônomo é solicitado
- **Então** rejeita antes do harness, sem mutar candidato, repositório ou versão
  ativa.

## Fora de escopo

- Sandbox OS real, subprocessos, providers, rede, filesystem e ferramentas.
- Ativação, rollout, alteração de runtime ou persistência de relatórios.
- PR automática, instalação de dependências ou execução de código da candidata.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-834 | A primeira fatia representa a sandbox por identidade lógica e fixture declarativa. | confirmada | A integração com sandbox OS fica para uma fatia posterior com dependências explícitas. |

## Perguntas em aberto

Nenhuma.
