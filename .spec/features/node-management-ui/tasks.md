# Tasks: node management UI

> feature: node-management-ui

## T-1462 — UI administrativa de nodes remotos [pendente]

- Refs: US-1452, AC-1495, AC-1496, AC-1497, AC-1498, AC-1499
- Arquivos: frontend/src/api/node-management.ts, frontend/src/settings/nodes/NodeList.tsx, frontend/src/settings/nodes/NodeDetail.tsx, frontend/src/settings/nodes/NodeList.css, frontend/tests/node_management_contract.test.ts, frontend/tests/node_management_ui.test.tsx

## Suposições

- ASM-1462: a bridge Tauri expõe `list_nodes`, `get_node`, `revoke_node`
  e tipos `NodeStatus`/`NodeHealth` redigidos. Esses tipos são parte do
  contrato da PR-251 e a PR não inventa serialização crua.
- ASM-1463: a fonte de verdade (Tauri bridge) é responsável por aplicar
  autorização e rate limiting; a UI apenas reflete o que ela devolve.
