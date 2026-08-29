# Desktop notifications

A feature de notificações desktop combina uma política pura e bounded no
`agent-runtime` com um adapter OS isolado no Tauri. A policy não chama APIs OS
diretamente: ela aceita somente sinais com escopo de projeto/execução, produz
severidade explícita, redige título/corpo não confiáveis, limita a quantidade
por janela e constrói apenas destinos `hank://runs/<project>/<run>` validados.

A deduplicação usa a tupla `(project_id, run_id, event_id)`. Assim, o mesmo
identificador de evento pode aparecer em projetos ou execuções diferentes sem
ser suprimido por engano; somente a mesma tripla é considerada duplicata.

## Integração Tauri 2

A entrega OS usa o plugin oficial `tauri-plugin-notification` **2.3.3**, do
upstream `tauri-apps/plugins-workspace`, com licença MIT OR Apache-2.0. O plugin
é registrado somente no bootstrap Tauri e o `NotificationWorker` é criado com
`TauriNotificationSink` durante o setup da aplicação.

O runtime não importa Tauri. O adapter converte os estados do plugin para a
boundary interna `PermissionState`, implementa `NotificationSink` e expõe o
resultado `DeliveryOutcome`. `Granted` permite a entrega; `Denied`, `Prompt`,
`PromptWithRationale`, erro de consulta e permissão indisponível seguem o
fallback controlado. Erros de entrega, mutex envenenado e falhas de permissão
são registrados sem título/corpo bruto e não interrompem o scheduler.

A capability da janela principal concede somente:

- `notification:allow-is-permission-granted`;
- `notification:allow-permission-state`;
- `notification:allow-request-permission`;
- `notification:allow-notify`.

O conjunto `notification:default` não é usado. O `deep_link` permanece na
decisão interna; como a API desktop usada neste slice não fornece a navegação
de clique necessária para o contrato, nenhum link arbitrário é enviado ao OS.

## Verificação

O workflow `.github/workflows/onp-sdd-evidence.yml` executa
`verify desktop-notifications` com o caminho Tauri habilitado e publica os
JSONs de verificação junto com os artefatos de evidência. A prova nativa é
complementada pelo gate de build desktop e pelo E2E suportado no CI.

Comandos locais executados neste slice:

```text
node --test test/desktop-notifications-bootstrap-contract.test.mjs
HANK_SKIP_TAURI=1 node tools/run-feature-tests.mjs desktop-notifications
cargo test --package agent-runtime --test notifications_contract --locked
cargo fmt --all -- --check
git diff --check
```

O host local pode não possuir WebKitGTK/Tauri; nesse caso, o resultado nativo
local é `NO_PROOF`, não PASS. Os contracts Rust atuais são executados pelo
runner com `exitcode`, portanto o audit pode classificá-los como
`PROVA_FRACA` por falta de granularidade por teste. Isso não substitui a prova
remota: a validação autoritativa do adapter e do ciclo desktop permanece nos
required checks do CI do commit exato.

## Segurança e não-escopo

- Não renderizar prompt, payload bruto ou credencial.
- Não auto-aprovar ações.
- Não aceitar URL externa, path local ou parâmetro desconhecido.
- Não bloquear scheduler por falha de entrega.
- Não alegar permissão OS concedida fora do adapter real.
- Não introduzir API Tauri no `agent-runtime`.
