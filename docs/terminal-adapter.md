# Terminal adapter

`TerminalAdapter` não executa processos por conta própria. Ele recebe `TerminalRequest`, valida operation key e `max_rounds` (1–8), e delega integralmente ao `run_process`.

A operação é deduplicada por key; falha do primitive remove a key para permitir retry controlado. O resultado identifica `round: 1` e preserva `ProcessResult`. PTY, shell livre, terminal persistente, sudo e instalação de pacotes permanecem fora do contrato.
