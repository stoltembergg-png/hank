# Workflow run viewer

`RunViewerModel` é uma projeção read-only bounded para um project/run:

- aceita somente o project configurado;
- fixa o `run_id` após o primeiro snapshot;
- rejeita generation/sequence stale;
- limita nodes e eventos;
- ordena timeline por sequence e timestamp;
- rejeita sequences duplicadas;
- mantém estados `pending`, `running`, `paused`, `unknown`, `recovered`, `completed` e `failed` explícitos;
- redige URLs, tokens, passwords, paths e page content antes da projeção;
- não expõe mutações (`canMutate = false`);
- não executa cancel/resume/reconcile sem Application API autorizada.

O componente React usa landmarks e roles acessíveis (`status`, listas de nodes e timeline).
