# Spec: Memory core

> feature: memory-core
> status: em-implementacao

## Contexto

A memória é uma entidade de domínio persistente, mas este incremento não cria
repository, extraction, retrieval, embeddings, UI ou gravação automática pelo
modelo. Conteúdo e proveniência são dados não confiáveis.

## Histórias

### US-629 — Modelar memória persistente segura

Como Agent Runtime, quero uma entidade Memory com escopo, proveniência,
confiança e lifecycle versionado, para que candidatos não virem instruções
persistentes sem validação ou aprovação.

#### AC-731 — Memória válida é candidate e bounded

- **Dado** project identity, tipo fechado, conteúdo e proveniência válidos
- **Quando** uma memória é criada e validada
- **Então** nasce como `candidate`, possui versão inicial 1 e respeita limites
  de conteúdo e confiança

#### AC-732 — Conteúdo ausente, oversized ou confiança inválida nega

- **Dado** conteúdo vazio, acima do limite ou confidence fora de 0..1
- **Quando** a entidade é validada
- **Então** retorna erro tipado sem persistir ou alterar autorização

#### AC-733 — Aprovação, archival e restore são versionados

- **Dado** uma memória candidate
- **Quando** ela é aprovada, arquivada e restaurada
- **Então** cada transição incrementa a versão e mantém o estado determinístico

#### AC-734 — Memória arquivada não pode ser aprovada diretamente

- **Dado** uma memória archived
- **Quando** approval é solicitada sem restore
- **Então** a transição nega sem alterar a versão ou o estado

### US-630 — Persistir memória com isolamento project-scoped

Como Agent Runtime, quero persistir memória com queries sempre vinculadas ao
projeto, para que archive, dedupe e optimistic version não cruzem boundaries.

#### AC-735 — CRUD e active list exigem project scope

- **Dado** memória válida e projeto autorizado
- **Quando** create, get ou list active são executados
- **Então** a transação usa project scope obrigatório e memórias archived não
  aparecem na lista ativa

#### AC-736 — Duplicata e conflito de versão falham sem mutação

- **Dado** memória já persistida ou versão esperada divergente
- **Quando** create/ archive ocorre novamente
- **Então** a operação falha sem duplicar ou alterar a versão persistida

### US-631 — Classificar memória com taxonomia estável

Como Memory Domain, quero tipos fechados e versionados para memória, para que
extractors futuros não inventem categorias nem tratem conteúdo como instrução.

#### AC-737 — Oito tipos têm wire format e hints explícitos

- **Dado** um dos tipos fact, preference, decision, lesson, project_context,
  technical_context, failure ou successful_pattern
- **Quando** o tipo é parseado e seus hints consultados
- **Então** a classificação é determinística, serializável e fornece retention
  e importance defaults explícitos

#### AC-738 — Tipo desconhecido e claim privilegiado negam

- **Dado** tipo não reconhecido ou conteúdo que tenta se declarar system/
  developer/instrução privilegiada ou contém secret-like material
- **Quando** a taxonomia valida
- **Então** retorna erro tipado sem alterar provenance ou autorização

#### AC-739 — Provenance permanece separado da classificação

- **Dado** classificação válida com source UserInput, AgentOutput ou Inferred
- **Quando** o conteúdo é validado
- **Então** provenance não é promovida a instrução confiável nem substituída
  pelo tipo

#### AC-740 — Versionamento é backward-compatible

- **Dado** wire value válido da versão atual
- **Quando** é serializado e parseado novamente
- **Então** preserva o tipo e expõe a versão de taxonomia sem depender de storage

## Fora de escopo

- Repository e migrações;
- extraction automática;
- retrieval, embeddings e ranking;
- UI;
- escrita automática pelo modelo;
- execução de instruções contidas no conteúdo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-733 | `Approved` representa a memória ativa neste slice. | confirmada | O repository e a policy de aprovação poderão especializar o lifecycle em incrementos seguintes. |

## Perguntas em aberto

Nenhuma.
