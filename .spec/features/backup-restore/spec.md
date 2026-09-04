# Spec: backup restore

> feature: backup-restore
> status: em-implementacao

### US-1700 — Restaurar backup validado sem corromper o perfil ativo

Como runtime, quero restaurar um backup SQLite verificado em um perfil explicitamente
selecionado, para recuperar estado persistente sem promover dados parciais, incompatíveis
ou de outro perfil.

#### AC-1701 — Stage, migração e promoção são atômicos

- **Dado** um backup verificado e um destino `.db` explícito dentro da raiz de perfis
- **Quando** o restore for executado
- **Então** o banco é copiado para staging bounded, validado/migrado isoladamente e só
  depois promovido por rename; um destino existente é preservado durante a promoção.

#### AC-1702 — Preflight e compatibilidade falham fechado

- **Dado** digest, schema ou profile identity incompatível
- **Quando** o preflight ou o dry-run for executado
- **Então** o restore não toca o destino vivo, informa compatibilidade e rejeita schema
  incompatível antes do staging de promoção.

#### AC-1703 — Idempotência e autorização são vinculadas à intenção

- **Dado** `restore_id`, origem, destino, actor e confirmação explícitos
- **Quando** a mesma solicitação for repetida após uma promoção concluída
- **Então** o request digest e o receipt vinculam a intenção completa, e o retry retorna
  o resultado durável sem uma segunda promoção.

#### AC-1704 — Lock e caminhos inseguros não produzem efeitos

- **Dado** destino bloqueado, fora da raiz, symlink, inválido ou de outro profile
- **Quando** o restore for solicitado
- **Então** a operação falha fechado antes de alterar o perfil e não segue symlink nem
  atravessa o allowlist de caminho.

#### AC-1705 — Falha de limite limpa staging

- **Dado** um artefato maior que o limite ou uma falha durante a preparação
- **Quando** o restore falhar
- **Então** staging/receipts temporários são removidos, o destino anterior permanece
  utilizável e nenhum sucesso de restore é registrado.

## Segurança

- A origem é aceita somente após `DatabaseBackupService::verify`, incluindo digest,
  tamanho, identidade e `PRAGMA integrity_check`.
- O destino precisa ser um arquivo `.db` direto da raiz canônica configurada; symlinks,
  traversal, arquivos especiais e conflito origem/destino são rejeitados.
- O lock é exclusivo por destino. A autorização é opaca, bounded e vinculada ao digest
  do request; bytes de segredo não entram nesta API nem no receipt.
- Migrations, integridade e hash rodam no staging. A promoção usa nomes derivados do
  destino e receipt durável para retry idempotente.

## Suposições

- ASM-1706: o adapter de aplicação fornece autorização de operador e coordena a mesma
  fronteira de lock com escritores do perfil; esta fatia não implementa UI ou comando
  privilegiado.
- ASM-1707: migrations são a autoridade para upgrades suportados; downgrade não é
  inferido nem executado.
- ASM-1708: crash injection real, simulação de disk-full do SO e restore cross-user
  pertencem a testes/incrementos de integração posteriores; estes contratos não os
  reivindicam como executados.

## Perguntas em aberto

Nenhuma.
