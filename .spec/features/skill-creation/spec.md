# Spec: Governed Skill creation

> feature: skill-creation
> status: implementada

## Contexto

Esta feature adiciona a fronteira explícita de criação de Skills de projeto.
Ela recebe documento, arquivos e evidência declarativa fornecidos pelo
chamador, executa parser, harness determinístico e validação bounded antes de
persistir uma única versão `Draft` no repositório imutável.

A criação exige actor, capability `skill:create` escopada ao projeto, policy,
budget e trace coerentes. O resultado é redigido e não altera uma versão ativa.

## História

### US-647 — Criar uma Skill governada como Draft

Como operador de um projeto, quero registrar uma candidata declarativa para
revisão, para que uma Skill nova seja auditável sem ativação ou execução
implícita.

#### AC-803 — Criar somente Draft após todos os gates

- **Dado** documento e arquivos de uma Skill `Project` com fixture determinística
  e evidência de validação compatíveis
- **Quando** o serviço recebe capability, policy, budget e trace válidos
- **Então** persiste uma versão `Draft` com hash e relatório redigido, sem pin
  ou ativação e sem expor instruções brutas.

#### AC-804 — Criação idêntica é idempotente

- **Dado** uma candidata já registrada com a mesma identidade e conteúdo
- **Quando** a mesma solicitação é repetida
- **Então** o serviço retorna o Draft existente, marca a operação como sem
  mudança e não cria uma segunda versão.

#### AC-805 — Conteúdo privilegiado permanece bloqueado

- **Dado** fixture que solicita script, rede ou mutação do host
- **Quando** a criação é avaliada
- **Então** falha fechada antes da persistência, sem executar o passo e sem
  criar uma cabeça de Skill.

#### AC-806 — Capability, policy e budget são obrigatórios

- **Dado** capability fora do projeto, policy não autorizada ou budget inválido
- **Quando** o serviço recebe a solicitação
- **Então** rejeita a criação sem persistência ou concessão de autoridade ao
  documento.

#### AC-807 — Descarte é explícito e não ativa conteúdo

- **Dado** um Draft de projeto e confirmação com capability de descarte
- **Quando** o operador o descarta
- **Então** a versão é arquivada de forma idempotente, mantendo pin e estado
  ativo inalterados.

#### AC-808 — Tool expõe somente metadados redigidos

- **Dado** uma chamada `skill.create` com schema, identidade e policy Allow
- **Quando** o registry executa a tool
- **Então** retorna status, identidade, revisão, hashes e digest de validação,
  sem devolver documento ou instruções.

#### AC-809 — Tool não executa sem confirmação efetiva

- **Dado** uma chamada com decisão `AskOnce` ou outra decisão não confirmada
- **Quando** a tool recebe a requisição
- **Então** responde `PermissionDenied` e não persiste a candidata.

## Fora de escopo

- Ativar, promover, publicar globalmente ou instalar uma Skill.
- Executar scripts, processos, rede, filesystem real ou providers.
- Editar uma versão ativa ou resolver dependências implicitamente.
- Gerar candidatas automaticamente; isso pertence ao próximo incremento.
- Persistir texto de instrução em respostas ou relatórios de tool.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-810 | O parser, harness e validador existentes são as autoridades de criação. | confirmada | A boundary compõe os três em memória e só persiste após relatório `Passed`. |
| ASM-811 | O repositório é a última fronteira de persistência do Draft. | confirmada | `SqliteSkillRepository::create` recebe apenas Skill de projeto, parsed e estado `Draft`. |

## Perguntas em aberto

Nenhuma.
