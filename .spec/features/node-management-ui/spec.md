# Spec: node management UI

> feature: node-management-ui
> status: em-implementacao

### US-1452 — UI administrativa de nodes remotos autenticados

Como usuário autorizado, quero listar, inspecionar e revogar nodes remotos
autenticados sem nunca expor material de credencial, para que eu consiga auditar
o runtime remoto e interromper pareamento suspeito com confiança.

#### AC-1495 — Lista de nodes renderiza somente estado autenticado e saúde

- **Dado** resposta do bridge Tauri com lista de nodes autenticados
- **Quando** o componente `<NodeList>` renderiza a lista
- **Então** cada item exibe node_id, peer, project, capabilities bounded,
  `state` (active|expired|revoked|unknown) e `health` (healthy|stale|unreachable);
  `CredentialRef`, tokens e material bruto de protocolo nunca aparecem no DOM
  (texto, atributos ou data-attributes).

#### AC-1496 — Ação de revoke é confirmada e devolve estado terminal

- **Dado** um node com state `active`
- **Quando** o usuário clica em revoke e confirma o diálogo
- **Então** a bridge é chamada com o node_id e scope exato (project+actor);
  o item passa a `revoked` na próxima renderização e o botão revoke some;
  cancelar o diálogo não emite nenhuma chamada à bridge.

#### AC-1497 — Detalhe de node escapa texto hostil sem executar nada

- **Dado** um node cuja `node_id`, `peer` ou `display_name` contenha
  caracteres HTML/JS/scripts
- **Quando** o componente `<NodeDetail>` renderiza esses campos
- **Então** o texto é renderizado como texto puro (escape padrão React),
  nenhum `<script>`, atributo `onerror`/`onclick`, `javascript:` URI ou
  `data:text/html` é executado, e o teste verifica via `dangerouslySetInnerHTML`
  ausente.

#### AC-1498 — Resposta stale da bridge é fail-closed

- **Dado** resposta de `list_nodes` com campo `stale_since_ms` anterior
  ao wall clock atual mais `STALE_RESPONSE_THRESHOLD_MS`
- **Quando** o componente recebe a resposta
- **Então** o node é exibido com `health: stale`, a ação de revoke fica
  desabilitada e um banner de aviso informa que a ação pode ser imprecisa;
  o usuário pode reabrir a lista manualmente.

#### AC-1499 — Acessibilidade mínima da listagem e do detalhe

- **Dado** o componente `<NodeList>` e `<NodeDetail>` montados
- **Quando** o DOM é inspecionado
- **Então** o contêiner da lista tem `role="list"` e cada item `role="listitem"`;
  o botão revoke tem `aria-label="Revoke node {node_id}"`; o diálogo de
  confirmação tem `aria-modal="true"` e foco preso no botão confirmar;
  o componente responde a navegação por teclado (Tab/Shift+Tab/Enter/Esc).

## Segurança

- Nenhuma chamada à bridge revela o conteúdo de `CredentialRef` ou tokens; o
  componente consome somente o subconjunto de tipos de `NodeStatus` já
  redigido no backend.
- Cancelar o diálogo de revoke **não** dispara nenhuma chamada; apenas a
  confirmação explícita envia `revoke_node` para a bridge.
- Renderização de texto hostil é testada para impedir execução de HTML/JS
  embutido em `node_id`/`peer`/`display_name`.

## Suposições

- ASM-1462: a bridge Tauri expõe `list_nodes`, `get_node`, `revoke_node`
  e tipos `NodeStatus`/`NodeHealth` redigidos. Esses tipos são parte do
  contrato da PR-251 e a PR não inventa serialização crua.
- ASM-1463: a fonte de verdade (Tauri bridge) é responsável por aplicar
  autorização e rate limiting; a UI apenas reflete o que ela devolve.

## Perguntas em aberto

Nenhuma.
