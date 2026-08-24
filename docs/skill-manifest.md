# Skill manifest contract

O manifesto de uma Skill é metadata declarativa, versionada e não confiável. Ele descreve a identidade, origem, escopo, arquivos, capabilities declaradas, orçamento, policy e trace de uma versão. O manifesto não concede as capabilities declaradas, não importa uma Skill global, não altera a hierarquia de instruções e não executa scripts.

## Boundary

`agent_core::SkillManifest::validate` é a primeira barreira antes de persistência ou parsing posterior. Ela aplica limites determinísticos e rejeita:

- nome fora do formato `a-z0-9[._-]`, versão que não é SemVer ou metadata vazia/oversized;
- paths absolutos, traversal, duplicados, digests inválidos e ausência de `SKILL.md`;
- capabilities duplicadas, acesso a `Secret` e metadata de capability com escopo inseguro;
- dependências duplicadas ou com requisito de versão inválido;
- referências de origem inconsistentes, markers de secrets e campos desconhecidos;
- policy que tente permitir mutação silenciosa do runtime ou override de `system`/`security`.

O schema machine-readable é [`skills/schema/manifest.json`](../skills/schema/manifest.json). O tipo Rust aplica também as relações que JSON Schema sozinho não expressa, como a correspondência entre `tests` e arquivos com role `test`.

## Escopo e importação

Uma Skill `project` só é válida quando o agregado carrega `ProjectId`. Uma Skill `global` não carrega projeto; seu eventual import para um projeto é uma decisão explícita de uma etapa posterior. A validação não cria grants, não compartilha estado e não executa qualquer arquivo.

## Capabilities e instruções

`capabilities` é uma declaração consultável por `capability_is_declared`, não uma autorização. A autorização real continua pertencendo ao Permission Engine, com principal, projeto, policy, approval, budget e trace. Skills nunca recebem acesso a secrets por confiança transitiva.

O manifesto não possui campo para escolher camada de instrução. `SKILL.md` é sempre conteúdo da camada `skill`; campos desconhecidos como `instruction_source: system` são rejeitados. `allow_instruction_override` e `allow_runtime_mutation` precisam permanecer `false`.

## Lifecycle

O conjunto de estados é `draft → testing → active → deprecated → archived`, com `blocked` como quarentena. Transições são explícitas e `archived` é terminal. Ativação exige pin da versão exata do manifesto; rollback preserva a versão anterior e não executa conteúdo.

O manifesto continua sem executar conteúdo. O parser declarativo de `SKILL.md` está documentado em [`skill-parser.md`](skill-parser.md); repository, loader, importação global, UI, evaluator e execução de scripts permanecem nos cards PR-138 em diante.
