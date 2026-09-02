# Review Remediation Agent

O workflow `Review remediation agent` transforma uma finding concreta do CodeRabbit ou
do Aikido em uma proposta limitada pelo Xiaomi MiMo v2.5. A viabilidade só é aceita
quando existe um unified diff aplicável, dentro dos limites de segurança e aprovado
pelos gates determinísticos no SHA exato da PR de origem.

## Limites operacionais

- Modelo fixo: `mimo-v2.5`.
- Endpoint fixo: `https://api.xiaomimimo.com/v1`.
- Credencial única: `HANK_REVIEW_REMEDIATION_MIMO_API_KEY`, configurada somente como
  secret do Environment protegido `XIAOMI_MIMO_API_KEY`.
- Apenas findings CodeRabbit/Aikido do mesmo repositório são considerados.
- PRs de fork, findings genéricas, SHA stale, duplicatas e ciclos esgotados terminam
  em `NOOP` ou `HUMAN_REQUIRED`.
- Marcadores de duplicidade e ciclo só são aceitos quando publicados por
  `github-actions[bot]`; comentários de usuários não podem bloquear a automação.
- O patch é limitado a 10 arquivos, 500 linhas alteradas e 64 KiB de diff; cada
  arquivo textual resultante também é limitado a 256 KiB. Workflows, actions,
  políticas, secrets, credenciais, binários, symlinks, submodules e manifests ou
  lockfiles de dependências são proibidos.

O agente não aprova, faz merge, rebase, force-push, publica release, altera proteção
de branch ou resolve a conversa do reviewer. A conversa original permanece disponível
para revisão humana.

## Fluxo

1. `collect` lê o evento e a API do GitHub, valida identidade, SHA, finding e
   fingerprint. Durante o primeiro rollout, enquanto o helper ainda não estiver na
   branch padrão, o job encerra como `NOOP` para não executar código recém-introduzido
   pela PR.
2. `propose` é o único job que recebe `HANK_REVIEW_REMEDIATION_MIMO_API_KEY`; a entrada externa é
   redigida na fronteira do provedor e qualquer material secreto não classificado
   encerra como `HUMAN_REQUIRED`. O modelo retorna um patch ou `NO_PATCH`.
3. `validate` aplica o patch em um target detached no SHA exato, sem credencial MiMo
   e sem permissão de escrita. O helper confiável executa o gate `semantic-syntax`
   apenas com parsers de sintaxe para arquivos JavaScript/Rust, além de
   `git diff --check`; para Rust, o `rustfmt --emit stdout` valida o parse sem exigir
   que o patch esteja formatado. Exclusões válidas são ignoradas pelo parser depois
   de aplicadas. Nenhum script de build, teste ou pacote controlado pela PR é executado.
4. `publish` revalida digest/tree, identidade da PR e branch de origem, e cria somente
  uma PR em rascunho para a branch de origem. Os cinco gates determinísticos
  (`source-head`, aplicabilidade, limites da árvore, whitespace e `semantic-syntax`)
  precisam estar em `PASS`; imediatamente antes da publicação, o agente verifica
  novamente o marcador e a branch determinística para evitar duplicatas. Se a
  branch já existir sem uma PR, uma reserva ainda apontando para o SHA de origem
  pode ser avançada por fast-forward por um publisher confiável; uma branch já
  concluída só é reutilizada após conferir pai e árvore exatos. A criação da PR é
  idempotente. Os checks obrigatórios desta PR em
  rascunho continuam sendo a autoridade final para código Rust, frontend, Tauri e E2E.

Os artefatos carregam apenas descriptors, digests, patch bounded e evidência redigida.
Reasoning do provedor, tokens e respostas HTTP brutas não são persistidos. As leituras
de JSON, patches e arquivos allowlisted usam caminhos relativos validados e descritores
de arquivo com `O_NOFOLLOW` quando o sistema oferece essa proteção.

Cada evento usa uma chave de concorrência não cancelável baseada no identificador
durável da review ou do check run. Eventos distintos não ocupam a mesma fila; uma
reentrega do mesmo evento é serializada. O coletor rejeita eventos sem o
identificador necessário, e o branch determinístico mantém a idempotência entre
eventos diferentes.
Antes do commit, o agente relê os marcadores do publisher confiável e a branch
determinística. A criação da branch determinística no GitHub é uma reivindicação
atômica. Uma branch existente é classificada pelo commit remoto: a reserva no SHA
de origem é avançada por fast-forward; uma remediação já publicada exige pai e
árvore exatos. Em ambos os casos, o push é sem force-push e uma corrida é
revalidada antes da recuperação da PR. A criação da PR valida que a branch aponta
para a base original e que a PR existente continua em rascunho; uma corrida de
criação é reconsultada antes de ser considerada concluída.
Uma branch criada por uma execução interrompida não bloqueia o finding: a coleta
mantém `READY`; a próxima execução identifica a reserva pelo ponteiro no SHA de
origem e a avança por fast-forward, ou valida pai e árvore se ela já contiver a
remediação.

## Configuração, rotação e rollback

Antes de habilitar o workflow, revogue e faça a rotação de qualquer credencial que
tenha sido colada em chat, depois grave somente o novo valor em
`Settings → Environments → XIAOMI_MIMO_API_KEY → Environment secrets → HANK_REVIEW_REMEDIATION_MIMO_API_KEY`.
O nome do secret deve ser `HANK_REVIEW_REMEDIATION_MIMO_API_KEY`; não crie uma cópia
com esse valor em `Settings → Secrets and variables → Actions`. Configure pelo menos
uma regra de proteção (por exemplo, required reviewer) no Environment. O job `propose`
é o único vinculado a esse Environment e falha fechado se o secret faltar. A proteção
é aplicada pela configuração do GitHub, não por uma chamada administrativa do job.
O valor nunca
deve entrar no repositório, prompt, log, comentário, artefato ou linha de comando.

Para interromper a automação, remova o secret ou reverta/desabilite o workflow. Se uma
PR em rascunho indesejada, feche-a e remova somente a branch
`review-remediation/pr-<number>/...`; a PR de origem permanece inalterada. Em suspeita
de exposição, desabilite primeiro, revogue/rote o secret e investigue os logs e
artefatos retidos.

Os testes locais e de CI usam transporte falso e não exigem acesso ao provedor. O
Actionlint, Quality Integrity e os checks obrigatórios da PR em rascunho devem passar antes
de qualquer decisão humana de merge.
