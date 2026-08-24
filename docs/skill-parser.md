# Skill parser contract

O parser de Skill transforma um pacote fornecido pelo host em dados tipados e
diagnósticos. Ele não lê o filesystem, acessa a rede, resolve referências,
altera runtime, importa uma Skill ou executa scripts.

## Grammar and input boundary

`SKILL.md` começa com um bloco JSON delimitado por linhas `---`. O objeto é o
`SkillManifest` completo, serializado com campos fechados; campos desconhecidos
como `instruction_source: system` são rejeitados. O restante do arquivo é
Markdown não confiável.

```text
---
{ ... SkillManifest JSON ... }
---
# Section
Untrusted instruction content.
```

O host fornece explicitamente `SkillParseRequest { document, files,
project_id }`. Arquivos além de `SKILL.md` precisam estar declarados pelo
manifesto e são retornados como `SkillArtifact` com role e conteúdo, sem
qualquer tentativa de execução.

## Bounded parsing

`SkillParserLimits` aplica limites determinísticos ao documento, frontmatter,
sections, headings, links, JSON nesting e artefatos por role. Os defaults são:

- documento: 256 KiB; frontmatter: 64 KiB;
- section: 64 KiB; 64 sections; 128 links;
- JSON depth: 16; heading depth: 6;
- script: 128 KiB; template: 128 KiB; reference/test: 256 KiB.

Markdown vazio, frontmatter malformado, nesting profundo, headings duplicados,
artefatos ausentes/duplicados/oversized e limites excedidos falham com um
diagnóstico tipado. O parser também rejeita caracteres de controle não
permitidos e não usa fallback permissivo para frontmatter inválido.

## Links and trust

Links relativos só podem apontar para paths declarados pelo manifesto. Paths
absolutos, traversal (`..`), separators alternativos, traversal percent-encoded
e schemes executáveis (`javascript:`, `data:`, `file:` e similares) são
rejeitados. Links `http(s)` são apenas referências externas e geram diagnóstico;
nenhum conteúdo remoto é buscado.

Sections contendo marcadores de tentativa de override da hierarquia de
instruções permanecem dados, mas recebem diagnóstico `InstructionOverride` e a
Skill inteira fica `quarantined`. O parser não promove texto a camadas `system`
ou `security`, e a provenance contém somente IDs, versão, origem, escopo,
budget/trace derivados do manifesto e contagens — nunca conteúdo bruto.

## API and non-goals

O contrato está em `agent_core::SkillParser`,
`agent_core::SkillParseRequest` e `agent_core::ParsedSkill`. O resultado separa:

- `manifest`: metadata declarativa já validada pelo contrato PR-136;
- `instructions`: sections Markdown não confiáveis;
- `artifacts`: scripts/templates/references/tests como dados;
- `links`, `diagnostics`, `quarantined` e provenance bounded.

Repository, version history, loader, activation, global import, evaluator,
code generation e execução continuam fora deste parser e pertencem aos
incrementos seguintes.
