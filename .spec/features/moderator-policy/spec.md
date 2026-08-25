# Spec: Moderator policy

> feature: moderator-policy
> status: implementada

## História de usuário

### US-914 — Roteamento moderado e versionado

Como grupo multi-agent, quero uma policy explícita de moderator, para que apenas
membros elegíveis sejam roteados e nenhum hard safety gate seja sobreposto.

#### AC-915 — Membro elegível passa somente com gates

- **Dado** target elegível e cycle/depth/budget aprovados
- **Quando** a policy decide
- **Então** retorna `Route`; qualquer hard gate negado retorna deny específico.

#### AC-916 — Spoof e target inelegível falham fechado

- **Dado** message text, moderator ID ou target não registrado como elegível
- **Quando** a policy decide
- **Então** não há route.

#### AC-917 — Rollback restaura snapshot anterior

- **Dado** policy versionada e snapshot mais antigo
- **Quando** rollback é aplicado
- **Então** conteúdo/limite anterior é restaurado em nova versão; snapshot
  atual é rejeitado.

## Fora de escopo

- rounds, synthesis, UI, providers e policy auto-mutável.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-918 | Moderator policy é domínio puro e recebe resultados cycle/depth/budget como gates. | confirmada | Nenhum gate é recalculado ou sobreposto. |

## Perguntas em aberto

Nenhuma.
