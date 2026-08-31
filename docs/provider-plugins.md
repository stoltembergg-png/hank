# Provider plugins

`provider-core` expõe `ProviderPluginAdapter` como boundary provider-neutral sobre `ModelProvider`. O adapter preserva IDs, versão, capabilities, complete, stream, health e list-models normalizados, mas nega todas as operações enquanto o plugin não estiver aprovado.

O contrato não implementa SDK, rede, processo, armazenamento de credenciais ou provider específico. Credenciais entram somente como `CredentialRef` opaco; lifecycle e Permission Engine permanecem decisões externas.
