# Spec: Governed Skill draft editor

> feature: skill-editor
> status: implementada

## Contexto

Esta feature implementa o editor de rascunhos do PR-145. O editor permite
alterar o manifest, as instruções Markdown e referências declaradas de uma
Skill de projeto, mas mantém parser, validação, persistência, capability,
budget, trace e versionamento no runtime/bridge confiável.

O editor nunca altera a versão ativa no lugar, não executa scripts ou
instruções, não acessa SQLite no frontend e não usa armazenamento persistente
do navegador para conteúdo de Skill.

## História

### US-644 — Editar uma Skill como nova versão governada

Como operador de um projeto, quero revisar uma Skill em um rascunho validado e
descartável, para que eu possa corrigir conteúdo sem ativar ou substituir a
versão que já está em uso.

#### AC-787 — Carregar somente conteúdo bounded do projeto selecionado

- **Dado** um `project_id` e `skill_id` válidos
- **Quando** o editor carrega a Skill pela ponte tipada
- **Então** a resposta é limitada ao projeto selecionado, contém somente
  manifest/Markdown/referências declaradas e não expõe acesso a filesystem,
  SQLite ou conteúdo de outro projeto.

#### AC-788 — Validar antes de persistir e colocar conteúdo suspeito em quarentena

- **Dado** um documento Markdown e seus arquivos explícitos
- **Quando** o operador solicita validação ou envia conteúdo com erro de parser,
  escape de referência ou tentativa de override de instruções
- **Então** o mesmo parser do runtime retorna diagnóstico bounded, rejeita ou
  coloca em quarentena o rascunho e nenhum comando de save persiste conteúdo
  inválido/quarentenado.

#### AC-789 — Salvar como versão Draft com confirmação e contexto governado

- **Dado** um rascunho válido, actor, capability, policy, budget, trace e
  `expected_revision`
- **Quando** o operador confirma explicitamente o salvamento
- **Então** o repositório cria uma versão imutável `Draft`, aplica deduplicação
  por conteúdo e mantém status, pin, versão e revisão da cabeça ativa
  inalterados; sem confirmação nenhum save é enviado.

#### AC-790 — Isolar edição local e descartar sem ativação

- **Dado** uma troca de projeto/Skill, rollback ou descarte explícito
- **Quando** o editor troca o contexto ou arquiva um Draft elegível
- **Então** edições locais não atravessam projetos, o descarte não move a
  cabeça ativa e nenhuma ação do editor executa artefatos ou oferece ativação
  silenciosa.

## Fora de escopo

- Ativar, promover ou executar uma Skill.
- Editar a fonte global ou instalar dependências.
- Acesso direto ao filesystem, SQLite, secrets ou APIs remotas pelo frontend.
- Autosave, armazenamento local do navegador ou preview HTML executável.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-734 | O parser existente é a única autoridade para validar o documento do editor. | confirmada | `SkillDraftService` chama `SkillParser` antes de `create_draft`; a ponte não possui parser alternativo. |
| ASM-735 | A revisão da cabeça ativa é suficiente para impedir save concorrente. | confirmada | `create_draft` compara `expected_revision` dentro da transação e nunca move `skill_heads.current_version`. |

## Perguntas em aberto

Nenhuma.
