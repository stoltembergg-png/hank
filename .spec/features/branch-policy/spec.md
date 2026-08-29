# Spec: Branch policy

> feature: branch-policy
> status: em-implementacao

## Contexto

PR-206 fecha a boundary de segurança entre task/worktree e mutações de branch.
A policy é pura e determinística: recebe uma solicitação já identificada e
retorna uma decisão explícita. Ela não executa Git, consulta GitHub, acessa
filesystem, altera a policy ou recebe credentials.

## História

### US-1313 — Mutação de branch vinculada e controlada

Como runtime de desenvolvimento, quero permitir somente mutações de branch
compatíveis com projeto, repositório, task, owner, actor e revisão de policy,
para impedir branch arbitrária, push direto e bypass de revisão.

#### AC-1313 — Branch de task e actor autorizado @spec:AC-1313

- **Dado** uma policy válida e uma solicitação com projeto, repositório, task, owner e actor compatíveis
- **Quando** avalio `local_commit` ou `push` na branch derivada exatamente da task
- **Então** recebo decisão `Allowed` com a revisão da policy, sem executar Git ou alterar estado

#### AC-1314 — Protected branch e operações destrutivas falham fechado @spec:AC-1314

- **Dado** uma branch protegida, `force_push` ou `merge`
- **Quando** avalio a mutação
- **Então** recebo uma negação tipada e nenhuma operação é autorizada por fallback

#### AC-1315 — Escopo, ownership e revisão stale são obrigatórios @spec:AC-1315

- **Dado** projeto/repositório/task/actor incompatível, branch que não corresponde à task ou revisão diferente
- **Quando** avalio a mutação
- **Então** recebo negação determinística antes de qualquer efeito e a policy permanece imutável

## Fora de escopo

- Execução de Git, criação de branch, commit, push, force-push ou merge
- GitHub live ruleset enforcement, criação de PR e revisão remota
- Persistência, credentials, secrets, release signing e UI
- Task-to-branch mapping persistente; esse contrato pertence à PR-207

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-1316 | O chamador fornece identidade bounded e a revisão esperada da policy. | confirmada | O crate avalia somente dados; autenticação e carregamento autorizado ficam fora desta boundary. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-627 | A policy deve controlar GitHub live rulesets? | respondida | Não neste slice; enforcement remoto permanece fora do escopo. |
