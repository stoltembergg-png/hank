# Spec: remote project support boundary

> feature: remote-project-support
> status: implementada

## História de usuário

### US-3002 — Vincular um projeto remoto ao node autorizado

Como runtime remoto, quero vincular um `ProjectId` a um node/peer autorizado e versionado,
para que referências a workflows, sessões e artefatos não cruzem projetos, não escapem da
raiz de armazenamento e não exfiltrar credenciais.

#### AC-3015 — Descriptor vincula projeto ao node e peer exatos

- **Dado** um descriptor de projeto com node e peer declarados
- **Quando** o registry faz o bind com o mesmo node/peer
- **Então** o binding resolve o projeto a partir desse node e preserva a versão.

#### AC-3016 — Projeto ou node divergente é rejeitado fail-closed

- **Dado** um bind para node divergente ou uma resolução de projeto nunca vinculado
- **Quando** o registry valida
- **Então** retorna erro de node divergente ou projeto não vinculado, sem efeito.

#### AC-3017 — Versão stale ou mais antiga conflita e exige reconciliação

- **Dado** um binding já existente em uma versão
- **Quando** um descriptor com versão mais antiga tenta vincular
- **Então** falha como conflito de versão; somente `reconcile` para versão estritamente
  mais nova avança o binding.

#### AC-3018 — Entrada acima do limite ou capability desconhecida é rejeitada

- **Dado** um descriptor com capability acima do tamanho máximo
- **Quando** é construído
- **Então** retorna erro de capability inválida; um escopo vazio é explícito e válido.

#### AC-3019 — Referências tipadas não carregam caminhos de arquivo crus

- **Dado** workflows, sessões e artefatos como IDs tipados
- **Quando** atravessam o boundary do descriptor
- **Então** permanecem identificadores tipados; nenhum caminho/string de filesystem cru é aceito.

#### AC-3020 — Escopo de capability é explícito e negado por padrão

- **Dado** um descriptor com um conjunto bounded de capabilities
- **Quando** o binding é consultado
- **Então** apenas capabilities declaradas são expostas; não declaradas permanecem ausentes.

#### AC-3021 — Binding sobrevive a restart via reconstrução do ledger

- **Dado** um registry reconstruído a partir do mesmo descriptor versionado
- **Quando** o projeto é resolvido no node autorizado
- **Então** a versão e o binding são preservados, sem novo transporte ou resolução de credencial.

#### AC-3022 — Rebind da mesma versão é idempotente

- **Dado** um descriptor já vinculado
- **Quando** o mesmo descriptor/versão é vinculado de novo
- **Então** não há conflito; a resolução mantém a versão original.

## Segurança e fronteira de prova

- O module é transport-neutral e não toca storage root, filesystem, rede, shell, provider
  ou credential store.
- O DTO carrega apenas IDs tipados e um escopo bounded de capabilities; nenhum caminho de
  filesystem, `CredentialRef`, token, password ou secret entra no payload aceito.
- Acesso cross-project, node divergente e versão stale falham fechado; a reconciliação
  avança apenas para versão estritamente mais nova após validação exata de node/peer.
- Esta feature prova somente o contrato offline com fixtures determinísticos. Não prova
  sync arbritrário, multi-master, UI de projeto ou integração operacional; a classificação
  de produção permanece `NO_PROOF`.

## Suposições

- ASM-2106: sincronização e replicação multi-master de projeto remoto são trabalho
  posterior; este card fecha apenas o binding fail-closed e a semântica de versão.

## Perguntas em aberto

Nenhuma.