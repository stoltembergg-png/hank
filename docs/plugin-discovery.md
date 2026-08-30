# Plugin discovery

`plugin-core` expõe discovery somente para fontes já declaradas dentro de uma raiz allowlist. O resultado é um catálogo determinístico de entradas `Staged`; nenhum código é carregado ou executado, e toda entrada permanece com execução desabilitada.

A descoberta rejeita fontes fora da raiz, entradas acima do limite, IDs duplicados e revisões de API não suportadas. Digest, provenance e trust state continuam vinculados ao manifest canônico; instalação, rede, loader e ativação pertencem a etapas posteriores.
