# Skill test framework — fixture declarativa

A primeira fatia do framework usa `agent_runtime::skill_testing` para validar
fixtures como dados bounded. `SkillFixture` exige projeto, Skill, versão,
trace, passos e orçamento explícitos. O runner só aceita `AssertLabel` e
produz um relatório determinístico por digest SHA-256.

Passos de script, rede e mutação do host são rejeitados antes do relatório.
Nenhum passo chama provider, filesystem, processo, rede, SQLite, secrets ou
lifecycle de Skill. O relatório sempre mantém `activation_requested: false`.

## Limites e uso

- até 64 passos;
- `max_steps` obrigatório e limitado;
- versão limitada a 64 bytes;
- labels e destinos limitados;
- fixture inválida falha fechada;
- reruns do mesmo fixture preservam identidade e digest.

Esta API é uma boundary de teste, não uma boundary de ativação. Próximas
fatias podem adicionar contexto MockProvider, captura de trace e sandbox
isolada, mas não devem transformar fixture declarativa em execução arbitrária.
