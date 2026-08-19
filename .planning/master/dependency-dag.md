# Dependency DAG — PR-001..PR-270

**Status:** DAG de planejamento normalizado; não é prova de execução.  
**Fontes:** os três arquivos em `../queue/`, reconciliados com o source, o specification review e as architecture boundaries.

## Baseline executada

- M0 PR-001..PR-004: formalmente merged em `main` no SHA `34525d2396747cb45d9c5001efbdf8e30880eb00`.
- PR-002, PR-003 e PR-004 foram validadas na cadeia empilhada antes do merge do predecessor.
- Gates de qualidade previstos nos cards seguintes foram incorporados à baseline: fmt, Clippy, Rust tests/build, Frontend audit/lint/typecheck/test/build, Actionlint, CodeQL, Tauri e ONP.
- O próximo predecessor não implementado é PR-011; PR-005–PR-010 e PR-012 não devem ser reexecutadas como trabalho duplicado.
- Evidência final ONP: run `32209782480`, artifact identificado pelo SHA final, `audit --ci` PASS.


## 1. Método e resultado mecânico

O parser lê somente o campo `Dependências anteriores`, expande `PR-a..PR-b` inclusivamente, ignora a frase explícita `PR-218+ não é dependência`, verifica existência e deduplica arestas. A direção normativa abaixo é `predecessor → dependent`.

| Verificação | Resultado |
|---|---:|
| Cards analisados | 270 |
| IDs únicos/sequenciais | 270; PR-001–PR-270; PASS estrutural |
| Ranges | 95 + 77 + 98; sem lacuna/duplicata |
| Categorias válidas | 9 observadas; PASS |
| Referências backward expandidas | 1.256 |
| IDs inexistentes | 0 |
| Cross-range edges expandidas | 128 |
| Ciclos após normalização | 0; topological sort cobriu 270/270 |
| Referências numéricas para frente revisadas | 5, mais 1 menção explicitamente negativa |
| Nível máximo do caminho crítico | 91 arestas de dependência |

O resultado `PASS` acima significa apenas que a estrutura do grafo foi analisada; não significa que contratos, código, testes, CI, review, segurança ou artefatos existam.

## 2. Reconciliações fechadas

As filas permanecem intactas. O DAG usa estas decisões para não transformar texto ambíguo em ciclo ou predecessor falso:

| Texto na fila | Normalização do DAG | Estado |
|---|---|---|
| PR-105 menciona PR-110 “para timeout ... antes de release” | PR-110 é gate de release/saída de M5, não predecessor de PR-105; o card de HTTP não precisa violar a ordem de implementação. | Fechado como decisão de planejamento; correção textual da fila ainda `NO_PROOF`. |
| PR-150 diz que PR-218+ “não é dependência” | A referência é excluída; PR-150 depende apenas do bloco de skills. | Fechado. |
| PR-227 lista PR-228 | Rollback deve preceder rollout: aresta normativa PR-228 → PR-227. | Fechado; fila original permanece com blocker documental. |
| PR-235 lista PR-236 | Permissions precedem discovery: PR-236 → PR-235. | Fechado; fila original permanece com blocker documental. |
| PR-248/249 listam PR-250 | Credential isolation precede remote tool/project: PR-250 → PR-248/249; PR-250 também precede PR-251. | Fechado; fila original permanece com blocker documental. |

Nenhuma referência desconhecida ou ciclo foi inventado para “fazer caber” a fila. A regra de desbloqueio é fail-closed: se a correção não for registrada no card ou o predecessor não tiver evidência, o card continua bloqueado.

## 3. Caminho crítico

Um caminho máximo encontrado pela ordenação normalizada é:

```text
PR-001 → PR-002 → PR-003 → PR-004 → PR-008 → PR-011 → PR-013 → PR-014 → PR-015 → PR-016 → PR-017 → PR-018 → PR-019 → PR-020 →
PR-021 → PR-022 → PR-056 → PR-057 → PR-058 → PR-060 → PR-061 → PR-062 → PR-067 → PR-068 → PR-069 → PR-070 → PR-071 → PR-072 →
PR-073 → PR-076 → PR-077 → PR-078 → PR-079 → PR-081 → PR-082 → PR-083 → PR-084 → PR-085 → PR-086 → PR-087 → PR-088 → PR-089 →
PR-090 → PR-109 → PR-110 → PR-111 → PR-133 → PR-134 → PR-155 → PR-156 → PR-158 → PR-160 → PR-161 → PR-162 → PR-163 → PR-165 →
PR-166 → PR-167 → PR-169 → PR-170 → PR-171 → PR-172 → PR-173 → PR-174 → PR-175 → PR-176 → PR-177 → PR-181 → PR-182 → PR-186 →
PR-187 → PR-188 → PR-189 → PR-190 → PR-191 → PR-192 → PR-195 → PR-252 → PR-253 → PR-254 → PR-255 → PR-256 → PR-259 → PR-260 →
PR-261 → PR-262 → PR-265 → PR-266 → PR-267 → PR-268 → PR-269 → PR-270
```

Esse caminho é um instrumento de priorização, não uma promessa de que cada PR deve ser implementada sem replanejamento. Qualquer mudança em contrato, threat boundary, schema, migration, provider, permission, trace ou release reinicia a validação do sufixo afetado.

## 4. Ondas paralelas e gates

Uma onda é elegível somente depois dos predecessores diretos, do gate da onda e do review independente. Cards na mesma linha podem ser paralelos quando não compartilham schema/arquivo/ownership não isolado; a lista é uma decomposição operacional, não uma autorização para ignorar arestas do grafo.

| Onda | Gate de entrada | Lanes paralelas elegíveis |
|---|---|---|
| W0 | Nenhum; preflight de branch/worktree | PR-001 |
| W1 | Topologia do workspace | PR-002 e PR-003, respeitando PR-002→PR-003 quando o card exigir |
| W2 | Build/manifest base | CI, quality, security, docs e fixtures de PR-004–PR-020 em lanes locais conforme dependências |
| W3 | IDs/errors/events/storage contracts | Project schema/repository/services PR-021–PR-035; UI PR-036–PR-038 somente após Application API |
| W4 | Agent entity, policy, budget e isolation | Provider contracts PR-056–PR-060; Agent CRUD/UI PR-039–PR-055 só conforme seus predecessores |
| W5 | Provider-core + secrets/auth contracts | Adapter lanes PR-061–PR-066; registry/health/fallback/settings/discovery PR-067–PR-077 com seus gates |
| W6 | Session/message/context contracts | Storage/context/state/chat lanes PR-078–PR-095; streaming e UI não bypassam Application |
| W7 | Tool schema/registry/permission | Filesystem/process/terminal/HTTP/Git lanes PR-100–PR-108; rendering/timeout/approval PR-109–PR-111 |
| W8 | Python worker protocol e sandbox | Worker/transport/lifecycle/SDK lanes PR-112–PR-121; core deve continuar sem Python |
| W9 | Memory entity/repository/taxonomy | Extract/score/dedupe/retrieval lanes PR-125–PR-131; UI/edit/isolation/policy PR-132–PR-135 |
| W10 | Skill manifest/parser/loader/version | Project/global/binding/UI/editor/test/validation lanes PR-140–PR-148; evaluator/candidate/test/activation/rollback PR-149–PR-154 |
| W11 | Group/invocation graph | Membership/session/mention/protocol lanes PR-155–PR-161; cycle/depth/parallel/budget/policy/synthesis/UI PR-162–PR-172 |
| W12 | Durable workflow entity/node/edge/recovery | Node handlers PR-178–PR-185 podem ser lanes separadas; state/log/editor/viewer PR-186–PR-190 aguardam persistence/recovery |
| W13 | Scheduler entity/persistence/lease | Interval/cron/one-shot PR-192–PR-194; worker/policy/concurrency/integrations/history/notifications PR-196–PR-203 |
| W14 | Repository/worktree/branch/task policy | Agent profiles PR-208–PR-212 em paralelo; generation/review/CI/fix/release workflows PR-213–PR-217 em sequência de gates |
| W15 | Proposal/evaluation/regression/score | Proposals PR-221–PR-223; rollout PR-227 somente depois de rollback PR-228 e gates de avaliação |
| W16 | MCP transport/auth/permissions e plugin manifest | MCP clients PR-233–PR-237 e plugin discovery/lifecycle/permission PR-239–PR-241 conforme edges; provider/tool plugins PR-242–PR-243 por último |
| W17 | Remote transport/protocol/auth/event/credentials | PR-248, PR-249 e PR-251 podem ser lanes após PR-250 e seus próprios predecessores |
| W18 | Recovery/backup/restore/migration/secrets/limits/audit | PR-252–PR-259 por lanes, com PR-259 fechando auditoria das mudanças |
| W19 | Hardening e compatibility evidence | PR-260–PR-265 em lanes parcialmente paralelas; todas dependem de suas bases, não só do número da PR |
| W20 | Signing/install/update/rollback | PR-266 → PR-267 → PR-268 → PR-269; PR-270 é gate final e não pode promover artefato sem evidência completa |

Cross-range edges críticos incluem Project/Agent isolation → tools/memory/skills/groups, provider contracts → chat/memory, permission/confirmation → Python/MCP/remote, workflow recovery → scheduler, repository policy → development agents, rollback/evaluation → self-rollout e credential isolation → remote effects. Eles são bloqueadores de contrato, não apenas relações de arquivo.

## 5. Condição de desbloqueio universal

Antes de iniciar qualquer card, o agente deve provar: predecessor normalizado presente; base SHA/tree/status; scope e non-goals; dependências justificadas; testes planejados; security/migration/observability/docs impact; rollback; reviewer independente; e issue futura para qualquer item explicitamente fora do escopo. Após qualquer rebase, CI tardio ou resultado assíncrono, a identidade do SHA/tree/policy deve ser revalidada.

