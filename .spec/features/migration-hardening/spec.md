# Spec: migration hardening

> feature: migration-hardening
> status: em-implementacao

### US-1800 — Aplicar upgrades de schema com uma barreira recuperável

Como runtime, quero conhecer exatamente o conjunto de migrations embutidas e bloquear
upgrades sem preflight, backup verificável ou estado recuperável, para que um profile
não seja alterado por schema drift, downgrade ou execução concorrente.

#### AC-1801 — Manifesto ordenado e determinístico

- **Dado** o conjunto de migrations SQL embutidas no runtime
- **Quando** o manifesto for construído
- **Então** cada versão, descrição, checksum e flag transacional aparece em ordem
  crescente, e o digest do manifesto é estável para o mesmo conjunto.

#### AC-1802 — Preflight classifica clean, estável e upgrade

- **Dado** um profile vazio, atualizado ou com migrations pendentes
- **Quando** o preflight for executado com uma versão alvo explícita
- **Então** clean install e banco atualizado são aceitos, upgrade exige backup
  verificável da versão atual e nenhum SQL é executado pelo preflight.

#### AC-1803 — Drift, schema desconhecido e downgrade falham fechado

- **Dado** checksum alterado, versão aplicada ausente do manifesto, schema sem
  histórico SQLx ou alvo menor que o schema atual
- **Quando** o gate for avaliado
- **Então** retorna uma falha tipada antes de executar migration ou apagar dados.

#### AC-1804 — Execução registra estado e é idempotente

- **Dado** uma solicitação com operation ID bounded
- **Quando** a execução começa, termina ou falha e a solicitação é repetida
- **Então** estados `started`, `applied` e `failed` são persistidos sem segredos,
  e o retry aplicado não duplica migrations nem registros de execução.

#### AC-1805 — Upgrade usa transaction e forward-fix

- **Dado** um upgrade suportado com backup da versão corrente
- **Quando** a execução for autorizada pelo gate
- **Então** o runner transacional do SQLx é usado; falha deixa o último estado
  conhecido e a recuperação prevista é novo forward-fix ou restore do backup,
  nunca downgrade SQL implícito.

#### AC-1806 — Concorrência não duplica uma operação

- **Dado** duas tentativas concorrentes para o mesmo operation ID e profile
- **Quando** ambas alcançam o gate
- **Então** somente uma execução é registrada como aplicada e a outra retorna
  conflito ou o resultado idempotente, sem duplicar a linha de estado.

## Segurança

- O manifesto contém somente metadados de migration e digests; SQL não é exposto em
  logs, receipts ou estado operacional.
- Backup é aceito como prova somente com schema igual ao ponto de partida observado;
  drift, estado dirty e alvo incompatível falham antes de qualquer migration.
- Migrations seguem forward-only. O gate não chama `undo` nem executa SQL reverso.
- O estado operacional é bounded, parametrizado e não contém prompts, credentials ou
  valores de segredo.

## Suposições

- ASM-1807: migrations específicas de produto e migração de secrets permanecem nos
  cards PR-256+; esta entrega endurece o gate ao redor do runner existente.
- ASM-1808: crash/power-loss real e disk-full do SO continuam limites de infraestrutura
  e não são reivindicados pelos testes offline deste card.

## Perguntas em aberto

Nenhuma.
