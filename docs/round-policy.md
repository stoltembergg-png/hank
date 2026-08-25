# Round policy

`RoundPolicy` mantém project/group/session/moderator identity e controla rounds,
turns, no-progress e terminal reason.

O segundo turn consecutivo sem progresso encerra a policy. Budget/error/cancel
podem encerrar explicitamente. Turn IDs são deduplicados para que retries não
incrementem counters. O estado não interpreta prompts nem agenda execução.
