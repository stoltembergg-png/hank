# Skill creation

`agent_runtime::skill_creation` é a boundary explícita para registrar novas
Skills de projeto como `Draft`. Ela recebe documento, arquivos e fixture
declarativa; não lê filesystem, resolve referências, executa scripts, acessa
rede/providers, publica globalmente ou ativa conteúdo.

## Fluxo governado

1. valida actor, capability `skill:create` escopada ao projeto, policy, budget
   e trace;
2. executa o `SkillParser` com o projeto explícito;
3. executa `DeterministicSkillTestRunner`, que aceita apenas asserções
   declarativas e bloqueia script, rede e mutação do host;
4. executa `SkillValidationService` com grafo bounded e relatório redigido;
5. persiste uma versão `Draft` no `SqliteSkillRepository` somente após todos os
   gates, com deduplicação por identidade e conteúdo.

O descarte exige confirmação e capability `skill:delete`; ele arquiva o Draft
de forma idempotente e não move pin, versão ativa ou qualquer estado global.

## Tool `skill.create`

A tool declara schema versionado, ambiente Host, capability `skill:create`,
limites de input/output e lifecycle `draft_only`. O payload de sucesso contém
somente identidade, status, revisão, hash de conteúdo e digest de validação.
Decisões `AskOnce`/`Deny`, ausência de actor, schema inválido ou budget
excedido são rejeitados antes de qualquer escrita.
