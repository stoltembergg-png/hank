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

### US-632 — Extrair candidatos sem auto-ativação

Como Memory Pipeline, quero transformar uma sugestão de conversação em candidate
pending com provenance, para que conteúdo não confiável nunca vire memória ativa
sem review.

#### AC-741 — Candidate válido preserva identidade e provenance

- **Dado** project/session identity, source message, tipo, conteúdo e confiança
  válidos
- **Quando** o extractor processa a sugestão
- **Então** emite candidate `pending` com IDs, source e confidence bounded

#### AC-742 — Identidade, provenance e limites ausentes negam

- **Dado** projeto ausente, source message ausente ou confidence/content inválido
- **Quando** o extractor processa a sugestão
- **Então** retorna erro tipado sem emitir candidate parcial

#### AC-743 — Injection e secret-like content não são candidatos confiáveis

- **Dado** conteúdo que tenta override de policy, claim system/developer ou
  contém secret-like material
- **Quando** o extractor valida a sugestão
- **Então** rejeita como untrusted data e não altera provenance

#### AC-744 — Extractor não grava nem ativa memória

- **Dado** candidate válido
- **Quando** o extractor retorna o resultado
- **Então** somente dados `pending` são emitidos, sem repository write ou
  operação de activation

### US-633 — Calcular importance explicável e bounded

Como Memory Policy, quero pontuar candidates sem confiar no texto ou na
prioridade sugerida pelo modelo, para orientar retenção sem auto-ativação.

#### AC-745 — Mesmo fixture produz score determinístico

- **Dado** os mesmos fatores, policy e versões
- **Quando** o scorer calcula importance
- **Então** produz o mesmo valor bounded, factors explicáveis e policy/trace
  preservados

#### AC-746 — Baixa confiança e item efêmero ficam abaixo do threshold

- **Dado** confidence baixa, recência antiga e repetição zero
- **Quando** o scorer avalia
- **Então** `eligible` é false para o threshold configurado

#### AC-747 — Texto não manipula score nem explanation

- **Dado** conteúdo que afirma prioridade, tenta override ou contém secret
- **Quando** o scorer calcula
- **Então** o texto não aumenta o score nem aparece na explanation

#### AC-748 — Policy e identity inválidas falham fechadas

- **Dado** threshold/policy inválida ou trace ausente
- **Quando** o scorer recebe o input
- **Então** retorna erro bounded sem score elegível

### US-634 — Deduplicar candidates sem perda de provenance

Como Memory Pipeline, quero detectar duplicates exactos e conflitos antes da
persistência, para reduzir redundância sem mesclar conteúdo divergente.

#### AC-749 — Duplicate exacto é determinístico e scoped

- **Dado** mesmo projeto/agente/tipo/chave e conteúdo normalizado equivalente
- **Quando** o índice decide
- **Então** retorna duplicate com ID existente; outro projeto retorna `New`

#### AC-750 — Conflito permanece revisável

- **Dado** mesma chave scoped com conteúdo divergente
- **Quando** o índice decide
- **Então** retorna conflict sem sobrescrever conteúdo ou provenance

#### AC-751 — Retry é idempotente e rollback restaura o índice

- **Dado** entry já committed ou rollback solicitado
- **Quando** a operação é repetida
- **Então** duplicate identity falha sem duplicar e rollback remove somente a
  entry indicada

#### AC-752 — Input é bounded e cross-scope nunca casa

- **Dado** conteúdo acima do limite ou mesmo texto em outro projeto
- **Quando** dedupe é avaliado
- **Então** rejeita input inválido ou retorna `New`, sem cruzar escopo

### US-635 — Recuperar memória por keywords com policy

Como Context Builder, quero buscar memórias por termos com filtros e limites,
para que retrieval seja explicável, project-scoped e independente de vectors.

#### AC-753 — Query retorna somente escopo e status permitidos

- **Dado** records approved de projetos/agentes diferentes
- **Quando** keyword query é executada
- **Então** retorna somente o projeto/agente solicitado em ranking determinístico

#### AC-754 — Archived e query oversized são bloqueados

- **Dado** memória archived ou termos acima do limite
- **Quando** retrieval é executado
- **Então** archived não aparece e input oversized falha sem scan

#### AC-755 — Duplicatas e budget de bytes são bounded

- **Dado** IDs duplicados ou budget de bytes pequeno
- **Quando** records são indexados/consultados
- **Então** duplicata é rejeitada e resultados excedentes são truncados sem payload parcial

#### AC-756 — Identity/trace ausentes falham fechadas

- **Dado** query sem project identity ou trace
- **Quando** retrieval é executado
- **Então** retorna erro tipado sem consulta

### US-636 — Expor interface provider-agnostic de embeddings

Como Memory Domain, quero um contrato de embeddings com mock determinístico,
para que vector backends futuros não acoplem provider, custo ou segredo ao core.

#### AC-757 — Mock retorna dimensão e identidade explícitas

- **Dado** request válido com model/version/dimensions e references
- **Quando** o mock gera embeddings
- **Então** retorna vetor na dimensão pedida e preserva model/version/trace

#### AC-758 — Model, dimensão, batch, projeto e budget são validados

- **Dado** request com identity/model/dimension/batch/budget inválidos
- **Quando** o provider é chamado
- **Então** falha fechado sem vetor parcial

#### AC-759 — Cancelamento encerra o trace sem resultado

- **Dado** request cancelado
- **Quando** embedding é solicitado
- **Então** retorna cancelamento tipado sem produzir vetor

#### AC-760 — References e batch são bounded sem texto bruto

- **Dado** references ou batch acima do limite
- **Quando** embedding é solicitado
- **Então** rejeita sem transportar ou persistir conteúdo bruto

### US-637 — Indexar embeddings com backend vetorial opcional

Como Memory Retrieval, quero um índice vetorial local e scoped, para consultar
embeddings sem misturar tenants nem substituir repository/taxonomy.

#### AC-761 — Query é scoped, ranked e dimension-checked

- **Dado** records ativos do mesmo projeto/model/version
- **Quando** nearest-neighbor query é executada
- **Então** retorna ranking determinístico e rejeita dimensão divergente

#### AC-762 — Upsert é idempotente e archive remove do índice ativo

- **Dado** mesma identidade ou record archived
- **Quando** upsert/archive ocorre
- **Então** upsert substitui deterministicamente e archived não é consultado

#### AC-763 — k e bytes são bounded

- **Dado** k inválido ou budget de bytes pequeno
- **Quando** query ocorre
- **Então** falha ou retorna somente records inteiros dentro do budget

#### AC-764 — Rebuild falho preserva índice anterior

- **Dado** rebuild com dimensão/model incompatível
- **Quando** rebuild falha
- **Então** o índice anterior permanece consultável sem perda

### US-638 — Selecionar memória confiável dentro do contexto

Como Context Builder, quero selecionar apenas memória aprovada, autorizada e
project/agent-scoped dentro de um orçamento bounded, para que retrieval não
vaze escopo nem permita que conteúdo não confiável override a hierarquia de
instruções.

#### AC-765 — Filtros de scope, status e policy ocorrem antes do ranking

- **Dado** candidates de projetos/agentes diferentes, archived ou negados por policy/capability
- **Quando** o selector recebe project, agent, trace e request autorizado
- **Então** somente candidates approved e scoped passam ao ranking determinístico

#### AC-766 — Budget, dedupe e ordering são bounded

- **Dado** candidates duplicados com scores diferentes e budget de tokens limitado
- **Quando** a seleção é executada
- **Então** mantém o melhor candidate por chave, respeita o budget e retorna ordering explicável

#### AC-767 — Conteúdo de memória permanece untrusted e injection não entra no contexto

- **Dado** memória com prompt injection, secret-like marker ou instrução privilegiada
- **Quando** a seleção é executada
- **Então** omite o conteúdo, não o transforma em instrução e o caminho sem memória permanece seguro

#### AC-768 — Identity, cancellation e invalid input falham fechadas

- **Dado** trace ausente, candidate inválido, budget inválido ou request cancelado
- **Quando** o selector é executado
- **Então** retorna erro tipado sem seleção parcial, escrita ou efeito externo

### US-639 — Inspecionar memória project-scoped na interface

Como operador do projeto, quero revisar memórias recuperadas por projeto e
lifecycle antes de qualquer edição, para identificar provenance incorreta,
injection ou conteúdo arquivado sem acessar storage diretamente.

#### AC-769 — A UI consulta e renderiza somente o projeto selecionado

- **Dado** project identity selecionada e resposta contendo records de projetos diferentes
- **Quando** a tela de memória carrega ou troca de projeto
- **Então** o request carrega `project_id` e nenhum record foreign-project é renderizado

#### AC-770 — Lifecycle, provenance, scores e trace são visíveis

- **Dado** candidates approved, candidate e archived
- **Quando** os cards são renderizados
- **Então** status distingue candidate de active e exibe type, provenance, confidence, importance e trace

#### AC-771 — Conteúdo é seguro, bounded e não persistido no browser

- **Dado** conteúdo com HTML, injection, secret-like marker ou tamanho grande
- **Quando** a UI exibe a prévia
- **Então** React escapa, valores secret-like são redacted, a prévia é truncada e não usa localStorage/SQLite

#### AC-772 — Filtros, loading, erro e acessibilidade são explícitos

- **Dado** filtros de status/tipo, carregamento, resposta vazia ou falha da API
- **Quando** o usuário interage com a tela
- **Então** estados são anunciados, filtros são bounded e a interface permanece navegável por controles sem editar memória

## Fora de escopo

- Repository e migrações;
- extraction automática;
- Provider remoto e seleção de provider;
- retrieval bruto fora do selector;
- edição e escrita de memória pela UI;
- escrita automática pelo modelo;
- execução de instruções contidas no conteúdo.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-733 | `Approved` representa a memória ativa neste slice. | confirmada | O repository e a policy de aprovação poderão especializar o lifecycle em incrementos seguintes. |

## Perguntas em aberto

Nenhuma.
