# Hyperplan W0 — reconciliação do Round 3

**Estado:** revisão reconciliada; não é implementação e não altera o veredito de readiness.

**Batch de origem:** `deleg_983807e3` — cinco papéis, Round 3, resultados estruturados, sem uso de ferramentas pelos delegados.

**Reconciliado em:** `2026-08-17T15:55:11-03:00`.

## Limite de evidência capturada

- Repositório: `stoltembergg-png/hank`.
- Ref no momento da captura: `main`.
- Commit capturado: `dd88d2c140b3d85022b9272dd71740caf1ee6ed4`.
- Tree capturada: `0eb00af8b827cb41948f18989ce7544a99cd8d56`.
- `w0-contract-gate` no SHA capturado: `completed/success`.
- Branch protection reconsultada no momento da captura: `w0-contract-gate`, strict, `enforce_admins=true`, histórico linear, sem force-push e sem deleção.
- Suíte local na captura: `node --test` com 19/19 testes; architecture validator e queue validator passam.
- ONP na captura: 16/16 critérios provados; `onp-spec audit --ci` limpo.
- Não existe implementação Rust/Tauri de produto nem runner real de agentes; portanto comportamento local, enforcement externo e autoridade runtime continuam separados.

Este bloco é um snapshot histórico de evidência, não uma afirmação permanente sobre o topo de `main`. O merge que introduziu ou atualizou este relatório necessariamente cria uma identidade posterior; qualquer decisão atual deve reconsultar `main`, branch protection e checks no GitHub. O snapshot anterior (`bdd53d2`/`efc1242`) permanece histórico da PR #4.

## Disposições preservadas

| Origem | Disposição | Reconciliação | Ação/status |
|---|---|---|---|
| S1 | `REFINE` | Não remover scaffold, manifests, fixtures ou runner sem prova de redundância. | Preservar os artefatos W0; nenhuma remoção ampla. |
| S2 | `REFINE` | Manter defesa contra topologia futura e duplicação, sem criar camadas artificiais. | Só adicionar artefato quando houver gate e teste correspondente. |
| V1 | `REFINE` | Testes de adapter/edge sustentam apenas comportamento local; não provam execução real, enforcement ou authority. | W0 permanece `PARTIAL/NO_PROOF`; adapter executável é gate separado. |
| V2 | `DEFEND` | IDs duplicados, ciclos e edges não declaradas são invariantes locais do graph validator. | Corrigido e merged em `89dbabb`; coberto por testes negativos. |
| V3 | `DEFEND` | Um `PASS` precisa consumir cinco reports e validar identidade; `reason/evidence` isolados não bastam. | Corrigido e merged em `89dbabb`; reports/identity são exigidos. |
| V4 | `DEFEND` | Reviewer, path containment, escopo e identidade são pré-condições do execution gate. | Corrigido e merged em `89dbabb`; casos negativos passam. |
| V5 | `REFINE` | Leases, idempotência e fencing são gap de lifecycle/concurrency quando houver múltiplos atores; não são prova genérica de runtime ausente em todos os contratos W0. | Registrar como gate de GOV-003/runtime, sem alegar que foi implementado. |
| R1 | `DEFEND` | Não há autoridade para declarar W0 `RESOLVED`. | Mantido `PARTIAL/NO_PROOF`. |
| R2 | `DEFEND` | Testes locais não provam enforcement nem authority. | Mantida a separação behavior/enforcement/authority. |
| R3 | `REFINE` | A fila de 270 é estrutural, mas não se pode inferir escopo individual de cada card sem evidência. | Não reduzir a fila; manter 270 cards e registrar divergências. |
| R4 | `DEFEND` | M16 e PR-001 têm planejamento, não execução de produto. | Não iniciar PR-001 de produto. |
| R5 | `CONCEDE` | A contestação sobre ausência de reconsulta era verdadeira para o snapshot antigo, não para o estado atual. | Considerado superseded pela verificação live desta revisão. |
| A1 | `REFINE` | A arquitetura normativa descreve `frontend local`, mas o grafo AB-001 ainda não o modela. | Próximo contrato estreito: layer `frontend`, ownership e edges de bridge/Application; sem daemon/IPC. |
| A2 | `CONCEDE` | Documentação/schema não substitui adapter executável. | Exigir fake/CLI real quando a implementação de produto começar; não fabricar prova agora. |
| A3 | `REFINE` | A Application boundary deve ter port/DTO tipado; strings podem ser somente identidade serializada. | Próximo contrato: schema tipado de Command/Result/Event envelope. |
| A4 | `CONCEDE` | Markdown e labels não são contrato canônico suficiente para máquinas. | Próximo gate: snapshot/schema estruturado versionado e equivalência Markdown→canonical. |
| A5 | `CONCEDE` | Prompt e evaluator permissivo não são execution gate fail-closed. | Próximo gate: runner que valide identidade, evidência ausente/desconhecida/duplicada e deny-before-write. |
| C1 | `REFINE` | Boundary-first e minimalismo permanecem; não remover topologia necessária. | Preservar fronteiras e manifests existentes. |
| C2 | `REFINE` | Parser-first, policy-as-data e validator único permanecem; runner/prompt devem ser adapters finos. | Evitar duplicação de política entre prose, prompt e runner. |
| C3 | `CONCEDE` | Clean-room que apaga topologia ou não integra reports não é evidência. | Extensão mínima nas fronteiras atuais; não reconstruir scaffold. |
| C4 | `DEFEND` | Estados `PASS`, `BLOCKED` e `NO_PROOF` com identity explícita são necessários. | Manter gate e exigir edge/evidence para cada transição. |
| C5 | `DEFEND` | Daemon/IPC continuam fora do escopo W0 atual. | Não iniciar implementação de daemon/IPC nesta wave. |

## Round 2 cross-attack tardio

**Batch:** `deleg_7056c666`.

O lote chegou depois da reconciliação do Round 3. Quatro papéis produziram ataques estruturados; o papel `researcher` foi interrompido após 10 chamadas e não entregou o contrato final. Portanto esta rodada tem **roster degradado** e não é tratada como Hyperplan completo nem como nova autoridade factual.

### Sobreviventes incorporados

- **S1/S2:** preservar scaffold, manifests, fixtures e uma topologia declarativa mínima; não remover artefatos sem prova de redundância e não criar executor futuro.
- **V2/V3/V4:** os ataques `STANDS` confirmam que conectividade/integridade do grafo, reports integrados e deny-before-write são barreiras mínimas; as correções da PR #2 permanecem válidas.
- **V1:** refinar o risco de oracle. Testes locais continuam úteis, mas as expectativas devem vir de fixtures/manifests independentes; isso não transforma adapter real em requisito já executado.
- **V5:** restringir leases/heartbeat/fencing a recursos realmente concorrentes; estados one-shot continuam precisando de terminalidade e timeout, sem daemon obrigatório.
- **R1/R2:** manter `PARTIAL/NO_PROOF` e separar intenção documental, comportamento, enforcement e autoridade.
- **R3:** a queue não deve virar segundo runtime; o próximo gate deve ligar item, owner, artefato e teste sem reduzir os 270 cards.
- **R4:** distinguir ausência de implementação de ausência de prova; planejamento não é progresso operacional, mas também não prova que o produto esteja ausente em qualquer árvore futura.
- **R5:** autoridade GitHub só vale com consulta remota atual; o snapshot atual já foi reconsultado e está registrado no limite de evidência acima.
- **A1:** modelar uma matriz de rotas/owners para frontend, classificando rota requerida, adapter ou fora de escopo; não inventar UI para preencher o grafo.
- **A2/A5:** documentação e prompt não são adapter/runner; exigir vínculo a implementação ou declarar contrato-only e resultado observável.
- **A3:** strings podem ser IDs/aliases intencionais, mas resolução deve ser centralizada, validada e fail-closed em rename/missing dependency.
- **A4:** normalizar por vocabulário canônico e aliases explícitos, validando colisões semânticas; não reduzir a dívida por contagem.
- **C1/C3/C5:** manter boundary-first, clean-room mínimo e daemon/IPC fora do W0 atual.
- **C2/C4:** parser não substitui semântica; o enum de estados deve distinguir `PASS`, `FAIL`, `BLOCKED`, `NO_PROOF` e não promover ausência de execução a sucesso.

### Impacto na decisão

A rodada tardia **não altera** o veredito `W0 = PARTIAL/NO_PROOF`, não reabre V2/V3/V4 e não autoriza PR-001 de produto. Ela apenas estreita os próximos gates:

1. W0-R1 deve ser uma matriz de rotas/ownership e boundary tipada, não implementação de frontend.
2. W0-R2 deve validar vocabulário/aliases e colisões semânticas do snapshot canônico, não só contagem de labels.
3. W0-R3 deve comparar manifest/fixture independente com o resultado do runner, sem duplicar a policy no prompt.
4. W0-R4 deve aplicar leases/fencing somente a concorrência real e manter estados de não execução não-promovíveis.

Não foram adotadas novas alegações externas a partir do papel `researcher` interrompido.

## Round 2 researcher retry

**Batch:** `deleg_2123e610`.

O retry do papel `researcher` entregou os quatro objetos exigidos. Todos os ataques foram `ATTACK` e marcaram `no evidence found` porque o bundle Round 1 continha apenas resumos e nomes de fontes, não os artefatos/código/resultados necessários para verificar os claims.

- **SKEPTIC:** S1/S2 não podem ser usados para afirmar que requisitos W0 ou topologia futura são comprovadamente redundantes.
- **VALIDATOR:** V1–V5 não podem ser usados como prova primária de comportamento específico sem código, execução ou casos verificáveis no bundle.
- **ARCHITECT:** A1–A5 não podem ser tratados como lacunas confirmadas somente pelo resumo; exigem leitura direta dos grafos, rotas, schemas e runner.
- **CREATIVE:** C1–C5 são propostas de desenho, não evidência de cobertura dos contratos W0.

### Impacto

O retry **não altera** o veredito nem reabre as correções V2/V3/V4. Ele reforça a regra de proveniência: claims dos delegados são observações fornecidas, enquanto as decisões atuais devem depender dos artefatos do worktree, testes executados e consultas live já registradas separadamente. Nenhuma nova alegação factual foi promovida a partir do bundle sem evidência.

## Planner final — resultado inválido

**Dispatch:** `deleg_161ad91c`.

O planner final recebeu o bundle de W0, mas foi interrompido enquanto aguardava resposta do modelo (`status=interrupted`, oito chamadas de ferramenta). Não retornou o objeto estruturado exigido pelo schema, tarefas ordenadas, dependências, critérios, comandos ou gates verificáveis.

O bundle parcial contém decisões e riscos úteis, mas não é um plano executável e foi produzido contra uma identidade anterior do repositório. Portanto:

- o resultado não é apresentado como plano Hyperplan;
- nenhum comando, SHA, autoridade GitHub ou merge é inferido dele;
- `NO_PLAN` permanece o estado do planner final;
- qualquer novo planner deverá receber o estado atual e uma nova revisão de proveniência, não este bundle stale;
- o veredito W0 e os gates já reconciliados permanecem inalterados.

## Round 1 tardio — reconciliação contra o estado atual

**Batch:** `deleg_c5df3eea`.

O lote Round 1 foi concluído por cinco papéis, mas foi produzido antes das correções e merges posteriores. Seus claims foram tratados como input de revisão, não como snapshot atual; somente os pontos revalidados contra os artefatos e checks atuais foram preservados.

### Disposições

- **SKEPTIC S1–S6 — rejeitados como remoção ampla, refinados como controle de escopo:** não remover ONP/spec/tasks, prova, fixtures, schemas, validator, execution contract ou manifests sem inventário. PRs de produto, shell/frontend, CI e runtime continuam fora do fechamento W0, mas permanecem na roadmap; leases duráveis pertencem a W2/downstream.
- **VALIDATOR V1 — residual:** fixtures/declarations não provam adapter fake/CLI/Tauri executável; manter `NO_PROOF` sem fabricar execução.
- **VALIDATOR V2 — parcialmente corrigido:** IDs duplicados, ciclos e edges fora de `allowed_edges` agora falham; continua aberto o vínculo com política independente, owners de comandos e Cargo graph real.
- **VALIDATOR V3 — parcialmente corrigido:** o gate agora exige cinco reports e identidade; continua aberto o vínculo do gate com queue/DAG, `gate_of` e predecessor de PR-001.
- **VALIDATOR V4 — parcialmente corrigido:** reviewer ausente, branch literal proibida, identity e paths inseguros têm cobertura; continuam abertos refs equivalentes de branch, comandos autorizados, canonicalização Windows/UNC/junction e scanner de env/log/artifact.
- **VALIDATOR V5/V6 — residual:** leases/idempotência/fencing e tentativa/retry/cancel/rebase/CI tardio exigem state machine downstream; não são implementados no W0 atual.
- **RESEARCHER R1–R6 — mantidos como limite de prova:** W0 não é `RESOLVED`; comportamento local não prova enforcement/authority; queue/M16/PR-001 são planning até gate executado. Nenhuma autoridade externa antiga foi reutilizada sem consulta live.
- **ARCHITECT A1 — refinado:** frontend deve ser uma matriz de rotas/owners, com apresentação → bridge/Application, sem inventar UI nem segunda autoridade.
- **ARCHITECT A2 — refinado:** a boundary `infrastructure` pode permanecer um package, mas provider/storage/tool/event devem ter adapters leaf e owners semânticos únicos; não criar novos processos nesta wave.
- **ARCHITECT A3 — refinado:** lifecycle/compatibilidade deve ter contrato estruturado mínimo (`start/ready/stop/fail`, generation, cancellation, terminal outcome e schema compatibility), sem prometer runtime real.
- **ARCHITECT A4–A7 — preservados como W0-R2/W0-R3:** snapshot canônico da queue com aliases/colisões explícitos; runner como autoridade deny-before-write; reviewer/evidence binding; política de grafo independente dos dados do grafo.
- **CREATIVE C1–C6 — alinhados:** manter boundary-first, parser/policy como adapters finos, clean-room mínimo, estados fechados e daemon/IPC fora do W0; nenhum scaffold necessário deve ser removido por estética.

### Refinamento dos próximos gates

1. **W0-R1:** acrescentar política independente do grafo, matriz frontend/route, owners leaf de infrastructure e DTO/lifecycle tipado.
2. **W0-R2:** validar headings, labels, aliases e colisões semânticas; ligar snapshot canônico ao índice/DAG e distinguir `depends_on` de `gate_of` para bloquear PR-001 corretamente.
3. **W0-R3:** canonicalizar refs/paths Windows, validar comandos e efeitos, exigir denylist independente, secret scan e identidade autenticada; prompt permanece apenas apresentação do manifest.
4. **W0-R4:** modelar `run_id/attempt_id`, terminal winner, invalidation e precedência cancel×completion; leases só entram quando houver concorrência real.

A rodada não autoriza apagar os artefatos ONP ou iniciar produto. O veredito continua `W0 = PARTIAL/NO_PROOF`.

## Tentativa Round 3 duplicada — resultado inválido

**Batch:** `deleg_d76b0535`.

Os cinco papéis foram interrompidos antes de retornar os objetos estruturados exigidos. Apenas um fragmento parcial de S1 apareceu; não há disposições/evidence/surviving_fragment completos para V1–V5, R1–R5, A1–A5 ou C1–C5.

A tentativa não substitui a rodada Round 3 completa já reconciliada em `deleg_983807e3`. O fragmento de S1 não foi adotado, nenhuma nova evidência foi promovida e o veredito W0 permanece inalterado.

## O que foi realmente fechado

1. O graph validator agora rejeita IDs duplicados, ciclos e edges não declaradas.
2. O execution gate agora rejeita reviewer ausente, identidade SHA/tree incompleta, paths absolutos/traversal e scope inseguro.
3. O W0 gate agora exige os cinco reports com SHA/tree/policy/schema compatíveis.
4. Os três pontos acima têm testes negativos executados localmente e foram executados no workflow protegido do GitHub.

Esses itens fecham falsos verdes do validator; não fecham a ausência de runtime de agente, adapter real, secret scanner, leases, fencing, clean-room autenticado ou reviewer independente externo.

## Próximos gates ordenados

### W0-R1 — Frontend e boundary tipada

- Atualizar `AB-001` e `architecture-graph` para modelar `frontend` como camada de apresentação, owner, lifecycle e dependências permitidas.
- Declarar a ponte `frontend → tauri-shell` e o fluxo tipado para Application sem transformar UI em owner de domínio/storage.
- Adicionar schema/testes negativos para frontend bypassando shell/Application ou acessando infraestrutura.
- Não adicionar daemon, IPC, runtime de produto ou dependência Tauri ao core.

### W0-R2 — Snapshot canônico da queue

- Definir representação estruturada versionada dos 270 cards.
- Validar equivalência entre headings/campos Markdown e o snapshot canônico.
- Rejeitar labels desconhecidos, campos `Arquivos prováveis` não normalizados e divergências de dependência; nunca normalizar silenciosamente.
- Manter a cardinalidade 270 e o índice M16 `PR-252..PR-270`.

### W0-R3 — Runner fail-closed

- Implementar o preflight/runner que consome `PR-EXECUTION-CONTRACT.schema.json` e `evidence-manifest.schema.json`.
- Negar antes de qualquer write em branch/worktree/path/comando/identity inválidos.
- Validar reports ausentes, desconhecidos, duplicados ou stale; capturar digests e identidade do run.
- Adicionar fixtures adversariais para env/log/artifact secret e clean-room; sem valores reais.

### W0-R4 — Lifecycle e concorrência

- Definir lease, idempotency key, fencing, retry, timeout, cancelamento, crash recovery e quarantine.
- Executar fault matrix com duplicate dispatch e stale completion.
- Permitir concorrência somente após a evidência de autoridade e isolamento correspondente.

### Pós-W0 — Adapter real e produto

A prova de que fake/CLI/Tauri exercitam o mesmo caso de uso pertence à implementação do core Rust/Tauri e seus adapters. Não deve ser simulada por fixtures documentais nem antecipada como `PASS` no W0 contratual.

## Veredito

`W0 = PARTIAL/NO_PROOF`.

Os findings V2/V3/V4 foram corrigidos como comportamento local verificável. Os findings A1/A3/A4/A5 continuam decisões/gates de contrato; A2 e a execução real permanecem dependentes da implementação do produto. Nenhuma evidência desta reconciliação autoriza iniciar PR-001 de produto ou declarar os cinco blockers resolvidos.
