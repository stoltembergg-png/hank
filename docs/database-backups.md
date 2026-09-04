# Database backups

`agent_runtime::backup::DatabaseBackupService` cria snapshots bounded de um perfil
SQLite file-backed usando `VACUUM INTO`. A operação lê um estado consistente do banco
sem pausar o runtime para copiar páginas manualmente.

## Artifact format

Cada backup publicado dentro da raiz configurada contém dois arquivos derivados do
mesmo `backup_id`:

- `backup-<uuid>.db`: snapshot SQLite;
- `backup-<uuid>.manifest.json`: formato, profile, schema, versão do app, SHA/tree,
  policy, tamanho, SHA-256, timestamp e referência opaca de proteção.

O banco temporário é sincronizado e hasheado antes de ser renomeado. O manifesto é
escrito e sincronizado antes de sua publicação final. Um par só é aceito quando o
manifesto aponta para o nome derivado do mesmo ID, o tamanho e o digest conferem e
`PRAGMA integrity_check` retorna `ok`.

## Protection and privacy

O contrato não recebe nem serializa bytes de segredo, tokens, prompts ou conteúdo de
provider. `BackupProtection::OsPolicy` contém apenas um handle opaco para a política
de proteção do sistema operacional. A raiz e suas permissões são responsabilidade do
adapter de aplicação/OS; este card não afirma criptografia concreta.

## Retention and operations

`enforce_retention` mantém os backups verificados mais recentes até os limites de
quantidade e bytes. Manifestos inválidos, temporários e symlinks são ignorados e nunca
são apagados pela retenção. O serviço falha fechado para banco em memória, origem
ausente, destino fora da raiz, artefato grande ou digest divergente.

Restore, upload remoto, rotação de chaves, permissões OS e criptografia concreta são
non-goals desta entrega e pertencem às etapas posteriores.
