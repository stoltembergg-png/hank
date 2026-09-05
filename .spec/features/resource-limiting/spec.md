# Spec: resource limiting

> feature: resource-limiting
> status: em-implementacao

### US-2010 — Reservar recursos bounded antes do trabalho

Como runtime, quero reservar capacidades tipadas por escopo antes do trabalho e liberá-las em
terminal/falha/timeout, para que CPU, memória, disco, handles, fila e subprocessos não ultrapassem
limites sem criar bypass por identidade ou projeto.

#### AC-2011 — Quotas e demands são bounded e fail-closed

- **Dado** uma quota ou demand zero, acima dos máximos ou sem dimensão positiva
- **Quando** ela é criada
- **Então** a criação falha sem registrar estado.

- **Dado** um timeout maior que o máximo
- **Quando** uma reserva é solicitada
- **Então** a reserva falha sem mutação.

#### AC-2012 — Reserva multidimensional atômica

- **Dado** project, node e global registrados com capacidade suficiente
- **Quando** uma reserva é solicitada para os três scopes
- **Então** uma única reservation id atualiza os três scopes e retorna receipt bounded.

- **Dado** que um scope não tem uma dimensão suficiente
- **Quando** a reserva é solicitada
- **Então** nenhum scope é alterado.

#### AC-2013 — Release e recuperação de timeout

- **Dado** uma reserva ativa
- **Quando** release é chamado uma vez
- **Então** todas as dimensões voltam ao uso anterior; segundo release falha explicitamente.

- **Dado** uma reserva expirada
- **Quando** o relógio monotônico é avançado e reap é chamado
- **Então** reap libera o uso e a reserva não pode ser liberada novamente.

#### AC-2014 — Isolamento por scope

- **Dado** project A e project B com quotas independentes
- **Quando** A satura sua quota
- **Então** B ainda pode reservar; nenhum payload altera a chave tipada do scope.

#### AC-2015 — Relógio e ledger bounded

- **Dado** um timestamp menor que o anterior ou capacidade de scopes/reservas esgotada
- **Quando** uma operação é feita
- **Então** ela falha closed sem aceitar a operação parcialmente.

## Fora de escopo

- medição real de CPU/memória/disco, kill ou quarantine de processos;
- persistência, serviço distribuído, sandbox do sistema operacional e integração de provider;
- claims de performance ou enforcement no host sem fixture executada.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
