# Database restore

`agent_runtime::restore::DatabaseRestoreService` restaura um backup SQLite já verificável
em um destino explícito. A fronteira prepara e valida uma cópia isolada antes de qualquer
mudança no perfil ativo.

## Procedimento

1. O adapter autorizado escolhe `restore_id`, backup, `target_profile_id`, arquivo `.db`
   direto da raiz de perfis e `target_schema_version`.
2. O adapter calcula o request digest com a intenção completa e fornece a autorização
   opaca do operador. O digest vincula actor, confirmação, modo dry-run, origem, destino
   e schema; ele não substitui autenticação.
3. Execute primeiro com `dry_run: true`. O preflight verifica manifesto, digest, tamanho,
   integridade, backup/profile identity e a matriz de schema sem escrever o banco alvo.
4. Com preflight compatível e confirmação explícita, o serviço adquire o lock derivado do
   destino, copia o banco para staging bounded e roda migrations, schema check, integrity
   check e hash somente no staging.
5. O destino existente e seus sidecars `-wal`/`-shm` são renomeados para os nomes
   temporários de previous; o stage completo é renomeado para o destino; sidecars de
   stage são removidos antes de publicar o receipt. Se uma etapa de promoção falhar, o
   serviço tenta recolocar o trio previous e remove os temporários seguros.
6. Um retry com o mesmo request e receipt íntegro repete o cleanup pendente e só então
   retorna `AlreadyApplied`; divergências de receipt, origem ou hash falham como conflito
   e exigem investigação do operador.

## Segurança e limites

- A origem passa por `DatabaseBackupService::verify`; artefatos com digest, tamanho,
  schema, profile ou integridade divergentes não chegam à promoção.
- O target deve ser um `.db` direto dentro de `RestorePolicy::target_root`. Caminhos fora
  da raiz, traversal, symlinks, arquivos especiais e origem igual ao destino são rejeitados.
- O lock impede dois restores concorrentes do mesmo destino. O adapter deve coordenar esse
  marcador com escritores do perfil antes de expor a operação; este módulo não afirma que
  todos os writers de produção já o utilizam.
- Nenhum segredo, token, prompt ou ciphertext é aceito ou serializado pelo contrato. A
  proteção concreta e a autorização pertencem às fronteiras de Secrets Broker/Application API.
- O limite de bytes cobre cópia e hash. Falhas deixam o destino anterior intacto quando
  possível; cleanup de stage/previous falho retorna erro explícito e pode ser repetido com
  o mesmo request, sem declarar `Applied` novamente.

## Compatibilidade e last-known-good

Backups com schema menor podem ser atualizados pelas migrations no stage até o schema
atual do runtime. Como esta camada usa o runner completo de migrations, o target explícito
precisa ser exatamente `current_schema_version`; um target antigo, um backup mais novo que
o target ou um target acima do runtime são incompatíveis e não são promovidos. Não há
downgrade indiscriminado.

O arquivo anterior só é descartado depois da publicação do receipt. Se o cleanup posterior
falhar, o receipt e o novo target continuam identificáveis, mas a chamada retorna erro; a
retentativa com o mesmo request pode concluir a limpeza antes de devolver `AlreadyApplied`.
Em uma falha de promoção, o operador deve preservar e investigar os artefatos de recuperação
conforme a política da aplicação; esta entrega não executa restore automático após qualquer
erro nem afirma um drill real de crash/power-loss.

## Operação posterior

A integração de comando/UI, a autorização concreta, a coordenação de writers, testes de
kill por fase e a simulação de disk-full do sistema operacional são responsabilidades de
incrementos de integração posteriores. Esta entrega prova o contrato bounded no runtime e
não declara restore produtivo executado.
