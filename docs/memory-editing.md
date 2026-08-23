# Edição explícita de memória

A edição de memória é uma operação editorial iniciada por uma pessoa na
superfície desktop. O frontend não acessa SQLite: ele lê memórias pelo bridge
Tauri e envia uma mutation tipada para o serviço de aplicação, sempre limitada
ao `project_id` selecionado.

## Fluxo editorial

1. O painel carrega as memórias do projeto por `list_memories`. A resposta é
   filtrada novamente no frontend para impedir que um registro de outro projeto
   seja exibido.
2. A pessoa edita conteúdo, resumo e importância, ou escolhe uma transição de
   lifecycle: aprovar, rejeitar, arquivar ou restaurar.
3. Cada escrita exige confirmação explícita no painel. Cancelar a confirmação
   encerra o fluxo sem chamar o bridge.
4. O painel envia `mutate_memory` com `project_id`, `memory_id`, `actor_id`,
   `trace_id`, `operation_id`, capability `memory.write`, `expected_version`,
   `confirmed: true` e a operação discriminada.
5. O bridge valida confirmação, capability e identificadores antes de delegar
   ao `MemoryMutationService`. O serviço valida contexto, política, escopo,
   bounds, transição e versão; o repository aplica a atualização parametrizada.
6. Uma resposta aceita atualiza a lista. Uma rejeição mantém o card local e
   exibe o motivo de conflito de versão ou a rejeição sem mutação.

## Lifecycle e rollback

| Estado | Ação editorial | Resultado |
| --- | --- | --- |
| `candidate` | Aprovar | `approved` |
| `candidate` | Rejeitar | `rejected` |
| `approved` | Arquivar | `archived` |
| `archived` | Restaurar | `approved` |

Toda mutation aceita incrementa `version`. O cliente envia a versão que leu e
uma versão stale falha sem sobrescrever o conteúdo, status ou provenance. A
identidade da operação é deduplicada por projeto durante a vida do serviço;
reenviar a mesma operação nessa janela não cria uma segunda transição. Esse
ledger de operações é em memória, portanto não substitui um registro
persistentemente idempotente após reinício do processo.

Neste slice, rollback significa principalmente reverter arquivamento com
`restore` e preservar o registro quando uma operação falha. Não existe uma
operação implícita de desfazer conteúdo nem uma transição `rejected`→`approved`:
para corrigir conteúdo, é necessário recarregar a memória e enviar uma nova
edição com a versão vigente.

## Policy, capability e auditoria

O único capability aceito pelo bridge é `memory.write`. O contexto obrigatório
carrega actor, projeto, trace, operação e versão esperada; contexto incompleto,
capability divergente, política negada, conteúdo fora dos limites ou escopo
estrangeiro falham fechados antes da persistência.

Na ponte Tauri atual, a decisão local de policy é representada por
`policy_allowed: true` depois das validações do bridge; a camada de serviço
continua rejeitando contextos que tragam uma decisão negada. Um evaluator de
policy externo ainda não faz parte desta superfície.

O bridge escreve somente metadados delimitados no log de tracing:
projeto, memória, actor, trace, operação e tipo da mutation. Conteúdo,
segredos e payload da edição não entram nesse evento. A UI também limita o
preview e mascara padrões comuns de credenciais antes de renderizar o texto.
Isso reduz exposição acidental, mas não transforma o conteúdo da memória em
segredo: a pessoa autorizada ainda pode vê-lo no formulário de edição.

O slice atual não possui um ledger de auditoria persistente. O log delimitado é
evidência operacional da solicitação; retenção, consulta e exportação de uma
trilha persistente de auditoria continuam sendo uma evolução posterior.

O comando de leitura usa `list_active`, por isso não retorna memórias
arquivadas. O serviço e o comando `restore` existem, mas a descoberta de uma
memória arquivada pelo painel requer uma futura consulta explícita de
arquivadas.

## Contratos verificados

Os contratos AC-773–AC-776 cobrem mutation válida com contexto e versão,
lifecycle explícito e reversível, falhas fail-closed por escopo/policy/
capability/bounds/concurrency e prevenção de replay. A cobertura está dividida
entre o serviço de aplicação, o bridge Tauri e o painel:

- `crates/agent-runtime/tests/memory_edit_service_contract.rs` cobre serviço,
  lifecycle, rejeições, conflito de versão e operação duplicada;
- `apps/desktop/src-tauri/tests/tauri_ac_tests.rs` cobre o registro dos
  comandos e a fronteira tipada sem SQL direto no shell;
- `frontend/tests/memory_panel_contract.test.tsx` cobre envelope confirmado,
  ausência de dispatch ao cancelar e conflito visível sem alterar o estado;
- a execução `onp-spec verify memory-core` registra a evidência PASS em
  `.spec/verification/memory-core.json`.
