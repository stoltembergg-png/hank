# Autonomous bounded Skill testing

A primeira fatia do teste autônomo de Skills é deliberadamente não executável
no sentido de host effects. Ela recebe uma candidata Draft e uma fixture
declarativa, valida identidade/capability/scope, aplica limites bounded e
reutiliza `DeterministicSkillTestRunner`.

O relatório contém apenas IDs, status, contadores e digests. Cancelamento,
limite de rounds/depth/steps, capability divergente, sandbox fora do projeto e
fixture privilegiada falham fechados. Nenhuma versão ativa, repositório,
processo, provider, rede, filesystem ou ferramenta é alterada.

A sandbox OS e a execução real de efeitos são escopo posterior e exigem
contratos próprios de isolamento, cleanup, timeout e rollback.
