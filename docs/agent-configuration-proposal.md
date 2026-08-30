# Agent configuration proposal

`agent-core::agent_configuration_proposal` cria um diff typed e proposal-only de configuração de agente. Preserva a versão ativa, classifica a precedence e gera fingerprint determinístico.

Alterações de system/security instruction são imutáveis. Deltas de capability, autonomia ou budget exigem aprovação explícita; sem ela são bloqueados. O artefato não ativa runtime, conecta provider, grava configuração ativa ou acessa credenciais.
