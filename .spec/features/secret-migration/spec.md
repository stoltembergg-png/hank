# Spec: secret migration

> feature: secret-migration
> status: em-implementacao

### US-1900 — Migrar credenciais legadas sem expor material

Como operador de um perfil atualizado, quero migrar uma credencial legada para
um destino seguro verificável, para que a fonte antiga só seja revogada depois
de um cutover confirmado e um erro preserve uma recuperação segura.

#### AC-1901 — Detecção de fonte não revela material

- **Dado** um descritor de fonte legado com identidade de projeto, conta,
  referência opaca e tipo limitado
- **Quando** a migração inspeciona a fonte
- **Então** a inspeção retorna somente estado de disponibilidade e não lê,
  serializa ou registra o material do segredo

#### AC-1902 — Preflight vincula escopo e consentimento

- **Dado** uma requisição de migração com actor, política e destino
- **Quando** o projeto do contexto, da fonte e do destino diverge, a autorização
  expira ou a política não autoriza revogar a fonte
- **Então** a operação falha fechado antes de ler ou inspecionar a fonte

#### AC-1903 — Staging recebe apenas envelope cifrado bounded

- **Dado** material legado válido
- **Quando** a migração prepara o staging
- **Então** o codec recebe o material apenas em memória, o staging recebe um
  envelope cifrado bounded e o journal persiste somente um receipt opaco

#### AC-1904 — Destino é verificado antes do cutover

- **Dado** um envelope staged e um destino scoped
- **Quando** o destino recebe o material e a verificação opaca é executada
- **Então** a fonte legada não é revogada antes de a verificação retornar
  sucesso

#### AC-1905 — Falha preserva fonte e entra em quarentena

- **Dado** uma falha de leitura, codec, staging, destino ou verificação
- **Quando** o passo falha
- **Então** o journal registra `Quarantined`, mantém o receipt staged quando
  existir, não revoga a fonte e não inclui segredo no erro

#### AC-1906 — Retry e restart são recuperáveis e idempotentes

- **Dado** uma migração em quarentena com staging válido
- **Quando** um retry explicitamente autorizado retoma a operação
- **Então** ele usa o envelope existente sem reler a fonte, e uma migração
  aplicada repetida não executa um segundo cutover

#### AC-1907 — Revogação é a última etapa

- **Dado** um destino escrito e verificado
- **Quando** o cutover é concluído
- **Então** a fonte legada é revogada somente depois da verificação, e falha
  de revogação mantém o journal em quarentena para retry

#### AC-1908 — Journal e observabilidade são redigidos

- **Dado** qualquer estado da migração
- **Quando** o registro é persistido ou exibido em debug/erro
- **Então** são mantidos somente IDs, escopos, estados e classes de falha
  bounded; nenhum token, segredo, ciphertext ou caminho cru é exposto

## Segurança

- O coordinator é transport-neutral e recebe portas para fonte legada, codec de
  criptografia autenticada, staging cifrado, destino e journal durável.
- `SecretMaterial` existe somente em memória e continua sujeito à limpeza no
  drop; nenhum adapter desta feature pode gravá-lo em SQLite, `.env`, frontend,
  logs, traces, artifacts ou backup não criptografado.
- A fonte e o destino devem pertencer ao mesmo projeto do contexto e o actor
  deve coincidir com a autorização; qualquer divergência falha fechado.
- O core não fornece fallback plaintext nem implementa acesso direto a OS
  keychain/Stronghold. Esses adapters e a migração de dados reais ficam fora
  desta fatia.

## Suposições

- ASM-1901: implementações concretas de OS keychain/Stronghold, formatos legados
  de produção e persistência durável do journal serão conectadas por adapters
  posteriores; os contratos desta feature usam ports injetáveis e mocks.
- ASM-1902: `CredentialRef` e `CredentialAccount` continuam sendo as únicas
  identidades opacas de provider/account; o destino pode representar rename de
  provider desde que permaneça no mesmo projeto autorizado.

## Perguntas em aberto

Nenhuma.
