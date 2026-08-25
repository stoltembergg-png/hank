# Spec: workflow crash recovery

> feature: workflow-crash-recovery
> status: em-implementacao

### US-1050 — Recuperar runs expirados sem repetir efeitos desconhecidos

Como runtime, quero detectar leases expirados, fencing por epoch e classificar nodes após restart
para retomar apenas trabalho pending/replay-safe e pausar efeitos potencialmente desconhecidos.

#### AC-1051 — Lease e epoch impedem split-brain

- **Dado** um run com lease ativo
- **Quando** outro runner tenta assumir antes da expiração ou o runner antigo usa seu epoch
- **Então** a tomada é rejeitada até expirar e o runner antigo não passa no fence.

#### AC-1052 — Scanner bounded classifica recovery

- **Dado** node running com lease expirado
- **Quando** o scanner recupera o run com orçamento finito
- **Então** o node vira `unknown`/quarantine e exige reconciliação, sem executar capability.

#### AC-1053 — Relatório é determinístico e fail-closed

- **Dado** leases, generation e estados persistidos
- **Quando** a recuperação é executada
- **Então** o relatório é ordenado, redigido e limita candidatos; corrupção/identidade divergente não promove estado.

## Suposições

- ASM-1054: decisão humana de reconcile e execução posterior pertencem a camadas futuras; recovery apenas classifica e pausa.

## Perguntas em aberto

Nenhuma.
