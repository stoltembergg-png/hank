# Skill editor

O editor de Skills é uma superfície de revisão de conteúdo de projeto. Ele
carrega metadados bounded pela ponte Tauri e mantém manifest JSON, Markdown e
referências apenas no estado da tela até uma confirmação explícita.

## Fluxo governado

1. O operador seleciona uma Skill de projeto e escolhe **Editar rascunho**.
2. O editor carrega o documento pela API tipada, sem acesso a SQLite ou
   filesystem no frontend.
3. **Validar rascunho** envia o mesmo documento e arquivos declarados ao
   `SkillParser`. Diagnósticos de parser e tentativas de override são exibidos
   como texto bounded; conteúdo em quarentena não pode ser salvo.
4. **Salvar rascunho** exige confirmação do operador e envia actor, capability,
   policy, budget, trace e revisão esperada. O runtime cria uma versão Draft
   imutável, deduplica conteúdo idêntico e não move a cabeça ativa.
5. **Descartar edição local** limpa o estado da tela; o descarte persistido é
   uma ação separada e confirmada que somente arquiva um Draft elegível.

Trocar o projeto ou a Skill descarta o estado não salvo. Autosave e
`localStorage`/`sessionStorage` estão desativados. O editor não executa
scripts, não instala dependências e não fornece ação de ativação.

## Bridge

Os comandos bounded são `get_skill_editor`, `validate_skill_draft`,
`save_skill_draft` e `discard_skill_draft`. O bridge rejeita capability fora de
`skill.edit`/`skill.discard`, confirmação ausente, identidade inválida,
travessia de caminho, payload acima dos limites e respostas de outro projeto.
O runtime continua sendo a autoridade para parser, policy, budget, revisão,
deduplicação e persistência.

## Limites e privacidade

- documento do editor: 64 KiB;
- referências: até 32 arquivos, 16 KiB por arquivo;
- caminhos relativos bounded, sem `..` ou caminho absoluto;
- diagnósticos e erros não retornam o conteúdo bruto do documento;
- eventos de versão carregam hash, versão, policy, budget, trace e revisão, não
  o texto das instruções ou artefatos.

## ONP mapping

- AC-787: `frontend/tests/skill_editor_contract.test.tsx`, bridge Tauri e
  `get_skill_editor`.
- AC-788: parser/service tests e validação/quarentena do editor.
- AC-789: `saving_draft_parses_and_keeps_active_head_unchanged` e teste de
  confirmação frontend.
- AC-790: deduplicação/descarte no runtime e isolamento de projeto no editor.
