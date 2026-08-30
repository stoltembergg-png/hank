# Plugin manifest

`plugin-core` define o manifest canônico e bounded que precede loader/discovery. Ele vincula plugin ID, semantic version, API revision, entrypoint, capabilities, OS, dependências, isolamento e provenance.

Capabilities não são herdadas. Manifestos unsigned ou sem provenance ficam `Untrusted`; capabilities overbroad, valores secret-like, versões inválidas e ciclos de dependência são rejeitados. O digest é determinístico. Esta camada não instala, carrega ou ativa plugins.
