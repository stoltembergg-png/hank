# Spec: Provenance-bound Skill candidate generation

> feature: skill-candidate
> status: implementada

## Contexto

Esta feature transforma uma proposta bounded e referências de observações em
um candidato de Skill project-scoped. A proposta continua não confiável: o
parser separa instruções de dados, a policy limita capabilities, e qualquer
tentativa de injection, segredo, script ou alteração de escopo entra em
quarentena. O serviço não possui repositório e não ativa, publica, executa ou
altera uma versão existente.

## História

### US-649 — Propor evolução com provenance sem auto-publicação

Como mantenedor de Skills, quero receber uma proposta bound a observações,
para que melhorias úteis possam seguir para avaliação sem persistir conversa,
conceder capabilities ou alterar o runtime.

#### AC-817 — Proposta válida vira draft project-scoped

- **Dado** proposal parseável, observação bounded e contexto project/agent
  autorizado
- **Quando** a geração é solicitada
- **Então** retorna `Draft` com observações normalizadas, base de rollback e
  handoff hash-only bound a trace/capability/policy/budget.

#### AC-818 — Provenance, escopo, policy e budget são obrigatórios

- **Dado** actor, capability, policy, budget, trace ou escopo ausente/divergente
- **Quando** a geração é solicitada
- **Então** falha fechada antes de produzir um draft.

#### AC-819 — Injection e capability escalation entram em quarentena

- **Dado** instrução que tenta alterar hierarquia ou capability fora da policy
- **Quando** o parser/generator processa a proposta
- **Então** retorna `Quarantined` sem aprovação ou handoff executável.

#### AC-820 — Observações duplicadas são determinísticas

- **Dado** a mesma observation ID repetida com a mesma evidência
- **Quando** a geração normaliza as referências
- **Então** mantém uma referência; conflito de digest para a mesma ID falha.

#### AC-821 — Output malformado ou poisoned não vira candidato ativo

- **Dado** documento inválido, path inseguro, script ou conteúdo sensível
- **Quando** a proposta é processada
- **Então** falha ou é quarentenada, nunca produz `Draft` confiável.

#### AC-822 — Candidato não ativa e descarte é idempotente

- **Dado** candidato `Draft`
- **Quando** descarte é solicitado uma ou mais vezes
- **Então** fica `Discarded`, preserva hashes/rollback e não toca repositório,
  head ativo ou runtime.

#### AC-823 — Handoff é redigido e deduplicável

- **Dado** a mesma proposta, observações e contexto governado
- **Quando** a geração é repetida
- **Então** IDs/digests são determinísticos; o handoff não contém instruções,
  conversa, secrets ou payload bruto e muda quando policy muda.

## Fora de escopo

- Persistir, ativar, promover, publicar, testar autonomamente ou fazer rollout.
- Executar provider, tool, script, rede, filesystem ou Git.
- Gerar PR, alterar runtime code ou modificar camadas system/security.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-824 | Observações são referências externas bounded, não texto de conversa. | confirmada | O request recebe apenas ID, digest e fonte; o handoff preserva somente hashes. |
| ASM-825 | Avaliação permanece uma boundary posterior e não ativante. | confirmada | O handoff é metadado; nenhum evaluator/repository/lifecycle API é chamado. |

## Perguntas em aberto

Nenhuma.
