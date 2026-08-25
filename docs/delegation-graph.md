# Delegation graph view contract

`DelegationGraphStore` é uma projeção read-only de eventos do InvocationGraph.
Aceita somente project/session atuais, mantém IDs estáveis, status, depth, round,
budget e denial reason, e gera edges somente quando o parent já é conhecido.

Nodes e edges são bounded e deterministicamente ordenados. Duplicates,
foreign scope, parent desconhecido e labels acima do limite são rejeitados.
Labels devem ser escaped no renderer. `cancel` permanece deliberadamente sem
mutação: ações futuras devem passar por API governada.
