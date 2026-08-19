# Blockers

**Estado atual:** nenhum blocker global ativo para a baseline M0.

A closure final está registrada em [`global-blocker-closure.md`](global-blocker-closure.md).

## Resolved

- `ONP TOOLCHAIN`: `VERIFIED` via snapshot versionado, manifest SHA-256 e entrypoint clean-room.
- `VERIFY_OBSOLETO`: resolvido por artifact ONP no SHA final `34525d2`.
- `AC_SEM_PROVA`: AC-101..AC-105 comprovados no workflow Tauri/ONP do SHA final.
- `Actionlint`: `VERIFIED PASS` no workflow Quality integrity.
- `CodeQL`: Rust e JavaScript/TypeScript `PASS`, sem alerts abertos no fechamento.
- `Tauri`: evidência remota Ubuntu 24.04 `PASS`; host local classificado `UNSUPPORTED`, não blocker global.
- `GitHub enforcement`: required contexts reais configurados e verificados via API.
- `PR-001..PR-004`: formalmente merged; `main` final `34525d2`.

## Histórico

Falhas transitórias de mirror/timeout do apt ocorreram durante o bootstrap ONP.
Foram tratadas com retry/timeout determinístico e reexecução; não houve skip de
testes nem promoção artificial de evidência.
