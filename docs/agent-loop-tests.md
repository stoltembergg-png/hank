# Agent loop test contract

PR-264 validates a deterministic, bounded synthetic loop. It covers provider-to-tool
turn progression, permission denial, idempotent tool replay, delegation cycle/depth,
budget, cancellation, stale events, invalid policy, finite turns, and trace digest.
No live provider, model output, network, shell, filesystem effect, credential, or
production availability claim is included.
