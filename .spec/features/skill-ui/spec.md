# Spec: Scoped Skill management UI

> feature: skill-ui
> status: rascunho

## Contexto

Esta feature implementa a superfície de consulta e governança de Skills do
PR-144. A UI consome somente a API tipada do desktop; persistência, isolamento
de projeto, capability, aprovação, trace e mutação permanecem atrás da ponte
Tauri e do `ProjectSkillService`.

## Histórias

### US-642 — Consultar e governar Skills no projeto correto

Como operador de um projeto, quero consultar Skills de projeto e fontes globais
com seu estado, versão, binding, política, orçamento e proveniência, para que eu
possa distinguir o que está disponível sem importar conteúdo ou alterar runtime
silenciosamente.

#### AC-781 — Listar Skills com escopo e proveniência bounded

- **Dado** um projeto selecionado e um escopo `project` ou `global`
- **Quando** a tela consulta a API tipada de Skills
- **Então** exibe no máximo 50 registros do escopo/projeto correspondente,
  versões, lifecycle, binding, policy, budget, trace e digest da referência sem
  acessar SQLite no frontend; fontes globais sem import explícito permanecem
  indisponíveis.

#### AC-782 — Governar rollback sem cruzar projeto ou binding

- **Dado** um binding ativo e uma revisão otimista
- **Quando** o operador confirma o rollback pela UI
- **Então** a ponte valida projeto, skill, trace, approval, capability,
  confirmação e revisão, delega ao serviço de domínio e rejeita respostas ou
  bindings de outro projeto, sem executar Skill nem sobrescrever seu histórico.

## Fora de escopo

- Editor ou criação de Skill.
- Execução, ativação silenciosa ou carregamento de conteúdo de Skill.
- Acesso direto do frontend à SQLite.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-618 | A ponte desktop pode construir a capability de configuração no limite confiável. | confirmada | `ProjectSkillService` valida capability, policy, trace e revisão antes da mutação. |

## Perguntas em aberto

Nenhuma.
