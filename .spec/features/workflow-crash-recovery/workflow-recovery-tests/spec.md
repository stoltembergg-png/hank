# Spec: workflow recovery tests

> feature: workflow-recovery-tests
> status: implementada

## Histórias

### US-2400 — Recuperar workflows sem efeitos duplicados

Como mantenedor, quero uma matriz determinística de recuperação para classificar
leases expirados, fencing e efeitos desconhecidos sem executar capabilities.

#### AC-2401 — Lease fencing
- **Dado** um workflow com lease vigente
- **Quando** outro runner tentar fencing
- **Então** o store rejeita o lease e não autoriza o runner antigo após expiração.

#### AC-2402 — Recuperação bounded
- **Dado** runs com lease expirado
- **Quando** a recuperação executar com limite válido
- **Então** processa no máximo o limite, incrementa generation e emite candidatos bounded.

#### AC-2403 — Efeito desconhecido fail-closed
- **Dado** um node running durante recuperação
- **Quando** o lease for recuperado
- **Então** classifica `unknown`, exige reconciliação e não marca execução de capability.

#### AC-2404 — Idempotência da recuperação
- **Dado** o mesmo run recuperado duas vezes
- **Quando** a segunda recuperação executar
- **Então** o novo lease não duplica o candidato enquanto o lease permanecer vigente.

#### AC-2405 — Entradas inválidas
- **Dado** identidade, TTL ou limite inválido
- **Quando** o store receber a entrada
- **Então** retorna erro tipado sem alterar o banco.

## Fora de escopo
- Providers reais, processos mortos, produção, backup ou zero data loss.
- Execução de capabilities durante recovery.

## Suposições
Nenhuma.

## Perguntas em aberto
Nenhuma.

## DoD
- Contratos Rust exercitam AC-2401..AC-2405.
- Runner TAP executa o conjunto canônico e preserva identidade SHA/tree.
- Verify ONP e workflow dedicado passam.
