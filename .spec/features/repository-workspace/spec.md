# Spec: repository workspace manager

> feature: repository-workspace
> status: em-implementacao

## Contexto

PR-204 adiciona a boundary de ownership para workspaces de repositórios usados por agentes de desenvolvimento. O contrato fica no domínio puro (`agent-core`): ele recebe uma raiz já canonicalizada por um adapter de infraestrutura e não acessa filesystem, Git, shell, storage ou secrets. A canonicalização física, a leitura de status e a persistência serão adapters posteriores, mantendo AI-006, AI-012, AI-026 e AI-031.

## Histórias

### US-1301 — Workspace project-scoped e bounded

Como runtime de desenvolvimento, quero registrar um workspace com project/repository ownership e controlar leases exclusivos, para que agentes não editem caminhos errados nem concorram pelo mesmo workspace.

#### AC-1301 — Registro preserva ownership e raiz canonicalizada @spec:AC-1301

- **Dado** um `project_id`, `repository_id`, `workspace_id` e uma raiz canonicalizada absoluta, todos dentro dos limites de tamanho
- **Quando** registro o workspace no manager
- **Então** o registro é aceito e a consulta retorna exatamente o mesmo ownership e a raiz canonicalizada, sem acessar filesystem ou executar Git

#### AC-1302 — Raiz inválida ou traversal é rejeitado @spec:AC-1302

- **Dado** uma raiz vazia, relativa, com segmento `.`/`..`, controle ou acima do limite
- **Quando** tento registrar o workspace
- **Então** recebo `DomainError::Validation` antes de criar qualquer registro

#### AC-1303 — Lease concorrente falha de modo determinístico @spec:AC-1303

- **Dado** um workspace registrado sem lease
- **Quando** o primeiro holder adquire o lease e outro holder tenta adquirir o mesmo workspace
- **Então** a primeira aquisição retorna um token com epoch determinístico e a segunda falha com `DomainError::ConcurrencyConflict`, sem substituir o holder original

#### AC-1304 — Release exige token exato e permite reacquisition monotônica @spec:AC-1304

- **Dado** um workspace com lease ativo
- **Quando** libero com token errado ou tento reutilizar token já liberado
- **Então** a operação falha sem alterar o lease; depois do release correto, nova aquisição recebe epoch maior que o anterior

#### AC-1305 — Registro duplicado e cross-project falham sem mutação @spec:AC-1305

- **Dado** um workspace já registrado ou uma tentativa de reutilizar a mesma raiz em outro project
- **Quando** registro a segunda entrada
- **Então** recebo erro tipado (`Duplicate` ou `Validation`) e o registro original permanece intacto

## Fora de escopo

- Canonicalização física de symlinks ou acesso direto ao filesystem
- Execução de Git, status/diff, commit, worktrees ou branches
- Persistência SQLite, migrations, UI, PRs ou shell
- Checkout/clone, credentials, secrets ou alteração de arquivos
- Revalidação após restart e snapshot dirty/unsupported, que dependem de adapters/integração posteriores

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-1301 | O runtime fornecerá a raiz fisicamente canonicalizada antes de chamar o domínio. | confirmada | `agent-core` valida apenas forma, limites e segmentos; adapter externo fará `realpath`/equivalente. |
| ASM-1302 | Um workspace possui no máximo um lease ativo por vez. | confirmada | O manager rejeita qualquer segundo holder até o token atual ser liberado. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-625 | Qual adapter persistirá leases entre reinícios? | respondida | Card posterior de storage/runtime; este slice mantém lease bounded em memória e não afirma recuperação após restart. |
