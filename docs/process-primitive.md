# Process primitive

`ProcessSpec` representa programa, argv, cwd, ambiente explícito, allowlist, roots autorizadas, permission, timeout, limite de output e trace. O primitive chama `Command` diretamente com `env_clear`; shell names (`sh`, `bash`, `zsh`, `fish`, `cmd`, PowerShell) são rejeitados.

O cwd precisa estar em uma root autorizada. O processo recebe stdin nulo, stdout/stderr piped, timeout bounded e cancelamento por `AtomicBool`. Timeout/cancelamento mata o filho e aguarda sua saída. Output é limitado e linhas com marcadores de secret/token/password/api_key são substituídas por `[redacted]`.

PTY, terminal interativo, sudo, instalação e shell livre ficam fora deste card.
