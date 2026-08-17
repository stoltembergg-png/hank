# AB-001 — Matriz de camadas, ownership e lifecycle

| Camada | Responsabilidade | Owner único | Dependências permitidas | Processo/lifecycle | Entrada/saída | Estado de prova |
|---|---|---|---|---|---|---|
| `agent-core` | regras de domínio, ports e invariantes | Core maintainer | tipos/std, ports próprias | biblioteca; sem boot externo | commands/result/events versionados | `NO_PROOF` |
| `application-api` | casos de uso, autorização de entrada e envelopes | Application owner | `agent-core`, contratos | serviço; inicia/encerra por host | request → result/event | `NO_PROOF` |
| `agent-runtime` | execução, cancelamento, retry, lease e correlation | Runtime owner | application contracts, broker interfaces | run state machine; recovery explícito | run command → trace/result | `NO_PROOF` |
| `infrastructure` | storage, provider, tool e event adapters | Infrastructure owner | ports/core; SDKs concretos isolados | recursos externos com close/failure | adapter contract | `NO_PROOF` |
| `tauri-shell` | janela, bridge, eventos e packaging | Desktop owner | application-api; Tauri | processo desktop; sem regra de domínio | UI intent ↔ API envelope | `NO_PROOF` |
| `cli-adapter` | superfície não-Tauri para os mesmos use cases | CLI owner | application-api; terminal I/O | processo CLI; sem regra de domínio | CLI input ↔ API envelope | `NO_PROOF` |
| `fake-adapter` | fixture determinística para contract tests | Test owner | application-api; test-support | processo de teste; sem Tauri | fixture ↔ API envelope | `NO_PROOF` |

## Edges permitidas

- `tauri-shell → application-api`
- `cli-adapter → application-api`
- `fake-adapter → application-api`
- `application-api → agent-core`
- `agent-runtime → application-api`
- `infrastructure → ports/application contracts`

## Edges permitidas

A lista `allowed_edges` é obrigatória e tipa cada relação `from → to`; uma edge ausente nessa lista, um ID de layer duplicado ou um ciclo retorna `BLOCKED`. `allowed_dependencies` não pode substituir uma port concreta por texto livre; `ports/application contracts` permanece somente uma marcação de `NO_PROOF` até receber identidade de contrato.

## Edges proibidas

- `agent-core → tauri-shell`
- `agent-core → provider concreto/storage concreto`
- `tauri-shell → SQLite/filesystem/provider/tool`
- `cli-adapter → regra de domínio privada`
- `infrastructure concreta → UI`

Qualquer edge, owner duplicado ou lifecycle ausente resulta `BLOCKED`/`NO_PROOF`; este documento não transforma o estado em `PASS` sem validator executado.
