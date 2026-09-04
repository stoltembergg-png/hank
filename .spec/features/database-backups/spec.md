# Spec: database backups

> feature: database-backups
> status: em-implementacao

### US-1600 — Backup SQLite verificável e isolado

Como runtime, quero gerar um snapshot consistente do perfil SQLite e um manifesto
redigido, para que o estado persistente possa ser validado sem expor segredos nem
aceitar artefatos incompletos.

#### AC-1601 — Snapshot online preserva um estado consistente

- **Dado** um perfil SQLite file-backed com migrations aplicadas e atividade de escrita
- **Quando** `DatabaseBackupService::create` gerar um backup
- **Então** o artefato deve ser criado por snapshot online, abrir como SQLite válido e
  conter apenas um estado committed do banco.

#### AC-1602 — Manifesto vincula identidade, tamanho e digest

- **Dado** um manifesto e seu banco associado
- **Quando** `DatabaseBackupService::verify` for executado
- **Então** a verificação deve conferir versão de formato, profile, schema, SHA-256,
  tamanho e `PRAGMA integrity_check`, rejeitando qualquer divergência.

#### AC-1603 — Destino e origem inválidos falham fechado

- **Dado** um caminho fora da raiz configurada, uma origem em memória ou uma tentativa
  de traversal/symlink
- **Quando** criação ou verificação for solicitada
- **Então** nenhum artefato aceito deve ser produzido ou lido fora da raiz canônica, e
  uma raiz configurada como symlink deve ser rejeitada.

#### AC-1604 — Publicação interrompida não vira backup aceito

- **Dado** uma falha durante snapshot, hash, escrita do manifesto ou limite de tamanho
- **Quando** a operação terminar
- **Então** temporários e pares incompletos devem ser removidos ou ignorados, e a
  retenção deve excluir somente pares verificados mais antigos.

#### AC-1605 — Proteção e metadados não vazam material sensível

- **Dado** uma referência opaca de proteção e metadados bounded
- **Quando** o manifesto ou a retenção forem persistidos
- **Então** nenhum token, prompt ou segredo deve ser serializado, e o manifesto deve
  registrar somente a referência de proteção aprovada e dados redigidos.

## Segurança

- O serviço usa `VACUUM INTO` para obter snapshot online e publica o manifesto por
  último, depois de sincronizar e calcular o digest do banco.
- O destino é derivado de um UUID dentro da raiz canônica; não existe destino arbitrário
  por chamada nem fallback para upload remoto.
- O contrato aceita apenas uma referência de proteção por política de sistema; bytes de
  segredo não entram na API. Restore, criptografia concreta e keychain pertencem às
  fronteiras posteriores.
- Retenção só remove pares com manifesto válido e nomes derivados do próprio backup.

## Suposições

- ASM-1600: credenciais ficam fora do SQLite e são mantidas pelo Secrets Broker; o
  manifesto registra somente um handle opaco.
- ASM-1601: a raiz configurada é criada e protegida pelo adapter de aplicação/OS; este
  card não altera permissões globais nem publica backups remotamente.
- ASM-1602: o backup é validado antes de ser consumido por restore, que pertence à PR-254.

## Perguntas em aberto

Nenhuma.
