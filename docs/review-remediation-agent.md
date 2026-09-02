# Review Remediation Agent

O workflow `Review remediation agent` transforma uma finding concreta do CodeRabbit ou
do Aikido em uma proposta limitada pelo Xiaomi MiMo v2.5. A viabilidade só é aceita
quando existe um unified diff aplicável, dentro dos limites de segurança e aprovado
pelo check determinístico de whitespace no SHA exato da PR de origem.

## Limites operacionais

- Modelo fixo: `mimo-v2.5`.
- Endpoint fixo: `https://api.xiaomimimo.com/v1`.
- Credencial única: `XIAOMI_MIMO_API_KEY`, configurada como secret de Actions.
- Apenas findings CodeRabbit/Aikido do mesmo repositório são considerados.
- PRs de fork, findings genéricas, SHA stale, duplicatas e ciclos esgotados terminam
  em `NOOP` ou `HUMAN_REQUIRED`.
- O patch é limitado a 10 arquivos, 500 linhas alteradas e 64 KiB; workflows,
  actions, políticas, secrets, credenciais, binários, symlinks e submodules são
  proibidos.

O agente não aprova, faz merge, rebase, force-push, publica release, altera proteção
de branch ou resolve a conversa do reviewer. A conversa original permanece disponível
para revisão humana.

## Fluxo

1. `collect` lê o evento e a API do GitHub, valida identidade, SHA, finding e
   fingerprint. Durante o primeiro rollout, enquanto o helper ainda não estiver na
   branch padrão, o job encerra como `NOOP` para não executar código recém-introduzido
   pela PR.
2. `propose` é o único job que recebe `XIAOMI_MIMO_API_KEY`; o modelo retorna um
   patch ou `NO_PATCH`.
3. `validate` aplica o patch em um target detached no SHA exato, sem credencial MiMo
   e sem permissão de escrita, e executa apenas `git diff --check`. Esse job nunca
   executa scripts de build, teste ou pacote controlados pela PR.
4. `publish` revalida digest/tree, identidade da PR e branch de origem, e cria somente
   uma draft PR para a branch de origem. Os checks obrigatórios dessa draft continuam
   sendo a autoridade final para código Rust, frontend, Tauri e E2E.

Os artefatos carregam apenas descriptors, digests, patch bounded e evidência redigida.
Reasoning do provedor, tokens e respostas HTTP brutas não são persistidos.

## Configuração, rotação e rollback

Antes de habilitar o workflow, revogue e faça a rotação de qualquer credencial que
tenha sido colada em chat, depois grave somente o novo valor em
`Settings → Secrets and variables → Actions → XIAOMI_MIMO_API_KEY`. O valor nunca
deve entrar no repositório, prompt, log, comentário, artefato ou linha de comando.

Para interromper a automação, remova o secret ou reverta/desabilite o workflow. Se uma
draft PR for indesejada, feche-a e remova somente a branch
`review-remediation/pr-<number>/...`; a PR de origem permanece inalterada. Em suspeita
de exposição, desabilite primeiro, revogue/rote o secret e investigue os logs e
artefatos retidos.

Os testes locais e de CI usam transporte falso e não exigem acesso ao provedor. O
Actionlint, Quality Integrity e os checks obrigatórios da draft PR devem passar antes
de qualquer decisão humana de merge.
