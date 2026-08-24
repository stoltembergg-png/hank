# Skill validation

`agent_runtime::skill_validation` é o gate confiável entre uma Skill
`ParsedSkill` e uma transição de lifecycle. A API opera somente sobre dados já
fornecidos pelo chamador; não lê filesystem, resolve links, executa scripts,
acessa rede/providers ou altera a cabeça persistida.

## Gates

O relatório verifica, em ordem estável:

- identidade de projeto, Skill, versão, actor e trace;
- schema/invariantes do manifest e quarentena do parser;
- policy e capabilities declaradas, permitindo somente capacidades read-only
  suportadas e presentes na policy recebida;
- paths relativos, artefatos declarados e links sem escape ou link externo;
- grafo de dependências sem duplicação, ciclo ou profundidade acima do limite;
- testes declarados e `SkillTestReport` PASS da mesma identidade, sem pedido de
  ativação;
- budget do manifest dentro do budget de policy.

Qualquer falha gera `Quarantined` e razões bounded. O relatório contém apenas
identidade, regras, razões, schema e hashes de policy, budget, conteúdo e
evidência; instruções, scripts, referências e texto de erro não são copiados.

## Lifecycle, rollback e remediação

`SqliteSkillRepository::promote` e `rollback` exigem um
`SkillValidationReport` PASS. O repositório verifica novamente schema,
identidade, digest do candidato, regras e digest do relatório antes de alterar
o estado. Evidência ausente, de outra versão, stale ou adulterada é recusada.

Uma candidata quarentenada deve ser corrigida e testada novamente. O relatório
não concede capability nem instala dependências. A versão anterior permanece a
referência de rollback; a próxima etapa pode persistir os relatórios para
auditoria sem mudar esta boundary.
