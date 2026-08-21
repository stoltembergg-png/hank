# HTTP tool

`execute_http` exige `PermissionDecision::Allowed`, URL `http`/`https`, host no allowlist, timeout/limite válidos e headers não sensíveis. `localhost`, loopback, link-local, unspecified, private IPv4 e unique-local IPv6 são bloqueados por padrão.

O client usa `reqwest` blocking com `rustls-tls`, timeout bounded e redirects desabilitados. A resposta retorna status, body UTF-8 limitado, flag de truncamento e trace ID. Authorization, Cookie, token e secret headers são rejeitados; credenciais não são recebidas ou persistidas pelo contrato.
